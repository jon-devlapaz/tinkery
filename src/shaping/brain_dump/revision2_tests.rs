use super::c16_tests::{app, parts};
use super::tests::settle;
use super::*;

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
fn repeated_check_is_missing_and_required_question_precedes_vague_outcome() {
    let a = app(
        serde_json::json!({"parts":{"who":"My mom","outcome":"spots a fake text before tapping anything","why":"scam texts arrive","done_when":"My mom spots a fake text before tapping anything"},"open":["outcome"],"questions":[{"target":"outcome","text":"Did I get it?"}]}),
    );
    let g = a.guess.unwrap();
    assert!(g.parts.done_when.is_none());
    assert_eq!(g.questions[0].target, Some(Part::DoneWhen));
    assert!(!g.questions[0].text.contains("Did I get it"));
    let a = app(
        serde_json::json!({"parts":{"who":"New users","outcome":"return","why":"return"},"questions":[{"target":"outcome","text":"Should this be the goal?"}]}),
    );
    assert!(a.guess.as_ref().unwrap().parts.why.is_none());
    assert_eq!(a.focused_question().unwrap().target, Some(Part::Why));
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
        "My mom can spot a fake before tapping anything.\nMust: one step.\nMust: simple.\nMust not: require installing an app."
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
    let g = a.guess.unwrap();
    assert!(g.framings[0].supports.is_empty());
    assert!(
        g.unresolved_notes
            .iter()
            .any(|s| s.contains("possible explanation"))
    );
    assert!(g.unresolved_notes.iter().any(|s| s.contains("unverified")));
    let a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
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
        sources: a.sources.clone(),
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
    let reading = a.guess.as_ref().unwrap().framings[0].text.clone();
    assert!(!reading.contains("renaming") && !reading.contains("editing"));
    let paper = a.paper();
    assert!(paper.contains("also in your dump: renaming a tab; release builds"));
    assert!(paper.contains("for the seed: editing can wait"));
    let text = snapshot(160, 40, &mut a, false).unwrap();
    assert!(text.contains("anything to add or change?"));
    a.review_goal();
    let review = a.goal_review.as_ref().unwrap();
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
    assert!(a.changed_lines.is_none());
    a.paste("answer");
    a.submit();
    settle(&mut a);
    assert!(
        a.changed_lines
            .as_ref()
            .unwrap()
            .contains("Why: a different problem.")
    );
    let unchanged = a.guess.as_ref().unwrap().framings[0]
        .text
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
        let unchanged_cell = &buffer[(a.agent_area.x, a.agent_area.y + 1)];
        let changed_cell = &buffer[(a.agent_area.x, a.agent_area.y + 2)];
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
    assert!(a.changed_lines.is_none());
    a.changed_lines = Some(Default::default());
    a.paste("more");
    assert!(a.changed_lines.is_none());
    a.changed_lines = Some(Default::default());
    a.submit();
    assert!(a.changed_lines.is_none());
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
    assert!(a.changed_lines.is_none());
    let palette = Palette::new(true);
    assert_eq!(
        a.reading_line_style(&a.guess.as_ref().unwrap().framings[0].text, palette),
        palette.ink
    );
}
