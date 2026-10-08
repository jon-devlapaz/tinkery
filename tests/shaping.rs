use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use tinkery::shaping::{Focus, Note, Shape, snapshot};

fn area() -> Rect {
    Rect::new(0, 0, 100, 30)
}
fn key(app: &mut Shape, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), area());
}
fn finish(app: &mut Shape) {
    for _ in 0..5 {
        app.tick(Duration::from_secs(1), area());
    }
}

#[test]
fn note_card_and_paper_are_readable_at_100_by_30() {
    let screen = snapshot(100, 30, &mut Shape::default(), false).unwrap();
    assert_eq!(screen.lines().count(), 30);
    for text in [
        "tinkery / seed",
        "simulated / unsaved",
        "Your note",
        "Working paper",
        "+---------------------------+",
        "I keep losing track",
        "A place to begin",
        "F2 gives it a first shape.",
        "F2 send",
    ] {
        assert!(screen.contains(text), "Missing {text}\n{screen}");
    }
    assert!(!screen.contains("confirmed for intake"));
}

#[test]
fn writer_captures_a_copy_never_mutates_card_or_steals_focus() {
    let mut app = Shape::default();
    let original = app.note.text.clone();
    key(&mut app, KeyCode::F(2));
    assert!(app.drafting());
    key(&mut app, KeyCode::Enter);
    app.paste(" An extra thought.", area());
    let changed = app.note.text.clone();
    finish(&mut app);
    assert_eq!(app.note.text, changed);
    assert!(app.editing);
    assert_eq!(app.focus, Focus::Card);
    assert_eq!(app.sent_note(), Some(original.as_str()));
    assert!(
        app.paper()
            .contains("Keep the reasons behind important decisions")
    );
    assert!(!app.paper().contains("An extra thought."));
    assert!(!app.drafting());
    key(&mut app, KeyCode::F(2));
    finish(&mut app);
    assert!(app.paper().contains("An extra thought."));
    assert_eq!(app.note.text, changed);
}

#[test]
fn keyboard_editing_paste_and_q_do_not_trigger_commands() {
    let mut app = Shape::default();
    key(&mut app, KeyCode::Enter);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        area(),
    );
    app.paste("A note.\r\nSecond line.\x1b", area());
    assert_eq!(app.note.text, "A note.\nSecond line.");
    key(&mut app, KeyCode::Char('q'));
    assert!(!app.quit);
    assert!(app.note.text.ends_with('q'));
    key(&mut app, KeyCode::Backspace);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Char('?'));
    assert!(app.note.text.ends_with("\n?"));
    assert!(!app.help);
    key(&mut app, KeyCode::Esc);
    assert!(!app.editing);
    app.paste("not inserted", area());
    assert!(!app.note.text.contains("not inserted"));
    key(&mut app, KeyCode::Char('q'));
    assert!(app.quit);
}

#[test]
fn empty_notes_and_oversize_pastes_have_explicit_errors() {
    let mut app = Shape::default();
    key(&mut app, KeyCode::Enter);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        area(),
    );
    key(&mut app, KeyCode::F(2));
    assert!(!app.drafting());
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("Write a thought on the card first.")
    );
    app.paste(&"a".repeat(4097), area());
    assert!(app.note.text.is_empty());
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("4096 bytes. Nothing inserted.")
    );
}

#[test]
fn pause_help_and_small_terminal_freeze_the_writer() {
    let mut app = Shape::default();
    app.send();
    let first = app.paper();
    key(&mut app, KeyCode::Char('x'));
    finish(&mut app);
    assert_eq!(app.paper(), first);
    key(&mut app, KeyCode::Char('x'));
    key(&mut app, KeyCode::Char('?'));
    finish(&mut app);
    assert_eq!(app.paper(), first);
    key(&mut app, KeyCode::Esc);
    app.tick(Duration::from_secs(60), Rect::new(0, 0, 60, 20));
    assert_eq!(app.paper(), first);
    finish(&mut app);
    assert!(!app.drafting());
}

#[test]
fn long_papers_scroll_without_following_new_sections() {
    let mut app = Shape::default();
    app.note = Note::new(&"a long note with several words\n".repeat(30));
    app.send();
    key(&mut app, KeyCode::Tab);
    app.tick(Duration::from_secs(1), area());
    snapshot(100, 30, &mut app, false).unwrap();
    key(&mut app, KeyCode::PageDown);
    let scrolled = snapshot(100, 30, &mut app, false).unwrap();
    app.tick(Duration::from_secs(1), area());
    let updated = snapshot(100, 30, &mut app, false).unwrap();
    assert_eq!(scrolled.lines().nth(8), updated.lines().nth(8));
    assert_eq!(app.focus, Focus::Paper);
    key(&mut app, KeyCode::Home);
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("A first shape")
    );
}

#[test]
fn unicode_editing_moves_and_deletes_whole_graphemes() {
    let mut note = Note::new("e\u{301}🙂界");
    note.left();
    note.delete();
    assert_eq!(note.text, "e\u{301}🙂");
    note.backspace();
    assert_eq!(note.text, "e\u{301}");
    note.backspace();
    assert!(note.text.is_empty());
    note.insert("👩‍💻").unwrap();
    note.left();
    assert_eq!(note.cursor, 0);
    note.right();
    assert_eq!(note.cursor, note.text.len());
    note.backspace();
    assert!(note.text.is_empty());
    // Removing the separator joins two regional indicators into one grapheme.
    let mut note = Note::new("🇦x🇧");
    note.left();
    note.backspace();
    assert_eq!(note.text, "🇦🇧");
    note.wrap(8);
    note.delete();
    assert!(note.text.is_empty());
}

#[test]
fn wrapping_keeps_words_and_wide_characters_inside_the_card() {
    let mut note = Note::new("hello there world\n界🙂");
    let wrapped = note.wrap(8);
    assert!(wrapped.lines.iter().any(|line| line == "there "));
    assert!(
        wrapped
            .lines
            .iter()
            .all(|line| unicode_width::UnicodeWidthStr::width(line.as_str()) <= 8)
    );
    note.home();
    assert!(note.text[note.cursor..].starts_with("界"));
    note.move_row(false, 8);
    assert!(note.cursor < note.text.find('\n').unwrap());
    note.move_row(true, 8);
    assert!(note.text.is_char_boundary(note.cursor));
}

#[test]
fn help_resize_release_and_ctrl_c_keep_the_prototype_safe() {
    let mut app = Shape::default();
    key(&mut app, KeyCode::Char('?'));
    assert!(
        snapshot(80, 24, &mut app, true)
            .unwrap()
            .contains("Template writer. No sessions or saves.")
    );
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Enter);
    app.paste(" More context.", area());
    for (w, h) in [(80, 24), (100, 30), (180, 60), (1, 1)] {
        snapshot(w, h, &mut app, false).unwrap();
    }
    let before = app.note.text.clone();
    let mut event = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
    event.kind = KeyEventKind::Release;
    app.handle_key(event, area());
    assert_eq!(app.note.text, before);
    app.handle_key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Rect::new(0, 0, 1, 1),
    );
    assert!(app.quit);
}
