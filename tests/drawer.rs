use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};
use tinkery::{App, Palette, Pane, render, snapshot};

fn key(app: &mut App, code: KeyCode) {
    app.handle_key(
        KeyEvent::new(code, KeyModifiers::NONE),
        Rect::new(0, 0, 100, 30),
    );
}

#[test]
fn drawer_restores_focus_selection_artifact_and_reading_position() {
    for original in [Pane::Work, Pane::Artifact] {
        let mut app = App::default();
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Enter);
        snapshot(100, 30, &mut app, false).unwrap();
        key(&mut app, KeyCode::End);
        key(&mut app, KeyCode::Right);
        key(
            &mut app,
            if original == Pane::Work {
                KeyCode::Char('1')
            } else {
                KeyCode::Char('2')
            },
        );
        let before = snapshot(100, 30, &mut app, false).unwrap();
        key(&mut app, KeyCode::Char('a'));
        assert_eq!(app.pane, Pane::Attention);
        let drawer = snapshot(100, 30, &mut app, false).unwrap();
        assert!(drawer.contains("Attention"));
        assert!(drawer.contains("Separate artifacts or"));
        assert!(drawer.contains("Example evidence"));
        assert!(drawer.contains("demo / read only"));
        // Navigation in the drawer never selects different work or artifacts.
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.selected, 1);
        assert!(app.evidence);
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.pane, original);
        assert_eq!(snapshot(100, 30, &mut app, false).unwrap(), before);
    }
}

#[test]
fn drawer_can_close_with_toggle_or_tab_and_help_preserves_it() {
    let mut app = App::default();
    for close in [
        KeyCode::Char('a'),
        KeyCode::Char('3'),
        KeyCode::Tab,
        KeyCode::BackTab,
    ] {
        key(&mut app, KeyCode::Char('3'));
        assert_eq!(app.pane, Pane::Attention);
        key(&mut app, close);
        assert_eq!(app.pane, Pane::Work);
    }
    key(&mut app, KeyCode::Char('a'));
    key(&mut app, KeyCode::Char('?'));
    assert!(snapshot(100, 30, &mut app, false).unwrap().contains("Keys"));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.pane, Pane::Attention);
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("Example question")
    );
    key(&mut app, KeyCode::Char('q'));
    assert!(app.quit);
}

#[test]
fn compact_header_footer_and_carbon_document_at_100_by_30() {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| render(frame, &mut App::default(), Palette::new(false)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(2, 1)].symbol(), "t");
    assert_eq!(buffer[(82, 1)].symbol(), "d");
    assert_eq!(buffer[(26, 4)].symbol(), "A");
    assert_eq!(buffer[(26, 4)].fg, Color::Rgb(16, 15, 15));
    assert_eq!(buffer[(26, 8)].fg, Color::Rgb(16, 15, 15));
    // Jade belongs to the active navigation only, not the document or chrome.
    let jade = Color::Rgb(31, 122, 114);
    for y in 0..30 {
        for x in 0..100 {
            if buffer[(x, y)].fg == jade {
                assert!(x < 22 && (y == 4 || y == 6), "unexpected jade at {x},{y}");
            }
        }
    }
    let screen = snapshot(100, 30, &mut App::default(), false).unwrap();
    assert!(
        screen
            .lines()
            .nth(28)
            .unwrap()
            .contains("1 question waiting [a]")
    );
    assert!(screen.lines().nth(28).unwrap().contains("q quit"));
    assert!(screen.lines().last().unwrap().trim().is_empty());
}

#[test]
fn drawer_and_help_are_readable_at_minimum_size_and_recover_from_resize() {
    let mut app = App::default();
    key(&mut app, KeyCode::Char('a'));
    for (width, height) in [(80, 24), (100, 30), (120, 36), (180, 60)] {
        let drawer = snapshot(width, height, &mut app, true).unwrap();
        assert!(drawer.contains("Attention"));
        assert!(drawer.contains("Example question"));
        assert!(drawer.contains("q quit"));
        key(&mut app, KeyCode::Char('?'));
        let help = snapshot(width, height, &mut app, true).unwrap();
        assert!(help.contains("All content is embedded. No files are written."));
        key(&mut app, KeyCode::Esc);
    }
    let small = snapshot(60, 20, &mut app, false).unwrap();
    assert!(small.contains("Resize to at least 80 x 24."));
    app.handle_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(app.pane, Pane::Attention);
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("Example question")
    );
}
