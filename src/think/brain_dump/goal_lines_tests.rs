use super::readiness_tests::{app, app_with_source, parts};
use super::tests::settle;
use super::*;

/// Scripted classifications prove display/handoff mechanics, not prompt compliance or semantic repair.
#[test]
fn recorded_169_exclusions_and_affirmative_deferrals_keep_their_scripted_categories() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("exclusion_replay.json")).unwrap();
    let words = fixture["words"].as_str().unwrap();
    let recorded: serde_json::Value =
        serde_json::from_str(fixture["raw"].as_str().unwrap()).unwrap();
    let excluded = [
        "fix incomplete discovery (#167 F2) in this slice",
        "fix process lifetime (F3) in this slice",
        "fix stale return (#165) in this slice",
        "add selected-detail verification in this slice",
    ];
    // Negative control: the old model reply stays misclassified; no decoder heuristic repairs it.
    let old = app_with_source(recorded.clone(), words);
    assert_eq!(old.conversation.board().unwrap().deferred.len(), 4);
    assert!(
        !old.conversation
            .board()
            .unwrap()
            .parts
            .must_not
            .iter()
            .any(|v| v.contains("incomplete discovery"))
    );
    assert_eq!(old.conversation.sources()[0].text, words);

    for later in [None, Some("After this slice, fix stale return (#165).")] {
        let source = match later {
            Some(request) => format!("{words}\n{request}"),
            None => words.to_owned(),
        };
        // Deliberately faithful fixture, not a response produced or corrected by the prompt.
        let mut faithful = recorded.clone();
        faithful["parts"]["must_not"]
            .as_array_mut()
            .unwrap()
            .extend(excluded.iter().map(|v| serde_json::json!(v)));
        faithful["deferred"] = serde_json::json!(later.into_iter().collect::<Vec<_>>());
        let mut a = app_with_source(faithful, &source);
        let board = a.conversation.board().unwrap();
        let goal = board.goal().unwrap().to_owned();
        for item in excluded {
            assert!(board.parts.must_not.iter().any(|v| v == item));
            assert!(goal.contains(&format!("Must not: {item}.")));
            assert!(!board.parts.must.iter().any(|v| v.contains(item)));
        }
        assert_eq!(board.deferred, later.into_iter().collect::<Vec<_>>());
        let paper = a.copy_text();
        assert!(!paper.contains("for the seed: Fix incomplete discovery"));
        if let Some(request) = later {
            assert!(paper.contains(&format!("for the seed: {request}")));
        }
        a.review_goal();
        let frozen = &a.handoff.review.as_ref().unwrap().affirmation;
        assert_eq!(frozen.goal, goal);
        assert_eq!(frozen.deferred, later.into_iter().collect::<Vec<_>>());
        assert_eq!(a.conversation.sources()[0].text, source);
    }
}

#[test]
fn possessive_subject_in_stub_outcome_is_composed_once() {
    const ORIGINAL: &str = "Shape's goal list says when some sessions couldn't be read, instead of claiming nothing is waiting. ...";
    for subject in ["Shape's", "Shape’s"] {
        let headline = format!("{subject} goal list shows an incomplete-read notice");
        let mut a = app_with_source(
            serde_json::json!({"parts": {
                "who": "Shape", "outcome": headline
            }}),
            ORIGINAL,
        );
        let goal = format!("{headline}.");
        assert_eq!(a.conversation.board().unwrap().goal(), Some(goal.as_str()));
        assert_eq!(a.conversation.sources()[0].text, ORIGINAL);
        for mono in [false, true] {
            let screen = snapshot(320, 40, &mut a, mono).unwrap();
            assert!(screen.contains(&goal));
            assert!(!screen.contains("Shape Shape"));
        }
        assert!(a.copy_text().contains(&goal));
        a.review_goal();
        assert_eq!(a.handoff.review.as_ref().unwrap().affirmation.goal, goal);
        assert_eq!(a.conversation.sources()[0].text, ORIGINAL);
    }
}
#[test]
fn known_intent_leads_without_inventing_a_beneficiary() {
    let original =
        "Turn observed product problems into verified improvements. Start with one product.";
    let mut a = app_with_source(
        serde_json::json!({"parts": {
            "outcome": "turn observed product problems into verified improvements",
            "why": "shipping irrelevant software wastes effort",
            "done_when": "evidence shows whether the improvement goal was achieved",
            "must": ["start with one product"]
        }, "questions": []}),
        original,
    );
    let board = a.conversation.board().unwrap();
    let goal = board.goal().unwrap().to_owned();
    assert!(goal.starts_with("Turn observed product problems into verified improvements.\nWhy:"));
    assert!(board.parts.who.is_none());
    assert!(!a.goal_ready());
    assert!(board.questions.is_empty());
    assert!(goal.contains("Must: start with one product."));
    assert!(a.copy_text().contains(&goal));
    assert!(
        snapshot(200, 40, &mut a, true)
            .unwrap()
            .contains("Turn observed product problems")
    );
    a.review_goal();
    assert_eq!(a.handoff.review.as_ref().unwrap().affirmation.goal, goal);
    assert_eq!(a.conversation.sources()[0].text, original);
}

#[test]
fn consequential_question_survives_an_unrelated_missing_beneficiary() {
    let a = app(serde_json::json!({"parts": {
        "outcome": "turn observed problems into verified improvements",
        "why": "shipping irrelevant software wastes effort",
        "done_when": "evidence shows whether the goal was achieved"
    }, "decisions": [{
        "part": "must_not", "text": "Whether a proposed experiment may change production",
        "answers": ["isolated experiments only", "production changes after approval"]
    }], "questions": [{"target": "must_not", "text": "May the first experiment change production after approval?"}]}));
    let board = a.conversation.board().unwrap();
    assert!(board.parts.who.is_none());
    assert_eq!(board.questions[0].target, Some(Part::MustNot));
    assert_eq!(
        board.questions[0].text,
        "May the first experiment change production after approval?"
    );
    assert!(!a.goal_ready());
}

#[test]
fn missing_beneficiary_does_not_reopen_a_settled_outcome() {
    let a = app(serde_json::json!({"parts": {
        "outcome": "turn observed problems into verified improvements",
        "why": "shipping irrelevant software wastes effort",
        "done_when": "evidence shows whether the goal was achieved"
    }, "questions": [{"target": "outcome", "text": "Should this instead just write code?"}]}));
    let board = a.conversation.board().unwrap();
    assert!(board.questions.is_empty());
    assert!(board.parts.who.is_none());
    assert!(!a.goal_ready());
}

#[test]
fn subject_prefix_without_a_boundary_is_not_removed() {
    for outcome in [
        "Shapes stay distinct",
        "Shape'sake stays literal",
        "Shape’sake stays literal",
    ] {
        let p = GoalParts {
            who: Some("Shape".into()),
            outcome: Some(outcome.into()),
            ..Default::default()
        };
        assert_eq!(p.goal_line().unwrap(), format!("{outcome}, for Shape"));
    }
}
#[test]
fn standalone_outcomes_keep_spelling_through_display_copy_and_confirmation() {
    for (who, outcome, when, headline) in [
        (
            "developers",
            "Count whitespace-separated words in one UTF-8 text file and print the count.",
            Some("in this README-only scratch repo"),
            "Count whitespace-separated words in one UTF-8 text file and print the count, for developers, in this README-only scratch repo.",
        ),
        (
            "I",
            "Read a local count.",
            None,
            "Read a local count, for me.",
        ),
        (
            "I",
            "Count words.",
            Some("For the first slice"),
            "Count words, for me. For the first slice.",
        ),
        (
            "I",
            "Count words.",
            Some("London only"),
            "Count words, for me. London only.",
        ),
        (
            "I",
            "count words",
            Some("for the first slice"),
            "I count words, for the first slice.",
        ),
        (
            "developers",
            "Tally prints a local count.",
            None,
            "Tally prints a local count, for developers.",
        ),
        (
            "People who sign up",
            "return for a second session",
            None,
            "People who sign up return for a second session.",
        ),
    ] {
        let original = format!("{who}\n{outcome}");
        let mut a = app_with_source(
            serde_json::json!({"parts": {"who": who, "outcome": outcome, "when": when}}),
            &original,
        );
        assert_eq!(a.conversation.board().unwrap().goal(), Some(headline));
        for mono in [false, true] {
            assert!(snapshot(320, 40, &mut a, mono).unwrap().contains(headline));
        }
        assert!(a.copy_text().contains(headline));
        a.review_goal();
        assert_eq!(
            a.handoff.review.as_ref().unwrap().affirmation.goal,
            headline
        );
        assert_eq!(a.conversation.sources()[0].text, original);
    }
}

#[test]
fn outcome_is_a_behaviour_without_forced_can() {
    let p = GoalParts {
        who: Some("People who sign up".into()),
        outcome: Some("return for a second session".into()),
        ..Default::default()
    };
    assert_eq!(
        p.compose().as_deref(),
        Some("People who sign up return for a second session.")
    );
}
#[test]
fn repeated_checks_stay_missing_without_displacing_open_intent() {
    let a = app(
        serde_json::json!({"parts":{"who":"My mom","outcome":"spots a fake text before tapping anything","why":"scam texts arrive","done_when":"My mom spots a fake text before tapping anything"},"open":["outcome"],"questions":[{"target":"outcome","text":"Which suspicious messages should this cover?"}]}),
    );
    let g = a.conversation.board().unwrap();
    assert!(g.parts.done_when.is_none());
    assert_eq!(g.questions[0].target, Some(Part::Outcome));
    assert_eq!(
        g.questions[0].text,
        "Which suspicious messages should this cover?"
    );
    assert!(!a.goal_ready());
    let a = app(
        serde_json::json!({"parts":{"who":"New users","outcome":"return","why":"return"},"questions":[{"target":"outcome","text":"Should this be the goal?"}]}),
    );
    assert!(a.conversation.board().unwrap().parts.why.is_none());
    assert!(a.conversation.focused_question().is_none());
    assert!(!a.goal_ready());
}
#[test]
fn timing_and_capacity_are_not_duplicated_and_constraints_each_get_a_line() {
    let p = GoalParts {
        who: Some("My mom".into()),
        outcome: Some("can can spot a fake before tapping anything".into()),
        when: Some("before tapping anything".into()),
        must: vec!["one step; one step".into(), "simple".into()],
        must_not: vec!["require installing an app".into()],
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "My mom can spot a fake before tapping anything.\nMust: one step; one step.\nMust: simple.\nMust not: require installing an app."
    );
}
#[test]
fn repeated_subjects_and_uncited_supports_are_not_fabricated_citations() {
    let p = GoalParts {
        who: Some("New users".into()),
        outcome: Some("new users return".into()),
        ..Default::default()
    };
    assert_eq!(p.compose().as_deref(), Some("New users return."));
    let a = app(
        serde_json::json!({"parts":parts(),"supports":["A possible explanation",{"option":"Try a mechanism"}],"open":["A report remains unverified"]}),
    );
    let g = a.conversation.board().unwrap();
    assert!(g.reading.as_ref().unwrap().supports.is_empty());
    assert!(
        g.unresolved_notes
            .iter()
            .any(|s| s.contains("possible explanation"))
    );
    assert!(g.unresolved_notes.iter().any(|s| s.contains("unverified")));
    let a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.conversation.sources().to_vec(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    assert!(
        Guess::decode(
            r#"{"supports":[{"source":99,"quote":"invented"}]}"#,
            &r,
            &mut vec![]
        )
        .is_err()
    );
}
#[test]
fn fact_and_owned_history_guards_run_before_question_priority_filtering() {
    let a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.conversation.sources().to_vec(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut g = Guess::decode(
        r#"{"parts":{"who":"I","outcome":"return"}}"#,
        &r,
        &mut vec![],
    )
    .unwrap();
    g.questions = vec![Question {
        target: Some(Part::Outcome),
        id: "scope-addition-9".into(),
        text: "wrong target history".into(),
    }];
    assert!(board::Board::verify(g, &r).is_err());
}
#[test]
fn quiet_asides_survive_paper_review_and_are_not_constraints() {
    let mut a = app(
        serde_json::json!({"parts":parts(),"other_goals":["renaming a tab","release builds"],"deferred":["editing can wait"]}),
    );
    assert!(a.goal_ready());
    let reading = a.conversation.board().unwrap().goal().unwrap().to_owned();
    assert!(!reading.contains("renaming") && !reading.contains("editing"));
    let paper = a.copy_text();
    assert!(paper.contains("also in your dump: renaming a tab; release builds"));
    assert!(paper.contains("for the seed: editing can wait"));
    let text = snapshot(160, 40, &mut a, false).unwrap();
    assert!(text.contains("anything to add or change?"));
    a.review_goal();
    let review = a.handoff.review.as_ref().unwrap();
    assert_eq!(
        review.affirmation.other_goals,
        ["renaming a tab", "release builds"]
    );
    assert_eq!(review.affirmation.deferred, ["editing can wait"]);
    assert!(review.affirmation.unresolved_notes.is_empty());
    assert_eq!(review.affirmation.goal, reading);
}
#[test]
fn open_notes_cannot_contradict_ready_and_self_labels_never_render() {
    let mut a = app(
        serde_json::json!({"parts":parts(),"unresolved_notes":["PROVISIONAL assumption: a boundary is unclear"]}),
    );
    assert!(!a.goal_ready());
    let reading = snapshot(160, 40, &mut a, false).unwrap();
    assert!(!reading.contains("Nothing I'm unsure about"));
    assert!(reading.contains("Some parts remain open"));
    assert!(reading.contains("anything to add or change?"));
    a.review_goal();
    let text = snapshot(160, 40, &mut a, false).unwrap();
    assert!(text.contains("Still open"));
    assert!(
        !text.contains("PROVISIONAL")
            && !text.contains("Running out of questions")
            && !text.contains("Creates a Seed Me session")
    );
    assert!(text.contains("type confirm to save. You can't edit it after."));
}
struct Change {
    same: bool,
}
impl BoardHost for Change {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        let mut p = parts();
        if r.sources.len() > 1 && !self.same {
            p.why = Some("a different problem".into());
        }
        Guess::decode(&serde_json::json!({"parts":p}).to_string(), &r, &mut vec![])
    }
}
#[test]
fn changed_lines_ink_unchanged_muted_until_any_keystroke_paste_or_send() {
    let mut a = BrainDump::with_host(Arc::new(Change { same: false }));
    a.paste("original");
    a.submit();
    settle(&mut a);
    assert!(a.conversation.changed_lines().is_none());
    a.paste("answer");
    a.submit();
    settle(&mut a);
    assert!(
        a.conversation
            .changed_lines()
            .unwrap()
            .contains("Why: a different problem.")
    );
    let unchanged = a
        .conversation
        .board()
        .unwrap()
        .goal()
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    for mono in [false, true] {
        let palette = Palette::new(mono);
        assert_eq!(
            a.reading_line_style("Why: a different problem.", palette),
            palette.ink
        );
        assert_eq!(a.reading_line_style(&unchanged, palette), palette.muted);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(320, 40)).unwrap();
        terminal.draw(|f| render(f, &mut a, palette)).unwrap();
        let buffer = terminal.backend().buffer();
        let unchanged_cell = &buffer[(a.screen.agent_area.x, a.screen.agent_area.y + 1)];
        let changed_cell = &buffer[(a.screen.agent_area.x, a.screen.agent_area.y + 2)];
        if mono {
            assert!(
                unchanged_cell
                    .modifier
                    .contains(ratatui::style::Modifier::DIM)
            );
            assert!(
                !changed_cell
                    .modifier
                    .contains(ratatui::style::Modifier::DIM)
            );
        } else {
            assert_eq!(unchanged_cell.fg, palette.muted.fg.unwrap());
            assert_eq!(changed_cell.fg, palette.ink.fg.unwrap());
        }
    }
    a.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert!(a.conversation.changed_lines().is_none());
    a.conversation.set_changed_lines(Some(Default::default()));
    a.paste("more");
    assert!(a.conversation.changed_lines().is_none());
    a.conversation.set_changed_lines(Some(Default::default()));
    a.submit();
    assert!(a.conversation.changed_lines().is_none());
    settle(&mut a);
}
#[test]
fn unchanged_answer_never_mutes_the_whole_block() {
    let mut a = BrainDump::with_host(Arc::new(Change { same: true }));
    a.paste("original");
    a.submit();
    settle(&mut a);
    a.paste("answer");
    a.submit();
    settle(&mut a);
    assert!(a.conversation.changed_lines().is_none());
    let palette = Palette::new(true);
    assert_eq!(
        a.reading_line_style(a.conversation.board().unwrap().goal().unwrap(), palette),
        palette.ink
    );
}
