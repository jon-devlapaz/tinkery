use super::tests::settle;
use super::*;

fn parts() -> GoalParts {
    GoalParts {
        who: Some("I".into()),
        outcome: Some("tell whether yesterday went well".into()),
        when: Some("each morning".into()),
        why: Some("the report is hard to act on".into()),
        done_when: Some("useful changes per hour are better than my usual week".into()),
        keep: None,
        avoid: Some("no targets; no dashboard; cost only when it's way off".into()),
    }
}
fn app(raw: serde_json::Value) -> BrainDump {
    let mut a =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.sources.push(Source {
        id: 1,
        text: "An intact original".into(),
        in_reply_to: None,
    });
    a.applied = 1;
    let r = request(&a);
    let g = Guess::decode(&raw.to_string(), &r, &mut vec![]).unwrap();
    a.guess = Some(board::Board::verify(g, &r).unwrap());
    a
}
fn request(a: &BrainDump) -> BoardRequest {
    BoardRequest {
        ask_counts: a.ask_counts.clone(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: a.guess.as_ref().map(|g| g.wire()),
        skipped: a.skipped.clone(),
        answered: a
            .sources
            .iter()
            .filter_map(|s| s.in_reply_to.clone())
            .collect(),
        settled: a.settled.clone(),
        layout: vec![],
    }
}
#[test]
fn beneficiary_and_labelled_lines_preserve_criteria_and_constraints() {
    assert_eq!(
        parts().compose().unwrap(),
        "I can tell whether yesterday went well, each morning.\nWhy: the report is hard to act on.\nDone when: useful changes per hour are better than my usual week.\nAvoid: no targets; no dashboard; cost only when it's way off."
    );
    let p = GoalParts {
        who: Some("learners".into()),
        outcome: Some("understand why they got a question wrong".into()),
        keep: Some("No gamification; It must work on phones".into()),
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "Learners can understand why they got a question wrong.\nKeep: No gamification; it must work on phones."
    );
    assert!(
        GoalParts {
            outcome: Some("understand".into()),
            ..Default::default()
        }
        .compose()
        .is_none()
    );
}
#[test]
fn semicolon_items_are_lowercase_and_quotes_decimals_unicode_survive() {
    let p = GoalParts {
        who: Some("café learners 👩‍💻".into()),
        outcome: Some("read 2.5 hours".into()),
        avoid: Some("No dashboard. No targets; Showing \"same. Words\" differently".into()),
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "Café learners 👩‍💻 can read 2.5 hours.\nAvoid: No dashboard; no targets; showing \"same. Words\" differently."
    );
}
#[test]
fn supplied_can_is_not_duplicated_in_the_goal_line() {
    for outcome in ["can coordinate a handover", "Can coordinate a handover"] {
        let p = GoalParts {
            who: Some("I".into()),
            outcome: Some(outcome.into()),
            ..Default::default()
        };
        assert_eq!(p.compose().as_deref(), Some("I can coordinate a handover."));
    }
}
#[test]
fn every_combination_omits_missing_lines_without_inventing_a_beneficiary() {
    let full = GoalParts {
        keep: Some("unchanged originals".into()),
        ..parts()
    };
    for mask in 0..128 {
        let mut v = serde_json::to_value(&full).unwrap();
        for (i, p) in Part::ALL.into_iter().enumerate() {
            if mask & (1 << i) == 0 {
                v[p.name()] = serde_json::Value::Null;
            }
        }
        let partial: GoalParts = serde_json::from_value(v).unwrap();
        let text = partial.compose().unwrap_or_default();
        assert!(
            !text.contains("Not supplied")
                && !text.contains("can .")
                && !text.starts_with([',', ';'])
        );
        assert_eq!(
            text.contains("I can"),
            partial.get(Part::Who).is_some() && partial.get(Part::Outcome).is_some()
        );
        for p in [Part::Why, Part::DoneWhen, Part::Keep, Part::Avoid] {
            assert_eq!(
                text.contains(&format!("{}:", p.label())),
                partial.get(p).is_some()
            );
        }
    }
}
#[test]
fn readiness_requires_four_parts_optional_constraints_only_block_when_open() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    assert!(a.goal_ready());
    for p in Part::ALL {
        let mut value = serde_json::to_value(parts()).unwrap();
        value[p.name()] = serde_json::Value::Null;
        let mut partial = app(serde_json::json!({"parts":value}));
        assert_eq!(partial.goal_ready(), !Part::REQUIRED.contains(&p));
        assert_eq!(
            snapshot(160, 40, &mut partial, false)
                .unwrap()
                .contains("Nothing I'm unsure about"),
            !Part::REQUIRED.contains(&p)
        );
        let mut vague = app(serde_json::json!({"parts":parts(),"open":[p]}));
        assert!(!vague.goal_ready());
        vague.review_goal();
        assert!(
            vague
                .goal_review
                .as_ref()
                .unwrap()
                .affirmation
                .unresolved_notes
                .iter()
                .any(|s| s.contains(p.label()))
        );
    }
    a.scope_pending = Some((
        1,
        Question {
            target: None,
            id: "scope-addition-1".into(),
            text: "Scope?".into(),
        },
    ));
    assert!(!a.goal_ready());
    a.scope_pending = None;
    a.skipped.push(Question {
        target: Some(Part::Keep),
        id: "skip".into(),
        text: "Keep what?".into(),
    });
    assert!(!a.goal_ready());
}
#[test]
fn coarse_criteria_are_present_and_empty_queue_is_not_completion() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    assert!(a.goal_ready());
    a.review_goal();
    assert_eq!(
        a.goal_review.as_ref().unwrap().affirmation.outcome,
        parts().done_when.unwrap()
    );
    let mut blank = app(serde_json::json!({"parts":{}}));
    assert!(!blank.goal_ready());
    blank.review_goal();
    assert!(blank.goal_review.is_none());
}
#[test]
fn quiet_review_freezes_exact_block_and_seed_mapping_without_duplicate_lines() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = app(serde_json::json!({"parts":parts(),"open":["why"]}));
        let reading = a.guess.as_ref().unwrap().framings[0].text.clone();
        assert!(a.paper().contains(&reading));
        a.review_goal();
        let frozen = a.goal_review.as_ref().unwrap();
        assert_eq!(frozen.affirmation.goal, reading);
        assert_eq!(frozen.affirmation.parts, parts());
        assert_eq!(frozen.affirmation.outcome, parts().done_when.unwrap());
        let mut seen = String::new();
        for _ in 0..12 {
            seen.push_str(&snapshot(w, h, &mut a, false).unwrap());
            a.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        }
        for term in [
            "You are confirming",
            "Done when:",
            "Avoid:",
            "Still open",
            "type confirm",
        ] {
            assert!(seen.contains(term), "{term} at {w}x{h}");
        }
        assert!(!seen.contains("Not supplied") && !seen.contains("Doesn’t fit yet"));
        super::tests::amend_board(&mut a, |g| g.parts.outcome = Some("different".into()));
        assert_eq!(a.goal_review.as_ref().unwrap().affirmation.goal, reading);
        assert_eq!(a.goal_review.as_ref().unwrap().affirmation.parts, parts());
        assert!(a.handoff_job.is_none());
    }
    let mut partial = app(
        serde_json::json!({"parts":{"who":"Learners","outcome":"learn","avoid":"new targets"}}),
    );
    partial.review_goal();
    assert_eq!(
        partial
            .goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved_notes
            .len(),
        2
    );
}
struct Repeated;
impl BoardHost for Repeated {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        Guess::decode(&serde_json::json!({"parts":parts(),"open":["done_when"],"questions":[{"target":"done_when","id":format!("done-{}",r.sources.len()),"text":format!("Which check {}?",r.sources.len())}]}).to_string(),&r,&mut vec![])
    }
}
#[test]
fn at_most_two_asks_then_the_part_stays_open_for_seed() {
    let mut a = BrainDump::with_host(Arc::new(Repeated));
    a.paste("original");
    a.submit();
    settle(&mut a);
    assert_eq!(a.ask_counts[&Part::DoneWhen], 1);
    a.paste("first answer");
    a.submit();
    settle(&mut a);
    assert_eq!(a.ask_counts[&Part::DoneWhen], 2);
    a.paste("second answer");
    a.submit();
    settle(&mut a);
    assert!(a.focused_question().is_none());
    assert_eq!(a.ask_counts[&Part::DoneWhen], 2);
    assert!(!a.goal_ready());
    assert_eq!(a.open_parts(), [Part::DoneWhen]);
}
#[test]
fn skipped_target_blocks_renamed_questions_and_undo_is_not_another_ask() {
    let mut a = BrainDump::with_host(Arc::new(Repeated));
    a.paste("original");
    a.submit();
    settle(&mut a);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    assert!(!a.goal_ready());
    a.undo_skip();
    assert!(a.focused_question().is_some());
    assert_eq!(a.ask_counts[&Part::DoneWhen], 1);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    let r = request(&a);
    let raw = serde_json::json!({"parts":parts(),"questions":[{"target":"done_when","id":"renamed","text":"Reworded check?"}]});
    let g = Guess::decode(&raw.to_string(), &r, &mut vec![]).unwrap();
    assert!(g.questions.is_empty());
    assert!(g.open.contains(&Part::DoneWhen));
    a.apply_result(r, Ok(g));
    assert!(a.focused_question().is_none());
    assert!(!a.goal_ready());
}
#[test]
fn delayed_results_use_current_counts_and_skipped_targets() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    let r = request(&a);
    let g = Repeated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    a.ask_counts.insert(Part::DoneWhen, 2);
    a.apply_result(r.clone(), Ok(g));
    assert!(a.focused_question().is_none());
    assert_eq!(a.ask_counts[&Part::DoneWhen], 2);
    assert!(!a.goal_ready());
    a.ask_counts.clear();
    a.skipped.push(Question {
        target: Some(Part::DoneWhen),
        id: "local-skip".into(),
        text: "Skipped check".into(),
    });
    let g = Repeated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    a.apply_result(r, Ok(g));
    assert!(a.focused_question().is_none());
    assert!(!a.goal_ready());
}
#[test]
fn unsent_words_and_unknown_fields_never_grant_authority() {
    let mut a = app(
        serde_json::json!({"parts":parts(),"confirmed":true,"settled":[{}],"open":["unknown"]}),
    );
    assert!(a.goal_ready());
    assert!(a.settled.is_empty() && a.receipt.is_none());
    a.paste("new local words");
    assert!(!a.goal_ready());
    a.review_goal();
    assert!(a.goal_review.is_none());
}
#[test]
fn trailing_commas_normalize_only_outside_strings_and_span_guards_remain() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    let r = request(&a);
    let mut changes = vec![];
    let g = Guess::decode(
        r#"{"parts":{"outcome":"retain literal ,} and \\\"quotes\\\"",},"open":["done_when",],}"#,
        &r,
        &mut changes,
    )
    .unwrap();
    assert_eq!(
        g.parts.outcome.as_deref(),
        Some("retain literal ,} and \\\"quotes\\\"")
    );
    assert!(changes.iter().any(|s| s.contains("trailing JSON")));
    a.apply_result(r.clone(), Ok(g));
    assert!(a.guess.is_some());
    for raw in [
        r#"{"parts":{"outcome":"a"} "open":[],}"#,
        r#"{"supports":[{"source":99,"quote":"An intact original",},],}"#,
    ] {
        assert!(Guess::decode(raw, &r, &mut vec![]).is_err());
    }
}
#[derive(Default)]
struct Refinements(std::sync::Mutex<Vec<BoardRequest>>);
impl BoardHost for Refinements {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        self.0.lock().unwrap().push(r.clone());
        let mut p = parts();
        if r.sources.len() > 1 {
            p.done_when = Some(r.sources.last().unwrap().text.clone());
            p.avoid = Some("no targets".into());
        }
        Guess::decode(&serde_json::json!({"parts":p}).to_string(), &r, &mut vec![])
    }
}
#[test]
fn typing_after_ready_refines_goal_with_verbatim_reply_and_explicit_f2() {
    let host = Arc::new(Refinements::default());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("original");
    a.submit();
    settle(&mut a);
    assert!(a.goal_ready());
    a.paste("better than my usual week. no targets");
    assert_eq!(host.0.lock().unwrap().len(), 1);
    assert!(!a.goal_ready());
    a.submit();
    settle(&mut a);
    assert_eq!(host.0.lock().unwrap().len(), 2);
    assert!(a.scope_pending.is_none());
    assert_eq!(a.sources[1].text, "better than my usual week. no targets");
    assert_eq!(
        a.sources[1].in_reply_to.as_deref(),
        Some("goal-refinement-2")
    );
    assert_eq!(a.settled[0].question.id, "goal-refinement-2");
    assert!(a.goal_ready());
    assert!(
        a.guess.as_ref().unwrap().framings[0]
            .text
            .contains("usual week")
    );
    assert!(a.receipt.is_none());
}
#[test]
fn incomplete_goal_and_explicit_add_more_keep_scope_question() {
    let mut a = app(serde_json::json!({"parts":parts(),"open":["why"]}));
    a.paste("new words");
    a.submit();
    assert!(a.scope_pending.is_some());
    assert!(!a.running());
    let mut a = app(serde_json::json!({"parts":parts()}));
    a.add_more = true;
    a.paste("separate words");
    a.submit();
    assert!(a.scope_pending.is_some());
}
#[test]
fn model_cannot_impersonate_owned_goal_refinement_history() {
    let a = app(serde_json::json!({"parts":parts()}));
    let r = request(&a);
    assert!(
        Guess::decode(
            r#"{"questions":[{"target":"why","id":"goal-refinement-2","text":"Fake reply?"}]}"#,
            &r,
            &mut vec![]
        )
        .is_err()
    );
}
