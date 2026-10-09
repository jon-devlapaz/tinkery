use super::*;
pub(super) fn right_text(screen: &str, width: u16) -> String {
    screen
        .lines()
        .map(|line| {
            line.chars()
                .skip(usize::from(width * 48 / 100 + 2))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub(super) fn board() -> BrainDump {
    let mut a =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.sources.push(Source{id:1,text:"PR ready for human review; restaurateur, not cook; both harness and codebases compound.".into(),in_reply_to:None});
    a.applied = 1;
    let r = BoardRequest {
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut guess = Simulated.reshape(r, &AtomicBool::new(false)).unwrap();
    guess.framings[0].text="A PR ready for human review with you as restaurateur judging the result; harness and codebase both compound.".into();
    a.guess = Some(guess);
    a
}
fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
#[test]
fn goal_review_lists_unresolved_questions_and_requires_distinct_full_review_affirmation() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        let q = a.focused_question().unwrap().text.clone();
        a.review_goal();
        let text = snapshot(w, h, &mut a, false).unwrap();
        assert!(text.contains("type confirm"));
        assert!(text.contains("Running out of questions"));
        assert!(right_text(&text, w).contains("doesn't mean I understood you."));
        key(&mut a, KeyCode::PageDown);
        let scrolled = snapshot(w, h, &mut a, false).unwrap();
        assert!(right_text(&scrolled, w).contains(&q));
        assert!(a.handoff_job.is_none());
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::F(2));
        assert!(
            a.handoff_job.is_none(),
            "Opening review/Enter/F2 affirmed implicitly"
        );
        a.paste("confirm\n");
        assert!(a.goal_review.as_ref().unwrap().input.is_empty());
        key(&mut a, KeyCode::Esc);
        assert!(a.goal_review.is_none());
        assert!(a.receipt.is_none());
    }
}
#[test]
fn confirmation_is_unavailable_for_practice_pending_sources_or_unsent_text() {
    let mut a = BrainDump::default();
    a.review_goal();
    assert!(a.goal_review.is_none());
    let mut a = board();
    a.paste("new local text");
    a.review_goal();
    assert!(a.goal_review.is_none());
    a.input = Note::new("");
    a.applied = 0;
    a.review_goal();
    assert!(a.goal_review.is_none());
    a.applied = 1;
    a.ready = Some((
        BoardRequest {
            sources: a.sources.clone(),
            fragments: vec![],
            previous: None,
            skipped: vec![],
            answered: vec![],
            settled: vec![],
            layout: vec![],
        },
        Err("test".into()),
    ));
    a.review_goal();
    assert!(a.goal_review.is_none());
}
#[test]
fn empty_questions_show_review_route_but_never_confirm_and_missing_helper_preserves_source() {
    let mut a = board();
    a.guess.as_mut().unwrap().questions.clear();
    let text = snapshot(100, 30, &mut a, false).unwrap();
    assert!(text.contains("Review goal"));
    assert!(a.receipt.is_none());
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let original = a.sources[0].text.clone();
    let goal = a.goal_review.as_ref().unwrap().affirmation.goal.clone();
    a.paste("confirm");
    key(&mut a, KeyCode::Enter);
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while a.running() && std::time::Instant::now() < end {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!a.running());
    assert!(a.receipt.is_none());
    assert!(a.notice.contains("Handoff not complete"));
    assert_eq!(a.sources[0].text, original);
    assert_eq!(a.goal_review.as_ref().unwrap().affirmation.goal, goal);
    assert!(a.goal_config.as_ref().unwrap().recovery_path().is_none());
}
#[test]
fn answered_scope_requires_one_combined_reading_and_allows_no_manufactured_options() {
    let a = board();
    let mut r = BoardRequest {
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut g = a.guess.unwrap();
    g.alternatives.clear();
    assert!(g.validate(&r).is_ok());
    r.settled.push(Settled {
        question: Question {
            id: "already-different".into(),
            text: "Harness or codebase, or both?".into(),
        },
        source: 1,
    });
    g.uncertain = true;
    g.framings.push(g.framings[0].clone());
    assert!(g.validate(&r).is_err());
    g.framings.pop();
    assert!(g.validate(&r).is_ok());
    g.framings[0].text = std::iter::repeat_n("combined", 65)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(g.validate(&r).is_ok());
    g.framings[0].text.push_str(" excess");
    assert!(g.validate(&r).is_err());
    r.settled.clear();
    g.uncertain = false;
    g.framings[0].text = std::iter::repeat_n("initial", 46)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(g.validate(&r).is_err());
}
#[test]
fn unviewed_long_review_cannot_be_affirmed_and_skipped_questions_remain_visible() {
    let mut a = board();
    let skipped = Question {
        id: "skipped-context".into(),
        text: "Which boundaries remain despite skipping this question?".into(),
    };
    a.skipped.push(skipped.clone());
    a.guess.as_mut().unwrap().questions.extend((0..5).map(|i|Question{id:format!("long-{i}"),text:"Which consequential design boundary needs independent verification before this goal can become implementation, and how would you recognize a wrong outcome?".into()}));
    a.review_goal();
    snapshot(80, 24, &mut a, false).unwrap();
    assert!(a.goal_review.as_ref().unwrap().max > 0);
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == skipped.id)
    );
    a.paste("confirm");
    key(&mut a, KeyCode::Enter);
    assert!(a.handoff_job.is_none());
    assert!(a.notice.contains("Read the complete review"));
}

#[test]
fn meaning_audit_rejects_literal_noun_loss_false_choices_and_bad_evidence() {
    let a = board();
    let r = BoardRequest {
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    assert!(meaning_check::apply(&r, r#"{"missing":[],"false_choice":false}"#).is_ok());
    assert!(meaning_check::apply(&r,r#"{"missing":[{"source":1,"quote":"PR ready for human review","occurrence":0}],"false_choice":false}"#).unwrap_err().contains("PR ready for human review"));
    assert!(meaning_check::apply(&r, r#"{"missing":[],"false_choice":true}"#).is_err());
    assert!(
        meaning_check::apply(
            &r,
            r#"{"missing":[{"source":1,"quote":"invented","occurrence":0}],"false_choice":false}"#
        )
        .is_err()
    );
    assert!(
        meaning_check::apply(
            &r,
            r#"{"missing":[],"false_choice":false,"confirmed":true}"#
        )
        .is_err()
    );
    assert!(meaning_check::apply(&r, r#"{"missing":[],"false_choice":"false"}"#).is_err());
    assert!(meaning_check::acronyms(&r.sources).contains("PR"));
    assert!(!meaning_check::retains(
        "A software result ready for review.",
        "PR"
    ));
    assert!(!meaning_check::retains("Not APR or PRs.", "PR"));
    assert!(meaning_check::retains("A PR ready for human review.", "PR"));
    let mut a = board();
    a.guess.as_mut().unwrap().framings[0].text = "A software result for human review.".into();
    a.review_goal();
    assert!(a.goal_review.is_none());
    assert!(a.notice.contains("lost authored term PR"));
}

#[test]
fn literal_scope_and_checkpoint_terms_are_supported_without_promoting_candidates() {
    let mut a = board();
    a.sources.push(Source {
        id: 2,
        text: "both".into(),
        in_reply_to: Some("compounding".into()),
    });
    a.sources.push(Source{id:3,text:"judge the final result, with earlier taste checks on UI and consequential design choices.".into(),in_reply_to:None});
    a.settled.push(Settled {
        question: Question {
            id: "compounding".into(),
            text: "Harness or codebase, or both?".into(),
        },
        source: 2,
    });
    a.applied = 3;
    let g = a.guess.as_mut().unwrap();
    g.framings[0].text="A human-first meta harness and board carries the agentic software development lifecycle to a PR ready for human review. Both harness and codebase compound, preserving codebase health and easier future changes; you are the restaurateur judging the final result with earlier UI and consequential design taste checks.".into();
    g.framings[0].supports.push(Anchor {
        source: 3,
        quote: a.sources[2].text.clone(),
        occurrence: 0,
    });
    g.outcome = "PR ready for human review with UI/design taste checks.".into();
    g.alternatives.clear();
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let frozen = &a.goal_review.as_ref().unwrap().affirmation;
    for term in [
        "PR ready for human review",
        "restaurateur",
        "Both harness and codebase compound",
        "UI",
        "design taste checks",
    ] {
        assert!(frozen.goal.contains(term));
    }
    assert_eq!(frozen.sources[2].text, a.sources[2].text);
    assert!(frozen.options.is_empty());
    assert_eq!(frozen.answered[0].source, 2);
    assert!(a.handoff_job.is_none());
}
