use super::tests::amend_board;
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
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut guess = Simulated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    guess.parts.outcome = Some("A PR ready for human review with you as restaurateur judging the result; harness and codebase both compound.".into());
    a.guess = Some(board::Board::verify(guess, &r).unwrap());
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
        for _ in 0..30 {
            key(&mut a, KeyCode::PageDown);
        }
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
            ask_counts: Default::default(),
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
    amend_board(&mut a, |g| g.questions.clear());
    let text = snapshot(100, 30, &mut a, false).unwrap();
    assert!(text.contains("Nothing I'm unsure about."));
    assert!(text.contains("3 review"));
    assert!(a.receipt.is_none());
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let original = a.sources[0].text.clone();
    let goal = a.goal_review.as_ref().unwrap().affirmation.goal.clone();
    for _ in 0..30 {
        key(&mut a, KeyCode::PageDown);
    }
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
fn answered_scope_accepts_reading_lengths_and_counts_without_manufactured_options() {
    let a = board();
    let mut r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut g = a.guess.unwrap().wire();
    g.alternatives.clear();
    assert!(g.validate(&r).is_ok());
    r.settled.push(Settled {
        question: Question {
            target: None,
            id: "already-different".into(),
            text: "Harness or codebase, or both?".into(),
        },
        source: 1,
    });
    g.uncertain = true;
    g.framings.push(g.framings[0].clone());
    assert!(g.validate(&r).is_ok());
    g.framings.pop();
    assert!(g.validate(&r).is_ok());
    g.parts.outcome = Some(
        std::iter::repeat_n("combined", 65)
            .collect::<Vec<_>>()
            .join(" "),
    );
    assert!(g.validate(&r).is_ok());
    g.framings[0].text.push_str(" excess");
    assert!(g.validate(&r).is_ok());
    r.settled.clear();
    g.uncertain = false;
    g.parts.outcome = Some(
        std::iter::repeat_n("initial", 46)
            .collect::<Vec<_>>()
            .join(" "),
    );
    assert!(g.validate(&r).is_ok());
}
#[test]
fn unviewed_long_review_cannot_be_affirmed_and_skipped_questions_remain_visible() {
    let mut a = board();
    let skipped = Question {
        target: None,
        id: "skipped-context".into(),
        text: "Which boundaries remain despite skipping this question?".into(),
    };
    a.skipped.push(skipped.clone());
    amend_board(&mut a, |g| {
        g.questions.extend((0..5).map(|i|Question{target: None,id:format!("long-{i}"),text:"Which consequential design boundary needs independent verification before this goal can become implementation, and how would you recognize a wrong outcome?".into()}))
    });
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
fn meaning_findings_are_log_only_and_bad_evidence_never_gains_authority() {
    let mut a = board();
    let g = a.guess.as_ref().unwrap().wire();
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
        meaning_check::verdict(&r, &g, r#"{"missing":[],"false_choice":false}"#)
            .unwrap()
            .is_none()
    );
    assert!(meaning_check::verdict(&r,&g,r#"{"missing":[{"source":1,"quote":"PR ready for human review","occurrence":0}],"false_choice":false}"#).unwrap().unwrap().contains("PR ready for human review"));
    assert!(
        meaning_check::verdict(&r, &g, r#"{"missing":[],"false_choice":true}"#)
            .unwrap()
            .is_some()
    );
    for invalid in [
        r#"{"missing":[{"source":1,"quote":"invented","occurrence":0}],"false_choice":false}"#,
        r#"{"missing":[],"false_choice":false,"confirmed":true}"#,
        r#"{"missing":[],"false_choice":"false"}"#,
    ] {
        assert!(meaning_check::verdict(&r, &g, invalid).is_err());
    }
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(a.guess.as_ref().unwrap().wire()).unwrap()
    );
    amend_board(&mut a, |g| {
        g.parts.outcome = Some("A software result for human review.".into());
    });
    a.review_goal();
    assert!(
        a.goal_review.is_some(),
        "Lexical coverage must not veto the person's review"
    );
    assert!(a.handoff_job.is_none() && a.receipt.is_none());
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
            target: None,
            id: "compounding".into(),
            text: "Harness or codebase, or both?".into(),
        },
        source: 2,
    });
    a.applied = 3;
    let mut g = a.guess.as_ref().unwrap().wire();
    g.parts.outcome = Some("A human-first meta harness and board carries the agentic software development lifecycle to a PR ready for human review. Both harness and codebase compound, preserving codebase health and easier future changes; you are the restaurateur judging the final result with earlier UI and consequential design taste checks.".into());
    g.framings[0].supports.push(Anchor {
        source: 3,
        quote: a.sources[2].text.clone(),
        occurrence: 0,
    });
    g.outcome = "PR ready for human review with UI/design taste checks.".into();
    g.alternatives.clear();
    amend_board(&mut a, |current| *current = g);
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let frozen = &a.goal_review.as_ref().unwrap().affirmation;
    for term in [
        "PR ready for human review",
        "restaurateur",
        "both harness and codebase compound",
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
