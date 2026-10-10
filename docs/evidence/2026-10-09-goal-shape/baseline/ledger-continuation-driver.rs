use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
use tinkery::shaping::brain_dump::{self, BoardHost, BoardRequest, BrainDump, Guess};
struct Replay {
    responses: Mutex<Vec<Guess>>,
}
impl BoardHost for Replay {
    fn reshape(&self, request: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        let mut queue = self.responses.lock().unwrap();
        assert!(
            !queue.is_empty(),
            "Unexpected model request in remaining local actions"
        );
        let response = queue.remove(0);
        response.validate(&request)?;
        Ok(response)
    }
}
fn settle(app: &mut BrainDump) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.running() && Instant::now() < deadline {
        app.tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!app.running());
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 4);
    assert_eq!(args[2], "--seed-session-root");
    let root = PathBuf::from(&args[0]);
    let skill = PathBuf::from(&args[1]);
    let seed_root = PathBuf::from(&args[3]);
    assert!(!seed_root.exists() && seed_root.starts_with(std::env::temp_dir()));
    std::fs::create_dir(&seed_root).unwrap();
    let read = |name: &str| {
        serde_json::from_slice::<Value>(&std::fs::read(root.join(format!("{name}.json"))).unwrap())
            .unwrap()
    };
    let initial = read("00-first");
    let checkpoint = read("01-answer");
    let guess = |record: &Value| {
        serde_json::from_value::<Guess>(
            record["decisions_cumulative"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|d| d["mode"] == "blocking")
                .unwrap()["candidate"]
                .clone(),
        )
        .unwrap()
    };
    let host = Arc::new(Replay {
        responses: Mutex::new(vec![guess(&initial), guess(&checkpoint)]),
    });
    let mut app = BrainDump::with_host(host.clone()).with_seed_me(skill, Some(seed_root.clone()));
    app.paste(initial["sources"][0]["text"].as_str().unwrap());
    app.submit();
    settle(&mut app);
    app.paste("a failed job should override everything. cost only matters if its way off");
    app.submit();
    settle(&mut app);
    assert_eq!(
        serde_json::to_value(&app.guess).unwrap(),
        checkpoint["board"]
    );
    assert_eq!(
        serde_json::to_value(&app.sources).unwrap(),
        checkpoint["sources"]
    );
    assert!(app.focused_question().is_none());
    let mut events = vec![];
    for (index, text) in [(2, "better than my usual week. no targets"), (3, "skip")] {
        let before = app
            .focused_question()
            .map(|q| json!({"id":q.id,"text":q.text}));
        if index == 2 {
            app.paste(text);
            app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        } else {
            app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
        }
        assert!(!app.running() && !app.advisory_running());
        assert!(app.receipt.is_none());
        let name = format!("{index:02}-local-continuation");
        for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            std::fs::write(
                root.join(format!("{name}-{w}x{h}.txt")),
                brain_dump::snapshot(w, h, &mut app, false).unwrap(),
            )
            .unwrap();
        }
        events.push(json!({"round":index,"fixed_action":text,"question_before":before,"question_after":app.focused_question().map(|q|json!({"id":q.id,"text":q.text})),"sources":app.sources,"board":app.guess,"notice":app.notice,"provider_calls":0}));
    }
    assert!(host.responses.lock().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(&seed_root).unwrap().count(), 0);
    let value = json!({"method":"Offline restore of exact captured live round-1 board and sources through current-main submit/apply pipeline, then remaining actual F2/F8 actions; no substitute/new model response","checkpoint_equal":true,"additional_provider_calls":0,"helper_invocations":0,"goal_confirmed":false,"seed_session_root":seed_root,"session_root_empty":true,"first_empty_question_round":1,"final_empty_question_round":3,"second_answer_classification":"added words, not an answer; scope subsequently skipped; not included in the reading","events":events});
    std::fs::write(
        root.join("fixed-rounds-continuation.json"),
        serde_json::to_string_pretty(&value).unwrap() + "\n",
    )
    .unwrap();
    println!("{value}");
}
