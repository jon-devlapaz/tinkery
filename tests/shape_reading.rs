//! Shape reading limits use public input, rendered text and real TEST ledger files.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tinkery::{
    Palette,
    pi::CallBudget,
    seed_me::ledger::{self, Value},
    shape::{Agent, Shape},
};

const STAMP: &str = "2026-10-25T00:00:00+00:00";

fn fixture(goal: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-large-seed-")
        .tempdir()
        .unwrap();
    let session = ledger::initialize(root.path(), STAMP).unwrap();
    let original = ledger::load(&session).unwrap();
    let mut state = ledger::editable(&original).unwrap().to_json().unwrap();
    state["goal"] = goal.into();
    state["origin"] = "goal".into();
    state["nodes"] = serde_json::json!([{
        "id":"goal", "kind":"decision", "status":"settled", "prerequisites":[], "evidence":[],
        "owner":"TEST", "gate":"TEST", "answer":goal, "authority":"user", "authority_source":"TEST fixture"
    }]);
    let current = ledger::publish(
        &session,
        &Value::from(state),
        &original["version"],
        &Value::from("TEST fixture"),
        None,
        STAMP,
    )
    .unwrap();
    let checks = Value::from(
        serde_json::json!([{"statement":"TEST keeps originals", "kind":"judge", "source":"TEST fixture"}]),
    );
    ledger::set_checks(&session, &checks).unwrap();
    let seed = format!(
        "status: draft\n# Seed: TEST large\nrevision: r1\n## You are confirming\nTEST keeps originals\n\n## Now  (the agent keeps these current)\n{body}\n\n## Goal\nTEST goal\n\n{}\n## Constraints and exclusions\nTEST only\n\n{}## Open questions\nNone\n\n## Knowledge map\nTEST only\n\n## Evidence\nTEST seed final tail 中 e\u{301} 👩‍💻\n",
        ledger::decisions_block(&current).unwrap(),
        ledger::checks_block(&checks).unwrap()
    );
    fs::write(session.join("seed-contract.md"), seed).unwrap();
    (root, session)
}

fn files(session: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(session)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

fn key(shape: &mut Shape, code: KeyCode) {
    shape.key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn screen(shape: &mut Shape, width: u16, height: u16, body_height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            shape.render(
                frame,
                Rect::new(0, 0, width, body_height),
                Palette::new(true),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn opened(session: &Path) -> Shape {
    let agent = Agent::new(
        PathBuf::from("/tinkery-TEST-no-provider"),
        "TEST/fake".into(),
        String::new(),
        Some(session.into()),
        CallBudget::new(0),
        None,
    )
    .unwrap();
    let mut shape = Shape::session(session).with_agent(Some(Arc::new(agent)));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        shape.tick();
        let shown = screen(&mut shape, 80, 24, 20);
        if shown.contains("Seed draft r1") || shown.contains("Too long to show") {
            break;
        }
        assert!(Instant::now() < deadline, "TEST seed never loaded");
        thread::sleep(Duration::from_millis(1));
    }
    shape
}

#[test]
fn oversized_readings_are_refused_and_f3_changes_nothing() {
    for source in ["seed", "review", "so far"] {
        let goal = if source == "so far" {
            "g".repeat(60_000)
        } else {
            "TEST goal".into()
        };
        let body = "x".repeat(if source == "seed" { 60_000 } else { 30_000 });
        let (_root, session) = fixture(&goal, &body);
        if source == "review" {
            let seed = fs::read_to_string(session.join("seed-contract.md")).unwrap();
            let version = ledger::load(&session).unwrap()["version"]
                .to_json()
                .unwrap()
                .as_u64()
                .unwrap();
            let review = ledger::CompletionReview {
                contradictions: vec![],
                decided: vec!["r".repeat(30_000)],
                done: vec![],
                failure_cases: vec![ledger::FailureCase {
                    statement: "TEST keeps originals".into(),
                    case: "TEST changed originals".into(),
                }],
            };
            ledger::record_teach_back(
                &session,
                version,
                "r1",
                &seed,
                &ledger::TeachBackRecord::Review(&review),
                STAMP,
                None,
            )
            .unwrap();
        }
        let before = files(&session);
        let mut shape = opened(&session);
        shape.paste("TEST confirm only a readable seed");
        if source == "so far" {
            key(&mut shape, KeyCode::F(5));
        }
        for _ in 0..4_000 {
            key(&mut shape, KeyCode::PageDown);
        }
        let shown = screen(&mut shape, 80, 24, 20);
        assert!(
            shown.contains("Too long to show. Nothing was confirmed or changed."),
            "{source}: {shown}"
        );
        assert!(
            !shown.contains("End of seed") && !shown.contains("F3 confirm"),
            "{shown}"
        );
        key(&mut shape, KeyCode::F(3));
        assert!(!screen(&mut shape, 80, 24, 20).contains("esc stop"));
        for _ in 0..10 {
            shape.tick();
        }
        assert_eq!(files(&session), before);
        assert!(!session.join("seed-confirmation.json").exists());
    }
}

#[test]
fn a_zero_height_reading_never_authorizes_confirmation() {
    let (_root, session) = fixture("TEST exact goal", &"TEST small seed\n".repeat(100));
    let before = files(&session);
    let mut shape = opened(&session);
    shape.paste("TEST confirm only displayed words");
    screen(&mut shape, 80, 24, 1);
    for _ in 0..1_000 {
        key(&mut shape, KeyCode::PageDown);
    }
    assert!(screen(&mut shape, 80, 24, 1).contains("read to end"));
    key(&mut shape, KeyCode::F(3));
    assert!(!screen(&mut shape, 80, 24, 1).contains("esc stop"));
    for _ in 0..1_000 {
        key(&mut shape, KeyCode::PageUp);
    }
    assert!(screen(&mut shape, 80, 24, 20).contains("Nothing was confirmed"));
    for _ in 0..10 {
        shape.tick();
    }
    assert_eq!(files(&session), before);
    assert!(!session.join("seed-confirmation.json").exists());
}
