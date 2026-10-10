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

struct BudgetHost(Arc<PiHost>);
impl BoardHost for BudgetHost {
    fn reshape(&self,r:brain_dump::BoardRequest,c:&std::sync::atomic::AtomicBool)->Result<brain_dump::Guess,String>{self.0.reshape(r,c)}
    fn has_advisory(&self)->bool {
        let calls=std::fs::read_to_string(std::env::var("TINKERY_C16_TRACE").unwrap()+"/calls.jsonl").unwrap();
        let audits=calls.lines().map(|l|serde_json::from_str::<Value>(l).unwrap()).filter(|v|v["phase"]=="revised-2"&&v["kind"]=="meaning-preservation").count();
        audits<11 && calls.lines().count()<83
    }
    fn advisory(&self,r:brain_dump::BoardRequest,g:brain_dump::Guess,c:&std::sync::atomic::AtomicBool)->Decision{self.0.advisory(r,g,c)}
    fn diagnostics(&self)->Vec<String>{self.0.diagnostics()}
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        5,
        "OUTPUT INPUTS SKILL --seed-session-root TEMP_PARENT"
    );
    assert_eq!(args[3], "--seed-session-root");
    let output = PathBuf::from(&args[0]);
    let inputs = PathBuf::from(&args[1]);
    let skill = PathBuf::from(&args[2]);
    let session_parent = PathBuf::from(&args[4]);
    assert!(session_parent.is_absolute() && session_parent.starts_with(std::env::temp_dir()));
    assert!(!session_parent.exists());
    std::fs::create_dir(&session_parent).unwrap();
    let wrapper = PathBuf::from(std::env::var_os("TINKERY_C16_PI").unwrap());
    for name in [
        "books",
        "onboarding-emails",
        "trap",
        "ledger",
        "voice",
        "operator-goal",
        "vague-onboarding",
        "mom-scams",
        "multi-topic-ci",
    ] {
        let dir = output.join(name);
        std::fs::create_dir(&dir).unwrap();
        let original = std::fs::read_to_string(inputs.join(format!("{name}.txt"))).unwrap();
        std::fs::write(dir.join("original.txt"), &original).unwrap();
        let session_root = session_parent.join(name);
        std::fs::create_dir(&session_root).unwrap();
        let host = Arc::new(
            PiHost::new(
                wrapper.clone(),
                "openai-codex/gpt-5.6-luna".into(),
                skill.clone(),
            )
            .unwrap()
            .with_thinking("low")
            .unwrap(),
        );
        let mut app = BrainDump::with_host(Arc::new(BudgetHost(host.clone())))
            .with_seed_me(skill.clone(), Some(session_root.clone()));
        app.paste(&original);
        assert_eq!(app.input.text, original);
        let start = Instant::now();
        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        let mut rounds = vec![capture(
            &mut app,
            &host,
            &dir,
            "00-first",
            json!({"kind":"explicit F2","input":original}),
            start,
            true,
        )];
        let mut end = if app.guess.is_some() && app.focused_question().is_none() {
            Some(0)
        } else {
            None
        };
        if name == "ledger" || name == "vague-onboarding" {
            let answers = if name == "ledger" {vec![
                "a failed job should override everything. cost only matters if its way off",
                "better than my usual week. no targets",
                "skip",
            ]}else{vec!["people sign up then never come back. like 70% never do a second session","yes the second session. id know it worked if more than half come back within a week"]};
            for (i, answer) in answers.iter().enumerate() {
                let label = format!("{:02}-{}", i + 1, if i == 2 { "skip" } else { "answer" });
                let focused = app
                    .focused_question()
                    .map(|q| json!({"id":q.id,"text":q.text}));
                if app.guess.is_none() {
                    save(
                        &dir.join(format!("{label}-not-performed.json")),
                        &json!({"answer":answer,"reason":"No usable board/focused question; not converted into an added dump"}),
                    );
                    continue;
                }
                let start = Instant::now();
                if i == 2 {
                    app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
                } else {
                    app.paste(answer);
                    assert_eq!(app.input.text, *answer);
                    app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
                }
                let shaped = app.running();
                rounds.push(capture(&mut app,&host,&dir,&label,json!({"kind":if i==2{"F8 skip; no F2/no model request"}else{"fixed answer + F2; refine ready goal or answer focused question"},"question_before":focused,"answer":answer}),start,shaped));
                if end.is_none() && app.focused_question().is_none() {
                    end = Some(i + 1);
                }
            }
        }
        assert_eq!(app.sources[0].text, original);
        assert!(app.receipt.is_none());
        assert_eq!(
            std::fs::read_dir(&session_root).unwrap().count(),
            0,
            "Seed Me root is not empty"
        );
        save(
            &dir.join("journey.json"),
            &json!({"base_head":"8818e63cc4811fc0935409580589ddbdc0612b7f","branch":"think-goal-shape","runtime":"evaluated-source-hashes.json","ready_to_review":app.goal_ready(),"open_parts":app.open_parts(),"model":"openai-codex/gpt-5.6-luna","thinking":"low; --thinking low verified by wrapper","seed_session_root":session_root,"session_root_empty":true,"helper_invocations":0,"goal_confirmed":false,"rounds_until_no_question":if name=="ledger"||name=="vague-onboarding"{end.map(|v|json!(v)).unwrap_or(json!("never within the prescribed three rounds"))}else{json!("first board only")},"rounds":rounds}),
        );
    }
}
