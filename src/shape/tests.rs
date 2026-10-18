use super::*;
use crate::seed_me::shaping::{Decided, Shaped, Shaping};
use crossterm::event::KeyModifiers;
use std::sync::Arc;

const GOAL: &str = "TEST I can shape a goal calmly.\nWhy: I lose the thread between tools.\nDone when: I see the next question.\nMust: Keep my words.\nMust not: Run agents.";

fn done(stage: Shaped) -> Shape {
    let mut shape = Shape::session(None, Path::new("/tmp/TEST-shape/session"));
    let open = shape.open.as_mut().unwrap();
    open.skill = Some("/tmp/TEST-shape/SKILL.md".into());
    open.reading = Reading::Done(Ok(Shaping {
        goal: Some(GOAL.into()),
        stage,
        version: Some(3),
        intake: None,
        decided: vec![
            Decided {
                title: "TEST where records live".into(),
                answer: "Local for now.\nSynced later if I ask.".into(),
                authority: "user".into(),
            },
            Decided {
                title: "TEST format".into(),
                answer: "Markdown, as the README says.".into(),
                authority: "delegated".into(),
            },
        ],
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
fn verified_intake_copy_is_one_shot_and_feedback_can_be_retried_without_an_agent() {
    let mut shape = done(Shaped::Ready {
        revision: Some("r2".into()),
    });
    let Reading::Done(Ok(shaping)) = &mut shape.open.as_mut().unwrap().reading else {
        panic!()
    };
    let command = "python3 _system/scripts/sdlc.py stage <run> 1 --seed-contract '/tmp/TEST-shape/seed-contract.md' --seed-me-helper '/tmp/TEST-shape/scripts/session.py'";
    shaping.intake = Some(command.into());
    for sent in [false, true] {
        let text = screen(&mut shape, 80, 24, false);
        assert!(text.contains("c copy intake"), "{text}");
        shape.key(key(KeyCode::Char('c')), None);
        assert_eq!(shape.take_copy_request().as_deref(), Some(command));
        assert_eq!(shape.take_copy_request(), None);
        shape.copy_result(sent);
        let text = screen(&mut shape, 80, 24, false);
        assert!(
            text.contains(if sent {
                "Copied."
            } else {
                "Couldn't copy; try again."
            }),
            "{text}"
        );
    }
    shape.key(key(KeyCode::F(5)), None);
    shape.key(key(KeyCode::Char('c')), None);
    assert_eq!(shape.take_copy_request(), None);
}

#[test]
fn each_stage_shows_the_goal_as_confirmed_then_one_next_step() {
    let question = Shaped::Question {
        id: "storage".into(),
        title: "Where records live".into(),
        ask: "Keep records local or synced?".into(),
        found: vec![
            "README.md:3 says local only".into(),
            "no sync client".into(),
        ],
        suggestion: Some("Local.".into()),
    };
    let cases = [
        (question, "Where records live Keep records local or synced?"),
        (
            Shaped::Waiting,
            "Seed Me hasn't asked its next question yet.",
        ),
        (
            Shaped::Drafted {
                revision: Some("r2".into()),
                text: None,
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
                let (first, rest) = GOAL.split_once('\n').unwrap();
                assert!(text.contains(first), "{first} missing:\n{text}");
                for line in rest.lines() {
                    assert!(!text.contains(line), "{line} on screen:\n{text}");
                }
                let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
                assert!(flat.contains(said), "{said} missing:\n{text}");
                assert_eq!(text.matches("Next:").count(), 1, "{text}");
                assert!(!text.contains("Option"), "{text}");
                assert!(!text.contains("/tmp/TEST-shape"), "{text}");
                assert_eq!(
                    text.lines().last().unwrap().trim(),
                    "F5 so far   esc back   F10 menu"
                );
                assert_eq!(so_far(&mut shape, width, height, no_color), GOAL);
            }
        }
    }
}

/// Opens "So far" with F5 and returns the goal it shows, after checking the page around it and
/// that Esc closes it.
fn so_far(shape: &mut Shape, width: u16, height: u16, no_color: bool) -> String {
    shape.key(key(KeyCode::F(5)), None);
    assert!(!shape.takes_text(), "F5's page took words");
    let text = screen(shape, width, height, no_color);
    let rows: Vec<&str> = text.lines().map(str::trim_start).collect();
    let start = rows
        .iter()
        .position(|r| *r == "Your goal, as you confirmed it")
        .unwrap_or_else(|| panic!("no goal heading:\n{text}"));
    let end = start + rows[start..].iter().position(|r| r.is_empty()).unwrap();
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(
            "Decided TEST where records live: Local for now. Synced later if I ask. (you)"
        ),
        "{text}"
    );
    assert!(
        flat.contains("TEST format: Markdown, as the README says. (delegated)"),
        "a decision the operator didn't settle read as theirs:\n{text}"
    );
    assert!(text.contains("F5 close"), "{text}");
    assert!(
        !text.contains("esc back"),
        "a key bar on F5's page:\n{text}"
    );
    shape.key(key(KeyCode::Char('x')), None);
    shape.key(key(KeyCode::Esc), None);
    assert!(
        shape.open.as_ref().unwrap().so_far.is_none(),
        "Esc kept F5's page"
    );
    assert!(
        shape.open.as_ref().unwrap().typed.is_empty(),
        "F5's page took a letter"
    );
    rows[start + 1..end].join("\n")
}

#[test]
fn the_question_is_jade_and_the_suggestion_reads_as_a_guess_without_colour_too() {
    for no_color in [false, true] {
        let mut shape = done(Shaped::Question {
            id: "t".into(),
            title: "Label T".into(),
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
        for (needle, style) in [("Q?", palette.jade), ("Label T", palette.muted)] {
            assert_eq!(find(needle).add_modifier, style.add_modifier, "{needle}");
            if !no_color {
                assert_eq!(find(needle).fg, style.fg, "{needle}");
            }
        }
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
    assert!(
        text.contains("Seed Me refused to read this session: no ledger"),
        "{text}"
    );
    assert!(text.contains("Next: Esc goes back"), "{text}");
    failed.open.as_mut().unwrap().reading = Reading::Done(Err("Cannot locate Seed Me: x".into()));
    let text = screen(&mut failed, 100, 30, true);
    assert!(text.contains("Shape couldn't read this session."), "{text}");
    assert!(!text.contains("Cannot locate"), "{text}");
    assert!(failed.help_text().contains("Cannot locate Seed Me: x"));
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
        session("ready", Stage::SeedUnchecked),
        session("drafted", Stage::SeedDrafted),
        session("ended", Stage::Ended),
        session("unknown", Stage::Unknown),
    ];
    shape.offer(Ok(&sessions), 0);
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
    // Back at the list, Shape reads again: a goal whose seed was confirmed meanwhile leaves it.
    assert!(shape.wants_sessions());
    shape.offer(Ok(&sessions), 0);
    assert!(screen(&mut shape, 100, 30, true).contains("› drafted goal"));
    let mut moved = sessions.clone();
    moved[2].stage = Stage::SeedUnchecked;
    assert_eq!(shape.key(key(KeyCode::Enter), None), Leave::Stay);
    assert_eq!(shape.key(key(KeyCode::Esc), None), Leave::Stay);
    shape.offer(Ok(&moved), 0);
    let text = screen(&mut shape, 100, 30, true);
    assert!(
        text.contains("› saved goal") && !text.contains("drafted goal"),
        "{text}"
    );
    assert_eq!(shape.key(key(KeyCode::Esc), None), Leave::Back);
    let mut empty = Shape::list();
    empty.offer(Ok(&sessions[1..2]), 0);
    assert!(screen(&mut empty, 100, 30, true).contains("No goals are waiting for a seed."));
    let mut failed = Shape::list();
    failed.offer(Err("TEST read failed".into()), 0);
    assert!(screen(&mut failed, 100, 30, true).contains("couldn't be read"));
    assert!(failed.help_text().contains("TEST read failed"));
    let mut direct = done(Shaped::Waiting);
    assert_eq!(direct.key(key(KeyCode::Esc), None), Leave::Back);
}

#[test]
fn incomplete_lists_keep_the_count_and_f1_route() {
    for (width, height) in [(100, 30), (80, 24)] {
        for no_color in [false, true] {
            let mut shape = Shape::list();
            shape.offer(Ok(&[]), 1);
            let text = screen(&mut shape, width, height, no_color);
            let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                flat.contains("1 record couldn't be read. F1 shows why."),
                "{text}"
            );
            assert!(!flat.contains("No goals are waiting"), "{text}");
            assert!(
                shape
                    .help_text()
                    .contains("Some ledgers could not be read as Seed Me sessions")
            );
            shape.offer(Ok(&[]), 2);
            assert!(
                screen(&mut shape, width, height, no_color).contains("2 records couldn't be read.")
            );
            shape.offer(Err("TEST root failure".into()), 2);
            let failed = screen(&mut shape, width, height, no_color);
            assert!(!failed.contains("2 records"), "{failed}");
            assert!(shape.help_text().contains("TEST root failure"));
        }
    }
}

fn agent(budget: usize) -> Arc<Agent> {
    Arc::new(
        Agent::new(
            "/tmp/TEST-shape/never-run-pi".into(),
            "test/model".into(),
            "low".into(),
            None,
            crate::pi::CallBudget::new(budget),
            None,
        )
        .unwrap(),
    )
}

fn question() -> Shaped {
    Shaped::Question {
        id: "storage".into(),
        title: "Where records live".into(),
        ask: "Keep records local or synced?".into(),
        found: vec![],
        suggestion: None,
    }
}

#[test]
fn typing_writes_nothing_and_f2_refuses_empty_words_or_an_empty_budget_before_starting() {
    let mut shape = done(question()).with_agent(Some(agent(0)));
    assert!(shape.takes_text());
    assert_eq!(shape.key(key(KeyCode::F(2)), None), Leave::Stay);
    let text = screen(&mut shape, 100, 30, true);
    assert!(
        text.contains("Type your answer first; nothing was saved."),
        "{text}"
    );
    for c in "Local? Yes".chars() {
        shape.key(key(KeyCode::Char(c)), None);
    }
    shape.key(key(KeyCode::Backspace), None);
    shape.key(key(KeyCode::Char('s')), None);
    shape.paste("\nkept");
    shape.key(key(KeyCode::F(2)), None);
    let open = shape.open.as_ref().unwrap();
    assert!(open.running.is_none(), "a step started with no budget");
    assert_eq!(open.typed.text(), "Local? Yes\nkept");
    let text = screen(&mut shape, 100, 30, true);
    assert!(
        text.contains("  your answer\n  Local? Yes\n  kept"),
        "{text}"
    );
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("No model calls are left: this run's budget is 0."),
        "{text}"
    );
    assert!(
        flat.contains("Next: start Tinkery with --shape-budget CALLS"),
        "{text}"
    );
    assert_eq!(text.matches("Next:").count(), 1, "{text}");
    assert_eq!(
        text.lines().last().unwrap().trim(),
        "F2 save answer and look into next   F5 so far   esc back   F10 menu"
    );
}

#[test]
fn no_next_line_repeats_the_one_action_the_key_bar_names() {
    let dir = tempfile::tempdir().unwrap();
    let seed = Shaped::Drafted {
        revision: Some("r2".into()),
        text: Some("status: draft\n# Seed: TEST\nrevision: r2\n".into()),
    };
    for (stage, keys) in [
        (
            question(),
            "F2 save answer and look into next   F5 so far   esc back   F10 menu",
        ),
        (
            Shaped::Waiting,
            "F2 look into the next question   F5 so far   esc back   F10 menu",
        ),
        (seed, "F3 confirm seed r2   F5 so far   esc back   F10 menu"),
    ] {
        for (width, height) in [(100, 30), (80, 24)] {
            let mut shape = reading(done(stage.clone()).with_agent(Some(agent(1))), dir.path());
            let text = screen(&mut shape, width, height, true);
            assert_eq!(text.matches("Next:").count(), 0, "{text}");
            assert_eq!(text.lines().last().unwrap().trim(), keys, "{text}");
        }
    }
}

#[test]
fn while_a_step_runs_the_question_and_the_answer_stay_in_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut shape = reading(done(question()).with_agent(Some(agent(1))), dir.path());
    shape.paste("TEST keep it local");
    shape.key(key(KeyCode::F(2)), None);
    assert!(
        shape.open.as_ref().unwrap().running.is_some(),
        "no step started"
    );
    for (width, height) in [(100, 30), (80, 24)] {
        let text = screen(&mut shape, width, height, true);
        for shown in [
            "Where records live",
            "Keep records local or synced?",
            "your answer",
            "TEST keep it local",
            "Saving your answer.",
        ] {
            assert!(text.contains(shown), "{shown} missing:\n{text}");
        }
        assert!(!text.contains("Nothing found"), "{text}");
        assert_eq!(text.matches("Next:").count(), 0, "{text}");
        assert_eq!(text.lines().last().unwrap().trim(), "esc stop");
    }
    // F5 waits until the step is done, and help says what Esc does now.
    shape.key(key(KeyCode::F(5)), None);
    assert!(shape.open.as_ref().unwrap().so_far.is_none());
    assert!(
        shape
            .help_text()
            .contains("A step is running. Esc stops it.\nEsc stop · F10 menu")
    );
}

#[test]
fn a_tall_question_scrolls_while_the_answer_and_every_key_stay_in_view() {
    let dir = tempfile::tempdir().unwrap();
    // One finding can still overflow; the others belong behind F5.
    let found = vec![format!(
        "src/TEST/1.rs:1: {}",
        "this path prints the helper's error verbatim, stderr included. ".repeat(18)
    )];
    let tall = Shaped::Question {
        id: "storage".into(),
        title: "Where records live".into(),
        ask: "Keep records local or synced?".into(),
        found,
        suggestion: Some("Local, matching the README.".into()),
    };
    let mut shape = reading(done(tall).with_agent(Some(agent(1))), dir.path());
    shape.paste("TEST first line\nTEST last line");
    let text = screen(&mut shape, 80, 24, true);
    assert!(text.contains("Keep records local or synced?"), "{text}");
    assert!(
        text.contains("  your answer\n  TEST first line\n  TEST last line"),
        "{text}"
    );
    assert_eq!(
        text.lines().last().unwrap().trim(),
        "pgup/dn read   F2 save + look   F5 so far   esc back   F10 menu"
    );
    // Scrolling moves only the reading; the answer stays where it is.
    for _ in 0..30 {
        shape.key(key(KeyCode::PageDown), None);
    }
    let text = screen(&mut shape, 80, 24, true);
    assert!(text.contains("F4 picks another folder."), "{text}");
    assert!(text.contains("TEST last line"), "{text}");
}

#[test]
fn without_an_agent_shape_only_reads_and_letters_scroll() {
    let mut shape = done(question());
    assert!(!shape.takes_text());
    shape.key(key(KeyCode::Char('j')), None);
    assert!(shape.open.as_ref().unwrap().typed.is_empty());
    assert_eq!(shape.scroll, 1);
    shape.key(key(KeyCode::F(2)), None);
    assert!(shape.open.as_ref().unwrap().running.is_none());
    let text = screen(&mut shape, 100, 30, true);
    assert!(text.contains("tend's c copies the prompt"), "{text}");
}

#[test]
fn a_draft_shows_the_seed_and_f3_needs_the_operators_own_words() {
    let seed =
        "status: draft\n# Seed: TEST\nrevision: r2\n\n## You are confirming\nTEST seed body.\n";
    let mut shape = done(Shaped::Drafted {
        revision: Some("r2".into()),
        text: Some(seed.into()),
    })
    .with_agent(Some(agent(1)));
    let text = screen(&mut shape, 100, 30, true);
    assert!(text.contains("TEST seed body."), "{text}");
    assert!(!text.contains("Must not: Run agents."), "{text}");
    assert!(
        text.contains("  Seed draft r2\n\n  Seed: TEST\n\n  You are confirming\n  TEST seed body."),
        "{text}"
    );
    for raw in ["status:", "revision:", "#"] {
        assert!(!text.contains(raw), "{raw} shown:\n{text}");
    }
    assert!(
        text.contains("  your words\n  a line saying you confirm, then F3\n\n  Confirming the seed is not approval to build."),
        "{text}"
    );
    shape.key(key(KeyCode::F(2)), None);
    shape.key(key(KeyCode::F(3)), None);
    assert!(shape.open.as_ref().unwrap().running.is_none());
    assert!(
        screen(&mut shape, 100, 30, true).contains("Type your words first; nothing was confirmed.")
    );
}

#[test]
fn the_reply_is_a_question_or_a_seed_and_never_a_confirmation() {
    use step::{Next, decode};
    let asked = decode(
        "Here it is:\n```json\n{\"next\":\"question\",\"id\":\"Storage\",\"question\":\"Local?\",\"found\":[{\"checked\":\"README.md:3\",\"observed\":\"local only\"},{\"checked\":1}],\"suggestion\":null}\n```",
    )
    .unwrap();
    let Next::Question(ask) = asked else {
        panic!("{asked:?}")
    };
    assert_eq!(
        (ask.id.as_str(), ask.label.as_str(), ask.gate.as_str()),
        ("Storage", "Local?", "Local?")
    );
    assert_eq!(ask.found.len(), 1);
    assert_eq!(ask.suggestion, None);
    assert_eq!(
        decode("{\"next\":\"seed\",\"confirming\":\" Build it. \",\"checks\":[{\"statement\":\"TEST\",\"kind\":\"judge\",\"source\":\"TEST\"}]}").unwrap(),
        Next::Seed {
            confirming: "Build it.".into(),
            checks: serde_json::json!([{"statement":"TEST","kind":"judge","source":"TEST"}]),
            found: vec![],
        }
    );
    let Next::Seed { found, .. } = decode(
        "{\"next\":\"seed\",\"confirming\":\"Build it.\",\"checks\":[{\"statement\":\"TEST\",\"kind\":\"judge\",\"source\":\"TEST\"}],\"found\":[{\"checked\":\"src/lib.rs:9\",\"observed\":\"the palette\"}]}",
    )
    .unwrap() else {
        panic!("not a seed")
    };
    assert_eq!(
        (found[0].checked.as_str(), found[0].observed.as_str()),
        ("src/lib.rs:9", "the palette")
    );
    for reply in [
        "{\"next\":\"confirm\",\"revision\":\"r1\"}",
        "{\"next\":\"question\"}",
        "{\"next\":\"seed\"}",
        "no json",
        "",
    ] {
        assert!(decode(reply).is_err(), "{reply}");
    }
}

#[test]
fn stacker_r1_draft_stores_records_instead_of_model_markdown() {
    use crate::seed_me::{Affirmed, Config, answering};
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-214-")
        .tempdir()
        .unwrap();
    let skill =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".ci-seed-me/skills/seed-me/SKILL.md");
    let mut receipt = Config::new(skill.clone(), Some(root.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: GOAL.into(),
            outcome: "TEST Stacker".into(),
            options: vec![],
            originals: vec![],
            context: "{}".into(),
            source: "TEST fixture, not approval".into(),
        })
        .unwrap();
    receipt.stop_viewer().unwrap();
    let original = include_str!("../../tests/fixtures/stacker-r1-seed-contract.md");
    let legacy = original
        .split_once("## You are confirming\n")
        .unwrap()
        .1
        .split_once("## Evidence and uncertainty\n")
        .unwrap()
        .0
        .trim();
    let version = answering::read(&skill, &receipt.session).unwrap().version();
    let reply: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/stacker-r1-reply.json")).unwrap();
    assert!(
        answering::draft_seed(
            &skill,
            &receipt.session,
            version,
            legacy,
            &reply["checks"],
            &[],
            root.path()
        )
        .is_err()
    );
    answering::draft_seed(
        &skill,
        &receipt.session,
        version,
        reply["confirming"].as_str().unwrap(),
        &reply["checks"],
        &[],
        root.path(),
    )
    .unwrap();
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt.session.join("seed-checks.json")).unwrap())
            .unwrap();
    assert_eq!(stored, reply["checks"]);
    let upstream_fixture = skill
        .parent()
        .unwrap()
        .join("../../tests/fixtures/stacker-r1-seed-contract.md");
    assert_eq!(
        std::fs::read(upstream_fixture).unwrap(),
        original.as_bytes()
    );
    let (before_criteria, criteria) = legacy.split_once("## Acceptance criteria\n").unwrap();
    let (_, after_criteria) = criteria
        .split_once("## Affected users and systems")
        .unwrap();
    assert_eq!(
        reply["confirming"].as_str().unwrap(),
        format!("{before_criteria}## Affected users and systems{after_criteria}")
    );
    assert_eq!(reply["found"].as_array().unwrap().len(), 13);
    let decoded = step::decode(&format!(
        "My reading is provisional.\n{}\nNo checks were run.",
        include_str!("../../tests/fixtures/stacker-r1-reply.json")
    ))
    .unwrap();
    assert!(matches!(decoded, step::Next::Seed { checks, .. } if checks == stored));
    let seed_before = std::fs::read(receipt.session.join("seed-contract.md")).unwrap();
    let records_before = std::fs::read(receipt.session.join("seed-checks.json")).unwrap();
    let mut invalid = vec![
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::Value::Null,
    ];
    for (key, value) in [
        ("kind", serde_json::json!("unknown")),
        ("source", serde_json::json!("")),
        ("statement", serde_json::json!(42)),
        ("extra", serde_json::json!("unknown field")),
        ("cmd", serde_json::json!("not allowed on judge")),
    ] {
        let mut bad = stored.clone();
        bad[0][key] = value;
        invalid.push(bad);
    }
    for cwd in [
        "/tmp/TEST",
        "../TEST",
        "a/../TEST",
        "C:/TEST",
        "a\\TEST",
        "a\nb",
        "a\rb",
    ] {
        let mut bad = stored.clone();
        bad[1]["cwd"] = cwd.into();
        invalid.push(bad);
    }
    for key in ["missing", "expect", "cwd", "statement", "source"] {
        let mut bad = stored.clone();
        bad[1].as_object_mut().unwrap().remove(key);
        invalid.push(bad);
    }
    for bad in invalid {
        assert!(
            answering::draft_seed(
                &skill,
                &receipt.session,
                version,
                reply["confirming"].as_str().unwrap(),
                &bad,
                &[],
                root.path()
            )
            .is_err(),
            "accepted invalid checks: {bad}"
        );
        assert_eq!(
            std::fs::read(receipt.session.join("seed-contract.md")).unwrap(),
            seed_before
        );
        assert_eq!(
            std::fs::read(receipt.session.join("seed-checks.json")).unwrap(),
            records_before
        );
    }
    let render = std::process::Command::new("python3")
        .arg(skill.parent().unwrap().join("scripts/session.py"))
        .args(["seed", "checks", "render"])
        .arg(&receipt.session)
        .output()
        .unwrap();
    assert!(render.status.success());
    // The host writes canonical LF headings even when the model's prose uses CRLF.
    answering::draft_seed(
        &skill,
        &receipt.session,
        version,
        &reply["confirming"].as_str().unwrap().replace('\n', "\r\n"),
        &reply["checks"],
        &[],
        root.path(),
    )
    .unwrap();
    let draft = std::fs::read_to_string(receipt.session.join("seed-contract.md")).unwrap();
    assert!(draft.contains(std::str::from_utf8(&render.stdout).unwrap()));
    assert!(draft.contains("## Problem statement\nYou played Stacker"));
    assert!(
        draft.contains("You are confirming the goal: create a software version of Stacker.\r\n")
    );
    answering::confirm_seed(
        &skill,
        &receipt.session,
        "r2",
        "TEST fixture confirmation only",
    )
    .unwrap();
    assert!(matches!(
        crate::seed_me::shaping::read(&skill, &receipt.session)
            .unwrap()
            .stage,
        Shaped::Ready { .. }
    ));
    let confirmation: serde_json::Value = serde_json::from_slice(
        &std::fs::read(receipt.session.join("seed-confirmation.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(confirmation["schema_version"], 2);
    assert!(confirmation["checks_sha256"].as_str().is_some());
    std::fs::write(receipt.session.join("seed-checks.json"), "[]").unwrap();
    assert!(!matches!(
        crate::seed_me::shaping::read(&skill, &receipt.session)
            .unwrap()
            .stage,
        Shaped::Ready { .. }
    ));
    assert!(
        receipt.session.join("seed-checks.json").exists(),
        "Stacker r1 still uses handwritten criteria instead of bound records"
    );
}

// Pinned consumer expectations are deliberately independent of the producer's heading list.
fn missing_intake_sections(seed: &str, consumer: &str) -> Vec<String> {
    consumer
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|section| !seed.lines().any(|line| line == format!("## {section}")))
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
#[test]
fn a_stub_step_preserves_a_check_or_operator_judgment_for_every_done_when() {
    use crate::{
        pi::CallBudget,
        seed_me::{Affirmed, Config, answering, shaping},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    let temp = tempfile::Builder::new()
        .prefix("tinkery-TEST-183-")
        .tempdir()
        .unwrap();
    let skill = std::env::var_os("SEED_ME_TEST_SKILL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".ci-seed-me/skills/seed-me/SKILL.md")
        });
    let goal = "TEST I can check a seed.\nWhy: Intake needs an eval.\nDone when: Existing output is correct.\n- New output is correct.\n- The result feels calm.\nMust: Keep my words; existing check is ./TEST-check with expected output TEST existing output in cwd .\nMust not: Build without approval.";
    let mut receipt = Config::new(skill.clone(), Some(temp.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: goal.into(),
            outcome: "TEST checks".into(),
            options: vec![],
            originals: vec![],
            context: "{}".into(),
            source: "TEST fixture, not real approval".into(),
        })
        .unwrap();
    receipt.stop_viewer().unwrap();
    let confirming = include_str!("../../tests/fixtures/shape-seed-reply.txt").trim();
    let checks = include_str!("../../tests/fixtures/shape-checks.json");
    fs::write(temp.path().join("TEST-read.md"), "TEST observed content\n").unwrap();
    // Upgrade an older installation's undated draft through the actual drafting path.
    fs::write(
        receipt.session.join("seed-contract.md"),
        "status: draft\n# TEST\nrevision: r7\n\n## You are confirming\nTEST old draft.\n",
    )
    .unwrap();
    // A stub can prove instructions and transport, not a real model's choice of checks.
    let program = temp.path().join("TEST-pi");
    fs::write(
        &program,
        format!(
            r##"#!/usr/bin/env python3
import json, pathlib, sys
request = json.load(sys.stdin)
prompt = sys.argv[sys.argv.index('--system-prompt') + 1]
rules = ['every Done when', 'Check records', 'checks', 'provisional', 'you judge', 'Do not run',
         'Problem statement', 'Proposed outcome', 'Acceptance criteria', 'Constraints and boundaries',
         'Seed contract (artifact contract)', 'Accepted decisions and assumed defaults']
missing = [rule for rule in rules if rule not in prompt]
pathlib.Path('TEST-missing-rules.json').write_text(json.dumps(missing))
assert not missing, 'missing check-drafting instructions: ' + str(missing)
assert request['goal'] == {goal:?}
assert sys.argv[sys.argv.index('--tools') + 1] == 'read,grep,find,ls'
print(json.dumps({{'next': 'seed', 'confirming': {confirming:?}, 'checks': json.loads({checks:?}), 'found': [
    {{'checked': 'TEST-read.md:1', 'observed': 'TEST observed content'}}]}}))
"##
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    for name in ["TEST-check", "TEST-new-check"] {
        let command = temp.path().join(name);
        fs::write(&command, "#!/bin/sh\ntouch TEST-CHECK-WAS-RUN\n").unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let version = answering::read(&skill, &receipt.session).unwrap().version();
    let agent = Arc::new(
        step::Agent::new(
            program,
            "test/stub".into(),
            String::new(),
            Some(temp.path().into()),
            CallBudget::new(1),
            None,
        )
        .unwrap(),
    );
    let running = step::Running::start(
        step::Work::Look {
            expected: version,
            folder: temp.path().into(),
        },
        skill.clone(),
        receipt.session.clone(),
        agent.clone(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let result = loop {
        if let Some(result) = running.poll() {
            break result;
        }
        assert!(Instant::now() < deadline, "stub step timed out");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        fs::read_to_string(temp.path().join("TEST-missing-rules.json")).unwrap(),
        "[]",
        "stub did not receive per-criterion check instructions"
    );
    assert_eq!(result, Ok(step::Done::Drafted));
    assert_eq!(agent.remaining(), 0);
    let draft = shaping::read(&skill, &receipt.session).unwrap();
    assert_eq!(draft.goal.as_deref(), Some(goal));
    let Shaped::Drafted {
        revision,
        text: Some(text),
    } = draft.stage
    else {
        panic!("not a draft")
    };
    assert_eq!(revision.as_deref(), Some("r8"));
    let mut remaining = text.as_str();
    for paragraph in confirming.split("\n\n") {
        remaining = remaining
            .split_once(paragraph)
            .expect("model prose changed, dropped or reordered")
            .1;
    }
    assert!(text.starts_with("status: draft\n"));
    // Shape's real drafting path must hand intake the upstream section names, not just prose.
    let contract =
        fs::read_to_string(skill.parent().unwrap().join("references/seed-contract.md")).unwrap();
    for section in [
        "Problem statement",
        "Proposed outcome",
        "Acceptance criteria",
        "Affected users and systems",
        "Constraints and boundaries",
        "Knowledge map",
        "Suggested first slice",
        "Risks and verification",
        "Open questions and deferrals",
        "Owners and next steps",
        "Downstream handoff",
        "Evidence and uncertainty",
        "Decisions (from the ledger)",
        "Assumed (not confirmed by you)",
    ] {
        let declaration = if section == "Knowledge map" {
            "- **Knowledge map**, in plain words, four parts:".to_owned()
        } else if matches!(
            section,
            "Decisions (from the ledger)" | "Assumed (not confirmed by you)"
        ) {
            format!("## {section}")
        } else {
            format!("- {section}:")
        };
        assert!(
            contract.lines().any(|line| line.starts_with(&declaration)),
            "upstream contract changed: {section}"
        );
        assert!(
            text.lines().any(|line| line == format!("## {section}")),
            "Shape seed lacks canonical section {section}: {text}"
        );
    }
    let consumer = include_str!("../../tests/fixtures/tink-sdlc-intake-sections.txt");
    assert!(
        missing_intake_sections(&text, consumer).is_empty(),
        "consumer contract drift: {:?}",
        missing_intake_sections(&text, consumer)
    );
    let renamed_consumer = consumer.replace("Acceptance criteria\n", "Intake checks\n");
    assert_eq!(
        missing_intake_sections(&text, &renamed_consumer),
        ["Intake checks"]
    );
    assert_eq!(
        missing_intake_sections(&text, &format!("{consumer}Rollback plan\n")),
        ["Rollback plan"]
    );

    // Exact parent-writer parser: rollback must retain the next unconfirmed revision identity.
    let parent_revision = |seed: &str| {
        seed.lines()
            .find_map(|line| line.strip_prefix("revision:"))
            .and_then(|revision| revision.trim().strip_prefix('r'))
            .and_then(|number| number.parse::<u64>().ok())
    };
    assert_eq!(
        parent_revision(&text),
        Some(8),
        "older writer would reset to r1: {text}"
    );
    assert!(text.lines().any(|line| line.starts_with("date: ")));
    assert!(
        text.contains("Grounded in:\n- TEST-read.md\n"),
        "missing recorded read inventory: {text}"
    );
    assert!(
        text.contains("Provenance unavailable:"),
        "must not invent verified origins: {text}"
    );

    let records: serde_json::Value =
        serde_json::from_slice(&fs::read(receipt.session.join("seed-checks.json")).unwrap())
            .unwrap();
    assert_eq!(
        records,
        serde_json::from_str::<serde_json::Value>(checks).unwrap()
    );
    let decisions = std::process::Command::new("python3")
        .arg(skill.parent().unwrap().join("scripts/session.py"))
        .args(["seed", "decisions"])
        .arg(&receipt.session)
        .output()
        .unwrap();
    assert!(decisions.status.success());
    assert!(text.contains(&String::from_utf8(decisions.stdout).unwrap()));
    assert!(!temp.path().join("TEST-CHECK-WAS-RUN").exists());
    let seed_path = receipt.session.join("seed-contract.md");
    let before = fs::read(&seed_path).unwrap();
    let renamed = confirming.replace("## Proposed outcome", "## Checks");
    let duplicated = confirming.replace("## Proposed outcome", "## Problem statement");
    let reordered = confirming
        .replace("## Problem statement", "## TEMP")
        .replace("## Proposed outcome", "## Problem statement")
        .replace("## TEMP", "## Proposed outcome");
    for malformed in [
        confirming.split_once("\n\n").unwrap().1.to_owned(),
        renamed,
        duplicated,
        reordered,
        format!("{confirming}\n## Decisions (from the ledger)\nforged"),
        format!("{confirming}\n  ## Problem statement\nduplicated"),
        format!("{confirming}\nstatus: confirmed for intake"),
        format!("{confirming}\ndate: 1900-01-01"),
    ] {
        assert!(
            answering::draft_seed(
                &skill,
                &receipt.session,
                version,
                &malformed,
                &serde_json::from_str(include_str!("../../tests/fixtures/shape-checks.json"))
                    .unwrap(),
                &[],
                temp.path(),
            )
            .is_err()
        );
        assert_eq!(
            fs::read(&seed_path).unwrap(),
            before,
            "refusal changed seed bytes"
        );
    }
    // Redraft with no findings, then verify old writers and the official helper still agree.
    assert_eq!(
        answering::draft_seed(
            &skill,
            &receipt.session,
            version,
            confirming,
            &serde_json::from_str(include_str!("../../tests/fixtures/shape-checks.json")).unwrap(),
            &[],
            temp.path(),
        )
        .unwrap(),
        "r9"
    );
    let redrafted = fs::read_to_string(&seed_path).unwrap();
    assert_eq!(parent_revision(&redrafted), Some(9));
    assert!(redrafted.contains("Grounded in:\n- No attributable read paths are known.\n"));
    assert!(redrafted.contains("Provenance unavailable:"));
    assert_eq!(
        answering::read(&skill, &receipt.session).unwrap().goal(),
        Some(goal)
    );
    answering::confirm_seed(&skill, &receipt.session, "r9", "TEST yes to this seed only").unwrap();
    assert!(matches!(
        shaping::read(&skill, &receipt.session).unwrap().stage,
        Shaped::Ready { .. }
    ));
    assert!(!temp.path().join("TEST-CHECK-WAS-RUN").exists());
}

#[cfg(unix)]
#[test]
fn repository_checks_are_proposed_with_sources_but_never_executed() {
    use crate::{
        pi::CallBudget,
        seed_me::{Affirmed, Config, answering, shaping},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    let temp = tempfile::Builder::new()
        .prefix("tinkery-TEST-198-")
        .tempdir()
        .unwrap();
    let skill = std::env::var_os("SEED_ME_TEST_SKILL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".ci-seed-me/skills/seed-me/SKILL.md")
        });
    let goal = "TEST I can check a seed.\nWhy: Checks need evidence.\nDone when: The parsed value is new.\n- The future checker validates the result.\n- The result feels calm.\n- The missing boundary assertion detects the defect.\nMust: Keep my words. The proposed ./TEST-future-check does not exist yet.\nMust not: Run checks or build without approval.";
    fs::create_dir(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("README.md"),
        "cmd: python3 tests/check_value.py\nexpect: new value\ncwd: .\n",
    )
    .unwrap();
    fs::write(temp.path().join("value.txt"), "old\n").unwrap();
    fs::write(
        temp.path().join("tests/runner_only.py"),
        "print('TEST runner without the required boundary check')\n",
    )
    .unwrap();
    fs::write(temp.path().join("tests/check_value.py"),
        "from pathlib import Path\nPath('TEST-CHECK-WAS-RUN').touch()\nassert Path('value.txt').read_text().strip() == 'new'\nprint('new value')\n").unwrap();
    let mut receipt = Config::new(skill.clone(), Some(temp.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: goal.into(),
            outcome: "TEST checks".into(),
            options: vec![],
            originals: vec![],
            context: "{}".into(),
            source: "TEST fixture, not real approval".into(),
        })
        .unwrap();
    receipt.stop_viewer().unwrap();
    let program = temp.path().join("TEST-pi");
    let template = include_str!("../../tests/fixtures/shape-seed-reply.txt");
    fs::write(&program, format!(r##"#!/usr/bin/env python3
import json, pathlib, sys
request = json.load(sys.stdin)
assert request['goal'] == {goal:?}
assert 'python3 tests/check_value.py' not in json.dumps(request)
assert sys.argv[sys.argv.index('--tools') + 1] == 'read,grep,find,ls'
prompt = sys.argv[sys.argv.index('--system-prompt') + 1]
source = pathlib.Path('README.md').read_text().splitlines()
command, expected, cwd = [line.split(': ', 1)[1] for line in source]
assert pathlib.Path('tests/check_value.py').is_file()
assert not pathlib.Path('TEST-future-check').exists()
assert 'assert ' not in pathlib.Path('tests/runner_only.py').read_text()
# A policy-aware stub, not a claim about real-model reasoning or check quality.
can_propose = all(rule in prompt for rule in [
    'For a code-checkable criterion, propose', "cite the command's source", 'fail before the change'
]) and 'file inspection alone is not a settled' not in prompt
checks = [
    dict(statement='The parsed value is new.', kind='cmd' if can_propose else 'judge', source='README.md:1-3; proposed from repository inspection, not run.'),
    dict(statement='The future checker validates the result.', kind='provisional', source='TEST goal',
         missing='Once the named TEST-future-check exists.', cmd='./TEST-future-check', expect='result validated', cwd='.'),
    dict(statement='The result feels calm.', kind='judge', source='TEST goal', how_to_judge='The operator inspects calmness.'),
    dict(statement='The missing boundary assertion detects the defect.', kind='provisional', source='tests/runner_only.py:1',
         missing='The boundary assertion must be built; an existing runner is not that assertion.',
         expect='boundary checked after its assertion exists', cwd='.')]
if can_propose:
    checks[0].update(cmd=command, expect=expected, cwd=cwd)
    if 'not provisional merely because setup' not in prompt:
        checks[0].update(kind='provisional', missing='Build binary and prepare reversion probe first.')
else:
    checks[0]['how_to_judge'] = 'No invocation was settled.'
if 'Use provisional when the needed test/assertion or invocation is missing' not in prompt:
    checks[3].update(kind='cmd', cmd='python3 tests/runner_only.py')
    del checks[3]['missing']
body = {template:?}
start, end = body.index('## Suggested first slice\n'), body.index('## Risks and verification\n')
body = body[:start] + '## Suggested first slice\nProposal only: add the missing behavior, then use the check above.\n\n' + body[end:]
body = body.replace('no repository facts were recorded.', 'reported reads are listed below.')
print(json.dumps({{'next': 'seed', 'confirming': body, 'checks': checks, 'found': [
    {{'checked': 'README.md:1-3', 'observed': 'documents the focused behavior check'}},
    {{'checked': 'tests/check_value.py:1-4', 'observed': 'asserts the new value; current value.txt is old'}}]}}))
"##)).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let agent = Arc::new(
        step::Agent::new(
            program,
            "test/stub".into(),
            String::new(),
            Some(temp.path().into()),
            CallBudget::new(1),
            None,
        )
        .unwrap(),
    );
    let version = answering::read(&skill, &receipt.session).unwrap().version();
    let running = step::Running::start(
        step::Work::Look {
            expected: version,
            folder: temp.path().into(),
        },
        skill.clone(),
        receipt.session.clone(),
        agent.clone(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(result) = running.poll() {
            assert_eq!(result, Ok(step::Done::Drafted));
            break;
        }
        assert!(Instant::now() < deadline, "stub drafting timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    let seed = fs::read_to_string(receipt.session.join("seed-contract.md")).unwrap();
    let checks: serde_json::Value =
        serde_json::from_slice(&fs::read(receipt.session.join("seed-checks.json")).unwrap())
            .unwrap();
    assert_eq!(
        checks[0]["kind"], "cmd",
        "repository evidence was replaced by judgment or routine setup: {checks}"
    );
    assert_eq!(checks[0]["cmd"], "python3 tests/check_value.py");
    assert_eq!(
        checks[0]["source"],
        "README.md:1-3; proposed from repository inspection, not run."
    );
    assert_eq!(checks[1]["kind"], "provisional");
    assert_eq!(checks[1]["cmd"], "./TEST-future-check");
    assert_eq!(checks[2]["kind"], "judge");
    assert_eq!(
        checks[3]["kind"], "provisional",
        "a runner without the assertion is not a check"
    );
    assert!(checks[3].get("cmd").is_none());
    assert!(seed.starts_with("status: draft\n"));
    assert!(!receipt.session.join("seed-confirmation.json").exists());
    assert_eq!(
        answering::read(&skill, &receipt.session).unwrap().goal(),
        Some(goal)
    );
    assert!(!temp.path().join("TEST-CHECK-WAS-RUN").exists());
    assert!(!temp.path().join("TEST-future-check").exists());
    answering::confirm_seed(
        &skill,
        &receipt.session,
        "r1",
        "TEST yes to proposed checks, not implementation",
    )
    .unwrap();
    assert!(matches!(
        shaping::read(&skill, &receipt.session).unwrap().stage,
        Shaped::Ready { .. }
    ));
    assert_eq!(agent.remaining(), 0);
    assert!(!temp.path().join("TEST-CHECK-WAS-RUN").exists());
}

#[test]
fn prompt_recovery_separates_existing_checks_from_missing_assertions() {
    let prompt = step::prompt();
    for rule in [
        "Keep a known repository invocation with existing assertions for the criterion",
        "not provisional merely because setup",
        "state those prerequisites and intended red separately",
        "Use provisional when the needed test/assertion or invocation is missing",
        "never invent a command or substitute an existing generic runner for a missing regression",
        "under a targeted reversion of an already-present fix",
    ] {
        assert!(
            prompt.contains(rule),
            "missing check-availability distinction: {rule}"
        );
    }
    assert!(!prompt.contains("When no repository command exists or something must be built first"));
}

#[test]
fn prompt_recovery_does_not_turn_existing_behavior_into_a_new_goal_choice() {
    let prompt = step::prompt();
    assert!(prompt.contains("If the repo already appears to meet the confirmed goal, propose verification and repairs only for demonstrated gaps"));
    assert!(prompt.contains("do not ask whether to verify it or require extra changes"));
    assert!(prompt.contains("do not claim it is complete without evidence"));
}

#[cfg(unix)]
#[test]
fn prompt_recovery_existing_goal_drafts_without_reconfirmation() {
    assert_settled_intake_drafts(None);
}

#[test]
fn settled_intake_drafts_without_reopening_settled_or_deferred_decisions() {
    let prompt = step::prompt();
    for rule in [
        "settled or explicitly deferred decisions do not block it",
        "Never reopen what the goal",
        "a settled user-authority ledger answer settles",
        "record deferred choices as operator prerequisites in Open questions and deferrals",
        "choose no alternative",
        "When the goal and recorded answers settle what this draft must say, draft the seed",
        "genuinely unresolved ledger decisions still block drafting",
    ] {
        assert!(prompt.contains(rule), "missing settled-intake rule: {rule}");
    }
    assert!(!prompt.contains("If a decision only the operator can make is still open, ask"));
}

#[cfg(unix)]
#[test]
fn settled_intake_answer_drafts_without_reopening() {
    assert_settled_intake_drafts(Some(
        "TEST Keep storage local; that choice is settled. Do not ask it again.",
    ));
}

#[cfg(unix)]
#[test]
fn settled_intake_deferral_drafts_without_choosing_or_confirming() {
    assert_settled_intake_drafts(Some(
        "TEST Draft only. Defer local versus synced until before implementation; keep it as an operator prerequisite. Do not choose either or approve implementation.",
    ));
}

#[cfg(unix)]
fn assert_settled_intake_drafts(words: Option<&str>) {
    use crate::{
        pi::CallBudget,
        seed_me::{Affirmed, Config, answering},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    let skill = std::env::var_os("SEED_ME_TEST_SKILL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".ci-seed-me/skills/seed-me/SKILL.md")
        });
    let template = include_str!("../../tests/fixtures/shape-seed-reply.txt");
    let temp = tempfile::Builder::new()
        .prefix("tinkery-TEST-settled-")
        .tempdir()
        .unwrap();
    let mut receipt = Config::new(skill.clone(), Some(temp.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: GOAL.into(),
            outcome: "TEST draft".into(),
            options: vec![],
            originals: vec![],
            context: "{}".into(),
            source: "TEST fixture, not real approval".into(),
        })
        .unwrap();
    receipt.stop_viewer().unwrap();
    let version = answering::read(&skill, &receipt.session).unwrap().version();
    let version = if words.is_some() {
        let version = answering::ask(
            &skill,
            &receipt.session,
            version,
            &answering::Ask {
                id: "storage".into(),
                label: "TEST Storage".into(),
                question: "TEST Local or synced?".into(),
                gate: "TEST Storage disposition".into(),
                found: vec![],
                suggestion: None,
            },
            temp.path(),
        )
        .unwrap();
        // A genuinely open decision still refuses a draft before the answer is saved.
        assert!(
            answering::draft_seed(
                &skill,
                &receipt.session,
                version,
                template,
                &serde_json::from_str(include_str!("../../tests/fixtures/shape-checks.json"))
                    .unwrap(),
                &[],
                temp.path()
            )
            .is_err()
        );
        version
    } else {
        fs::write(
            temp.path().join("TEST-existing.txt"),
            "TEST Requested behavior already present.\n",
        )
        .unwrap();
        version
    };
    assert!(!receipt.session.join("seed-contract.md").exists());
    let words_json = serde_json::to_string(&words).unwrap();
    let checks = include_str!("../../tests/fixtures/shape-checks.json");
    let program = temp.path().join("TEST-pi");
    fs::write(&program, format!(r##"#!/usr/bin/env python3
import json, pathlib, sys
request = json.load(sys.stdin)
words = json.loads({words_json:?})
assert request['goal'] == {GOAL:?}
assert request['ledger']['current_question'] is None
assert all(n['status'] != 'unresolved' for n in request['ledger']['nodes'])
if words is not None:
    answer = next(n for n in request['ledger']['nodes'] if n['id'] == 'storage')
    assert answer['status'] == 'settled' and answer['authority'] == 'user'
    assert answer['answer'] == words
else:
    assert len(request['ledger']['nodes']) == 1
    assert pathlib.Path('TEST-existing.txt').read_text() == 'TEST Requested behavior already present.\n'
assert sys.argv[sys.argv.index('--tools') + 1] == 'read,grep,find,ls'
prompt = sys.argv[sys.argv.index('--system-prompt') + 1]
# This stub proves routing instructions and helper transport, not model reasoning.
drafts = all(rule in prompt for rule in [
    'settled or explicitly deferred decisions do not block it',
    'Never reopen what the goal',
    'When the goal and recorded answers settle what this draft must say, draft the seed',
    'record deferred choices as operator prerequisites in Open questions and deferrals',
    'choose no alternative',
    'genuinely unresolved ledger decisions still block drafting',
])
if words is None:
    drafts = drafts and 'do not ask whether to verify it or require extra changes' in prompt
if not drafts:
    print(json.dumps({{'next': 'question', 'id': 'reopened', 'question': 'TEST Verify existing behavior or require extra changes?', 'found': []}}))
else:
    body = {template:?}
    start, end = body.index('## Open questions and deferrals\n'), body.index('## Owners and next steps\n')
    recorded = words if words is not None else 'Existing behavior remains unverified; propose checks, not extra requirements.'
    body = body[:start] + '## Open questions and deferrals\nRecorded disposition: ' + recorded + '\n\n' + body[end:]
    print(json.dumps({{'next': 'seed', 'confirming': body, 'checks': json.loads({checks:?}), 'found': []}}))
"##)).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let agent = Arc::new(
        step::Agent::new(
            program,
            "test/stub".into(),
            String::new(),
            Some(temp.path().into()),
            CallBudget::new(1),
            None,
        )
        .unwrap(),
    );
    let work = match words {
        Some(words) => step::Work::Answer {
            id: "storage".into(),
            words: words.into(),
            expected: version,
            folder: temp.path().into(),
        },
        None => step::Work::Look {
            expected: version,
            folder: temp.path().into(),
        },
    };
    let running = step::Running::start(work, skill.clone(), receipt.session.clone(), agent.clone());
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(result) = running.poll() {
            assert_eq!(
                result,
                Ok(step::Done::Drafted),
                "settled intake reopened: {words:?}"
            );
            break;
        }
        assert!(Instant::now() < deadline, "stub drafting timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    let ledger = answering::read(&skill, &receipt.session).unwrap();
    assert_eq!(ledger.goal(), Some(GOAL));
    assert!(ledger.open().is_empty());
    let context: serde_json::Value = serde_json::from_str(&ledger.context()).unwrap();
    if let Some(words) = words {
        let answer = context["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "storage")
            .unwrap();
        assert_eq!(answer["answer"], words);
    } else {
        assert_eq!(context["nodes"].as_array().unwrap().len(), 1);
    }
    let seed = fs::read_to_string(receipt.session.join("seed-contract.md")).unwrap();
    assert!(seed.starts_with("status: draft\n"));
    assert!(
        seed.split("## Open questions and deferrals\n")
            .nth(1)
            .unwrap()
            .split("\n## ")
            .next()
            .unwrap()
            .contains(words.unwrap_or("Existing behavior remains unverified"))
    );
    assert!(!receipt.session.join("seed-confirmation.json").exists());
    assert_eq!(agent.remaining(), 0);
}

#[test]
fn the_prompt_names_exactly_the_fields_the_decoder_reads() {
    let prompt = step::prompt();
    let mut named = std::collections::BTreeSet::new();
    for piece in prompt.split('"').skip(1).step_by(2) {
        if prompt.contains(&format!("\"{piece}\":")) {
            named.insert(piece.to_owned());
        }
    }
    let decoded: std::collections::BTreeSet<_> =
        step::FIELDS.iter().map(|f| f.to_string()).collect();
    assert_eq!(named, decoded);
    for note in include_str!("prompt.txt")
        .lines()
        .filter_map(|line| line.strip_prefix("# "))
    {
        assert!(
            !prompt.contains(note),
            "prompt note reached the model: {note}"
        );
    }
}

/// `shape` with its next step's folder already found.
fn reading(mut shape: Shape, folder: &Path) -> Shape {
    shape.open.as_mut().unwrap().folder = Folder::Known(folder.to_path_buf());
    shape
}

#[test]
fn the_folder_a_step_reads_shows_before_f2_and_f4_picks_another() {
    let dir = tempfile::tempdir().unwrap();
    let (first, other) = (dir.path().join("TEST-first"), dir.path().join("TEST-other"));
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    for stage in [question(), Shaped::Waiting] {
        let mut shape = reading(done(stage).with_agent(Some(agent(1))), &first);
        let text = screen(&mut shape, 80, 24, true);
        assert!(
            text.contains("The step reads files in TEST-first. F4 picks another folder."),
            "{text}"
        );
        assert!(shape.help_text().contains(&format!(
            "The next step reads files in: {}",
            first.display()
        )));
        shape.key(key(KeyCode::F(4)), None);
        assert!(shape.takes_text());
        let text = screen(&mut shape, 80, 24, true);
        // A long folder path wraps, so compare without the wrapping whitespace.
        let squashed: String = text.split_whitespace().collect();
        assert!(
            squashed.contains(&format!("Folder:{}", first.display())),
            "{text}"
        );
        assert!(
            text.contains("Next: F4 uses this folder; Esc keeps TEST-first."),
            "{text}"
        );
        assert_eq!(text.matches("Next:").count(), 1, "{text}");
        assert_eq!(
            text.lines().last().unwrap().trim(),
            "enter newline   F4 use folder   esc cancel"
        );
        // Esc keeps the folder shown; the typed answer is untouched.
        shape.key(key(KeyCode::Char('x')), None);
        shape.key(key(KeyCode::Esc), None);
        assert!(shape.open.is_some(), "Esc while choosing left Shape");
        assert!(screen(&mut shape, 80, 24, true).contains("reads files in TEST-first."));
        assert!(shape.open.as_ref().unwrap().typed.is_empty());
        // A folder that isn't there is refused and the shown one stays.
        shape.key(key(KeyCode::F(4)), None);
        shape.paste("-missing");
        shape.key(key(KeyCode::F(4)), None);
        let text = screen(&mut shape, 100, 30, true);
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains("so the step still reads TEST-first."),
            "{text}"
        );
        assert!(text.contains("reads files in TEST-first."), "{text}");
        // An existing folder is used for the next step.
        shape.key(key(KeyCode::F(4)), None);
        for _ in 0..first.display().to_string().chars().count() + 20 {
            shape.key(key(KeyCode::Backspace), None);
        }
        shape.paste(&other.display().to_string());
        shape.key(key(KeyCode::F(4)), None);
        let text = screen(&mut shape, 100, 30, true);
        assert!(
            text.contains("The step reads files in TEST-other."),
            "{text}"
        );
        assert!(matches!(
            &shape.open.as_ref().unwrap().folder,
            Folder::Known(f) if *f == other.canonicalize().unwrap()
        ));
    }
}

#[test]
fn no_folder_is_offered_without_an_agent_or_a_step_and_no_step_starts_before_it_is_known() {
    let dir = tempfile::tempdir().unwrap();
    let mut read_only = reading(done(question()), dir.path());
    assert!(!screen(&mut read_only, 100, 30, true).contains("F4"));
    read_only.key(key(KeyCode::F(4)), None);
    assert!(read_only.open.as_ref().unwrap().choosing.is_none());
    let seed = Shaped::Drafted {
        revision: Some("r1".into()),
        text: Some("status: draft\n".into()),
    };
    let mut drafted = reading(done(seed).with_agent(Some(agent(1))), dir.path());
    assert!(!screen(&mut drafted, 100, 30, true).contains("F4"));
    let mut finding = done(Shaped::Waiting).with_agent(Some(agent(1)));
    assert!(screen(&mut finding, 100, 30, true).contains("Finding which folder the step reads."));
    finding.key(key(KeyCode::F(2)), None);
    assert!(finding.open.as_ref().unwrap().running.is_none());
    assert!(screen(&mut finding, 100, 30, true).contains("still finding which folder"));
}

#[test]
fn a_question_or_seed_with_nothing_found_is_flagged() {
    let mut shape = done(question());
    let text = screen(&mut shape, 100, 30, true);
    assert!(
        text.contains("Nothing found: this question records nothing checked in the repository."),
        "{text}"
    );
    let mut grounded = done(Shaped::Question {
        id: "storage".into(),
        title: "Where records live".into(),
        ask: "Keep records local or synced?".into(),
        found: vec!["README.md:3 says local only".into()],
        suggestion: None,
    });
    assert!(!screen(&mut grounded, 100, 30, true).contains("Nothing found"));
    let seed = format!(
        "status: draft\n# Seed: TEST\nrevision: r1\n\n## You are confirming\nTEST.\n\n## What's true today\n{}\n",
        crate::seed_me::answering::NOTHING_FOUND
    );
    let mut drafted = done(Shaped::Drafted {
        revision: Some("r1".into()),
        text: Some(seed),
    });
    let text = screen(&mut drafted, 80, 24, true);
    assert!(
        text.contains("Nothing found: this seed records nothing checked in the repository."),
        "{text}"
    );
}

#[test]
fn so_far_scrolls_on_a_short_screen_and_always_says_how_to_close() {
    let mut shape = done(Shaped::Waiting);
    shape.key(key(KeyCode::F(5)), None);
    let text = screen(&mut shape, 80, 14, true);
    assert!(text.contains("Your goal, as you confirmed it"), "{text}");
    assert!(text.contains("↑↓ scroll · F5 close"), "{text}");
    for _ in 0..20 {
        shape.key(key(KeyCode::Down), None);
    }
    let text = screen(&mut shape, 80, 14, true);
    assert!(text.contains("TEST format: Markdown"), "{text}");
    assert!(text.contains("↑↓ scroll · F5 close"), "{text}");
    shape.key(key(KeyCode::F(5)), None);
    assert!(
        shape.open.as_ref().unwrap().so_far.is_none(),
        "F5 didn't close its page"
    );
}

#[test]
fn a_long_answer_never_hides_the_question_or_what_a_running_step_is_doing() {
    let dir = tempfile::tempdir().unwrap();
    let mut shape = reading(done(question()).with_agent(Some(agent(1))), dir.path());
    let answer = (1..=20)
        .map(|i| format!("TEST line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    shape.paste(&answer);
    let text = screen(&mut shape, 80, 24, true);
    assert!(text.contains("Keep records local or synced?"), "{text}");
    assert!(text.contains("TEST line 20"), "{text}");
    shape.key(key(KeyCode::F(2)), None);
    assert!(
        shape.open.as_ref().unwrap().running.is_some(),
        "no step started"
    );
    let text = screen(&mut shape, 80, 24, true);
    assert!(text.contains("Keep records local or synced?"), "{text}");
    assert!(text.contains("Saving your answer."), "{text}");
    assert_eq!(text.lines().last().unwrap().trim(), "esc stop");
}

#[test]
fn a_pasted_cr_breaks_the_row_on_screen_and_the_words_stay_as_pasted() {
    let dir = tempfile::tempdir().unwrap();
    let mut shape = reading(done(question()).with_agent(Some(agent(1))), dir.path());
    shape.paste("TEST one\rTEST two\r\nTEST three");
    let text = screen(&mut shape, 80, 24, true);
    assert!(
        text.contains("  TEST one\n  TEST two\n  TEST three"),
        "{text}"
    );
    assert_eq!(
        shape.open.as_ref().unwrap().typed.text(),
        "TEST one\rTEST two\r\nTEST three"
    );
}

#[test]
fn enter_at_seed_confirmation_guides_without_editing_or_starting_a_step() {
    for words in ["", "TEST I confirm r1.", "TEST pasted\r\nwords"] {
        let mut shape = done(Shaped::Drafted {
            revision: Some("r1".into()),
            text: Some("TEST seed".into()),
        })
        .with_agent(Some(agent(0)));
        shape.paste(words);
        shape.key(key(KeyCode::Enter), None);
        let open = shape.open.as_ref().unwrap();
        assert_eq!(open.typed.text(), words);
        assert!(open.running.is_none());
        assert_eq!(shape.agent.as_ref().unwrap().remaining(), 0);
        assert!(
            screen(&mut shape, 80, 24, true)
                .contains("F3 confirms the seed; Enter does not add a line.")
        );
    }
}

#[test]
fn multiline_confirmation_is_refused_before_a_step_and_keeps_the_words() {
    for words in [
        "TEST yes\nrevision r1",
        "TEST yes\rrevision r1",
        "TEST yes\r\nrevision r1",
        "\n",
        "\r",
        "\r\n",
    ] {
        let mut shape = done(Shaped::Drafted {
            revision: Some("r1".into()),
            text: Some("TEST seed".into()),
        })
        .with_agent(Some(agent(0)));
        shape.paste(words);
        shape.key(key(KeyCode::F(3)), None);
        let open = shape.open.as_ref().unwrap();
        assert!(
            open.running.is_none(),
            "a multiline confirmation started a step"
        );
        assert_eq!(open.typed.text(), words);
        assert_eq!(shape.agent.as_ref().unwrap().remaining(), 0);
        let shown = screen(&mut shape, 80, 24, true);
        assert!(
            shown.contains("Put your confirming words on one line; nothing was confirmed."),
            "{shown}"
        );
    }
}

#[test]
fn folder_edits_keep_multiline_paths_including_trailing_whitespace() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["TEST folder\nnext", "TEST CR\rnext", "TEST CRLF\r\nnext\n "] {
        let folder = dir.path().join(name);
        std::fs::create_dir(&folder).unwrap();
        let mut shape = reading(done(question()).with_agent(Some(agent(0))), dir.path());
        shape.key(key(KeyCode::F(4)), None);
        shape.key(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            None,
        );
        shape.key(
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            None,
        );
        shape.paste(folder.to_str().unwrap());
        shape.key(key(KeyCode::Enter), None);
        assert!(
            shape.open.as_ref().unwrap().choosing.is_some(),
            "Enter applied the folder"
        );
        shape.key(key(KeyCode::Backspace), None);
        shape.key(key(KeyCode::F(4)), None);
        let open = shape.open.as_ref().unwrap();
        assert!(matches!(&open.folder, Folder::Known(p) if *p == folder.canonicalize().unwrap()));
        assert!(open.notice.is_none(), "the exact folder was refused");
        assert!(open.running.is_none());
    }
}

#[test]
fn a_pasted_folder_with_a_trailing_line_break_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("TEST-pasted");
    std::fs::create_dir(&folder).unwrap();
    let mut shape = reading(done(question()).with_agent(Some(agent(0))), dir.path());
    shape.key(key(KeyCode::F(4)), None);
    shape.key(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        None,
    );
    shape.key(
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
        None,
    );
    shape.paste(&format!("  {}\r\n", folder.display()));
    shape.key(key(KeyCode::F(4)), None);
    let open = shape.open.as_ref().unwrap();
    assert!(open.notice.is_none(), "the pasted folder was refused");
    assert!(matches!(&open.folder, Folder::Known(p) if *p == folder.canonicalize().unwrap()));
}

#[test]
fn a_long_editor_follows_its_cursor_and_leaves_half_the_body_for_reading() {
    use ratatui::style::Modifier;
    for (width, height) in [(80, 24), (100, 30), (80, 14)] {
        for no_color in [false, true] {
            let mut stage = question();
            if let Shaped::Question { found, .. } = &mut stage {
                *found = vec![format!("TEST evidence: {}", "reading ".repeat(360))];
            }
            let mut shape = done(stage).with_agent(Some(agent(0)));
            shape.paste(
                &(0..40)
                    .map(|i| format!("TEST row {i}: 界 {}", "long ".repeat(20)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            // Resize down from a larger viewport, then return from the end to the beginning.
            screen(&mut shape, width + 20, height + 6, no_color);
            shape.key(
                KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::ALT),
                None,
            );
            shape.key(key(KeyCode::Home), None);
            let backend = ratatui::backend::TestBackend::new(width, height);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    shape.render(
                        f,
                        Rect::new(2, 3, width - 4, height - 4),
                        Palette::new(no_color),
                    )
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let cursor = (3..height - 1)
                .flat_map(|y| (2..width - 2).map(move |x| (x, y)))
                .find(|&(x, y)| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
                .expect("the editor's cursor was hidden");
            assert_eq!(
                buffer[cursor].symbol(),
                "T",
                "cursor did not return to TEST row 0"
            );
            assert!(
                cursor.1 >= 3 + (height - 4) / 2,
                "the editor displaced more than half the reading"
            );
            assert_eq!(shape.scroll, 0, "arrows scrolled the reading");
            shape.key(key(KeyCode::PageDown), None);
            assert_eq!(shape.scroll, 5, "PgDn did not scroll the reading");
            let shown = screen(&mut shape, width, height, no_color);
            assert!(shown.contains("TEST row 0"), "{shown}");
            assert!(
                shown.lines().last().unwrap().contains("pgup/dn read"),
                "{shown}"
            );
            assert!(
                shown.lines().last().unwrap().contains("F10 menu"),
                "{shown}"
            );
        }
    }
}
