use super::*;

fn area() -> Rect {
    Rect::new(0, 0, 100, 30)
}
fn key(app: &mut Shape, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), area());
}
fn control(app: &mut Shape, c: char) {
    app.handle_key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        area(),
    );
}
fn mouse(app: &mut Shape, kind: MouseEventKind, x: u16, y: u16) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        area(),
    );
}
fn click(app: &mut Shape, x: u16, y: u16) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), x, y);
    mouse(app, MouseEventKind::Up(MouseButton::Left), x, y);
}
fn screen(app: &mut Shape) -> String {
    snapshot(100, 30, app, false).unwrap()
}
fn paper_columns(screen: &str) -> Vec<String> {
    screen
        .lines()
        .skip(4)
        .take(23)
        .map(|line| line.chars().skip(35).collect())
        .collect()
}

#[test]
fn actual_pinstar_node_and_completed_paper_fit_supported_sizes() {
    let mut app = Shape::canvas();
    let initial = screen(&mut app);
    assert!(initial.contains("Note canvas / Pinstar"));
    assert!(initial.contains('⇘')); // Upstream selection renderer, not our old card.
    assert!(initial.contains("I keep losing track"));
    let canvas = app.canvas.as_ref().unwrap();
    assert_eq!(canvas.state.data.nodes.len(), 1);
    assert!(canvas.state.data.edges.is_empty());
    assert!(!canvas.state.settings.persist_changes);
    assert!(canvas.state.path.as_os_str().is_empty());
    app.send();
    for _ in 0..4 {
        app.tick(Duration::from_secs(1), area());
    }
    let completed = screen(&mut app);
    assert!(completed.contains("One open question"));
    assert!(completed.contains("Draft / unconfirmed"));
    for (width, height) in [(80, 24), (100, 30), (180, 60)] {
        let view = snapshot(width, height, &mut app, true).unwrap();
        assert!(view.contains("Note canvas / Pinstar"));
        assert!(view.contains("Working paper / sample"));
        key(&mut app, KeyCode::Char('?'));
        let help = snapshot(width, height, &mut app, true).unwrap();
        assert!(help.contains("? / Esc      Close help"));
        key(&mut app, KeyCode::Esc);
    }
}

#[test]
fn upstream_double_click_and_editor_submit_live_text_without_losing_original() {
    let mut app = Shape::canvas();
    screen(&mut app);
    click(&mut app, 5, 10);
    assert!(!app.editing);
    click(&mut app, 5, 10);
    assert!(app.editing);
    control(&mut app, 'u');
    app.paste("A different thought.", area());
    key(&mut app, KeyCode::Char('q'));
    assert_eq!(app.note.text, "A different thought.q");
    assert!(!app.quit);
    key(&mut app, KeyCode::F(2));
    assert!(!app.editing);
    assert_eq!(app.sent_note(), Some("A different thought.q"));
    key(&mut app, KeyCode::Char('e'));
    app.paste("Another ", area());
    assert!(screen(&mut app).contains("Unsent changes"));
    assert_eq!(app.sent_note(), Some("A different thought.q"));
    click(&mut app, 40, 10);
    assert_eq!(app.focus, Focus::Paper);
    assert!(!app.editing);
    assert!(app.canvas.as_ref().unwrap().state.floating_editor.is_none());
}

#[test]
fn upstream_drag_pan_and_zoom_leave_paper_and_note_text_unchanged() {
    let mut app = Shape::canvas();
    app.send();
    let before = screen(&mut app);
    let original = app.note.text.clone();
    let pos = app.canvas.as_ref().unwrap().state.data.nodes[0].pos();
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 10, 12);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 13, 14);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 13, 14);
    assert_ne!(app.canvas.as_ref().unwrap().state.data.nodes[0].pos(), pos);
    let after = screen(&mut app);
    assert_eq!(paper_columns(&before), paper_columns(&after));
    let viewport = app.canvas.as_ref().unwrap().state.viewport_x;
    mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), 10, 10);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Middle), 15, 10);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Middle), 15, 10);
    assert_ne!(app.canvas.as_ref().unwrap().state.viewport_x, viewport);
    let zoom = app.canvas.as_ref().unwrap().state.zoom;
    mouse(&mut app, MouseEventKind::ScrollUp, 10, 10);
    assert!(app.canvas.as_ref().unwrap().state.zoom > zoom);
    control(&mut app, 'f');
    assert_eq!(app.note.text, original);
    assert_eq!(app.sent_note(), Some(original.as_str()));
    assert_eq!(paper_columns(&before), paper_columns(&screen(&mut app)));
}

#[test]
fn resize_and_extreme_projection_cannot_paint_over_the_paper() {
    let mut app = Shape::canvas();
    let before = screen(&mut app);
    let size = app.canvas.as_ref().unwrap().state.data.nodes[0].size();
    key(&mut app, KeyCode::Char('s'));
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 28, 20);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 30, 22);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 30, 22);
    assert_ne!(
        app.canvas.as_ref().unwrap().state.data.nodes[0].size(),
        size
    );
    key(&mut app, KeyCode::Esc);
    assert!(
        app.canvas
            .as_ref()
            .unwrap()
            .state
            .resizing_node_id
            .is_none()
    );
    if let pinstar::data::CanvasNode::Text(node) =
        &mut app.canvas.as_mut().unwrap().state.data.nodes[0]
    {
        node.width = 5000.0;
        node.height = 5000.0;
    }
    key(&mut app, KeyCode::Char('s'));
    assert_eq!(paper_columns(&before), paper_columns(&screen(&mut app)));
}

#[test]
fn inline_editor_sanitizes_and_rejects_overflow_without_corrupting_undo() {
    let mut app = Shape::canvas();
    screen(&mut app);
    key(&mut app, KeyCode::Enter);
    control(&mut app, 'u');
    app.paste("a\r\nb\x1b\t", area());
    assert_eq!(app.note.text, "a\nb");
    control(&mut app, 'u');
    app.paste(&"z".repeat(4096), area());
    key(&mut app, KeyCode::Char('!'));
    assert_eq!(app.note.text.len(), 4096);
    assert_eq!(
        app.notice,
        Some("Card limit: 4096 bytes. Nothing inserted.")
    );
    app.paste("!", area());
    assert_eq!(app.note.text.len(), 4096);
    control(&mut app, 'z');
    assert_eq!(app.note.text, "");
    key(&mut app, KeyCode::F(2));
    assert!(app.editing);
    assert!(app.sent_note().is_none());
    assert!(screen(&mut app).contains("Write a thought on the card first."));
}

#[test]
fn help_resize_and_mouse_release_outside_canvas_cancel_captured_gestures() {
    let mut app = Shape::canvas();
    screen(&mut app);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 10, 12);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 40, 12);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 40, 12);
    click(&mut app, 40, 12);
    assert_eq!(app.focus, Focus::Paper);
    key(&mut app, KeyCode::Esc);
    screen(&mut app);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), 10, 12);
    key(&mut app, KeyCode::Char('?'));
    assert!(app.help);
    let position = app.canvas.as_ref().unwrap().state.viewport_x;
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Middle), 15, 12);
    assert_eq!(app.canvas.as_ref().unwrap().state.viewport_x, position);
    key(&mut app, KeyCode::Esc);
    screen(&mut app);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), 10, 12);
    snapshot(60, 20, &mut app, false).unwrap();
    screen(&mut app);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Middle), 15, 12);
    assert_eq!(app.canvas.as_ref().unwrap().state.viewport_x, position);
    click(&mut app, 40, 12);
    assert_eq!(app.focus, Focus::Paper);
}

#[test]
fn upstream_inline_editor_retains_carbon_text_on_warm_paper() {
    let mut app = Shape::canvas();
    screen(&mut app);
    key(&mut app, KeyCode::Enter);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| render(frame, &mut app, Palette::new(false)))
        .unwrap();
    let cell = &terminal.backend().buffer()[(6, 10)];
    assert_eq!(cell.symbol(), "k");
    assert_eq!(cell.fg, ratatui::style::Color::Rgb(16, 15, 15));
    assert_eq!(cell.bg, ratatui::style::Color::Rgb(255, 252, 240));
    terminal
        .draw(|frame| render(frame, &mut app, Palette::new(true)))
        .unwrap();
    assert!(
        terminal.backend().buffer()[(4, 10)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
}

#[test]
fn memory_only_save_is_safe_even_with_a_writable_destination() {
    let mut app = Shape::canvas();
    screen(&mut app);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unsaved.canvas");
    assert!(!path.exists());
    app.canvas.as_mut().unwrap().state.path = path.clone();
    key(&mut app, KeyCode::Enter);
    app.paste("Changed ", area());
    key(&mut app, KeyCode::Esc);
    control(&mut app, 'z');
    control(&mut app, 'y');
    control(&mut app, 's');
    assert_eq!(app.notice, Some("Unsaved prototype: saving is disabled."));
    app.canvas.as_mut().unwrap().state.save().unwrap();
    assert!(!path.exists());
    assert!(app.canvas.as_ref().unwrap().state.data.edges.is_empty());
}
