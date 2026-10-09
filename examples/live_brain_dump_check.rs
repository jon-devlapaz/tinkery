use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tinkery::shaping::{
    brain_dump::{self, BrainDump},
    drafting::PiHost,
};
const DUMP: &str = "I keep losing track of which Tinkery checks used a real model and which just used a stub.\nI want a small receipt in the PR thread so the next person can see what's actually been checked before they dogfood it.\nBut I don't want to fill another form with every tiny edit.\nI also want to be able to say 'that question wasn't useful' and not get it again.";
fn capture(app: &mut BrainDump, dir: &Path, name: &str) {
    let paper = app.paper();
    std::fs::write(dir.join(format!("{name}-state.json")),serde_json::to_string_pretty(&serde_json::json!({"sources":app.sources,"settled":app.settled,"update":app.update,"notice":app.notice})).unwrap()+"\n").unwrap();
    std::fs::write(
        dir.join(format!("{name}-paper.json")),
        serde_json::to_string_pretty(&serde_json::json!({"paper":paper})).unwrap() + "\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.md")),
        paper
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_owned()
            + "\n",
    )
    .unwrap();
    if let Some(g) = &app.guess {
        std::fs::write(
            dir.join(format!("{name}-guess.json")),
            serde_json::to_string_pretty(g).unwrap() + "\n",
        )
        .unwrap();
        let mut anchors = vec![];
        for (reading, framing) in g.framings.iter().enumerate() {
            for anchor in &framing.supports {
                let range = anchor
                    .range(&app.sources)
                    .expect("Accepted anchor must match original");
                anchors.push(serde_json::json!({"role":"support","reading":reading+1,"anchor":anchor,"start":range.start,"end":range.end}));
            }
        }
        for anchor in &g.misfits {
            let range = anchor
                .range(&app.sources)
                .expect("Accepted anchor must match original");
            anchors.push(serde_json::json!({"role":"unresolved","anchor":anchor,"start":range.start,"end":range.end}));
        }
        std::fs::write(
            dir.join(format!("{name}-anchors.json")),
            serde_json::to_string_pretty(&anchors).unwrap() + "\n",
        )
        .unwrap();
    }
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let s = brain_dump::snapshot(w, h, app, false).unwrap();
        std::fs::write(
            dir.join(format!("{name}-{w}x{h}.json")),
            serde_json::to_string_pretty(
                &serde_json::json!({"width":w,"height":h,"lines":s.lines().collect::<Vec<_>>()}),
            )
            .unwrap()
                + "\n",
        )
        .unwrap();
    }
}
fn settle(app: &mut BrainDump) {
    let stop = Instant::now() + Duration::from_secs(200);
    while app.running() && Instant::now() < stop {
        app.tick();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!app.running());
}
fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("New evidence directory"));
    assert!(!dir.exists(), "Refuse overwrite");
    std::fs::create_dir_all(&dir).unwrap();
    let model = std::env::var("TINKERY_LIVE_MODEL").expect("Explicit model");
    let skill = std::env::var("TINKERY_SEED_ME").expect("Actual skill path");
    let program = std::env::var_os("TINKERY_LIVE_PI_COMMAND")
        .map(PathBuf::from)
        .unwrap_or_else(|| "pi".into());
    let host = PiHost::new(program, model.clone(), skill.into())
        .unwrap()
        .with_thinking("low")
        .unwrap();
    let mut app = BrainDump::with_host(Arc::new(host));
    capture(&mut app, &dir, "00-entry");
    assert!(app.guess.is_none());
    let dark = std::env::var_os("TINKERY_LIVE_DARK_MODE").is_some();
    let supplied = std::env::var_os("TINKERY_LIVE_DUMP_FILE");
    let dump = if let Some(path) = &supplied {
        std::fs::read_to_string(path).expect("Exact dump file")
    } else if dark {
        "Add a dark mode toggle to my blog. Reading at night is uncomfortable; the readers on hamster need the same relief.".to_owned()
    } else {
        DUMP.to_owned()
    };
    app.paste(&dump);
    capture(&mut app, &dir, "00-typing");
    assert!(app.guess.is_none());
    app.submit();
    settle(&mut app);
    capture(&mut app, &dir, "01-first");
    assert!(app.guess.is_some(), "{}", app.notice);
    let layout = app.layout();
    let first = app.focused_question().map(|q| q.text.clone());
    println!("FIRST QUESTION: {first:?}");
    if std::env::var_os("TINKERY_LIVE_FIRST_ONLY").is_some() {
        assert_eq!(app.sources.len(), 1);
        assert_eq!(app.sources[0].text, dump);
        assert!(app.fragments.is_empty(), "Automatic extraction occurred");
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('o'),
            crossterm::event::KeyModifiers::CONTROL,
        ));
        capture(&mut app, &dir, "02-originals");
        println!(
            "PASS: exact supplied intact dump only; no answer submitted. Semantic judgment still required."
        );
        return;
    }
    let answer = if let Some(path) = std::env::var_os("TINKERY_LIVE_ANSWER_FILE") {
        std::fs::read_to_string(path).expect("Exact answer file")
    } else if dark {
        "Following the system setting is fine; Hamster is the RSS reader.".to_owned()
    } else {
        "The distinction that matters is real-model evidence, stub mechanics, and the operator's own recognition. Put it in the PR thread; reusing links is fine rather than creating a new receipt format.".to_owned()
    };
    app.paste(&answer);
    app.submit();
    settle(&mut app);
    capture(&mut app, &dir, "02-answer");
    assert!(app.notice.starts_with("Reshaped"), "{}", app.notice);
    assert_eq!(&app.layout()[..layout.len()], layout.as_slice());
    println!(
        "NEXT QUESTION: {:?}",
        app.focused_question().map(|q| &q.text)
    );
    assert_eq!(app.sources[0].text, dump);
    assert_eq!(app.sources[1].text, answer);
    if let Some(path) = std::env::var_os("TINKERY_LIVE_SECOND_ANSWER_FILE") {
        let path = PathBuf::from(path);
        println!(
            "Waiting for explicitly test-authored second-answer fixture: {}",
            path.display()
        );
        let deadline = Instant::now() + Duration::from_secs(300);
        while !path.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        let second =
            std::fs::read_to_string(&path).expect("Second-answer fixture within bounded window");
        let second_question = app
            .focused_question()
            .expect("A second question must exist for this journey")
            .id
            .clone();
        app.paste(&second);
        app.submit();
        settle(&mut app);
        capture(&mut app, &dir, "03-second-answer");
        assert!(app.notice.starts_with("Reshaped"), "{}", app.notice);
        assert_eq!(app.sources[2].text, second);
        assert_eq!(
            app.sources[2].in_reply_to.as_deref(),
            Some(second_question.as_str())
        );
        assert!(
            app.focused_question()
                .is_none_or(|q| q.id != second_question)
        );
        println!(
            "AFTER SECOND ANSWER: {:?}",
            app.focused_question().map(|q| &q.text)
        );
        assert_eq!(app.sources[0].text, dump);
        assert_eq!(app.sources[1].text, answer);
        println!(
            "PASS: exact dump + operator first answer + test-authored second answer; human recognition still pending."
        );
        return;
    }
    if supplied.is_some() {
        println!("Captured exact supplied dump/answer; semantic judgment still required.");
        return;
    }
    if dark {
        assert_eq!(app.sources[0].text, dump);
        println!(
            "PASS: dark-mode regression captured; judge mechanism/media and alternatives manually"
        );
        return;
    }
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('n'),
        crossterm::event::KeyModifiers::CONTROL,
    ));
    app.paste("Sometimes I just want a quiet scratch space to think before the PR exists; don't turn everything I type into a handover checklist.");
    app.submit();
    settle(&mut app);
    capture(&mut app, &dir, "03-added-dump");
    assert!(app.notice.starts_with("Reshaped"), "{}", app.notice);
    assert_eq!(&app.layout()[..layout.len()], layout.as_slice());
    for f in &app.fragments {
        assert_eq!(&app.sources[f.source - 1].text[f.start..f.end], f.text);
    }
    assert_eq!(app.sources[0].text, DUMP);
    println!(
        "FINAL QUESTION: {:?}",
        app.focused_question().map(|q| &q.text)
    );
    println!(
        "PASS: actual {model}; source fidelity, submitted reshapes, and positions; NOT operator recognition"
    );
}
