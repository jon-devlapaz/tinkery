use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use tinkery::shaping::{Focus, Note, Shape, snapshot};

fn mouse(app: &mut Shape, kind: MouseEventKind, x: u16, y: u16, area: Rect) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
}

fn click(app: &mut Shape, x: u16, y: u16) {
    mouse(
        app,
        MouseEventKind::Down(MouseButton::Left),
        x,
        y,
        Rect::new(0, 0, 100, 30),
    );
}

fn locate(screen: &str, label: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(y, line)| line.find(label).map(|x| (x as u16, y as u16)))
        .unwrap()
}

fn click_control(app: &mut Shape, label: &str) {
    let screen = snapshot(100, 30, app, false).unwrap();
    let (x, y) = locate(&screen, label);
    click(app, x, y);
}

#[test]
fn card_click_places_cursor_and_paper_click_is_read_only() {
    let mut app = Shape::default();
    snapshot(100, 30, &mut app, false).unwrap();
    click(&mut app, 4, 8);
    assert!(app.editing);
    assert_eq!(app.note.cursor, 0);
    app.paste("First: ", Rect::new(0, 0, 100, 30));
    assert!(app.note.text.starts_with("First: I keep"));
    snapshot(100, 30, &mut app, false).unwrap();
    let note = app.note.text.clone();
    click(&mut app, 40, 10);
    assert!(!app.editing);
    assert_eq!(app.focus, Focus::Paper);
    app.paste("cannot edit paper", Rect::new(0, 0, 100, 30));
    assert_eq!(app.note.text, note);
    assert!(app.sent_note().is_none());
}

#[test]
fn clicks_handle_wrapped_text_wide_characters_and_blank_lines() {
    let mut app = Shape::default();
    app.note = Note::new("界🙂\n\nhello");
    snapshot(100, 30, &mut app, false).unwrap();
    click(&mut app, 5, 8); // The second cell of a wide grapheme.
    assert_eq!(app.note.cursor, 0);
    click(&mut app, 20, 9); // Empty logical line.
    assert!(app.note.text.is_char_boundary(app.note.cursor));
    app.paste("blank", Rect::new(0, 0, 100, 30));
    assert!(app.note.text.contains("\nblank\n"));
    app.note = Note::new(&"a".repeat(26));
    snapshot(100, 30, &mut app, false).unwrap();
    click(&mut app, 4, 9);
    assert_eq!(app.note.cursor, 25);
}

#[test]
fn footer_controls_send_pause_focus_help_and_quit() {
    let mut app = Shape::default();
    click_control(&mut app, "e edit");
    assert!(app.editing);
    click_control(&mut app, "F2 send");
    assert!(app.drafting());
    let first = app.paper();
    click_control(&mut app, "Drafting...");
    app.tick(Duration::from_secs(2), Rect::new(0, 0, 100, 30));
    assert_eq!(app.paper(), first);
    click_control(&mut app, "Paused");
    app.tick(Duration::from_secs(1), Rect::new(0, 0, 100, 30));
    assert_ne!(app.paper(), first);
    click_control(&mut app, "Tab focus");
    assert_eq!(app.focus, Focus::Paper);
    click_control(&mut app, "? help");
    assert!(app.help);
    snapshot(100, 30, &mut app, false).unwrap();
    click(&mut app, 4, 8);
    assert!(!app.editing); // Help prevents edits behind it.
    click_control(&mut app, "? close help");
    assert!(!app.help);
    click_control(&mut app, "e edit");
    click_control(&mut app, "Ctrl-C quit");
    assert!(app.quit);
}

#[test]
fn wheel_scrolls_each_surface_without_moving_focus_or_changing_note() {
    let mut app = Shape::default();
    app.note = Note::new(&"A long thought.\n".repeat(40));
    app.send();
    for _ in 0..4 {
        app.tick(Duration::from_secs(1), Rect::new(0, 0, 100, 30));
    }
    app.handle_key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        Rect::new(0, 0, 100, 30),
    );
    let original = app.note.text.clone();
    snapshot(100, 30, &mut app, false).unwrap();
    for _ in 0..20 {
        mouse(
            &mut app,
            MouseEventKind::ScrollUp,
            10,
            10,
            Rect::new(0, 0, 100, 30),
        );
    }
    let card_top = snapshot(100, 30, &mut app, false).unwrap();
    assert!(card_top.lines().nth(19).unwrap().contains("more below"));
    mouse(
        &mut app,
        MouseEventKind::ScrollDown,
        40,
        10,
        Rect::new(0, 0, 100, 30),
    );
    let scrolled = snapshot(100, 30, &mut app, false).unwrap();
    assert!(scrolled.contains("more above / below"));
    assert!(app.editing);
    assert_eq!(app.focus, Focus::Card);
    assert_eq!(app.note.text, original);
    click(&mut app, 4, 8); // Clicking the scrolled card uses the visible row.
    assert_eq!(app.note.cursor, 0);
    click(&mut app, 40, 26); // Continuation cue pages the paper.
    assert_eq!(app.focus, Focus::Paper);
    assert!(!app.editing);
}

#[test]
fn blank_space_right_clicks_releases_and_stale_resize_targets_do_nothing() {
    let mut app = Shape::default();
    snapshot(100, 30, &mut app, false).unwrap();
    for kind in [
        MouseEventKind::Moved,
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        mouse(&mut app, kind, 4, 8, Rect::new(0, 0, 100, 30));
    }
    click(&mut app, 0, 0);
    assert!(!app.editing);
    assert!(app.sent_note().is_none());
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        4,
        8,
        Rect::new(0, 0, 180, 60),
    );
    assert!(!app.editing);
    let screen = snapshot(180, 60, &mut app, false).unwrap();
    let (x, y) = locate(&screen, "Your note");
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        x + 2,
        y + 4,
        Rect::new(0, 0, 180, 60),
    );
    assert!(app.editing);
    let note = app.note.text.clone();
    snapshot(60, 20, &mut app, false).unwrap();
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        4,
        8,
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(app.note.text, note);
}
