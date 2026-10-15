use super::*;
use crate::seed_me::shaping::{Shaped, Shaping};
use crossterm::event::KeyModifiers;

const GOAL: &str = "TEST I can shape a goal calmly.\nWhy: I lose the thread between tools.\nDone when: I see the next question.\nMust: Keep my words.\nMust not: Run agents.";

fn done(stage: Shaped) -> Shape {
    let mut shape = Shape::session(None, Path::new("/tmp/TEST-shape/session"));
    let open = shape.open.as_mut().unwrap();
    open.skill = Some("/tmp/TEST-shape/SKILL.md".into());
    open.reading = Reading::Done(Ok(Shaping {
        goal: Some(GOAL.into()),
        stage,
    }));
    shape
}

fn screen(shape: &mut Shape, width: u16, height: u16, no_color: bool) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(2, 3, width - 4, height - 4);
            shape.render(frame, area, Palette::new(no_color))
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn each_stage_shows_the_goal_as_confirmed_then_one_next_step() {
    let question = Shaped::Question {
        title: "Where records live".into(),
        ask: "Keep records local or synced?".into(),
        found: vec![
            "README.md:3 says local only".into(),
            "no sync client".into(),
        ],
        suggestion: Some("Local.".into()),
    };
    let cases = [
        (
            question,
            "Where records live: Keep records local or synced?",
        ),
        (
            Shaped::Waiting,
            "Seed Me hasn't asked its next question yet.",
        ),
        (
            Shaped::Drafted {
                revision: Some("r2".into()),
            },
            "Seed draft r2 is waiting for you to confirm it.",
        ),
        (
            Shaped::Ready {
                revision: Some("r2".into()),
            },
            "Seed ready: you confirmed r2, and it still matches.",
        ),
        (
            Shaped::Unverified {
                reason: "the seed changed after revision r2 was confirmed".into(),
            },
            "but Seed Me can't verify it: the seed changed after revision r2",
        ),
        (
            Shaped::Ended {
                status: "completed".into(),
            },
            "This session has ended (completed).",
        ),
        (
            Shaped::Unknown {
                what: "This session has no confirmed goal.".into(),
            },
            "Shape can't tell where this seed stands.",
        ),
    ];
    for (stage, said) in cases {
        for (width, height) in [(100, 30), (80, 24)] {
            for no_color in [false, true] {
                let mut shape = done(stage.clone());
                let text = screen(&mut shape, width, height, no_color);
                for line in GOAL.lines() {
                    assert!(text.contains(line), "{line} missing:\n{text}");
                }
                let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
                assert!(flat.contains(said), "{said} missing:\n{text}");
                assert_eq!(text.matches("Next:").count(), 1, "{text}");
                assert!(!text.contains("Option"), "{text}");
                assert!(!text.contains("/tmp/TEST-shape"), "{text}");
                assert_eq!(
                    text.lines().last().unwrap().trim(),
                    "↑↓ scroll   esc back   F10 menu"
                );
            }
        }
    }
}

#[test]
fn the_question_is_jade_and_the_suggestion_reads_as_a_guess_without_colour_too() {
    for no_color in [false, true] {
        let mut shape = done(Shaped::Question {
            title: "T".into(),
            ask: "Q?".into(),
            found: vec![],
            suggestion: Some("S".into()),
        });
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| shape.render(f, Rect::new(2, 3, 96, 26), Palette::new(no_color)))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let find = |needle: &str| {
            (0..30)
                .find_map(|y| {
                    let row: String = (0..100).map(|x| buffer[(x, y)].symbol()).collect();
                    row.find(needle).map(|x| buffer[(x as u16, y)].style())
                })
                .unwrap()
        };
        let palette = Palette::new(no_color);
        assert_eq!(find("T: Q?").add_modifier, palette.jade.add_modifier);
        assert!(
            find("Suggestion: S")
                .add_modifier
                .contains(ratatui::style::Modifier::ITALIC)
        );
    }
}

#[test]
fn failures_and_missing_setup_are_said_plainly_with_the_reason_in_help() {
    let mut shape = Shape::session(None, Path::new("/tmp/TEST-shape/session"));
    let text = screen(&mut shape, 100, 30, true);
    assert!(text.contains("Seed Me, which isn't set up here."), "{text}");
    assert!(
        text.contains("Next: start Tinkery with --shape-pi"),
        "{text}"
    );
    let mut failed = done(Shaped::Waiting);
    failed.open.as_mut().unwrap().reading =
        Reading::Done(Err("Seed Me helper refused: no ledger".into()));
    let text = screen(&mut failed, 100, 30, true);
    assert!(text.contains("Shape couldn't read this session."), "{text}");
    assert!(!text.contains("no ledger"), "{text}");
    assert!(
        failed
            .help_text()
            .contains("Seed Me helper refused: no ledger")
    );
    assert!(failed.help_text().contains("/tmp/TEST-shape/session"));
}

#[test]
fn the_list_holds_only_goals_waiting_for_a_seed_and_esc_steps_back_one_level() {
    let session = |name: &str, stage| Session {
        id: Some(name.into()),
        path: PathBuf::from(format!("/tmp/TEST-shape/{name}")),
        goal: format!("{name} goal\nWhy: test."),
        stage,
        updated: String::new(),
    };
    let mut shape = Shape::list();
    assert!(shape.wants_sessions());
    assert!(screen(&mut shape, 100, 30, true).contains("Reading your sessions."));
    let sessions = [
        session("saved", Stage::GoalSaved),
        session("ready", Stage::SeedReady),
        session("drafted", Stage::SeedDrafted),
        session("ended", Stage::Ended),
        session("unknown", Stage::Unknown),
    ];
    shape.offer(Ok(&sessions));
    let text = screen(&mut shape, 100, 30, true);
    assert!(
        text.contains("› saved goal") && text.contains("goal saved"),
        "{text}"
    );
    assert!(
        text.contains("drafted goal") && text.contains("seed drafted"),
        "{text}"
    );
    for hidden in ["ready goal", "ended goal", "unknown goal"] {
        assert!(!text.contains(hidden), "{text}");
    }
    assert_eq!(shape.key(key(KeyCode::Down), None), Leave::Stay);
    assert_eq!(shape.key(key(KeyCode::Enter), None), Leave::Stay);
    let text = screen(&mut shape, 100, 30, true);
    assert!(text.contains("isn't set up here"), "{text}");
    assert!(shape.help_text().contains("/tmp/TEST-shape/drafted"));
    assert_eq!(shape.key(key(KeyCode::Esc), None), Leave::Stay);
    assert!(screen(&mut shape, 100, 30, true).contains("› drafted goal"));
    assert_eq!(shape.key(key(KeyCode::Esc), None), Leave::Back);
    let mut empty = Shape::list();
    empty.offer(Ok(&sessions[1..2]));
    assert!(screen(&mut empty, 100, 30, true).contains("No goals are waiting for a seed."));
    let mut failed = Shape::list();
    failed.offer(Err("TEST read failed".into()));
    assert!(screen(&mut failed, 100, 30, true).contains("couldn't be read"));
    assert!(failed.help_text().contains("TEST read failed"));
    let mut direct = done(Shaped::Waiting);
    assert_eq!(direct.key(key(KeyCode::Esc), None), Leave::Back);
}
