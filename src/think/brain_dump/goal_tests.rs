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
    a.conversation.sources_mut().push(Source{id:1,text:"PR ready for human review; restaurateur, not cook; both harness and codebases compound.".into(),in_reply_to:None});
    a.conversation.set_applied(1);
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
    let mut guess = Simulated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    guess.parts.outcome = Some("A PR ready for human review with you as restaurateur judging the result; harness and codebase both compound.".into());
    a.conversation
        .set_board(Some(board::Board::verify(guess, &r).unwrap()));
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
        let frozen = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
        let mut receipt = seed_me::Receipt::test("/tmp/TEST-receipt/session".into());
        receipt.goal = frozen.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        a.handoff.job = Some(rx);
        tx.send(Ok(receipt)).unwrap();
        a.conversation.set_board(None);
        a.goal_tick();
        assert!(a.handoff.review.is_none());
        assert_eq!(a.handoff.receipt.as_ref().unwrap().goal, frozen);
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
        for y in a.screen.agent_area.y..a.screen.agent_area.bottom() {
            for x in a.screen.agent_area.x..a.screen.agent_area.right() {
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
        assert!(!a.quitting() && !a.screen.original && !a.screen.details && !a.running());
        assert!(a.handoff.job.is_none());
        assert!(a.take_copy_request().is_none());
        assert_eq!(a.exit(), &Exit::Stay);
        assert_eq!(a.handoff.receipt.as_ref().unwrap().goal, frozen);
    }
}
#[test]
fn receipt_copy_is_exact_and_tend_is_an_explicit_request() {
    let mut a = board();
    a.handoff.receipt = Some(seed_me::Receipt::test("/tmp/TEST-receipt/session".into()));
    a.screen.key_bar = false;
    assert_eq!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .trim(),
        "c copy prompt   s open in tend   F10 menu"
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
    assert_eq!(a.exit(), &Exit::Stay);
    key(&mut a, KeyCode::Char('s'));
    assert_eq!(a.exit(), &Exit::Tend("/tmp/TEST-receipt/session".into()));
    assert!(!a.quitting() && !a.running());
    a.screen.help_all = true;
    let help = layout::help_text(&a);
    assert!(help.contains("s   open in tend"));
    assert!(!help.contains("send") && !help.contains("speak"));
    assert!(layout::how_text(&a).contains("/tmp/TEST-receipt/session"));
}
#[test]
fn goal_review_lists_unresolved_questions_and_requires_distinct_full_review_affirmation() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        let q = a.conversation.focused_question().unwrap().text.clone();
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
        assert!(a.handoff.job.is_none());
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::F(2));
        assert!(
            a.handoff.job.is_none(),
            "Opening review/Enter/F2 affirmed implicitly"
        );
        a.paste("confirm\n");
        assert!(a.handoff.review.as_ref().unwrap().input.is_empty());
        key(&mut a, KeyCode::Esc);
        assert!(a.handoff.review.is_none());
        assert!(a.handoff.receipt.is_none());
    }
}
#[test]
fn confirmation_is_unavailable_for_practice_pending_sources_or_unsent_text() {
    let mut a = BrainDump::default();
    a.review_goal();
    assert!(a.handoff.review.is_none());
    let mut a = board();
    a.paste("new local text");
    a.review_goal();
    assert!(a.handoff.review.is_none());
    a.input = Note::new("");
    a.conversation.set_applied(0);
    a.review_goal();
    assert!(a.handoff.review.is_none());
    a.conversation.set_applied(1);
    a.shaping.ready = Some((
        BoardRequest {
            ask_counts: Default::default(),
            sources: a.conversation.sources().to_vec(),
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
    assert!(a.handoff.review.is_none());
}
#[test]
fn empty_questions_show_review_route_but_never_confirm_and_missing_helper_preserves_source() {
    let mut a = board();
    amend_board(&mut a, |g| g.questions.clear());
    let text = snapshot(100, 30, &mut a, false).unwrap();
    assert!(text.contains("Nothing I'm unsure about."));
    assert!(text.contains("F3 review"));
    assert!(a.handoff.receipt.is_none());
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let original = a.conversation.sources()[0].text.clone();
    let goal = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
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
    assert!(a.handoff.receipt.is_none());
    assert!(a.notice.contains("Handoff not complete"));
    assert_eq!(a.conversation.sources()[0].text, original);
    assert_eq!(a.handoff.review.as_ref().unwrap().affirmation.goal, goal);
    assert!(a.handoff.config.as_ref().unwrap().recovery_path().is_none());
}
#[test]
fn answered_scope_accepts_reading_lengths_and_counts_without_manufactured_options() {
    let a = board();
    let mut r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.conversation.sources().to_vec(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut g = a.conversation.board().unwrap().wire();
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
    a.conversation.skipped_mut().push(skipped.clone());
    amend_board(&mut a, |g| {
        g.questions.extend((0..5).map(|i|Question{target: None,id:format!("long-{i}"),text:"Which consequential design boundary needs independent verification before this goal can become implementation, and how would you recognize a wrong outcome?".into()}))
    });
    a.review_goal();
    snapshot(80, 24, &mut a, false).unwrap();
    assert!(a.handoff.review.as_ref().unwrap().max > 0);
    assert!(
        a.handoff
            .review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == skipped.id)
    );
    a.paste("confirm");
    key(&mut a, KeyCode::Enter);
    assert!(a.handoff.job.is_none());
    assert!(a.notice.contains("Read to the end first"));
}

#[test]
fn lexical_coverage_never_vetoes_the_review() {
    let mut a = board();
    amend_board(&mut a, |g| {
        g.parts.outcome = Some("A software result for human review.".into());
    });
    a.review_goal();
    assert!(
        a.handoff.review.is_some(),
        "Lexical coverage must not veto the person's review"
    );
    assert!(a.handoff.job.is_none() && a.handoff.receipt.is_none());
}
#[test]
fn literal_scope_and_checkpoint_terms_are_supported_without_promoting_candidates() {
    let mut a = board();
    a.conversation.sources_mut().push(Source {
        id: 2,
        text: "both".into(),
        in_reply_to: Some("compounding".into()),
    });
    a.conversation.sources_mut().push(Source{id:3,text:"judge the final result, with earlier taste checks on UI and consequential design choices.".into(),in_reply_to:None});
    a.conversation.settled_mut().push(Settled {
        question: Question {
            target: None,
            id: "compounding".into(),
            text: "Harness or codebase, or both?".into(),
        },
        source: 2,
    });
    a.conversation.set_applied(3);
    let mut g = a.conversation.board().unwrap().wire();
    g.parts.outcome = Some("A human-first meta harness and board carries the agentic software development lifecycle to a PR ready for human review. Both harness and codebase compound, preserving codebase health and easier future changes; you are the restaurateur judging the final result with earlier UI and consequential design taste checks.".into());
    g.framings[0].supports.push(Anchor {
        source: 3,
        quote: a.conversation.sources()[2].text.clone(),
        occurrence: 0,
    });
    g.outcome = "PR ready for human review with UI/design taste checks.".into();
    g.alternatives.clear();
    amend_board(&mut a, |current| *current = g);
    a.review_goal();
    snapshot(100, 30, &mut a, false).unwrap();
    let frozen = &a.handoff.review.as_ref().unwrap().affirmation;
    for term in [
        "PR ready for human review",
        "restaurateur",
        "both harness and codebase compound",
        "UI",
        "design taste checks",
    ] {
        assert!(frozen.goal.contains(term));
    }
    assert_eq!(frozen.sources[2].text, a.conversation.sources()[2].text);
    assert!(frozen.options.is_empty());
    assert_eq!(frozen.answered[0].source, 2);
    assert!(a.handoff.job.is_none());
}
/// The right-pane text on the row Back is drawn on, from Back's column.
fn back_row(screen: &str, back: Rect) -> String {
    screen
        .lines()
        .nth(usize::from(back.y))
        .unwrap()
        .chars()
        .skip(usize::from(back.x))
        .collect::<String>()
        .trim_end()
        .to_owned()
}
#[test]
fn a_failed_save_says_so_whole_above_back_and_retries() {
    for (w, h) in [(80, 24), (100, 30), (160, 40)] {
        for no_color in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut a = board().with_record_root(dir.path().join("TEST-records"));
            a.review_goal();
            let goal = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
            snapshot(w, h, &mut a, no_color).unwrap();
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
            assert!(
                a.notice.starts_with("Handoff not complete:"),
                "{}",
                a.notice
            );
            let screen = snapshot(w, h, &mut a, no_color).unwrap();
            assert!(
                right_text(&screen, w)
                    .contains("Handoff not complete. F1 then h shows why; Enter retries."),
                "{w}x{h}: the whole failure and its next step\n{screen}"
            );
            assert_eq!(
                back_row(&screen, a.screen.back_button),
                "Back",
                "{w}x{h}: Back stands alone\n{screen}"
            );
            assert!(
                layout::how_text(&a).contains("Cannot locate Seed Me"),
                "{w}x{h}: the reason is in How it works"
            );
            assert!(a.handoff.receipt.is_none(), "no false Saved");
            assert_eq!(a.handoff.review.as_ref().unwrap().affirmation.goal, goal);
            key(&mut a, KeyCode::Enter);
            assert!(a.handoff.job.is_some(), "{w}x{h}: Enter retries");
            let mut receipt = seed_me::Receipt::test("/tmp/TEST-retry/session".into());
            receipt.goal = goal.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            a.handoff.job = Some(rx);
            tx.send(Ok(receipt)).unwrap();
            a.goal_tick();
            assert_eq!(a.handoff.receipt.as_ref().unwrap().goal, goal);
        }
    }
}
