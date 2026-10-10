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
    let result = json!({"actor":"assistant-driven revision 2 eval; fixed relayed answers, not operator recognition or affirmation","snapshot":name,"ready_to_review":app.goal_ready(),"open_parts":app.open_parts(),"action":action,"reading_text":reading,"board":board,"questions_from_board":questions,"focused_question":focused,"skip_action":action.get("kind").is_some_and(|v|v=="F8 skip; no F2/no model request"),"settled":app.settled,"sources":app.sources,"notice":app.notice,"shape_submitted":shaped,"board_available_ms":if shaped && app.guess.is_some(){Some(board_ms)}else{None},"step_total_ms":start.elapsed().as_millis(),"decisions_cumulative":decisions,"goal_confirmed":false,"helper_invocations":0,"seed_written":false});
    save(&dir.join(format!("{name}.json")), &result);
    println!(
        "{name}: reading={} focused={}",
        result["reading_text"], result["focused_question"]
    );
    result
}


struct Cached {responses:Vec<String>}
impl BoardHost for Cached {
 fn reshape(&self,r:brain_dump::BoardRequest,_:&std::sync::atomic::AtomicBool)->Result<brain_dump::Guess,String>{brain_dump::Guess::decode(&self.responses[r.sources.len()-1],&r,&mut vec![])}
}

fn main(){
 let args:Vec<String>=std::env::args().collect();let evidence=PathBuf::from(&args[1]);let root=evidence.join("offline-normalized");std::fs::create_dir_all(&root).unwrap();
 let skill=PathBuf::from(&args[2]);let session=PathBuf::from(&args[3]);assert!(session.starts_with(std::env::temp_dir())&&!session.exists());std::fs::create_dir(&session).unwrap();
 let host=Arc::new(PiHost::new(PathBuf::from("/offline/no-provider"),"test/model".into(),skill.clone()).unwrap());
 for name in ["books","onboarding-emails","trap","ledger","voice","operator-goal","vague-onboarding","mom-scams","multi-topic-ci"] {
    let dir=root.join(name);std::fs::create_dir_all(&dir).unwrap();
    let count=if name=="vague-onboarding"{3}else{1};
    let mut snapshots=vec![];let mut responses=vec![];
    for i in 0..count {let path=if i==0{evidence.join(name).join("00-first.json")}else{evidence.join(name).join(format!("{i:02}-answer.json"))};let old:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
      responses.push(old["decisions_cumulative"].as_array().unwrap().iter().rev().find(|d|d["mode"]=="blocking").unwrap()["raw"].as_str().unwrap().to_owned());snapshots.push(old);
    }
    let mut app=BrainDump::with_host(Arc::new(Cached{responses})).with_seed_me(skill.clone(),Some(session.join(name)));
    for (i,old) in snapshots.iter().enumerate() {
      let start=Instant::now();app.paste(old["sources"][i]["text"].as_str().unwrap());app.submit();
      let shape=true;
      capture(&mut app,&host,&dir,if i==0{"00-first"}else if i==1{"01-answer"}else{"02-answer"},json!({"method":"offline cached native raw response through ordinary submit/apply; zero provider calls","origin":old["action"]}),start,shape);
      assert_eq!(serde_json::to_value(&app.sources).unwrap(),old["sources"]);
    }
    save(&dir.join("provenance.json"),&json!({"provider_calls":0,"original_first_capture":evidence.join(name).join("00-first.json"),"current_code_normalization":true,"no_private_state_mutations":true,"no_helper_or_confirmation":true}));
 }
 save(&root.join("verification.json"),&json!({"provider_calls":0,"test_seed_root":session,"empty":std::fs::read_dir(&session).unwrap().next().is_none()}));
}
