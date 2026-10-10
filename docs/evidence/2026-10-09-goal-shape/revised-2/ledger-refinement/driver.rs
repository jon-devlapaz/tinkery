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


struct LastAnswer {pi:Arc<PiHost>, first:String, second:String, native:std::sync::atomic::AtomicBool}
impl BoardHost for LastAnswer {
    fn reshape(&self,r:brain_dump::BoardRequest,c:&std::sync::atomic::AtomicBool)->Result<brain_dump::Guess,String> {
        if r.sources.len()<=2 {brain_dump::Guess::decode(if r.sources.len()==1{&self.first}else{&self.second},&r,&mut vec![])}
        else {self.native.store(true,std::sync::atomic::Ordering::SeqCst);self.pi.reshape(r,c)}
    }
    fn has_advisory(&self)->bool{self.native.load(std::sync::atomic::Ordering::SeqCst)}
    fn advisory(&self,r:brain_dump::BoardRequest,g:brain_dump::Guess,c:&std::sync::atomic::AtomicBool)->Decision{self.pi.advisory(r,g,c)}
}

fn settle(app:&mut BrainDump){let start=Instant::now();while app.running()||app.advisory_running(){app.tick();assert!(start.elapsed()<Duration::from_secs(200));std::thread::sleep(Duration::from_millis(2));}}
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let dir=PathBuf::from(&args[1]);std::fs::create_dir_all(&dir).unwrap();
 let first:Value=serde_json::from_slice(&std::fs::read(dir.parent().unwrap().join("ledger/00-first.json")).unwrap()).unwrap();
 let second:Value=serde_json::from_slice(&std::fs::read(dir.parent().unwrap().join("ledger-followup/01-answer.json")).unwrap()).unwrap();
 let raw=|v:&Value|v["decisions_cumulative"].as_array().unwrap().iter().find(|d|d["mode"]=="blocking").unwrap()["raw"].as_str().unwrap().to_owned();
 let host=Arc::new(PiHost::new(PathBuf::from(std::env::var("TINKERY_C16_PI").unwrap()),"openai-codex/gpt-5.6-luna".into(),PathBuf::from(&args[2])).unwrap().with_thinking("low").unwrap());
 let checkpoint=Arc::new(LastAnswer{pi:host.clone(),first:raw(&first),second:raw(&second),native:std::sync::atomic::AtomicBool::new(false)});
 assert_eq!(args[3],"--seed-session-root");let session=PathBuf::from(&args[4]);assert!(session.starts_with(std::env::temp_dir())&&!session.exists());std::fs::create_dir(&session).unwrap();
 let mut app=BrainDump::with_host(checkpoint).with_seed_me(PathBuf::from(&args[2]),Some(session.clone()));
 app.paste(first["sources"][0]["text"].as_str().unwrap());app.submit();settle(&mut app);
 app.paste(second["sources"][1]["text"].as_str().unwrap());app.submit();settle(&mut app);
 assert_eq!(serde_json::to_value(&app.guess).unwrap(),second["board"]);
 assert_eq!(serde_json::to_value(&app.sources).unwrap(),second["sources"]);
 std::fs::write(dir.join("checkpoint.json"),serde_json::to_vec_pretty(&serde_json::json!({"native_after_answer_one_board_equal":true,"sources_equal":true,"restoration_calls":0,"method":"ordinary submit/apply cached raw first and first-answer responses; no private-state mutation"})).unwrap()).unwrap();
 app.paste("better than my usual week. no targets");app.submit();settle(&mut app);
 assert_eq!(app.sources[2].in_reply_to.as_deref(),Some("goal-refinement-3"));
 capture(&mut app,&host,&dir,"02-answer",json!({"input":"better than my usual week. no targets","key":"F2"}),Instant::now(),true);
 app.handle_key(KeyEvent::new(KeyCode::F(8),KeyModifiers::NONE));capture(&mut app,&host,&dir,"03-skip",json!({"key":"F8"}),Instant::now(),false);
 std::fs::write(dir.join("verification.json"),serde_json::to_vec_pretty(&json!({"test_seed_root":session,"empty":std::fs::read_dir(&session).unwrap().next().is_none()})).unwrap()).unwrap();
 assert!(app.receipt.is_none());
}
