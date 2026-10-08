use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};
use tinkery::{App, Palette, Pane, render, snapshot};

fn key(app: &mut App, code: KeyCode) {
    app.handle_key(
        KeyEvent::new(code, KeyModifiers::NONE),
        Rect::new(0, 0, 100, 30),
    );
}

#[test]
fn cabinet_at_100_by_30_is_explicit_and_readable() {
    let screen = snapshot(100, 30, &mut App::default(), false).unwrap();
    assert_eq!(screen.lines().count(), 30);
    assert!(screen.lines().all(|line| line.chars().count() == 100));
    for text in [
        "tinkery",
        "demo / read only",
        "Work",
        "A place for the work",
        "1 question waiting [a]",
        "Tab page",
        "[Brief]   Evidence",
    ] {
        assert!(screen.contains(text), "missing {text}\n{screen}");
    }
    assert!(!screen.lines().any(|line| line.trim() == "Attention"));
    assert!(!screen.contains("Does this feel like a"));
    assert!(!screen.contains("ARTIFACT"));
    assert!(!screen.contains("---"));
    assert!(!screen.contains("|"));
    assert_eq!(screen.matches("demo / read only").count(), 1);
}

#[test]
fn palette_matches_arcade_and_has_monochrome_fallback() {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| render(f, &mut App::default(), Palette::new(false)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].bg, Color::Rgb(255, 252, 240));
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(31, 122, 114))
    );
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.fg == Color::Rgb(16, 15, 15))
    );
    terminal
        .draw(|f| render(f, &mut App::default(), Palette::new(true)))
        .unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
    );
}

#[test]
fn selection_updates_both_artifact_and_attention_and_clamps() {
    let mut app = App::default();
    key(&mut app, KeyCode::Up);
    assert_eq!(app.selected, 0);
    key(&mut app, KeyCode::Down);
    let screen = snapshot(100, 30, &mut app, false).unwrap();
    assert!(screen.contains("Make the handoff legible"));
    assert!(!screen.contains("Separate artifacts or"));
    key(&mut app, KeyCode::Char('a'));
    let drawer = snapshot(100, 30, &mut app, false).unwrap();
    assert!(drawer.contains("Separate artifacts or"));
    assert!(drawer.contains("Attention"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::End);
    key(&mut app, KeyCode::Down);
    assert_eq!(app.selected, 2);
    key(&mut app, KeyCode::Home);
    assert_eq!(app.selected, 0);
}

#[test]
fn pane_focus_cycles_in_both_directions() {
    let mut app = App::default();
    for expected in [Pane::Artifact, Pane::Work, Pane::Artifact, Pane::Work] {
        key(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, expected);
    }
    key(&mut app, KeyCode::BackTab);
    assert_eq!(app.pane, Pane::Artifact);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pane, Pane::Artifact);
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.pane, Pane::Work);
}

#[test]
fn scrolling_reaches_end_and_switching_artifact_resets_it() {
    let mut app = App::default();
    let initial = snapshot(100, 30, &mut app, false).unwrap();
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::End);
    let end = snapshot(100, 30, &mut app, false).unwrap();
    assert_ne!(initial, end);
    assert!(end.contains("Later, not now"));
    assert!(!end.contains("A place for the work"));
    key(&mut app, KeyCode::Right);
    let evidence = snapshot(100, 30, &mut app, false).unwrap();
    assert!(evidence.contains("Example evidence"));
    assert!(evidence.contains("[Evidence]"));
    key(&mut app, KeyCode::Left);
    let brief = snapshot(100, 30, &mut app, false).unwrap();
    assert!(brief.contains("A place for the work"));
}

#[test]
fn help_is_modal_and_quit_still_works() {
    let mut app = App::default();
    key(&mut app, KeyCode::Char('?'));
    let screen = snapshot(100, 30, &mut app, false).unwrap();
    assert!(screen.contains("Keys"));
    assert!(screen.contains("All content is embedded. No files are written."));
    key(&mut app, KeyCode::Down);
    assert_eq!(app.selected, 0);
    key(&mut app, KeyCode::Esc);
    assert!(!app.help);
    key(&mut app, KeyCode::Char('?'));
    key(&mut app, KeyCode::Char('q'));
    assert!(app.quit);
}

#[test]
fn undersized_terminal_preserves_selection_but_allows_quit() {
    let mut app = App::default();
    for (width, height) in [(1, 1), (60, 20), (79, 30), (100, 23)] {
        let screen = snapshot(width, height, &mut app, false).unwrap();
        if width >= 60 {
            assert!(screen.contains("Resize to at least 80 x 24."));
        }
        app.handle_key(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            Rect::new(0, 0, width, height),
        );
        assert_eq!(app.selected, 0);
    }
    app.handle_key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Rect::new(0, 0, 1, 1),
    );
    assert!(app.quit);
}

#[test]
fn release_events_do_not_trigger_navigation() {
    let mut app = App::default();
    let mut event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    event.kind = KeyEventKind::Release;
    app.handle_key(event, Rect::new(0, 0, 100, 30));
    assert_eq!(app.selected, 0);
}

#[test]
fn resizing_clamps_scroll_and_keeps_every_pane_available() {
    let mut app = App::default();
    snapshot(80, 24, &mut app, false).unwrap();
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::End);
    for (width, height) in [(100, 30), (120, 36), (180, 60), (80, 24)] {
        let screen = snapshot(width, height, &mut app, false).unwrap();
        assert!(screen.contains("demo / read only"));
        assert!(screen.contains("1 question waiting [a]"));
        key(&mut app, KeyCode::Char('a'));
        let drawer = snapshot(width, height, &mut app, false).unwrap();
        assert!(drawer.contains("Attention"));
        key(&mut app, KeyCode::Esc);
        assert!(screen.contains("q quit"));
    }
}
