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
            unresolved_notes: vec![],
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
    let old = a.guess.clone().unwrap();
    a.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT));
    a.paste("Also fix a typo in the welcome email.");
    a.submit();
    assert!(!a.running());
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    assert!(
        a.focused_question()
            .unwrap()
            .id
            .starts_with("scope-addition-")
    );
    assert_eq!(a.source_view, 1);
    let view = snapshot(100, 30, &mut a, false).unwrap();
    assert!(view.contains("Also fix a typo"));
    assert!(a.annotations_visible.1);
    assert_eq!(a.guess.as_ref().unwrap().framings, old.framings);
    assert!(a.misfit_text().contains("scope not answered"));
    assert!(a.misfit_text().contains("welcome email"));
    a.submit();
    assert_eq!(host.requests.lock().unwrap().len(), 1);
    a.review_goal();
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == "scope-addition-2")
    );
    assert!(a.handoff_job.is_none());
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
    assert!(a.scope_pending.is_none() && a.receipt.is_none());
    assert!(
        a.guess
            .as_ref()
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
    assert_eq!(a.focused_question().unwrap().id, "books-note");
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
    assert_eq!(a.sources[1].text, "A welcome-email typo.");
    assert!(a.scope_pending.is_some());
    a.review_goal();
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == "scope-addition-2")
    );
    assert!(a.handoff_job.is_none());
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(6));
    key(&mut a, KeyCode::Char('u'));
    assert_eq!(a.focused_question().unwrap().id, "scope-addition-2");
    assert!(
        a.guess
            .as_ref()
            .unwrap()
            .questions
            .iter()
            .all(|q| q.id != "scope-addition-2")
    );
}
#[test]
fn f_keys_keep_aliases_modes_originals_copy_skip_undo_and_exit_guard() {
    let mut a = board();
    assert!(!a.board_focus);
    key(&mut a, KeyCode::F(6));
    assert!(a.board_focus);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("board controls")
    );
    let q = a.focused_question().unwrap().id.clone();
    key(&mut a, KeyCode::F(8));
    assert_eq!(a.skipped.len(), 1);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Question skipped")
    );
    key(&mut a, KeyCode::Char('u'));
    assert_eq!(a.focused_question().unwrap().id, q);
    assert!(a.skipped.is_empty());
    key(&mut a, KeyCode::F(9));
    assert!(a.take_copy_request().unwrap().contains("Tinkery"));
    a.copy_result(true);
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Copy sent")
    );
    key(&mut a, KeyCode::F(4));
    assert!(a.original);
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(5));
    assert!(a.details);
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(3));
    assert!(a.goal_review.is_some());
    snapshot(100, 30, &mut a, false).unwrap();
    let back = a.back_button;
    assert!(back.width > 0);
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: back.x,
            row: back.y,
            modifiers: KeyModifiers::NONE,
        },
        a.area,
    );
    assert!(a.goal_review.is_none() && a.handoff_job.is_none());
    key(&mut a, KeyCode::F(10));
    assert!(a.leave_prompt && !a.quit);
    key(&mut a, KeyCode::Esc);
    assert!(!a.leave_prompt);
    key(&mut a, KeyCode::F(10));
    key(&mut a, KeyCode::F(10));
    assert!(a.quit);
}
#[test]
fn undone_skip_survives_a_later_model_result_without_mutating_model_questions() {
    let mut a = board();
    let original = a.focused_question().unwrap().clone();
    key(&mut a, KeyCode::F(8));
    key(&mut a, KeyCode::F(6));
    key(&mut a, KeyCode::Char('u'));
    assert_eq!(a.focused_question().unwrap().id, original.id);
    let r = BoardRequest {
        sources: a.sources.clone(),
        fragments: vec![],
        previous: a.guess.as_ref().map(|g| g.wire()),
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut response = a.guess.as_ref().unwrap().wire();
    response.questions.clear();
    a.apply_result(r, Ok(response));
    assert!(a.guess.as_ref().unwrap().questions.is_empty());
    assert_eq!(a.focused_question().unwrap().id, original.id);
    a.review_goal();
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.id == original.id)
    );
    assert!(a.handoff_job.is_none());
}
#[test]
fn optional_bar_uses_reserved_row_without_moving_source_review_or_receipt() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        let before = snapshot(w, h, &mut a, false).unwrap();
        let rect = a.source_area;
        assert!(!before.lines().last().unwrap().contains("1 help"));
        key(&mut a, KeyCode::F(1));
        key(&mut a, KeyCode::Char('b'));
        key(&mut a, KeyCode::Esc);
        let after = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(rect, a.source_area);
        let bar = after.lines().last().unwrap();
        assert!(bar.contains("1 help") && bar.contains("10 exit"), "{bar}");
        assert_eq!(
            before.lines().take(usize::from(h - 1)).collect::<Vec<_>>(),
            after.lines().take(usize::from(h - 1)).collect::<Vec<_>>()
        );
        key(&mut a, KeyCode::F(3));
        let review = snapshot(w, h, &mut a, false).unwrap();
        assert!(right_text(&review, w).contains("Creates a Seed Me session"));
        assert!(review.contains("Back"));
        a.goal_review = None;
        a.receipt = Some(handoff::Receipt::test("/tmp/Tinkery-TEST/dogfood".into()));
        let receipt = snapshot(w, h, &mut a, false).unwrap();
        assert!(receipt.contains("Continue with Seed Me"));
        assert!(receipt.lines().last().unwrap().contains("10 exit"));
        assert_eq!(rect, a.source_area);
    }
    assert!(!BrainDump::default().key_bar);
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
    assert!(a.guess.is_none());
    let view = snapshot(100, 30, &mut a, false).unwrap();
    assert!(right_text(&view, 100).contains("The quoted line doesn't exist"));
    let help = yohaku::help_text(&a);
    assert!(help.starts_with("Right now\nReading failed"));
    assert!(!help.lines().nth(1).unwrap().contains("review"));
    key(&mut a, KeyCode::F(3));
    assert!(a.goal_review.is_none());
    assert_eq!(a.sources[0].text, "Original kept.");
}
#[test]
fn whole_board_coverage_records_candidate_location_and_never_requires_goal_overlap() {
    let mut a = board();
    let mut g = a.guess.as_ref().unwrap().wire();
    a.sources[0].text = "Learners feel dumb. Proposed AI tutor chat. Unverified question 4.".into();
    g.framings[0].text = "Learners leave a wrong answer feeling smarter.".into();
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
        sources: a.sources.clone(),
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
    assert!(a.goal_review.is_some());
    assert!(a.handoff_job.is_none());
}
#[test]
fn encoded_cap_counts_metadata_keeps_unsent_words_and_near_cap_is_visible() {
    let host = Arc::new(Counting::default());
    let mut a = BrainDump::with_host(host.clone());
    a.sources = vec![Source {
        id: 1,
        text: "x".repeat(4000),
        in_reply_to: None,
    }];
    a.fragments = (0..7)
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
    a.fragments.push(a.fragments[0].clone());
    let source = a.sources[0].text.clone();
    a.submit();
    assert!(!a.running());
    assert!(host.requests.lock().unwrap().is_empty());
    assert_eq!(a.sources[0].text, source);
    assert_eq!(a.input.text, "new words stay local");
    assert!(a.notice.contains("above 32 KiB"));
    assert_eq!(a.local_checks.last().unwrap().decision, "reject");
}
#[test]
fn rejected_schema_attempt_is_readable_lossless_and_never_promoted() {
    let mut a = board();
    let g = a.guess.as_ref().unwrap();
    let mut raw = serde_json::to_value(g).unwrap();
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
    assert!(why.contains(&g.framings[0].text));
    assert!(why.contains("precondition: An unverified capability is needed."));
    a.guess = None;
    a.last_failure = Some(d.reason.clone());
    a.applied = 0;
    a.review_goal();
    assert!(a.goal_review.is_none());
    assert_eq!(d.raw.as_ref().unwrap(), &text);
    assert!(a.handoff_job.is_none());
}
#[test]
fn model_prefix_is_removed_without_touching_originals_and_modified_keys_do_not_discard() {
    assert_eq!(
        agent_text("PROVISIONAL goal: PROVISIONAL: intended meaning"),
        "intended meaning"
    );
    let mut a = board();
    let original = a.sources[0].text.clone();
    a.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::ALT));
    assert!(!a.quit && !a.leave_prompt);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::CONTROL));
    assert!(a.skipped.is_empty());
    let mut repeated = KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE);
    repeated.kind = KeyEventKind::Repeat;
    a.handle_key(repeated);
    assert!(a.skipped.is_empty());
    assert_eq!(a.sources[0].text, original);
}
