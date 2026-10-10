use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tinkery::shaping::{
    brain_dump::{self, BoardHost, BrainDump, checks::Decision},
    drafting::PiHost,
};
fn save(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
}
fn capture(
    app: &mut BrainDump,
    host: &PiHost,
    dir: &Path,
    name: &str,
    action: Value,
    start: Instant,
    shaped: bool,
) -> Value {
    let deadline = start + Duration::from_secs(200);
    while app.running() && Instant::now() < deadline {
        app.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !app.running(),
        "Shaping did not finish within baseline deadline"
    );
    let board_ms = start.elapsed().as_millis();
    let board = serde_json::to_value(&app.guess).unwrap();
    let paper = app.paper();
    let reading = paper
        .split_once("\n## I think this is about… / guess\n\n")
        .and_then(|(_, s)| s.split_once("\n\nSupporting words:\n\n"))
        .map(|(s, _)| s)
        .unwrap_or("")
        .to_owned();
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        std::fs::write(
            dir.join(format!("{name}-{w}x{h}.txt")),
            brain_dump::snapshot(w, h, app, false).unwrap(),
        )
        .unwrap();
    }
    std::fs::write(dir.join(format!("{name}-paper.md")), paper).unwrap();
    while app.advisory_running() && Instant::now() < deadline {
        app.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !app.advisory_running(),
        "Advisory did not finish within baseline deadline"
    );
    assert_eq!(
        board,
        serde_json::to_value(&app.guess).unwrap(),
        "Audit mutated board"
    );
    assert!(app.receipt.is_none(), "Goal confirmed unexpectedly");
    let focused = app
        .focused_question()
        .map(|q| json!({"id":q.id,"text":q.text}));
    let questions = app
        .guess
        .as_ref()
        .map(|b| {
            b.questions
                .iter()
                .map(|q| json!({"id":q.id,"text":q.text}))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    app.review_goal();
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for page in 0..12 {
            std::fs::write(
                dir.join(format!("{name}-review-{w}x{h}-{page:02}.txt")),
                brain_dump::snapshot(w, h, app, false).unwrap(),
            )
            .unwrap();
            app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.review_goal();
    }
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let decisions = host
        .diagnostics()
        .iter()
        .map(|s| serde_json::from_str::<Decision>(s).unwrap())
        .collect::<Vec<_>>();
    let result = json!({"actor":"assistant-driven revised eval; fixed relayed answers, not operator recognition or affirmation","snapshot":name,"ready_to_review":app.goal_ready(),"open_parts":app.open_parts(),"action":action,"reading_text":reading,"board":board,"questions_from_board":questions,"focused_question":focused,"skip_action":action.get("kind").is_some_and(|v|v=="F8 skip; no F2/no model request"),"settled":app.settled,"sources":app.sources,"notice":app.notice,"shape_submitted":shaped,"board_available_ms":if shaped && app.guess.is_some(){Some(board_ms)}else{None},"step_total_ms":start.elapsed().as_millis(),"decisions_cumulative":decisions,"goal_confirmed":false,"helper_invocations":0,"seed_written":false});
    save(&dir.join(format!("{name}.json")), &result);
    println!(
        "{name}: reading={} focused={}",
        result["reading_text"], result["focused_question"]
    );
    result
}

struct Checkpoint {
    host: Arc<PiHost>,
    raw: String,
    first: std::sync::atomic::AtomicBool,
    live: std::sync::atomic::AtomicBool,
}
impl BoardHost for Checkpoint {
    fn reshape(
        &self,
        r: brain_dump::BoardRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<brain_dump::Guess, String> {
        if self.first.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.live.store(false, std::sync::atomic::Ordering::SeqCst);
            brain_dump::Guess::decode(&self.raw, &r, &mut vec![])
        } else {
            self.live.store(true, std::sync::atomic::Ordering::SeqCst);
            self.host.reshape(r, cancelled)
        }
    }
    fn has_advisory(&self) -> bool {
        self.live.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn advisory(
        &self,
        r: brain_dump::BoardRequest,
        g: brain_dump::Guess,
        c: &std::sync::atomic::AtomicBool,
    ) -> Decision {
        self.host.advisory(r, g, c)
    }
    fn diagnostics(&self) -> Vec<String> {
        self.host.diagnostics()
    }
}
fn main() {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(
        a.len(),
        5,
        "OUTPUT FIRST_CAPTURE SKILL --seed-session-root TEMP"
    );
    assert_eq!(a[3], "--seed-session-root");
    let dir = PathBuf::from(&a[0]);
    std::fs::create_dir_all(&dir).unwrap();
    let old: Value = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let raw = old["decisions_cumulative"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["mode"] == "blocking")
        .unwrap()["raw"]
        .as_str()
        .unwrap()
        .to_owned();
    let host = Arc::new(
        PiHost::new(
            PathBuf::from(std::env::var_os("TINKERY_C16_PI").unwrap()),
            "openai-codex/gpt-5.6-luna".into(),
            PathBuf::from(&a[2]),
        )
        .unwrap()
        .with_thinking("low")
        .unwrap(),
    );
    let checkpoint = Arc::new(Checkpoint {
        host: host.clone(),
        raw,
        first: std::sync::atomic::AtomicBool::new(true),
        live: std::sync::atomic::AtomicBool::new(false),
    });
    let session = PathBuf::from(&a[4]);
    assert!(!session.exists());
    assert!(session.starts_with(std::env::temp_dir()));
    std::fs::create_dir(&session).unwrap();
    let mut app =
        BrainDump::with_host(checkpoint).with_seed_me(PathBuf::from(&a[2]), Some(session.clone()));
    app.paste(old["sources"][0]["text"].as_str().unwrap());
    app.submit();
    let start = Instant::now();
    while app.running() {
        app.tick();
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!app.advisory_running());
    assert_eq!(serde_json::to_value(&app.guess).unwrap(), old["board"]);
    save(
        &dir.join("checkpoint.json"),
        &json!({"kind":"offline first-board restoration through submit/apply; zero provider calls","captured_board_equal":true,"origin_capture":a[1],"sources":app.sources}),
    );
    let mut rounds = vec![];
    for (i, answer) in [
        "a failed job should override everything. cost only matters if its way off",
        "better than my usual week. no targets",
        "skip",
    ]
    .iter()
    .enumerate()
    {
        let start = Instant::now();
        let before = app
            .focused_question()
            .map(|q| json!({"id":q.id,"text":q.text}));
        if i == 2 {
            app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
        } else {
            app.paste(answer);
            app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        }
        let shaped = app.running();
        rounds.push(capture(&mut app,&host,&dir,&format!("{:02}-{}",i+1,if i==2{"skip"}else{"answer"}),json!({"answer":answer,"question_before":before,"kind":if i==2{"F8 skip; no F2/no model request"}else{"fixed answer + explicit F2 from exact offline first-board checkpoint"}}),start,shaped));
    }
    assert_eq!(std::fs::read_dir(&session).unwrap().count(), 0);
    save(
        &dir.join("journey.json"),
        &json!({"rounds":rounds,"seed_session_root":session,"root_empty":true,"provider_budget":4,"helper_invocations":0,"goal_confirmed":false}),
    );
}
