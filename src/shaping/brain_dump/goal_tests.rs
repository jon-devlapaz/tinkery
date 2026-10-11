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
fn receipt_preserves_frozen_review_goal_in_ink_and_only_offers_manual_actions() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        a.review_goal();
        let frozen = a.goal_review.as_ref().unwrap().affirmation.goal.clone();
        let mut receipt = handoff::Receipt::test("/tmp/TEST-receipt/session".into());
        receipt.goal = frozen.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        a.handoff_job = Some(rx);
        tx.send(Ok(receipt)).unwrap();
        a.guess = None;
        a.goal_tick();
        assert!(a.goal_review.is_none());
        assert_eq!(a.receipt.as_ref().unwrap().goal, frozen);
        let text = snapshot(w, h, &mut a, false).unwrap();
        let right = right_text(&text, w);
        assert!(
            right.contains(&frozen.split_whitespace().collect::<Vec<_>>().join(" ")),
            "{right}"
        );
        assert!(!text.contains("later guess"));
        assert!(!text.contains("/tmp/TEST-receipt"));
        assert!(!text.contains("seed not written yet"));
        let backend = ratatui::backend::TestBackend::new(w, h);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let palette = Palette::new(false);
        terminal
            .draw(|f| super::render(f, &mut a, palette))
            .unwrap();
        let buffer = terminal.backend().buffer();
        for y in a.agent_area.y..a.agent_area.bottom() {
            for x in a.agent_area.x..a.agent_area.right() {
                let cell = &buffer[(x, y)];
                if cell.symbol() != " " {
                    assert_eq!(cell.fg, palette.ink.fg.unwrap());
                }
            }
        }
        for code in [
            KeyCode::F(2),
            KeyCode::F(3),
            KeyCode::F(4),
            KeyCode::F(5),
            KeyCode::F(6),
            KeyCode::F(8),
            KeyCode::F(9),
            KeyCode::Esc,
            KeyCode::Char('q'),
        ] {
            key(&mut a, code);
        }
        for c in ['n', 'g', 'o', 'd', 'c'] {
            a.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
        }
        assert!(!a.quit && !a.original && !a.details && !a.running());
        assert!(a.handoff_job.is_none());
        assert!(a.take_copy_request().is_none());
        assert!(a.take_shape_request().is_none());
        assert_eq!(a.receipt.as_ref().unwrap().goal, frozen);
    }
}
#[test]
fn receipt_copy_is_exact_and_shape_is_an_explicit_one_shot_request() {
    let mut a = board();
    a.receipt = Some(handoff::Receipt::test("/tmp/TEST-receipt/session".into()));
    a.key_bar = false;
    assert_eq!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .trim(),
        "c copy prompt   s open in shape   10 menu"
    );
    let expected = "Use the seed-me skill at \"/TEST/seed-me/SKILL.md\" to continue the existing Seed Me session at \"/tmp/TEST-receipt/session\". I want to continue this session and shape its confirmed goal into a seed contract.\n\nRead the session with the skill's scripts/session.py read and status commands before changing anything. Do not initialize a new session or replace the confirmed goal. Preserve recorded decisions, constraints, exclusions and unresolved context. Tinkery's answered and skipped questions are context, not additional settled Seed Me decisions. If the session is not active, stop and report its status.\n\nFollow the skill's interview and viewer lifecycle. Goal confirmation is not seed confirmation or permission to implement. Save the draft as this session's seed-contract.md; ask me to confirm the displayed revision before using session.py seed confirm. Do not start implementation.";
    key(&mut a, KeyCode::Char('c'));
    assert_eq!(a.take_copy_request().as_deref(), Some(expected));
    assert!(a.take_copy_request().is_none());
    a.copy_result(true);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Copied.")
    );
    a.copy_result(false);
    assert!(a.notice.starts_with("Clipboard send failed"));
    let mut repeated = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
    repeated.kind = KeyEventKind::Repeat;
    a.handle_key(repeated);
    assert!(a.take_shape_request().is_none());
    key(&mut a, KeyCode::Char('s'));
    assert_eq!(
        a.take_shape_request(),
        Some("/tmp/TEST-receipt/session".into())
    );
    assert!(a.take_shape_request().is_none());
    assert!(!a.quit && !a.running());
    a.help_all = true;
    let help = yohaku::help_text(&a);
    assert!(help.contains("s   open in shape"));
    assert!(!help.contains("send") && !help.contains("speak"));
    assert!(yohaku::how_text(&a).contains("/tmp/TEST-receipt/session"));
}
#[test]
fn goal_review_lists_unresolved_questions_and_requires_distinct_full_review_affirmation() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        let q = a.focused_question().unwrap().text.clone();
        a.review_goal();
        let text = snapshot(w, h, &mut a, false).unwrap();
        assert!(text.contains("type confirm"));
        assert!(!text.contains("Running out of questions"));
        assert!(right_text(&text, w).contains("type confirm to save."));
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
