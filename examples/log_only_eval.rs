use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tinkery::shaping::{
    brain_dump::{self, BoardHost, BrainDump, checks::Decision},
    drafting::PiHost,
};
fn audit_attempt(args: &[String]) {
    use std::sync::atomic::AtomicBool;
    use tinkery::shaping::brain_dump::{BoardRequest, Guess, Source};
    assert_eq!(
        args.len(),
        4,
        "--audit-attempt OUTPUT SKILL ORIGINAL PRIOR_DECISIONS; consumes one explicitly authorized advisory call only"
    );
    let output = PathBuf::from(&args[0]);
    assert!(!output.exists(), "Refuse overwrite");
    std::fs::create_dir_all(&output).unwrap();
    let source = std::fs::read_to_string(&args[2]).unwrap();
    let evidence: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&args[3]).unwrap()).unwrap();
    let raw = evidence["decisions"][0]["raw"]
        .as_str()
        .expect("Exact earlier shaping response required");
    let request = BoardRequest {
        ask_counts: Default::default(),
        sources: vec![Source {
            id: 1,
            text: source.clone(),
            in_reply_to: None,
        }],
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut normalizations = vec![];
    let replay = Guess::decode(raw, &request, &mut normalizations).map(|_| ());
    let program =
        std::env::var_os("TINKERY_EVAL_PI").expect("Explicit six-call enforcing wrapper required");
    let host = PiHost::new(
        program.into(),
        "openai-codex/gpt-5.6-luna".into(),
        args[1].clone().into(),
    )
    .unwrap()
    .with_default_thinking();
    let start = Instant::now();
    let decision = host
        .advisory_attempt(request, raw, &AtomicBool::new(false))
        .unwrap();
    let data = serde_json::json!({"actor":"assistant-driven TEST; raw rejected attempt only, not operator recognition","model":"openai-codex/gpt-5.6-luna","thinking":"Pi native default; --thinking omitted","shaping_not_rerun":true,"accepted_or_confirmed":false,"boundary_replay_v3":{"decision":if replay.is_ok(){"pass"}else{"reject"},"reason":replay.err(),"normalizations":normalizations},"advisory":decision,"elapsed_ms":start.elapsed().as_millis()});
    std::fs::write(output.join("original.txt"), source).unwrap();
    std::fs::write(output.join("raw-shaping.txt"), raw).unwrap();
    std::fs::write(
        output.join("decisions.json"),
        serde_json::to_string_pretty(&data).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", decision.text());
    println!("{}", data["boundary_replay_v3"]);
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--audit-attempt") {
        audit_attempt(&args[1..]);
        return;
    }
    assert_eq!(
        args.len(),
        5,
        "Usage: log_only_eval OUTPUT SKILL BOOKS ONBOARDING TRAP; explicit six-call operator authorization required"
    );
    let output = PathBuf::from(&args[0]);
    assert!(!output.exists(), "Refuse overwrite");
    std::fs::create_dir_all(&output).unwrap();
    let program =
        std::env::var_os("TINKERY_EVAL_PI").expect("Explicit budget-enforcing Pi wrapper required");
    for (name, path) in ["books", "onboarding-emails", "trap"]
        .into_iter()
        .zip(&args[2..])
    {
        let dump = std::fs::read_to_string(path).unwrap();
        let dir = output.join(name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("original.txt"), &dump).unwrap();
        let host = Arc::new(
            PiHost::new(
                PathBuf::from(&program),
                "openai-codex/gpt-5.6-luna".into(),
                PathBuf::from(&args[1]),
            )
            .unwrap()
            .with_default_thinking(),
        );
        let mut app = BrainDump::with_host(host.clone());
        app.paste(&dump);
        assert_eq!(app.input.text, dump);
        assert!(host.diagnostics().is_empty());
        let start = Instant::now();
        app.submit();
        let deadline = start + Duration::from_secs(200);
        while app.running() && Instant::now() < deadline {
            app.tick();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!app.running());
        let available = start.elapsed().as_millis();
        let first = serde_json::to_value(&app.guess).unwrap();
        std::fs::write(
            dir.join("first-board.json"),
            serde_json::to_string_pretty(&first).unwrap() + "\n",
        )
        .unwrap();
        for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let screen = brain_dump::snapshot(w, h, &mut app, false).unwrap();
            std::fs::write(dir.join(format!("first-{w}x{h}.txt")), screen).unwrap();
        }
        while app.advisory_running() && Instant::now() < deadline {
            app.tick();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!app.advisory_running());
        assert_eq!(
            first,
            serde_json::to_value(&app.guess).unwrap(),
            "Advisory changed the board"
        );
        assert_eq!(app.sources.len(), 1);
        assert_eq!(app.sources[0].text, dump);
        assert!(app.receipt.is_none());
        let decisions = host
            .diagnostics()
            .iter()
            .map(|s| serde_json::from_str::<Decision>(s).unwrap())
            .collect::<Vec<_>>();
        let data = serde_json::json!({"actor":"assistant-driven TEST eval; not operator recognition","model":"openai-codex/gpt-5.6-luna","thinking":"Pi native default; --thinking omitted","mode":"log-only; advisory flags not enabled","first_validated_board_ms":if app.guess.is_some(){Some(available)}else{None},"shaping_result_ms":available,"total_ms":start.elapsed().as_millis(),"status":if app.guess.is_some(){"displayed"}else{"structurally rejected"},"notice":app.notice,"decisions":decisions,"goal_confirmed":false,"seed_written":false,"helper_invocations":0});
        std::fs::write(
            dir.join("decisions.json"),
            serde_json::to_string_pretty(&data).unwrap() + "\n",
        )
        .unwrap();
        println!(
            "{name}: shaping result {available} ms, total {} ms",
            data["total_ms"]
        );
        for d in decisions {
            println!("{}", d.text());
        }
    }
}
