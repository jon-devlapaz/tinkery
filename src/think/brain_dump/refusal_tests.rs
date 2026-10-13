//! Refusals: a key that does nothing says why, where the person is looking, until the next
//! input (Yohaku articles 6 and 7).
use super::goal_tests::{board, right_text};
use super::*;

const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 40)];

fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn ctrl(a: &mut BrainDump, c: char) {
    a.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}
fn shown(a: &mut BrainDump, w: u16, h: u16, no_color: bool) -> String {
    right_text(&snapshot(w, h, a, no_color).unwrap(), w)
}
/// A goal whose review is longer than the review pane at every tested size.
fn long_goal() -> BrainDump {
    let mut a = board();
    a.conversation.skipped_mut().extend((0..8).map(|i| Question {
        target: None,
        id: format!("skipped-{i}"),
        text: format!("Boundary {i}: which consequential design boundary needs independent verification before this goal can become implementation?"),
    }));
    a
}

#[test]
fn every_fixed_refusal_fits_the_status_rows_of_the_narrowest_pane() {
    let narrowest = MIN_WIDTH - MIN_WIDTH * 48 / 100 - 4;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/think");
    let mut found = 0;
    for path in ["brain_dump.rs", "brain_dump/goal_view.rs"] {
        let text = std::fs::read_to_string(dir.join(path)).unwrap();
        for call in text.split("refuse(").skip(1) {
            let Some(literal) = call.trim_start().strip_prefix('"') else {
                continue; // a reason built at run time
            };
            let literal = literal.split('"').next().unwrap().replace("\\'", "'");
            let lines = Paragraph::new(literal.as_str())
                .wrap(Wrap { trim: false })
                .line_count(narrowest);
            assert!(
                lines <= usize::from(layout::STATUS_ROWS),
                "{literal:?} takes {lines} rows at {narrowest} columns"
            );
            found += 1;
        }
    }
    assert!(found >= 18, "found only {found} refusals");
}

#[test]
fn review_blocked_by_unsent_words_says_so_on_the_normal_screen_and_keeps_them() {
    for (w, h) in SIZES {
        for no_color in [false, true] {
            const WHY: &str = "Send (F2) or clear your unsent words, then review.";
            let mut a = board();
            let goal = a.conversation.board().unwrap().parts.compose();
            a.input = Note::new("one more constraint");
            assert!(!shown(&mut a, w, h, no_color).contains(WHY));
            key(&mut a, KeyCode::F(3));
            assert!(a.handoff.review.is_none() && !a.running());
            assert!(shown(&mut a, w, h, no_color).contains(WHY), "{w}x{h}");
            assert_eq!(a.input.text, "one more constraint");
            assert_eq!(a.conversation.board().unwrap().parts.compose(), goal);
            key(&mut a, KeyCode::Char('!'));
            assert!(!shown(&mut a, w, h, no_color).contains(WHY), "cleared");
            assert_eq!(a.notice(), WHY, "still the last result");
        }
    }
}

#[test]
fn other_review_refusals_name_their_own_reason() {
    let ready = |a: &mut BrainDump| {
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
        ))
    };
    for (w, h) in SIZES {
        let mut a = board();
        ready(&mut a);
        key(&mut a, KeyCode::F(3));
        assert!(
            shown(&mut a, w, h, false).contains("A reply is waiting; F2 applies it, then review.")
        );

        let mut a = board();
        a.conversation.set_applied(0);
        key(&mut a, KeyCode::F(3));
        let screen = shown(&mut a, w, h, false);
        assert!(
            screen.contains("The reading is behind your words; F2 retries, then review.")
                && !screen.contains("Reading couldn't be updated."),
            "the refusal wins the status row: {screen}"
        );

        let mut a = BrainDump::with_host(Arc::new(Simulated));
        a.conversation = board().conversation;
        key(&mut a, KeyCode::F(3));
        assert!(shown(&mut a, w, h, false).contains("Saving a goal needs real mode and Seed Me."));
        assert!(a.handoff.review.is_none());
    }
}

#[test]
fn card_and_selection_refusals_show_instead_of_hiding_in_last_result() {
    for (w, h) in SIZES {
        let mut a = board();
        ctrl(&mut a, 'x');
        assert!(
            shown(&mut a, w, h, false)
                .contains("No extracted cards. Select source text, then Ctrl-E.")
        );
        ctrl(&mut a, 'e');
        assert!(
            shown(&mut a, w, h, false)
                .contains("Select text on the intact dump, then Ctrl-E extracts it.")
        );
    }
}

#[test]
fn an_unread_review_says_to_scroll_on_enter_and_confirms_only_from_the_end() {
    for (w, h) in SIZES {
        for no_color in [false, true] {
            const WHY: &str = "Read to the end first: PgDn scrolls. Then type confirm and Enter.";
            let mut a = long_goal();
            a.review_goal();
            snapshot(w, h, &mut a, no_color).unwrap();
            assert!(
                a.handoff.review.as_ref().unwrap().max > 0,
                "{w}x{h}: review must overflow"
            );
            a.paste("confirm");
            key(&mut a, KeyCode::Enter);
            assert!(a.handoff.job.is_none() && a.handoff.receipt.is_none());
            let screen = snapshot(w, h, &mut a, no_color).unwrap();
            assert!(right_text(&screen, w).contains(WHY), "{w}x{h}\n{screen}");
            let back = a.screen.back_button;
            assert_eq!(
                screen
                    .lines()
                    .nth(usize::from(back.y))
                    .unwrap()
                    .chars()
                    .skip(usize::from(back.x))
                    .take(4)
                    .collect::<String>(),
                "Back",
                "{w}x{h}: the reason leaves Back in place\n{screen}"
            );
            assert_eq!(
                a.handoff.review.as_ref().unwrap().input,
                "confirm",
                "typing kept"
            );
            while a.handoff.review.as_ref().unwrap().scroll < a.handoff.review.as_ref().unwrap().max
            {
                key(&mut a, KeyCode::PageDown);
            }
            let screen = shown(&mut a, w, h, no_color);
            assert!(
                !screen.contains(WHY) && screen.contains("type confirm to save"),
                "{screen}"
            );
            key(&mut a, KeyCode::Enter);
            assert!(
                a.handoff.job.is_some(),
                "{w}x{h}: confirm works from the end"
            );
            a.handoff.job = None;
        }
    }
}

#[test]
fn a_misspelled_confirm_says_what_to_type() {
    for (w, h) in SIZES {
        let mut a = board();
        a.review_goal();
        snapshot(w, h, &mut a, false).unwrap();
        a.paste("confirn");
        key(&mut a, KeyCode::Enter);
        assert!(a.handoff.job.is_none());
        assert!(shown(&mut a, w, h, false).contains("Type confirm, then Enter. Nothing saved."));
    }
}
