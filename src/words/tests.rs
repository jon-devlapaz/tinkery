use super::*;

fn key(words: &mut Words, code: KeyCode, modifiers: KeyModifiers) {
    words.key(KeyEvent::new(code, modifiers));
    assert_eq!(display(words.text()), words.state.editor.lines().join("\n"));
}

#[test]
fn editing_a_word_and_replacing_a_selection_preserves_original_breaks() {
    use KeyCode::*;
    let mut words = Words::new("Local remote\r\nkeep\r漢字");
    key(&mut words, Up, KeyModifiers::NONE);
    key(&mut words, Up, KeyModifiers::NONE);
    key(&mut words, End, KeyModifiers::NONE);
    key(&mut words, Char('w'), KeyModifiers::CONTROL);
    words.insert("only");
    assert_eq!(words.text(), "Local only\r\nkeep\r漢字");
    key(&mut words, Home, KeyModifiers::NONE);
    for _ in 0..5 {
        key(&mut words, Right, KeyModifiers::SHIFT);
    }
    words.insert("TEST");
    assert_eq!(words.text(), "TEST only\r\nkeep\r漢字");
    key(&mut words, Enter, KeyModifiers::NONE);
    assert_eq!(words.text(), "TEST\n only\r\nkeep\r漢字");
}

#[test]
fn identical_newlines_keep_their_own_provenance_through_edits_and_undo() {
    use KeyCode::*;
    let mut words = Words::new("\r\n\r\n\r");
    key(&mut words, Left, KeyModifiers::NONE);
    key(&mut words, Backspace, KeyModifiers::NONE);
    assert_eq!(words.text(), "\r\n\r");
    key(&mut words, Char('u'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), "\r\n\r\n\r");
    key(&mut words, Char('r'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), "\r\n\r");
    key(&mut words, Left, KeyModifiers::SHIFT);
    words.insert("\n");
    assert_eq!(words.text(), "\n\r");
    key(&mut words, Char('u'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), "\r\n\r");
    key(&mut words, Char('r'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), "\n\r");
}

#[test]
fn edits_that_form_a_crlf_reproject_without_losing_bytes() {
    use KeyCode::*;
    for (original, action) in [("\n", None), ("\rX\n", Some(Delete))] {
        let mut words = Words::new(original);
        words.state.editor.move_cursor(CursorMove::Top);
        if let Some(action) = action {
            key(&mut words, Down, KeyModifiers::NONE);
            key(&mut words, action, KeyModifiers::NONE);
        } else {
            words.insert("\r");
        }
        assert_eq!(words.text(), "\r\n");
        assert_eq!(words.state.editor.lines(), &["", ""]);
        words.insert("TEST");
        assert_eq!(words.text(), "\r\nTEST");
    }
}

#[test]
fn copying_cutting_and_yanking_preserve_raw_line_endings() {
    use KeyCode::*;
    let original = "a\r\nb\rc";
    let mut words = Words::new(original);
    words.state.editor.move_cursor(CursorMove::Top);
    words.state.editor.move_cursor(CursorMove::Head);
    words.state.editor.start_selection();
    words.state.editor.move_cursor(CursorMove::Bottom);
    words.state.editor.move_cursor(CursorMove::End);
    key(&mut words, Char('x'), KeyModifiers::CONTROL);
    assert!(words.text().is_empty());
    key(&mut words, Char('y'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), original);
    words.state.editor.move_cursor(CursorMove::Top);
    words.state.editor.move_cursor(CursorMove::Head);
    words.state.editor.start_selection();
    words.state.editor.move_cursor(CursorMove::Bottom);
    words.state.editor.move_cursor(CursorMove::End);
    key(&mut words, Char('c'), KeyModifiers::CONTROL);
    words.state.editor.cancel_selection();
    key(&mut words, Char('y'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), format!("{original}{original}"));
    key(&mut words, Char('!'), KeyModifiers::NONE);
    key(&mut words, Char('y'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), format!("{original}{original}!{original}"));
}

#[test]
fn typing_after_copy_does_not_clear_the_raw_yank_buffer() {
    use KeyCode::*;
    let original = "TEST copy\r\nnext";
    let mut words = Words::new(original);
    words.state.editor.move_cursor(CursorMove::Top);
    words.state.editor.move_cursor(CursorMove::Head);
    words.state.editor.start_selection();
    words.state.editor.move_cursor(CursorMove::Bottom);
    words.state.editor.move_cursor(CursorMove::End);
    key(&mut words, Char('c'), KeyModifiers::CONTROL);
    words.state.editor.cancel_selection();
    key(&mut words, Char('!'), KeyModifiers::NONE);
    key(&mut words, Char('y'), KeyModifiers::CONTROL);
    assert_eq!(words.text(), format!("{original}!{original}"));
}

#[test]
fn mixed_motions_selection_edits_and_history_keep_raw_and_editor_in_sync() {
    use KeyCode::*;
    let mut words = Words::new("TEST α\r\n漢字\rwords here\nlast");
    let codes = [
        Left,
        Right,
        Up,
        Down,
        Home,
        End,
        Enter,
        Backspace,
        Delete,
        Tab,
        Char('w'),
        Char('u'),
        Char('r'),
        Char('x'),
        Char('c'),
        Char('y'),
        Char('界'),
    ];
    let modifiers = [
        KeyModifiers::NONE,
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
    ];
    let mut random = 163_u64;
    for i in 0..2000 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        key(
            &mut words,
            codes[(random >> 32) as usize % codes.len()],
            modifiers[(random >> 48) as usize % modifiers.len()],
        );
        if i % 31 == 0 {
            words.insert("\r\n\rTEST\n");
            assert_eq!(display(words.text()), words.state.editor.lines().join("\n"));
        }
    }
}
