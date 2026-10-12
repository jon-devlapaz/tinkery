use super::goal_tests::board;
use super::*;
fn press(a: &mut BrainDump, code: KeyCode, modifiers: KeyModifiers) {
    a.handle_key(KeyEvent::new(code, modifiers));
}
fn key(a: &mut BrainDump, code: KeyCode) {
    press(a, code, KeyModifiers::NONE);
}
fn ctrl_c(a: &mut BrainDump) {
    press(a, KeyCode::Char('c'), KeyModifiers::CONTROL);
}
fn shift_right(a: &mut BrainDump, times: usize) {
    for _ in 0..times {
        press(a, KeyCode::Right, KeyModifiers::SHIFT);
    }
}
fn ready() -> BrainDump {
    let mut a = board();
    snapshot(100, 30, &mut a, false).unwrap();
    a
}

#[test]
fn tab_and_shift_tab_move_between_writing_and_the_original_and_back() {
    let mut a = ready();
    assert!(!a.board_focus);
    key(&mut a, KeyCode::Tab);
    assert!(a.board_focus, "Tab enters the original");
    key(&mut a, KeyCode::Tab);
    assert!(!a.board_focus, "Tab returns to writing");
    key(&mut a, KeyCode::Tab);
    press(&mut a, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert!(!a.board_focus, "Shift-Tab is the reverse route back");
    press(&mut a, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert!(a.board_focus, "Shift-Tab also enters the original");
    assert!(
        a.sources[0].text.starts_with("PR ready"),
        "originals untouched"
    );
}

#[test]
fn tab_does_nothing_without_an_original_or_inside_modal_views() {
    let mut empty = BrainDump::default();
    key(&mut empty, KeyCode::Tab);
    assert!(!empty.board_focus && empty.input.text.is_empty());
    let mut a = ready();
    key(&mut a, KeyCode::F(4));
    assert!(a.original);
    key(&mut a, KeyCode::Tab);
    assert!(!a.board_focus, "the F4 reader keeps its own contract");
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::Tab);
    assert!(!a.board_focus, "help is not a focus surface");
}

#[test]
fn keyboard_only_selection_copies_exact_bytes_and_returns_to_writing() {
    let mut a = ready();
    let text = a.sources[0].text.clone();
    key(&mut a, KeyCode::Tab);
    shift_right(&mut a, 8);
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some(&text[..8]));
    assert!(!a.quit && !a.leave_prompt);
    key(&mut a, KeyCode::Tab);
    assert!(!a.board_focus && a.selection.is_none(), "leaving drops it");
    ctrl_c(&mut a);
    assert!(
        a.take_copy_request().is_none(),
        "no stale copy after returning"
    );
    key(&mut a, KeyCode::Char('x'));
    assert_eq!(a.input.text, "x", "ordinary typing reaches writing");
    assert_eq!(a.sources[0].text, text);
}

#[test]
fn a_stale_original_selection_is_never_copied_after_any_focus_change() {
    let mut a = ready();
    key(&mut a, KeyCode::Tab);
    shift_right(&mut a, 5);
    assert!(a.selection.is_some());
    key(&mut a, KeyCode::Char('q'));
    assert!(!a.board_focus && a.selection.is_none());
    assert_eq!(
        a.input.text, "q",
        "letters including q return to writing as text"
    );
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none());
    // Enter, Backspace and Delete in the original also return to writing first.
    for code in [KeyCode::Enter, KeyCode::Backspace, KeyCode::Delete] {
        key(&mut a, KeyCode::Tab);
        shift_right(&mut a, 3);
        key(&mut a, code);
        assert!(!a.board_focus && a.selection.is_none(), "{code:?}");
        ctrl_c(&mut a);
        assert!(a.take_copy_request().is_none(), "{code:?}");
    }
    // Clicking back into the writing box.
    key(&mut a, KeyCode::Tab);
    shift_right(&mut a, 3);
    let (x, y) = (a.input_area.x, a.input_area.y);
    let area = a.area;
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert!(!a.board_focus && a.selection.is_none());
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none());
    // A selection hidden behind the F4 reader is not the active surface either.
    key(&mut a, KeyCode::Tab);
    shift_right(&mut a, 3);
    key(&mut a, KeyCode::F(4));
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none());
}

#[test]
fn ctrl_c_without_a_selection_keeps_cancel_and_never_leaves() {
    let mut a = ready();
    key(&mut a, KeyCode::Tab);
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none() && !a.quit && !a.leave_prompt);
}

#[test]
fn f6_is_not_a_focus_key() {
    let mut a = ready();
    key(&mut a, KeyCode::F(6));
    assert!(!a.board_focus);
    key(&mut a, KeyCode::Tab);
    key(&mut a, KeyCode::F(6));
    assert!(a.board_focus, "F6 without voice leaves focus alone");
}

fn writing(text: &str) -> BrainDump {
    let mut a = ready();
    a.input = Note::new(text);
    snapshot(100, 30, &mut a, false).unwrap();
    a
}
fn click(a: &mut BrainDump, kind: MouseEventKind, column: u16, row: u16) {
    let area = a.area;
    a.handle_mouse(
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
}

#[test]
fn shift_arrows_select_in_writing_and_ctrl_c_copies_exact_bytes() {
    let mut a = writing("héllo wörld");
    a.input.cursor = 0;
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("hél"));
    assert!(!a.quit && !a.leave_prompt);
    key(&mut a, KeyCode::Right);
    assert_eq!(a.input.selection(), None);
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none(), "no selection: no copy");
}

#[test]
fn ctrl_a_selects_only_the_active_surface() {
    let mut a = writing("my words");
    let original = a.sources[0].text.clone();
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(a.input.selected_text(), Some("my words"));
    assert_eq!(a.selection, None, "the original is untouched");
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("my words"));
    key(&mut a, KeyCode::Tab);
    assert_eq!(
        a.input.selection(),
        None,
        "writing selection does not linger"
    );
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request(), Some(original.clone()));
    assert_eq!(a.input.text, "my words");
    // Read-only: typing leaves the original alone and goes to writing.
    key(&mut a, KeyCode::Char('z'));
    assert_eq!(a.sources[0].text, original);
    assert_eq!(
        a.input.text, "my wordsz",
        "typing goes to writing at its caret"
    );
}

#[test]
fn ctrl_a_in_the_original_never_spans_sources() {
    let mut a = ready();
    a.sources.push(Source {
        id: 2,
        text: "second original".into(),
        in_reply_to: Some("q1".into()),
    });
    a.source_view = 1;
    key(&mut a, KeyCode::Tab);
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("second original"));
}

#[test]
fn typing_paste_enter_and_delete_replace_the_selection_atomically() {
    let mut a = writing("keep SELECTED keep");
    a.input.cursor = 5;
    for _ in 0..8 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    key(&mut a, KeyCode::Char('q'));
    assert_eq!(a.input.text, "keep q keep", "q is text, and replaces");
    a.input.cursor = 5;
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    a.paste("S\r\nx\u{1b}[0m");
    assert_eq!(a.input.text, "keep S\nx[0m keep");
    a.input.select_all();
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.input.text, "\n");
    a.input = Note::new("abcdef");
    a.input.cursor = 1;
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    key(&mut a, KeyCode::Backspace);
    assert_eq!(a.input.text, "aef");
    a.input.select_all();
    key(&mut a, KeyCode::Delete);
    assert_eq!(a.input.text, "");
    for c in ['s', '?', 'q'] {
        key(&mut a, KeyCode::Char(c));
    }
    assert_eq!(a.input.text, "s?q", "s, ? and q stay text in writing");
}

#[test]
fn oversize_paste_over_a_selection_changes_nothing_and_says_why() {
    let base = "a".repeat(4090) + "BCDEFG";
    let mut a = writing(&base);
    a.input.cursor = 4090;
    for _ in 0..2 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    let before = (a.input.text.clone(), a.input.cursor, a.input.selection());
    a.paste(&"z".repeat(20));
    assert_eq!(
        (a.input.text.clone(), a.input.cursor, a.input.selection()),
        before
    );
    assert!(a.notice.contains("4096"));
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("BC"));
}

#[test]
fn mouse_drag_in_writing_selects_the_same_logical_bytes() {
    let mut a = writing("drag these words");
    let (x, y) = (a.input_area.x, a.input_area.y);
    click(&mut a, MouseEventKind::Down(MouseButton::Left), x, y);
    click(&mut a, MouseEventKind::Drag(MouseButton::Left), x + 4, y);
    click(&mut a, MouseEventKind::Up(MouseButton::Left), x + 4, y);
    assert_eq!(a.input.selected_text(), Some("drag"));
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("drag"));
    // Dragging backwards, beyond the box, still yields ordered logical bytes.
    click(&mut a, MouseEventKind::Down(MouseButton::Left), x + 10, y);
    click(
        &mut a,
        MouseEventKind::Drag(MouseButton::Left),
        x + 5,
        y + 20,
    );
    click(&mut a, MouseEventKind::Up(MouseButton::Left), x + 5, y + 20);
    let picked = a.input.selected_text().unwrap().to_owned();
    assert!("drag these words".contains(&picked) && !picked.is_empty());
    // A plain click collapses.
    click(&mut a, MouseEventKind::Down(MouseButton::Left), x + 2, y);
    click(&mut a, MouseEventKind::Up(MouseButton::Left), x + 2, y);
    assert_eq!(a.input.selection(), None);
}

#[test]
fn selection_highlight_is_drawn_reversed_for_wide_and_combined_text() {
    let mut a = writing("e\u{301}中 b");
    a.input.cursor = 0;
    for _ in 0..2 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    t.draw(|f| render(f, &mut a, Palette::new(true))).unwrap();
    let b = t.backend().buffer();
    let (x, y) = (a.input_area.x, a.input_area.y);
    assert!(
        b[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
    assert!(
        b[(x + 1, y)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
    assert!(
        !b[(x + 3, y)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
}

#[test]
fn a_selection_does_not_change_what_f10_guards() {
    let mut a = writing("all of this");
    a.input.cursor = 0;
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    assert_eq!(a.input.text, "all of this");
    key(&mut a, KeyCode::F(10));
    assert!(!a.quit, "unsent words still ask first");
    assert!(a.leave_prompt || a.quit, "F10 keeps its guard");
}

// ---- Repairs after dogfooding (#14): stale drags, board-focus letters, hidden originals, limit notice.

fn drag_b(a: &mut BrainDump) {
    let (x, y) = (a.input_area.x, a.input_area.y);
    click(a, MouseEventKind::Down(MouseButton::Left), x + 2, y);
    click(a, MouseEventKind::Drag(MouseButton::Left), x + 1, y);
    assert_eq!(a.input.selected_text(), Some("b"));
}
fn release_at_end(a: &mut BrainDump) {
    let (x, y) = (a.input_area.x, a.input_area.y);
    click(a, MouseEventKind::Up(MouseButton::Left), x + 4, y);
}

#[test]
fn an_edit_during_a_drag_ends_it_so_release_cannot_resurrect_a_stale_selection() {
    // Multibyte replacement (used to panic) and ASCII replacement (used to lose text).
    for (typed, expect) in [("é", "aéxc."), ("Z", "aZxc.")] {
        let mut a = writing("abc.");
        drag_b(&mut a);
        for c in typed.chars() {
            key(&mut a, KeyCode::Char(c));
        }
        assert_eq!(
            a.input.cursor,
            1 + typed.len(),
            "caret just after the replacement"
        );
        release_at_end(&mut a);
        assert_eq!(
            a.input.selection(),
            None,
            "{typed}: release selects nothing"
        );
        key(&mut a, KeyCode::Char('x'));
        assert_eq!(a.input.text, expect, "{typed}");
    }
}

#[test]
fn paste_delete_and_buffer_replacement_during_a_drag_end_it_too() {
    let mut a = writing("abc.");
    drag_b(&mut a);
    a.paste("é");
    release_at_end(&mut a);
    key(&mut a, KeyCode::Char('x'));
    assert_eq!(a.input.text, "aéxc.", "paste");
    for code in [KeyCode::Backspace, KeyCode::Delete] {
        let mut a = writing("abc.");
        drag_b(&mut a);
        key(&mut a, code);
        assert_eq!(a.input.text, "ac.", "{code:?}");
        release_at_end(&mut a);
        key(&mut a, KeyCode::Char('x'));
        assert_eq!(a.input.text, "axc.", "{code:?} then x");
    }
    let mut a = writing("abc.");
    drag_b(&mut a);
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    release_at_end(&mut a);
    key(&mut a, KeyCode::Char('x'));
    assert_eq!(a.input.text, "x", "Ctrl-U replaced the buffer");
}

#[test]
fn a_rejected_oversize_edit_during_a_drag_changes_nothing_and_keeps_the_drag() {
    let mut a = writing(&("a".repeat(4090) + "BCDEFG"));
    let (x, y) = (a.input_area.x, a.input_area.y);
    click(&mut a, MouseEventKind::Down(MouseButton::Left), x, y);
    click(&mut a, MouseEventKind::Drag(MouseButton::Left), x + 3, y);
    let before = (a.input.text.clone(), a.input.cursor, a.input.selection());
    a.paste(&"z".repeat(5000));
    assert_eq!(
        (a.input.text.clone(), a.input.cursor, a.input.selection()),
        before
    );
    click(&mut a, MouseEventKind::Drag(MouseButton::Left), x + 5, y);
    assert!(
        a.input.selection().is_some(),
        "an unchanged text keeps its drag"
    );
}

fn with_card() -> BrainDump {
    let mut a = ready();
    a.extract_range(1, 0, 2).unwrap();
    a
}

#[test]
fn ordinary_letters_in_original_focus_return_to_writing_and_do_nothing_else() {
    for c in ['s', 'y', 'o', 'd', 'u', '?', 'e', 'q'] {
        let mut a = with_card();
        let notice = a.notice.clone();
        key(&mut a, KeyCode::Tab);
        assert!(a.board_focus);
        key(&mut a, KeyCode::Char(c));
        assert_eq!(a.input.text, c.to_string(), "{c} is text");
        assert!(!a.board_focus, "{c} returned focus to writing");
        assert!(a.skipped.is_empty(), "{c} skipped nothing");
        assert!(a.take_copy_request().is_none(), "{c} copied nothing");
        assert!(
            !a.original && !a.details && !a.help && !a.show_extractions,
            "{c}"
        );
        assert_eq!(a.notice, notice, "{c} changed no state");
    }
}

#[test]
fn f8_f9_f7_and_modified_keys_still_work_from_either_focus() {
    for focus in [false, true] {
        let mut a = with_card();
        if focus {
            key(&mut a, KeyCode::Tab);
        }
        assert!(a.focused_question().is_some());
        key(&mut a, KeyCode::F(9));
        assert_eq!(a.take_copy_request(), Some(a.paper()), "F9 focus={focus}");
        key(&mut a, KeyCode::F(8));
        assert_eq!(a.skipped.len(), 1, "F8 focus={focus}");
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert!(a.skipped.is_empty(), "Ctrl-Z undoes the skip");
        key(&mut a, KeyCode::F(7));
        assert!(a.show_extractions, "F7 shows cards, focus={focus}");
        press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(!a.show_extractions, "Ctrl-X hides them");
        press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(a.show_extractions);
        assert!(a.input.text.is_empty());
    }
    let mut a = with_card();
    key(&mut a, KeyCode::Tab);
    press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert!(
        a.board_focus && a.input.text.is_empty(),
        "Ctrl-F is not text"
    );
    let mut none = ready();
    key(&mut none, KeyCode::F(7));
    assert!(none.notice.contains("No extracted cards") && !none.show_extractions);
}

#[test]
fn cards_covering_the_original_hide_its_selection_but_not_the_writing_box() {
    // Tab -> Ctrl-A -> cards -> Ctrl-C: nothing from the hidden original is copied.
    let mut a = with_card();
    key(&mut a, KeyCode::Tab);
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert!(a.selection.is_some());
    key(&mut a, KeyCode::F(7));
    assert_eq!(a.selection, None, "cards clear the original's selection");
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none());
    // Selecting while cards are displayed is refused too.
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none(), "no hidden select-all");
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    ctrl_c(&mut a);
    assert!(a.take_copy_request().is_none());
    // The visible writing box still selects and copies once it has focus.
    for c in "hello".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    assert!(a.show_extractions && !a.board_focus);
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    ctrl_c(&mut a);
    assert_eq!(a.take_copy_request().as_deref(), Some("hello"));
    // Tab does not walk into the hidden original, and cards close back to a copyable original.
    key(&mut a, KeyCode::Tab);
    assert!(!a.board_focus);
    key(&mut a, KeyCode::F(7));
    press(&mut a, KeyCode::Char('a'), KeyModifiers::CONTROL);
    ctrl_c(&mut a);
    assert_eq!(
        a.take_copy_request().as_deref(),
        Some("hello"),
        "the writing box stays selectable"
    );
}

#[test]
fn the_oversize_notice_is_rendered_cleared_by_the_next_input_and_beats_other_status_rows() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = writing(&("a".repeat(4090) + "BCDEFG"));
        a.input.cursor = 4090;
        for _ in 0..3 {
            press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        }
        let before = (a.input.text.clone(), a.input.cursor, a.input.selection());
        a.paste(&"z".repeat(20));
        assert_eq!(
            (a.input.text.clone(), a.input.cursor, a.input.selection()),
            before
        );
        let shown = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            shown.contains("Over 4096 bytes; nothing changed."),
            "{w}x{h}\n{shown}"
        );
        // Overlapping status: an unapplied reading would draw "Reading couldn't be updated."
        a.applied = 0;
        let shown = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            shown.contains("Over 4096 bytes"),
            "{w}x{h} with an unapplied reading\n{shown}"
        );
        // And a running request would draw "Thinking…" on the same row.
        let (_tx, rx) = std::sync::mpsc::channel();
        a.handoff_job = Some(rx);
        let shown = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            shown.contains("Over 4096 bytes") && !shown.contains("Thinking…"),
            "{w}x{h} running\n{shown}"
        );
        a.handoff_job = None;
        // The next input clears it: no permanent clutter.
        key(&mut a, KeyCode::Left);
        assert!(
            !snapshot(w, h, &mut a, false)
                .unwrap()
                .contains("Over 4096 bytes"),
            "{w}x{h}"
        );
        assert_eq!(a.input.text, before.0);
    }
}
