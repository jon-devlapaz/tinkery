use super::goal_tests::{board, right_text};
use super::tests::amend_board;
use super::*;
use std::sync::Mutex;
#[derive(Default)]
struct Counting {
    requests: Mutex<Vec<BoardRequest>>,
}
impl BoardHost for Counting {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        self.requests.lock().unwrap().push(r.clone());
        Ok(Guess {
            parts: GoalParts {
                who: Some("I".into()),
                outcome: Some("keep track of the books you've read".into()),
                ..Default::default()
            },
            open: vec![],
            other_goals: vec![],
            deferred: vec![],
            unresolved_notes: vec![],
            decisions: vec![],
            uncertain: false,
            framings: vec![Framing {
                text: "Keep track of the books you've read.".into(),
                supports: vec![Anchor {
                    source: 1,
                    quote: r.sources[0].text.clone(),
                    occurrence: 0,
                }],
            }],
            outcome: "See your books and remember what you thought of each.".into(),
            misfits: if r.sources.len() > 2 {
                vec![Anchor {
                    source: 2,
                    quote: r.sources[1].text.clone(),
                    occurrence: 0,
                }]
            } else {
                vec![]
            },
            questions: if r.settled.is_empty() {
                vec![Question {
                    target: None,
                    id: "books-note".into(),
                    text: "What would you like to remember about each book?".into(),
                }]
            } else {
                vec![]
            },
            alternatives: vec![],
        })
    }
}
fn key(a: &mut BrainDump, k: KeyCode) {
    a.handle_key(KeyEvent::new(k, KeyModifiers::NONE));
}
fn wait(a: &mut BrainDump) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while a.running() && std::time::Instant::now() < deadline {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!a.running());
}
#[test]
fn added_words_are_visible_unresolved_and_not_sent_until_scope_answer() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone()).with_seed_me("/missing/SKILL.md".into(), None);
    a.paste("Books I've read this year.");
    a.submit();
    wait(&mut a);
    let old = a.conversation.board().cloned().unwrap();
    a.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT));
    a.paste("Also fix a typo in the welcome email.");
    a.submit();
    assert!(!a.running());
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    assert!(
        a.conversation
            .focused_question()
            .unwrap()
            .id
            .starts_with("scope-addition-")
    );
    assert_eq!(a.screen.source_view, 1);
    let view = snapshot(100, 30, &mut a, false).unwrap();
    assert!(view.contains("Also fix a typo"));
    assert_eq!(a.conversation.board().unwrap().reading, old.reading);
    assert!(a.misfit_text().contains("scope not answered"));
    assert!(a.misfit_text().contains("welcome email"));
    a.submit();
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    a.review_goal();
    assert!(
        a.handoff
            .review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == "scope-addition-2")
    );
    assert!(a.handoff.job.is_none());
    key(&mut a, KeyCode::Esc);
    a.paste("keep it separate");
    a.submit();
    wait(&mut a);
    let requests = host.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].sources[1].text,
        "Also fix a typo in the welcome email."
    );
    assert_eq!(requests[1].sources[2].text, "keep it separate");
    assert_eq!(requests[1].settled[0].question.id, "scope-addition-2");
    assert_eq!(requests[1].settled[0].source, 3);
    assert!(a.conversation.scope_pending().is_none() && a.handoff.receipt.is_none());
    assert!(
        a.conversation
            .board()
            .unwrap()
            .misfits
            .iter()
            .any(|s| s.source == 2)
    );
}
#[test]
fn skipping_scope_does_not_include_added_words_in_later_model_context_or_confirm_them() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone()).with_seed_me("/missing/SKILL.md".into(), None);
    a.paste("Books I've read.");
    a.submit();
    wait(&mut a);
    a.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT));
    a.paste("A welcome-email typo.");
    a.submit();
    key(&mut a, KeyCode::F(8));
    assert_eq!(a.conversation.focused_question().unwrap().id, "books-note");
    a.paste("A short note about each book.");
    a.submit();
    wait(&mut a);
    let requests = host.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].sources.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert!(
        !serde_json::to_string(&requests[1])
            .unwrap()
            .contains("welcome-email typo")
    );
    drop(requests);
    assert_eq!(a.conversation.sources()[1].text, "A welcome-email typo.");
    assert!(a.conversation.scope_pending().is_some());
    a.review_goal();
    assert!(
        a.handoff
            .review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == "scope-addition-2")
    );
    assert!(a.handoff.job.is_none());
    key(&mut a, KeyCode::Esc);
    a.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(
        a.conversation.focused_question().unwrap().id,
        "scope-addition-2"
    );
    assert!(
        a.conversation
            .board()
            .unwrap()
            .questions
            .iter()
            .all(|q| q.id != "scope-addition-2")
    );
}
#[test]
fn f_keys_keep_aliases_modes_originals_copy_skip_undo_and_exit_guard() {
    let mut a = board();
    assert!(!a.screen.board_focus);
    key(&mut a, KeyCode::F(6));
    assert!(!a.screen.board_focus, "F6 no longer switches focus");
    let q = a.conversation.focused_question().unwrap().id.clone();
    key(&mut a, KeyCode::F(8));
    assert_eq!(a.conversation.skipped().len(), 1);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Question skipped")
    );
    a.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(a.conversation.focused_question().unwrap().id, q);
    assert!(a.conversation.skipped().is_empty());
    key(&mut a, KeyCode::F(9));
    assert!(a.take_copy_request().unwrap().contains("Tinkery"));
    a.copy_result(true);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Copy sent")
    );
    key(&mut a, KeyCode::F(4));
    assert!(a.screen.original);
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(5));
    assert!(a.screen.details);
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(3));
    assert!(a.handoff.review.is_some());
    snapshot(100, 30, &mut a, false).unwrap();
    let back = a.screen.back_button;
    assert!(back.width > 0);
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: back.x,
            row: back.y,
            modifiers: KeyModifiers::NONE,
        },
        a.screen.area,
    );
    assert!(a.handoff.review.is_none() && a.handoff.job.is_none());
    key(&mut a, KeyCode::F(10));
    assert!(a.screen.leave_prompt && !a.quitting());
    key(&mut a, KeyCode::Esc);
    assert!(!a.screen.leave_prompt);
    key(&mut a, KeyCode::F(10));
    key(&mut a, KeyCode::F(10));
    assert!(a.quitting());
}
#[test]
fn undone_skip_survives_a_later_model_result_without_mutating_model_questions() {
    let mut a = board();
    let original = a.conversation.focused_question().unwrap().clone();
    key(&mut a, KeyCode::F(8));
    a.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(a.conversation.focused_question().unwrap().id, original.id);
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.conversation.sources().to_vec(),
        fragments: vec![],
        previous: a.conversation.board().map(|g| g.wire()),
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut response = a.conversation.board().unwrap().wire();
    response.questions.clear();
    a.apply_result(r, Ok(response));
    assert!(a.conversation.board().unwrap().questions.is_empty());
    assert_eq!(a.conversation.focused_question().unwrap().id, original.id);
    a.review_goal();
    assert!(
        a.handoff
            .review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == original.id)
    );
    assert!(a.handoff.job.is_none());
}
#[test]
fn practice_never_offers_review_it_would_refuse() {
    for (real, seed_me) in [(false, true), (true, false), (false, false)] {
        let mut a = board();
        a.real = real;
        if !seed_me {
            a.handoff.config = None;
        }
        let bar = snapshot(100, 30, &mut a, false).unwrap();
        assert!(!bar.lines().last().unwrap().contains("F3"), "{bar}");
        assert!(!layout::key_bar_items(&a).contains(&("F3", "review")));
        a.screen.help = true;
        assert!(!layout::help_text(&a).contains("F3"));
        amend_board(&mut a, |g| g.questions.clear());
        let help = layout::help_text(&a);
        assert!(
            !help.contains("F3") && help.contains("Saving a goal needs real mode and Seed Me.")
        );
    }
    let mut a = board();
    assert!(layout::key_bar_items(&a).contains(&("F3", "review")));
    a.screen.help = true;
    assert!(layout::help_text(&a).contains("F3 reviews the goal."));
}
#[test]
fn key_bar_is_on_by_default_shows_only_possible_actions_and_keeps_layout() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        let with_bar = snapshot(w, h, &mut a, false).unwrap();
        let rect = a.screen.source_area;
        let bar = with_bar.lines().last().unwrap();
        assert!(bar.contains("F2 send") && bar.contains("F10 menu"), "{bar}");
        assert!(
            bar.contains("F8 skip") && bar.contains("F3 review"),
            "{bar}"
        );
        for gone in ["switch", "originals", "why", "copy", "exit", "1 help"] {
            assert!(!bar.contains(gone), "{gone} in {bar}");
        }
        key(&mut a, KeyCode::F(1));
        key(&mut a, KeyCode::Char('b'));
        key(&mut a, KeyCode::Esc);
        let without = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(rect, a.screen.source_area);
        assert!(!without.lines().last().unwrap().contains("send"));
        assert_eq!(
            with_bar
                .lines()
                .take(usize::from(h - 1))
                .collect::<Vec<_>>(),
            without.lines().take(usize::from(h - 1)).collect::<Vec<_>>()
        );
        a.screen.key_bar = true;
        key(&mut a, KeyCode::F(3));
        let review = snapshot(w, h, &mut a, false).unwrap();
        assert!(right_text(&review, w).contains("type confirm to save."));
        let bar = review.lines().last().unwrap();
        assert!(
            bar.contains("type confirm") && bar.contains("esc back"),
            "{bar}"
        );
        a.handoff.review = None;
        a.handoff.receipt = Some(seed_me::Receipt::test("/tmp/Tinkery-TEST/dogfood".into()));
        let receipt = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            super::goal_tests::right_text(&receipt, w)
                .contains("Next: shape it into a seed in Shape.")
        );
        assert_eq!(
            receipt.lines().last().unwrap().trim(),
            "c copy prompt   s open in tend   F10 menu"
        );
        assert_eq!(rect, a.screen.source_area);
    }
    let mut empty = BrainDump::default();
    assert!(empty.screen.key_bar);
    let bar = snapshot(100, 30, &mut empty, false).unwrap();
    assert_eq!(bar.lines().last().unwrap().trim(), "F2 send   F10 menu");
}
#[test]
fn failure_help_names_only_real_actions_and_exposes_reason_without_confirmation() {
    struct Failure;
    impl BoardHost for Failure {
        fn reshape(&self, _: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
            Err("The quoted line doesn't exist in original 1.".into())
        }
    }
    let mut a = BrainDump::with_host(Arc::new(Failure));
    a.paste("Original kept.");
    a.submit();
    wait(&mut a);
    assert!(a.conversation.board().is_none());
    let view = snapshot(100, 30, &mut a, false).unwrap();
    assert!(right_text(&view, 100).contains("The quoted line doesn't exist"));
    let help = layout::help_text(&a);
    assert!(help.starts_with("Right now\nThe reading failed"));
    assert!(!help.lines().nth(1).unwrap().contains("review"));
    key(&mut a, KeyCode::F(3));
    assert!(a.handoff.review.is_none());
    assert_eq!(a.conversation.sources()[0].text, "Original kept.");
}
#[test]
fn whole_board_coverage_records_candidate_location_and_never_requires_goal_overlap() {
    let mut a = board();
    let mut g = a.conversation.board().unwrap().wire();
    a.conversation.sources_mut()[0].text =
        "Learners feel dumb. Proposed AI tutor chat. Unverified question 4.".into();
    g.parts.outcome = Some("Learners leave a wrong answer feeling smarter.".into());
    g.outcome = g.framings[0].text.clone();
    g.framings[0].supports = vec![Anchor {
        source: 1,
        quote: "Learners feel dumb.".into(),
        occurrence: 0,
    }];
    g.alternatives = vec![
        Approach {
            label: "AI tutor chat (candidate)".into(),
            benefit: "Could explain mistakes; unverified.".into(),
            cost: "May be expensive.".into(),
            undo_cost: "Unknown.".into(),
        },
        Approach {
            label: "Improve written explanations".into(),
            benefit: "Could explain mistakes.".into(),
            cost: "Unknown.".into(),
            undo_cost: "Unknown.".into(),
        },
    ];
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
    g.validate(&r).unwrap();
    let coverage = checks::coverage(&r, &g);
    assert_eq!(
        coverage.iter().find(|c| c.term == "AI").unwrap().locations,
        vec!["candidate: AI tutor chat (candidate)"]
    );
    amend_board(&mut a, |current| *current = g);
    a.review_goal();
    assert!(a.handoff.review.is_some());
    assert!(a.handoff.job.is_none());
}
#[test]
fn encoded_cap_counts_metadata_keeps_unsent_words_and_near_cap_is_visible() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    *a.conversation.sources_mut() = vec![Source {
        id: 1,
        text: "x".repeat(4000),
        in_reply_to: None,
    }];
    *a.conversation.fragments_mut() = (0..7)
        .map(|i| Fragment {
            id: format!("f{i}"),
            source: 1,
            start: 0,
            end: 4000,
            text: "x".repeat(4000),
        })
        .collect();
    a.input = Note::new("new words stay local");
    assert!(a.encoded_bytes().unwrap() >= 28 * 1024);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("/ 32768 bytes")
    );
    let first = a.conversation.fragments()[0].clone();
    a.conversation.fragments_mut().push(first);
    let source = a.conversation.sources()[0].text.clone();
    a.submit();
    assert!(!a.running());
    assert!(host.requests.lock().unwrap().is_empty());
    assert_eq!(a.conversation.sources()[0].text, source);
    assert_eq!(a.input.text, "new words stay local");
    assert!(a.notice.contains("too long"));
    assert_eq!(a.local_checks.last().unwrap().decision, "reject");
}
#[test]
fn rejected_schema_attempt_is_readable_lossless_and_never_promoted() {
    let mut a = board();
    let g = a.conversation.board().unwrap();
    let mut raw = serde_json::to_value(g.wire()).unwrap();
    raw["alternatives"][0]["precondition"] =
        serde_json::json!("An unverified capability is needed.");
    let text = serde_json::to_string(&raw).unwrap();
    assert!(serde_json::from_str::<Guess>(&text).is_err());
    let mut d = checks::Decision::new(
        "structure-spans-history",
        "blocking",
        "reject",
        "Unknown field precondition; unsupported schema.".into(),
        std::time::Instant::now(),
    );
    d.raw = Some(text.clone());
    let logs = vec![serde_json::to_string(&d).unwrap()];
    let why = checks::why(&logs);
    assert!(why.contains("Rejected attempt — not confirmable"));
    assert!(why.contains(g.goal().unwrap()));
    assert!(why.contains("precondition: An unverified capability is needed."));
    a.conversation.set_board(None);
    a.conversation.set_last_failure(Some(d.reason.clone()));
    a.conversation.set_applied(0);
    a.review_goal();
    assert!(a.handoff.review.is_none());
    assert_eq!(d.raw.as_ref().unwrap(), &text);
    assert!(a.handoff.job.is_none());
}
#[test]
fn model_prefix_is_removed_without_touching_originals_and_modified_keys_do_not_discard() {
    assert_eq!(
        agent_text("PROVISIONAL goal: PROVISIONAL: intended meaning"),
        "Intended meaning"
    );
    let mut a = board();
    let original = a.conversation.sources()[0].text.clone();
    a.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::ALT));
    assert!(!a.quitting() && !a.screen.leave_prompt);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::CONTROL));
    assert!(a.conversation.skipped().is_empty());
    let mut repeated = KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE);
    repeated.kind = KeyEventKind::Repeat;
    a.handle_key(repeated);
    assert!(a.conversation.skipped().is_empty());
    assert_eq!(a.conversation.sources()[0].text, original);
}
#[test]
fn right_side_speaks_in_one_voice_with_one_reading() {
    let mut a = board();
    let one = right_text(&snapshot(100, 30, &mut a, false).unwrap(), 100);
    assert!(one.contains("I think you mean…"));
    assert!(one.contains("One thing I'm not sure about"));
    assert!(one.contains("your answer"));
    assert!(!one.contains("Review goal") && !one.contains("or maybe"));
    let bar = snapshot(100, 30, &mut a, false).unwrap();
    assert!(!bar.lines().last().unwrap().contains('7'));
    let before = a.conversation.board().unwrap().goal().map(str::to_owned);
    key(&mut a, KeyCode::F(7));
    assert_eq!(
        a.conversation.board().unwrap().goal().map(str::to_owned),
        before,
        "F7 switches no reading"
    );
    assert_eq!(agent_text("Possible goal: a reading."), "A reading.");
    assert_eq!(agent_text("Note: keep this colon"), "Note: keep this colon");
    assert_eq!(agent_text("PROVISIONAL goal: x"), "X");
    assert_eq!(
        agent_text("Alternative goal: the main outcome"),
        "The main outcome"
    );
}
#[test]
fn left_label_names_what_you_wrote_and_hides_while_only_the_dump() {
    let mut a = board();
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(!screen.contains("Words") && !screen.contains("your dump"));
    let q = a.conversation.focused_question().unwrap().clone();
    a.paste("my answer");
    a.submit();
    wait(&mut a);
    a.screen.source_view = 1;
    let label = a.source_label().unwrap();
    assert!(label.starts_with("your answer to: “"), "{label}");
    assert!(label.contains(&q.text.chars().take(20).collect::<String>()));
    a.screen.source_view = 0;
    assert_eq!(a.source_label().unwrap(), "your dump");
}
#[test]
fn answering_keeps_the_dump_in_view() {
    let mut a = board();
    let dump = a.conversation.sources()[0].text.clone();
    a.paste("my answer");
    a.submit();
    wait(&mut a);
    assert!(a.conversation.sources().len() >= 2);
    assert_eq!(a.screen.source_view, 0);
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    let first: String = dump
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(screen.contains(&first), "dump left view");
    assert!(screen.contains("your dump"));
}

fn encoded(a: &BrainDump, text: &str) -> usize {
    serde_json::to_vec(
        &a.conversation
            .stage(text, false, a.shaping.ready.is_some(), a.card_positions())
            .unwrap()
            .request,
    )
    .unwrap()
    .len()
}

#[test]
fn encoded_under_at_over_boundary_counts_json_and_preserves_refused_selection() {
    for target in [32767usize, 32768, 32769] {
        let host = Arc::new(Counting::default());
        let mut a = BrainDump::with_host(host.clone());
        let prefix = "\"\\\n\0e\u{301}中👩‍💻".repeat(120);
        let overhead = encoded(&a, &prefix);
        let raw = prefix + &"x".repeat(target - overhead);
        assert!(raw.len() < target && raw.len() > 4096);
        a.paste(&raw);
        a.input.select_all();
        assert_eq!(encoded(&a, &raw), target);
        assert_eq!(a.encoded_bytes(), Some(target));
        let before = (a.input.text.clone(), a.input.cursor, a.input.selection());
        a.submit();
        wait(&mut a);
        let requests = host.requests.lock().unwrap();
        if target > 32768 {
            assert!(requests.is_empty());
            assert_eq!(
                (a.input.text.clone(), a.input.cursor, a.input.selection()),
                before
            );
            assert!(a.conversation.sources().is_empty());
        } else {
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].sources[0].text.as_bytes(), raw.as_bytes());
        }
        eprintln!(
            "encoded={target}, raw={}, host starts={}",
            raw.len(),
            requests.len()
        );
    }
}

#[test]
fn draft_overflow_copies_completely_then_one_explicit_fitting_resend() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("initial words");
    a.submit();
    wait(&mut a);
    let old = a.conversation.board().unwrap().reading.clone();
    let source = a.conversation.sources()[0].text.clone();
    let large = "中\r\n\t\0".repeat(6000);
    a.paste(&large);
    a.input.select_all();
    a.submit();
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    assert_eq!(a.conversation.board().unwrap().reading, old);
    assert_eq!(a.conversation.sources()[0].text, source);
    assert_eq!(a.input.selected_text(), Some(large.as_str()));
    a.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.take_copy_request(), Some(large));
    a.paste("fitting answer");
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    a.submit();
    wait(&mut a);
    assert_eq!(host.requests.lock().unwrap().len(), 2);
    assert_eq!(a.conversation.sources()[1].text, "fitting answer");
}

#[test]
fn context_only_overflow_offers_copy_out_without_pruning_or_false_resend() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("initial");
    a.submit();
    wait(&mut a);
    a.conversation.sources_mut()[0].text = "retained\r\n\t\0".repeat(4000);
    let original = a.conversation.sources()[0].text.clone();
    a.paste("local words");
    a.input.select_all();
    let old = a.conversation.board().unwrap().reading.clone();
    assert!(encoded(&a, "") > 32768);
    a.submit();
    assert!(a.notice.starts_with("Earlier words"));
    assert!(layout::help_text(&a).contains("Shortening new words cannot"));
    assert_eq!(a.conversation.board().unwrap().reading, old);
    a.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.take_copy_request().as_deref(), Some("local words"));
    key(&mut a, KeyCode::Tab);
    a.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    a.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.take_copy_request(), Some(original.clone()));
    assert_eq!(a.conversation.sources()[0].text, original);
    assert_eq!(a.input.text, "local words");
    assert_eq!(host.requests.lock().unwrap().len(), 1);
}

#[test]
fn overflow_help_uses_semantic_kind_after_notice_rewording_and_f1() {
    for kind in [Refusal::DraftTooLong, Refusal::EarlierWordsTooLong] {
        let mut a = BrainDump::with_host(Arc::new(Counting::default()));
        a.refuse("Notice wording can change completely.");
        a.screen.refused = Some(kind);
        key(&mut a, KeyCode::F(1));
        let help = layout::help_text(&a);
        assert!(help.contains("Nothing was sent") && help.contains("Ctrl-A then Ctrl-C"));
        assert!(!help.contains("KiB") && !help.contains("encoded") && !help.contains("context"));
        if kind == Refusal::EarlierWordsTooLong {
            assert!(
                help.contains("Shortening new words cannot") && help.contains("Ctrl-PgUp/PgDn")
            );
            assert!(!help.contains("F2 sends once"));
        } else {
            assert!(help.contains("replace it with fewer words, then F2 sends once"));
        }
    }
    let mut a = BrainDump::with_host(Arc::new(Counting::default()));
    a.refuse("Context 32 KiB is just incidental notice text.");
    assert!(!layout::help_text(&a).contains("too long to send"));
}

#[test]
fn waiting_reply_contraction_sends_the_fitting_preview_once() {
    struct Gated {
        calls: Counting,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl BoardHost for Gated {
        fn reshape(&self, r: BoardRequest, cancel: &AtomicBool) -> Result<Guess, String> {
            let mut guess = self.calls.reshape(r, cancel)?;
            let count = self.calls.requests.lock().unwrap().len();
            if count == 1 {
                guess.parts.why = Some("w".repeat(12000));
            } else if count == 2 {
                self.release.lock().unwrap().recv().unwrap();
            }
            Ok(guess)
        }
    }
    let (release, receive) = std::sync::mpsc::channel();
    let host = Arc::new(Gated {
        calls: Counting::default(),
        release: Mutex::new(receive),
    });
    let mut a = BrainDump::with_host(host.clone());
    a.paste("initial exact words\r\n中");
    a.submit();
    wait(&mut a);
    assert!(a.conversation.focused_question().is_some());
    key(&mut a, KeyCode::F(8));
    a.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT));
    a.paste("additional exact words");
    a.submit();
    assert!(a.conversation.awaiting_scope());
    a.paste("include these words");
    let initial_bytes = encoded(&a, &a.input.text);
    assert!(initial_bytes <= 32768);
    a.submit();
    a.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    a.paste("next answer");
    release.send(()).unwrap();
    wait(&mut a);
    assert!(
        a.shaping.ready.is_some(),
        "tick defers the smaller reply while typing"
    );
    assert!(a.conversation.focused_question().is_some());
    // Derive padding from the actual staged serialization, including source metadata.
    let base = encoded(&a, "x");
    a.input = Note::new(&"x".repeat(32769 - base + 1));
    a.input.select_all();
    let words = a.input.text.clone();
    let staged = a
        .conversation
        .stage(&words, false, true, a.card_positions())
        .unwrap();
    assert!(
        !staged.addition,
        "this answers the focused question, not an unscoped addition"
    );
    let stale_bytes = encoded(&a, &words);
    let final_bytes = a.encoded_bytes().unwrap();
    assert_eq!(stale_bytes, 32769);
    assert!(final_bytes <= 32768 && final_bytes < stale_bytes);
    assert_eq!(host.calls.requests.lock().unwrap().len(), 2);
    a.submit();
    wait(&mut a);
    let requests = host.calls.requests.lock().unwrap();
    eprintln!(
        "contraction initial={initial_bytes}, stale={stale_bytes}, final={final_bytes}, host starts={}",
        requests.len()
    );
    assert_eq!(
        requests.len(),
        3,
        "fitting preview must send once, not refuse: {}",
        a.notice
    );
    assert_eq!(serde_json::to_vec(&requests[2]).unwrap().len(), final_bytes);
    assert_eq!(
        serde_json::to_vec(&requests[2].sources).unwrap(),
        serde_json::to_vec(a.conversation.sources()).unwrap()
    );
    assert_eq!(requests[2].sources[0].text, "initial exact words\r\n中");
    assert_eq!(requests[2].sources[1].text, "additional exact words");
    assert_eq!(requests[2].sources[2].text, "include these words");
    assert_eq!(requests[2].sources[3].text.as_bytes(), words.as_bytes());
    assert_eq!(a.local_checks.last().unwrap().decision, "pass");
    assert!(a.shaping.ready.is_none() && a.input.text.is_empty());
}

#[test]
fn waiting_reply_overflow_is_transactional_before_committing_local_words() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("initial");
    a.submit();
    wait(&mut a);
    let r = host.requests.lock().unwrap()[0].clone();
    let mut guess = host.reshape(r.clone(), &AtomicBool::new(false)).unwrap();
    guess.parts.why = Some("w".repeat(32000));
    a.shaping.ready = Some((r, Ok(guess)));
    a.paste(&"x".repeat(1500));
    a.input.select_all();
    assert!(encoded(&a, &a.input.text) < 32768);
    assert!(
        a.encoded_bytes().unwrap() > 32768,
        "preview includes the waiting reading"
    );
    let before = (a.input.text.clone(), a.input.cursor, a.input.selection());
    let old = a.conversation.board().unwrap().reading.clone();
    a.submit();
    assert_eq!(
        (a.input.text.clone(), a.input.cursor, a.input.selection()),
        before
    );
    assert_eq!(a.conversation.sources().len(), 1);
    assert_eq!(a.conversation.board().unwrap().reading, old);
    assert!(a.shaping.ready.is_some());
    assert_eq!(a.local_checks.last().unwrap().decision, "reject");
    assert!(!a.running());
    assert_eq!(
        host.requests.lock().unwrap().len(),
        2,
        "second call above is a fixture, not submit"
    );
}

#[test]
#[ignore = "rerunnable zero-call frame measurement: cargo test --release measure_large_paste_frames -- --ignored --nocapture"]
fn measure_large_paste_frames() {
    // Same large input as tests/selection_terminal.py.
    let sentinel = "\x1b]2;PASTE-TITLE-SENTINEL\x07\x1b]52;c;UEFTVEUtU0VOVElORUw=\x07";
    let mut words = format!("Typed evidence PREFIX {sentinel}\n");
    for i in 0..120 {
        words.push_str(&format!(
            "row {i:03}: e\u{301} 中 👩‍💻\r\nCR\rTAB\tESC\x1b[31m BEL\x07 NUL\0\n"
        ));
    }
    words.push_str(" SUFFIX copy exactly.");
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host);
    a.paste(&words);
    a.submit();
    wait(&mut a);
    a.paste("next answer");
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    let palette = Palette::new(false);
    terminal.draw(|f| render(f, &mut a, palette)).unwrap();
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(a.encoded_bytes());
    }
    eprintln!(
        "size preview: {:.3} us/frame",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let start = std::time::Instant::now();
    for _ in 0..200 {
        a.tick();
        terminal.draw(|f| render(f, &mut a, palette)).unwrap();
    }
    eprintln!(
        "100x30 frame: {:.3} us/frame; fixture={} bytes",
        start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0,
        words.len()
    );
}

fn assert_cached_size(a: &mut BrainDump) {
    let fresh = a
        .preview_send(&a.input.text)
        .ok()
        .filter(|preview| preview.waiting_failure.is_none())
        .map(|preview| preview.bytes);
    assert_eq!(a.encoded_bytes(), fresh);
    assert_eq!(a.shaping.encoded_bytes, Some(fresh));
    a.tick();
    assert_eq!(
        a.shaping.encoded_bytes,
        Some(fresh),
        "idle ticks keep the cache"
    );
    assert_eq!(a.encoded_bytes(), fresh);
}

#[test]
fn size_cache_tracks_edits_waiting_reply_scope_and_cards() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("original words");
    assert_cached_size(&mut a);
    key(&mut a, KeyCode::Char('x'));
    assert!(a.shaping.encoded_bytes.is_none());
    assert_cached_size(&mut a);
    a.submit();
    wait(&mut a);
    assert_cached_size(&mut a);

    // A worker reply lands while local words are present, so tick must invalidate.
    a.paste("answer");
    assert_cached_size(&mut a);
    let r = host.requests.lock().unwrap()[0].clone();
    let guess = host.reshape(r.clone(), &AtomicBool::new(false)).unwrap();
    a.shaping.request = Some(r);
    a.shaping.job = Some(Job::launch(move |_| Ok(guess)));
    wait(&mut a);
    assert!(a.shaping.ready.is_some());
    assert!(a.shaping.encoded_bytes.is_none());
    assert_cached_size(&mut a);
    a.submit();
    wait(&mut a);
    assert_cached_size(&mut a);

    a.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT));
    assert!(a.shaping.encoded_bytes.is_none());
    a.paste("added words");
    assert_cached_size(&mut a);
    a.submit();
    assert!(a.conversation.awaiting_scope());
    assert_cached_size(&mut a);
    a.paste("include them");
    assert_cached_size(&mut a);
    a.submit();
    wait(&mut a);
    assert!(!a.conversation.awaiting_scope());
    assert_cached_size(&mut a);

    a.extract_range(1, 0, 3).unwrap();
    assert!(a.shaping.encoded_bytes.is_none());
    assert!(!a.card_positions().is_empty());
    assert_cached_size(&mut a);
    let before = a.encoded_bytes();
    let CanvasNode::Text(node) = &mut a.screen.canvas.state.data.nodes[0] else {
        panic!("extracted text card");
    };
    node.x = 12345.0;
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, 100, 30),
    );
    assert!(a.shaping.encoded_bytes.is_none());
    assert_cached_size(&mut a);
    assert_ne!(a.encoded_bytes(), before);
}

#[test]
fn failed_waiting_reply_has_unknown_size_and_the_same_failure_as_submit() {
    for invalid_board in [false, true] {
        let host = Arc::new(Counting::default());
        let mut a = BrainDump::with_host(host.clone());
        a.paste("original words");
        a.submit();
        wait(&mut a);
        let r = host.requests.lock().unwrap()[0].clone();
        let result = if invalid_board {
            let mut guess = host.reshape(r.clone(), &AtomicBool::new(false)).unwrap();
            guess.framings[0].supports[0].quote = "invented support".into();
            Ok(guess)
        } else {
            Err("failed reply".to_owned())
        };
        let starts = host.requests.lock().unwrap().len();
        a.shaping.ready = Some((r.clone(), result.clone()));
        a.paste("next answer");
        let before = a.originals();
        let old = a.conversation.board().unwrap().goal().unwrap().to_owned();
        let mut applied = BrainDump {
            conversation: a.conversation.clone(),
            ..BrainDump::default()
        };
        applied.apply_result(r, result);
        let preview = a.preview_send(&a.input.text).unwrap();
        assert!(
            preview.waiting_failure.is_some(),
            "failed apply is not a known size"
        );
        assert_eq!(
            preview.waiting_failure.as_deref(),
            applied.conversation.last_failure()
        );
        assert_cached_size(&mut a);
        assert_eq!(a.encoded_bytes(), None);
        assert_eq!(a.originals(), before, "rendering commits nothing");
        a.submit();
        assert_eq!(
            a.conversation.last_failure(),
            preview.waiting_failure.as_deref()
        );
        assert_eq!(a.conversation.board().unwrap().goal(), Some(old.as_str()));
        assert!(a.shaping.ready.is_none());
        wait(&mut a);
        let requests = host.requests.lock().unwrap();
        assert_eq!(requests.len(), starts + 1, "the same F2 still retries once");
        let sent = requests.last().unwrap();
        assert_eq!(serde_json::to_vec(sent).unwrap().len(), preview.bytes);
        assert_eq!(sent.sources[0].text, "original words");
        assert_eq!(sent.sources[1].text, "next answer");
    }
}
