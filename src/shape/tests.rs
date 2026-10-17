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
fn incomplete_lists_keep_the_count_and_both_explanation_routes() {
    for (width, height) in [(100, 30), (80, 24)] {
        for no_color in [false, true] {
            let mut shape = Shape::list();
            shape.offer(Ok(&[]), 1);
            let text = screen(&mut shape, width, height, no_color);
            let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                flat.contains("1 record couldn't be read. F1 or Tend shows why."),
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
    let found = (1..=6)
        .map(|i| {
            format!(
                "src/TEST/{i}.rs:{i}: this path prints the helper's error verbatim, stderr included"
            )
        })
        .collect();
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
        text.contains("  your words\n\n\n  Confirming the seed is not approval to build."),
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
        decode("{\"next\":\"seed\",\"confirming\":\" Build it. \"}").unwrap(),
        Next::Seed {
            confirming: "Build it.".into(),
            found: vec![],
        }
    );
    let Next::Seed { found, .. } = decode(
        "{\"next\":\"seed\",\"confirming\":\"Build it.\",\"found\":[{\"checked\":\"src/lib.rs:9\",\"observed\":\"the palette\"}]}",
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
    // A stub can prove instructions and transport, not a real model's choice of checks.
    let program = temp.path().join("TEST-pi");
    fs::write(
        &program,
        format!(
            r##"#!/usr/bin/env python3
import json, pathlib, sys
request = json.load(sys.stdin)
prompt = sys.argv[sys.argv.index('--system-prompt') + 1]
rules = ['every Done when', 'cmd:', 'expect:', 'cwd:', 'provisional:', 'you judge', 'Do not run',
         'Problem statement', 'Proposed outcome', 'Acceptance criteria', 'Constraints and boundaries',
         'Seed contract (artifact contract)', 'Accepted decisions and assumed defaults']
missing = [rule for rule in rules if rule not in prompt]
pathlib.Path('TEST-missing-rules.json').write_text(json.dumps(missing))
assert not missing, 'missing check-drafting instructions: ' + str(missing)
assert request['goal'] == {goal:?}
assert sys.argv[sys.argv.index('--tools') + 1] == 'read,grep,find,ls'
print(json.dumps({{'next': 'seed', 'confirming': {confirming:?}, 'found': []}}))
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
    assert_eq!(revision.as_deref(), Some("r1"));
    assert!(
        text.contains(confirming),
        "checks changed or dropped: {text}"
    );
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
        "Accepted decisions and assumed defaults",
    ] {
        let declaration = if section == "Knowledge map" {
            "- **Knowledge map**, in plain words, four parts:".to_owned()
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
    for marker in ["cmd:", "expect:", "provisional:"] {
        assert!(
            contract.contains(marker),
            "upstream check syntax changed: {marker}"
        );
    }
    let criteria = text
        .split("## Acceptance criteria\n")
        .nth(1)
        .unwrap()
        .split("\n## ")
        .next()
        .unwrap();
    for check in [
        "Existing output is correct.\ncmd: ./TEST-check\nexpect: TEST existing output\ncwd: .",
        "New output is correct.\nprovisional: Once TEST-new-check exists.\ncmd: ./TEST-new-check\nexpect: TEST new output\ncwd: .",
        "The result feels calm.\nyou judge: Inspect the result for calmness.",
    ] {
        assert!(
            criteria.contains(check),
            "criterion changed or outside Acceptance criteria: {text}"
        );
    }
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
    let renamed = confirming.replace("## Acceptance criteria", "## Checks");
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
    ] {
        assert!(
            answering::draft_seed(
                &skill,
                &receipt.session,
                version,
                &malformed,
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
    // A dated revision must still increment and remain confirmable by the official helper.
    assert_eq!(
        answering::draft_seed(
            &skill,
            &receipt.session,
            version,
            confirming,
            &[],
            temp.path(),
        )
        .unwrap(),
        "r2"
    );
    assert_eq!(
        answering::read(&skill, &receipt.session).unwrap().goal(),
        Some(goal)
    );
    answering::confirm_seed(&skill, &receipt.session, "r2", "TEST yes to this seed only").unwrap();
    assert!(matches!(
        shaping::read(&skill, &receipt.session).unwrap().stage,
        Shaped::Ready { .. }
    ));
    assert!(!temp.path().join("TEST-CHECK-WAS-RUN").exists());
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
                *found = (0..30)
                    .map(|i| format!("TEST evidence {i}: {}", "reading ".repeat(12)))
                    .collect();
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
