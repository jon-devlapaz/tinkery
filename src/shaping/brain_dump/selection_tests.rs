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
