//! Shape separates current and running question lines and keeps So far label-only.
#![allow(clippy::disallowed_methods)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tinkery::{
    Palette,
    confirmation::GoalConfirmation,
    pi::CallBudget,
    seed_me::ledger::{self, Value},
    shape::{
        Agent, Open, Shape, answering,
        shaping::{self, Shaped},
    },
};

const STAMP: &str = "2026-11-27T00:00:00+00:00";
const NOTE: &str = "NOT independent: asked after findings were shown.";

fn fixture(ask: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-probe-separator-")
        .tempdir()
        .unwrap();
    let session = ledger::initialize(root.path(), STAMP).unwrap();
    let original = ledger::load(&session).unwrap();
    let mut state = ledger::editable(&original).unwrap().to_json().unwrap();
    state["goal"] = "TEST separate question lines".into();
    state["origin"] = "goal".into();
    state["nodes"] = serde_json::json!([
        {"id":"goal", "kind":"decision", "status":"settled", "prerequisites":[], "evidence":[], "owner":"TEST", "gate":"TEST", "answer":"TEST separate question lines", "authority":"user", "authority_source":"TEST fixture"},
        {"id":answering::PROBE.0, "kind":"decision", "status":"unresolved", "prerequisites":[], "evidence":[], "owner":"User", "gate":"Your own answer to the pre-mortem, or skip it", "label":"Probe: pre-mortem", "question":ask}
    ]);
    state["current_question"] = answering::PROBE.0.into();
    ledger::publish(
        &session,
        &Value::from(state),
        &original["version"],
        &Value::from("TEST fixture"),
        None,
        STAMP,
    )
    .unwrap();
    (root, session)
}

fn screen(shape: &mut Shape, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            shape.render(
                frame,
                Rect::new(0, 0, width, height - 4),
                Palette::new(true),
            )
        })
        .unwrap();
    cells(&terminal)
}

fn cells(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn opened(session: &Path) -> Shape {
    let agent = Agent::new(
        tinkery::pi::Drafting::new(
            tinkery::pi::Harness::Pi,
            PathBuf::from("/tinkery-TEST-no-provider"),
            "TEST/fake".into(),
            String::new(),
            CallBudget::new(1),
        )
        .unwrap(),
        Some(session.into()),
        None,
    );
    let confirmation = GoalConfirmation::Terminal {
        declaration: "TEST invalid declaration".into(),
        action: "TEST render only".into(),
        identity: "TEST no write authority".into(),
    };
    assert!(confirmation.actor().is_err());
    let mut shape = Shape::new(
        Open::Session(session),
        Some(Arc::new(agent)),
        &confirmation,
        false,
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        shape.tick();
        let shown = screen(&mut shape, 100, 30);
        if shown.contains("Probe: pre-mortem") && shown.contains("The step reads files in") {
            return shape;
        }
        assert!(Instant::now() < deadline, "Shape did not load: {shown}");
        thread::sleep(Duration::from_millis(1));
    }
}

fn separated(shown: &str) -> bool {
    let Some(end) = shown.find("reason?").map(|start| start + "reason?".len()) else {
        return false;
    };
    let Some(note) = shown.find(NOTE) else {
        return false;
    };
    note > end && shown[end..note].chars().all(char::is_whitespace)
}

fn so_far(shape: &mut Shape) {
    let help = shape.help_text();
    let digit = help
        .lines()
        .find_map(|line| {
            let (n, words) = line.split_once("  ")?;
            (n.len() == 1 && words.contains("So far")).then(|| n.chars().next().unwrap())
        })
        .unwrap_or_else(|| panic!("So far absent: {help}"));
    assert!(shape.pick(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE)));
}

#[test]
fn pre_mortem_note_stays_separate_in_every_question_view() {
    let ask = format!("{}\n{NOTE}", answering::PROBE.1);
    for (width, height) in [(100, 30), (80, 24)] {
        for view in ["normal", "running"] {
            let (_root, session) = fixture(&ask);
            let before = fs::read(session.join("ledger.json")).unwrap();
            let mut shape = opened(&session);
            if view == "running" {
                shape.paste("TEST answer");
                shape.key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
            }
            let shown = screen(&mut shape, width, height);
            assert!(
                shown.contains("Imagine this shipped and turned out wrong."),
                "{shown}"
            );
            assert!(separated(&shown), "{width}x{height} {view}: {shown}");
            if view == "running" {
                assert!(shown.contains("esc stop"), "{shown}");
            }
            drop(shape);
            assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
            let Shaped::Question { question, .. } = shaping::read(&session).unwrap().stage else {
                panic!("question lost")
            };
            assert_eq!(question.ask.as_bytes(), ask.as_bytes());
        }
    }
}

#[test]
fn separation_predicate_rejects_the_old_single_line_presentation() {
    for (width, height) in [(100, 30), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let old = format!("{}{NOTE}", answering::PROBE.1);
        terminal
            .draw(|frame| {
                frame.render_widget(
                    Paragraph::new(Line::styled(old.clone(), ratatui::style::Style::default()))
                        .wrap(Wrap { trim: false }),
                    Rect::new(0, 0, width.min(76), height),
                )
            })
            .unwrap();
        let shown = cells(&terminal);
        assert!(shown.contains("reason?NOT"), "{shown}");
        assert!(shown.contains(NOTE), "{shown}");
        assert!(!separated(&shown), "{shown}");
    }
}

#[test]
fn questions_without_findings_do_not_gain_an_independence_note() {
    for (width, height) in [(100, 30), (80, 24)] {
        let (_root, session) = fixture(answering::PROBE.1);
        let mut shape = opened(&session);
        for summary in [false, true] {
            if summary {
                so_far(&mut shape);
            }
            let shown = screen(&mut shape, width, height);
            if summary {
                assert!(shown.contains("Probe: pre-mortem"), "{shown}");
                assert!(!shown.contains("Imagine this shipped"), "{shown}");
            } else {
                assert!(shown.contains("reason?"), "{shown}");
            }
            assert!(!shown.contains("NOT independent"), "{shown}");
        }
        let Shaped::Question { question, .. } = shaping::read(&session).unwrap().stage else {
            panic!("question lost")
        };
        assert_eq!(question.ask, answering::PROBE.1);
    }
}

#[test]
fn so_far_lists_the_probe_label_not_its_question_text() {
    let ask = format!("{}\n{NOTE}", answering::PROBE.1);
    for (width, height) in [(100, 30), (80, 24)] {
        let (_root, session) = fixture(&ask);
        let mut shape = opened(&session);
        so_far(&mut shape);
        let shown = screen(&mut shape, width, height);
        assert!(shown.contains("Probe: pre-mortem"), "{shown}");
        assert!(!shown.contains("Imagine this shipped"), "{shown}");
        assert!(!shown.contains("NOT independent"), "{shown}");
    }
}
