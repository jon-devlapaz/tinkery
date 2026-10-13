use super::goal_tests::board;
use super::tests::amend_board;
use super::*;
fn key(a: &mut BrainDump, k: KeyCode) {
    a.handle_key(KeyEvent::new(k, KeyModifiers::NONE));
}
fn ctrl_c(a: &mut BrainDump) {
    a.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
}
fn leave(a: &mut BrainDump) {
    key(a, KeyCode::F(10));
}
#[test]
fn help_leads_with_context_groups_actions_and_keeps_internals_on_second_page() {
    let mut a = BrainDump::default();
    let now = layout::help_text(&a);
    assert_eq!(
        now,
        "Right now\nType, then F2 sends it.\n\nF1 every key · Esc close"
    );
    a.screen.help = true;
    a.screen.help_all = true;
    assert!(layout::help_text(&a).contains("practice reading; nothing leaves"));
    a = board();
    a.screen.help = true;
    assert!(layout::help_text(&a).starts_with("Right now\nType your answer"));
    assert!(
        layout::help_text(&a).lines().count() <= 5,
        "layer 1 stays short"
    );
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::F(1));
    assert!(
        !a.screen.help,
        "F1 from closed opens layer 1; then layer 2; then closes"
    );
    key(&mut a, KeyCode::F(1));
    assert!(a.screen.help && !a.screen.help_all);
    key(&mut a, KeyCode::F(1));
    assert!(a.screen.help && a.screen.help_all);
    let text = layout::help_text(&a);
    assert!(text.starts_with("Every key"));
    for heading in ["Write", "Look", "Finish", "Leave", "Words"] {
        assert!(text.contains(&format!("\n{heading}\n")));
    }
    for consequence in [
        "! Shift-F2  start a new dump",
        "! F8   skip the question",
        "! F10  back to the menu; asks before losing your draft",
    ] {
        assert!(text.contains(consequence), "{consequence}");
    }
    for gone in [
        "F6",
        "board",
        "source",
        "extract",
        "cards",
        "consequential",
        "after returning",
        "Ctrl-L",
        "fn",
        "Ctrl-C exits",
        " / Ctrl-",
    ] {
        assert!(!text.contains(gone), "{gone}");
    }
    for internal in [
        "shape/audit",
        "answer-continuity",
        "viewer",
        "pins its origin",
        "durable writes",
    ] {
        assert!(!text.contains(internal));
    }
    assert!(
        text.contains("reading   my guess") && text.contains("original  exactly what you typed")
    );
    key(&mut a, KeyCode::Esc);
    assert!(!a.screen.help && !a.screen.help_all);
    a.add_more = true;
    assert!(layout::help_text(&a).starts_with("Right now\nType a new dump"));
    a.add_more = false;
    a.review_goal();
    assert!(layout::help_text(&a).starts_with("Right now\nType confirm"));
    let frozen = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::Char('h'));
    assert!(a.screen.how && a.screen.help);
    let how = layout::how_text(&a);
    for s in [
        "One shaping call and one decision review run before the reading is shown",
        "at most two provider calls per send",
        "no automatic retry",
        "It doesn't approve a seed or start any work.",
        "session stays active",
    ] {
        assert!(how.contains(s));
    }
    for gone in ["audit", "background", "log-only"] {
        assert!(!how.contains(gone), "{gone}");
    }
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("How it works"));
    a.paste("confirm");
    assert!(a.handoff.review.as_ref().unwrap().input.is_empty());
    key(&mut a, KeyCode::Esc);
    assert!(a.screen.help && !a.screen.how);
    key(&mut a, KeyCode::Esc);
    assert!(!a.screen.help);
    a.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    let details = snapshot(100, 30, &mut a, false).unwrap();
    assert!(details.contains("h How it works"));
    key(&mut a, KeyCode::Char('h'));
    assert!(a.screen.how && a.screen.help && !a.screen.details);
    key(&mut a, KeyCode::F(1));
    assert_eq!(a.handoff.review.as_ref().unwrap().affirmation.goal, frozen);
    a.handoff.review = None;
    a.handoff.receipt = Some(seed_me::Receipt::test("/tmp/Tinkery-TEST/session".into()));
    let text = layout::help_text(&a);
    assert!(text.starts_with("Right now\nYour goal is saved"));
    assert!(text.contains("F10 back to the menu"));
}
#[test]
fn exit_guard_keeps_unsent_original_reply_and_frozen_review_until_explicit_leave() {
    for state in 0..5 {
        let mut a = if state == 0 {
            BrainDump::default()
        } else {
            board()
        };
        if state == 0 || state == 2 {
            a.paste("unsent words?");
        }
        if state == 3 {
            a.review_goal();
            a.paste("conf");
        }
        if state == 4 {
            a.conversation.set_board(None);
        }
        let source = a.conversation.sources().to_vec();
        let input = a.input.text.clone();
        let review = a.handoff.review.as_ref().map(|r| r.input.clone());
        ctrl_c(&mut a);
        assert!(
            !a.screen.leave_prompt && !a.quitting(),
            "Ctrl-C never leaves"
        );
        leave(&mut a);
        assert!(a.screen.leave_prompt && !a.quitting());
        let screen = snapshot(100, 30, &mut a, false).unwrap();
        assert_eq!(
            screen
                .matches("! Back to menu and lose this draft? y / n")
                .count(),
            1
        );
        assert!(screen.lines().nth(1).unwrap().contains("tinkery"));
        a.paste("y");
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::F(2));
        let mut repeated = KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE);
        repeated.kind = KeyEventKind::Repeat;
        a.handle_key(repeated);
        a.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::ALT));
        assert!(!a.quitting());
        key(&mut a, KeyCode::Char('n'));
        assert!(!a.screen.leave_prompt && !a.quitting());
        assert_eq!(
            serde_json::to_value(a.conversation.sources()).unwrap(),
            serde_json::to_value(&source).unwrap()
        );
        assert_eq!(a.input.text, input);
        assert_eq!(a.handoff.review.as_ref().map(|r| r.input.clone()), review);
        assert!(!a.running());
        leave(&mut a);
        key(&mut a, KeyCode::Esc);
        assert!(!a.screen.leave_prompt);
        leave(&mut a);
        if state % 2 == 0 {
            key(&mut a, KeyCode::Char('y'));
        } else {
            leave(&mut a);
        }
        assert!(a.quitting());
        assert!(a.handoff.receipt.is_none());
    }
}
#[test]
fn empty_and_confirmed_exit_immediately_but_pending_durable_handoff_waits() {
    let mut a = BrainDump::default();
    leave(&mut a);
    assert!(a.quitting() && !a.screen.leave_prompt);
    let mut a = board();
    a.handoff.receipt = Some(seed_me::Receipt::test("/tmp/Tinkery-TEST/session".into()));
    leave(&mut a);
    assert!(a.quitting() && !a.screen.leave_prompt && a.handoff.receipt.is_some());
    let mut a = board();
    a.review_goal();
    let (tx, rx) = std::sync::mpsc::channel();
    a.handoff.job = Some(rx);
    leave(&mut a);
    assert!(!a.quitting() && a.handoff.exit_after && !a.screen.leave_prompt);
    assert!(a.notice.contains("durable goal read-back"));
    tx.send(Err("TEST missing helper".into())).unwrap();
    a.tick();
    assert!(a.quitting() && a.notice.contains("TEST missing helper"));
}
#[test]
fn ctrl_c_copies_a_selection_cancels_sending_and_never_leaves() {
    let mut a = board();
    ctrl_c(&mut a);
    assert!(!a.quitting() && !a.screen.leave_prompt && a.take_copy_request().is_none());
    let text = a.conversation.sources()[0].text.clone();
    a.screen.board_focus = true;
    a.screen.selection = Some((0, 6));
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().unwrap(), text[..6]);
    assert!(!a.quitting() && !a.screen.leave_prompt);
    a.screen.selection = None;
    a.submit();
    if a.running() {
        ctrl_c(&mut a);
        assert!(!a.running() && a.notice.starts_with("Cancelled"));
    }
    assert!(!a.quitting() && !a.screen.leave_prompt);
}
#[test]
fn source_wrap_positions_header_and_scroll_survive_review_and_resize() {
    let text = format!(
        "i am worried that this thing has been overengineered\n{}\nfinal concern remains open",
        "A second exact line with café 👩‍💻, 中, and whitespace.\n".repeat(45)
    );
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        a.conversation.sources_mut()[0].text = text.clone();
        amend_board(&mut a, |g| g.framings[0].supports.clear());
        a.screen.source_scroll = 7;
        snapshot(w, h, &mut a, false).unwrap();
        a.review_goal();
        assert_eq!(a.screen.source_scroll, 0);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        for step in 0..5 {
            if step == 1 {
                a.handle_mouse(
                    MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column: 3,
                        row: 5,
                        modifiers: KeyModifiers::NONE,
                    },
                    a.screen.area,
                );
                assert!(a.screen.source_scroll > 0);
            }
            if step == 2 {
                key(&mut a, KeyCode::F(1));
            }
            if step == 3 {
                key(&mut a, KeyCode::Char('h'));
            }
            if step == 4 {
                ctrl_c(&mut a);
            }
            terminal
                .draw(|f| render(f, &mut a, Palette::new(false)))
                .unwrap();
            let b = terminal.backend().buffer();
            assert_eq!(b[(2, 1)].symbol(), "t");
            assert_eq!(b[(w - 3, 1)].symbol(), "?");
            let wrapped = Note::new(&text).wrap(a.screen.source_area.width);
            for (index, row, col) in &wrapped.positions {
                let Some(row) = row.checked_sub(usize::from(a.screen.source_scroll)) else {
                    continue;
                };
                if row >= usize::from(a.screen.source_area.height)
                    || *col >= usize::from(a.screen.source_area.width)
                {
                    continue;
                }
                let Some(g) = text[*index..].graphemes(true).next() else {
                    continue;
                };
                if g == "\n" {
                    continue;
                }
                assert_eq!(
                    b[(
                        a.screen.source_area.x + *col as u16,
                        a.screen.source_area.y + row as u16
                    )]
                        .symbol(),
                    g,
                    "{w} step {step} byte {index}"
                );
            }
        }
        assert_eq!(a.conversation.sources()[0].text, text);
        assert!(!a.quitting());
    }
}

#[test]
fn rust_marks_only_consequence_and_never_alone() {
    use ratatui::style::Color;
    let rust = Color::Rgb(160, 62, 36);
    let render = |a: &mut BrainDump, mono: bool| {
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, a, crate::Palette::new(mono))).unwrap();
        t.backend().buffer().clone()
    };
    let rows = |b: &ratatui::buffer::Buffer, colour: Color| -> Vec<String> {
        (0..b.area.height)
            .filter_map(|y| {
                let row: String = (0..b.area.width)
                    .filter(|&x| b[(x, y)].fg == colour)
                    .map(|x| b[(x, y)].symbol().to_owned())
                    .collect();
                (!row.trim().is_empty()).then(|| row.trim().to_owned())
            })
            .collect()
    };
    let mut a = board();
    assert!(
        rows(&render(&mut a, false), rust).is_empty(),
        "no rust when nothing is at stake"
    );
    leave(&mut a);
    let screen = render(&mut a, false);
    assert_eq!(rows(&screen, rust), vec!["!"]);
    let line = (0..screen.area.height)
        .map(|y| {
            (0..100)
                .map(|x| screen[(x, y)].symbol())
                .collect::<String>()
        })
        .find(|l| l.contains("! Back to menu and lose this draft? y / n"));
    assert!(line.is_some(), "the colour always comes with words");
    let y = (0..screen.area.height)
        .find(|&y| screen[(2, y)].symbol() == "!")
        .unwrap();
    assert!(
        (0..100).all(|x| screen[(x, y)].bg == Color::Rgb(255, 252, 240)),
        "the prompt row keeps the paper background"
    );
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::F(1));
    let help = render(&mut a, false);
    assert!(
        rows(&help, rust)
            .iter()
            .all(|r| r.chars().all(|c| c == '!' || c == ' '))
    );
    assert!(!rows(&help, rust).is_empty());
    let mono = render(&mut a, true);
    assert!(rows(&mono, rust).is_empty(), "NO_COLOR uses no colour");
}

#[test]
fn help_before_the_first_send_keeps_the_draft_in_view() {
    let mut a = BrainDump::default();
    a.paste("my half-finished thought");
    key(&mut a, KeyCode::F(1));
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("my half-finished thought"));
    assert!(screen.contains("Right now"));
    assert!(!screen.contains("Words 1 / 0"));
    a.submit();
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(!screen.contains("· writing") && !screen.contains("board controls"));
}
