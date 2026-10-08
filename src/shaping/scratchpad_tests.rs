use super::*;
use crate::shaping::EXAMPLE;

fn example_app() -> Scratchpad {
    let mut app = Scratchpad::default();
    if let CanvasNode::Text(note) = &mut app.canvas.state.data.nodes[0] {
        note.text = EXAMPLE.into();
    }
    app
}

fn area() -> Rect {
    Rect::new(0, 0, 100, 30)
}
fn screen(app: &mut Scratchpad) -> String {
    snapshot(100, 30, app, false).unwrap()
}
fn key(app: &mut Scratchpad, code: KeyCode) -> String {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), area());
    screen(app)
}
fn ctrl(app: &mut Scratchpad, c: char) -> String {
    app.handle_key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        area(),
    );
    screen(app)
}
fn mouse(app: &mut Scratchpad, kind: MouseEventKind, point: (u16, u16)) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column: point.0,
            row: point.1,
            modifiers: KeyModifiers::NONE,
        },
        area(),
    );
    screen(app);
}
fn click(app: &mut Scratchpad, point: (u16, u16)) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), point);
    mouse(app, MouseEventKind::Up(MouseButton::Left), point);
}
fn point(app: &Scratchpad, index: usize) -> (u16, u16) {
    let node = &app.notes()[index];
    let (x, y) = node.pos();
    let canvas = &app.canvas;
    (
        (canvas.area.x as f64
            + canvas.area.width as f64 / 2.0
            + (x - canvas.state.viewport_x) * canvas.state.zoom
            + 3.0) as u16,
        (canvas.area.y as f64
            + canvas.area.height as f64 / 2.0
            + (y - canvas.state.viewport_y) * canvas.state.zoom
            + 3.0) as u16,
    )
}
type NoteGeometry = (String, (f64, f64), (f64, f64));

fn geometry(app: &Scratchpad) -> Vec<NoteGeometry> {
    app.notes()
        .iter()
        .map(|note| (note.id().to_owned(), note.pos(), note.size()))
        .collect()
}
fn finish_writer(app: &mut Scratchpad) {
    for _ in 0..7 {
        app.tick(Duration::from_secs(1), area());
    }
}

#[test]
fn full_width_workspace_and_inspector_are_readable_at_supported_sizes() {
    for (width, height) in [(80, 24), (100, 30), (180, 60)] {
        let mut app = Scratchpad::default();
        let initial = snapshot(width, height, &mut app, false).unwrap();
        assert!(initial.contains("Scratchpad / Pinstar / 1 note"));
        assert!(initial.contains("What brought you here? A problem, hunch, or plan."));
        assert!(app.notes()[0].text().is_empty());
        assert!(!initial.contains("Working paper"));
        assert_eq!(app.canvas.area.width, width - 4);
        let size = Rect::new(0, 0, width, height);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), size);
        app.paste(EXAMPLE, size);
        app.handle_key(
            KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE),
            Rect::new(0, 0, width, height),
        );
        finish_writer(&mut app);
        let paper = snapshot(width, height, &mut app, true).unwrap();
        assert!(paper.contains("Working paper / sample"));
        assert!(paper.contains("From Note 1 / Draft / unconfirmed"));
        if width >= 100 {
            assert!(paper.contains("What you want to change"));
        }
        app.handle_key(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            Rect::new(0, 0, width, height),
        );
        let help = snapshot(width, height, &mut app, false).unwrap();
        assert!(help.contains("? / Esc close help"));
    }
}

#[test]
fn inspector_toggles_preserve_world_geometry_camera_and_visible_note_positions() {
    let mut app = example_app();
    let original = screen(&mut app);
    let geometry = geometry(&app);
    let camera = (
        app.canvas.state.viewport_x,
        app.canvas.state.viewport_y,
        app.canvas.state.zoom,
    );
    let original_point = point(&app, 0);
    for _ in 0..3 {
        key(&mut app, KeyCode::Char('p'));
        assert_eq!(super::tests::geometry(&app), geometry);
        assert_eq!(point(&app, 0), original_point);
        key(&mut app, KeyCode::Char('p'));
        assert_eq!(
            (
                app.canvas.state.viewport_x,
                app.canvas.state.viewport_y,
                app.canvas.state.zoom
            ),
            camera
        );
        assert_eq!(screen(&mut app), original);
    }
    key(&mut app, KeyCode::F(2));
    assert_eq!(super::tests::geometry(&app), geometry);
    assert_eq!(point(&app, 0), original_point);
}

#[test]
fn multiple_notes_edit_independently_and_f2_shapes_exactly_the_selected_live_note() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    assert!(app.editing());
    assert_eq!(app.notes().len(), 2);
    app.paste("A different idea.", area());
    let second = app.selected_note().unwrap().to_owned();
    key(&mut app, KeyCode::F(2));
    assert_eq!(app.sources[0].id, second);
    assert_eq!(app.sent_note(), Some("A different idea."));
    assert_eq!(app.notes()[0].text(), EXAMPLE);
    assert!(app.inspector_open());
    assert_eq!(app.focus, Focus::Card);
    finish_writer(&mut app);
    assert!(screen(&mut app).contains("From Note 2"));
    let first_point = point(&app, 0);
    let paper = app.paper();
    click(&mut app, first_point);
    assert_eq!(app.paper(), paper);
    assert_eq!(app.sources[0].id, second);
    key(&mut app, KeyCode::F(2));
    assert_eq!(app.sent_note(), Some(EXAMPLE));
    assert_eq!(app.sources[0].title, "Note 1");
    assert_eq!(app.notes()[1].text(), "A different idea.");
    assert!(app.canvas.state.data.edges.is_empty());
}

#[test]
fn missing_or_empty_selections_never_fall_back_to_a_different_note() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::F(2));
    let sent = app.sent_note().unwrap().to_owned();
    click(&mut app, (2, 4));
    key(&mut app, KeyCode::F(2));
    assert_eq!(app.notice, Some("Select one or more sticky notes first."));
    assert_eq!(app.sent_note(), Some(sent.as_str()));
    key(&mut app, KeyCode::Char('p'));
    key(&mut app, KeyCode::Char('n'));
    key(&mut app, KeyCode::F(2));
    assert_eq!(
        app.notice,
        Some("Write a thought on every selected note first.")
    );
    assert!(app.editing());
    key(&mut app, KeyCode::Esc);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), (3, 5));
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Right), (96, 24));
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), (96, 24));
    assert_eq!(app.canvas.state.selection.all().len(), 2);
    key(&mut app, KeyCode::F(2));
    assert_eq!(
        app.notice,
        Some("Write a thought on every selected note first.")
    );
    assert_eq!(app.sent_note(), Some(sent.as_str()));
}

#[test]
fn actual_pinstar_drag_crosses_the_old_pane_boundary_and_undo_redo_restore_it() {
    let mut app = example_app();
    screen(&mut app);
    let original = geometry(&app);
    let start = point(&app, 0);
    let end = (75, start.1 + 2);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), start);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), end);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), end);
    let moved = geometry(&app);
    assert!(point(&app, 0).0 > 60);
    assert_ne!(moved, original);
    ctrl(&mut app, 'z');
    assert_eq!(geometry(&app), original);
    ctrl(&mut app, 'y');
    assert_eq!(geometry(&app), moved);
    key(&mut app, KeyCode::Char('p'));
    assert_eq!(geometry(&app), moved);
    assert!(screen(&mut app).contains("Working paper"));
    key(&mut app, KeyCode::Char('p'));
    assert!(screen(&mut app).contains("I keep losing track"));
}

#[test]
fn creation_editing_and_deletion_use_upstream_undo_without_cross_note_corruption() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    app.paste("Second", area());
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'z');
    assert_eq!(app.notes()[1].text(), "");
    ctrl(&mut app, 'z');
    assert_eq!(app.notes().len(), 1);
    ctrl(&mut app, 'y');
    ctrl(&mut app, 'y');
    assert_eq!(app.notes()[1].text(), "Second");
    assert_eq!(app.notes()[0].text(), EXAMPLE);
    let second_point = point(&app, 1);
    click(&mut app, second_point);
    key(&mut app, KeyCode::Delete);
    assert_eq!(app.notes().len(), 1);
    ctrl(&mut app, 'z');
    assert_eq!(app.notes().len(), 2);
    assert_eq!(app.notes()[1].text(), "Second");
}

#[test]
fn right_click_menu_creates_notes_but_exposes_no_graph_files_or_external_editor() {
    let mut app = example_app();
    screen(&mut app);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), (3, 5));
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), (3, 5));
    let menu = app.canvas.state.context_menu.as_ref().unwrap();
    assert_eq!(menu.items.len(), 1);
    assert_eq!(menu.items[0].label, "Add Text Node");
    key(&mut app, KeyCode::Char('t'));
    assert_eq!(app.notes().len(), 2);
    assert!(app.editing());
    assert_eq!(app.notes()[1].title(), Some("Note 2"));
    key(&mut app, KeyCode::Esc);
    let at = point(&app, 0);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), at);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), at);
    let labels = app
        .canvas
        .state
        .context_menu
        .as_ref()
        .unwrap()
        .items
        .iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["Resize Node", "Delete Node"]);
    key(&mut app, KeyCode::Esc);
    assert!(app.canvas.state.data.edges.is_empty());
    assert!(!app.canvas.state.trigger_ext_editor);
    assert!(!app.canvas.state.trigger_image_picker);
}

#[test]
fn inspector_is_read_only_and_writer_growth_preserves_focus_scroll_and_hidden_canvas() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Enter);
    ctrl(&mut app, 'u');
    app.paste(&"A long thought.\n".repeat(35), area());
    key(&mut app, KeyCode::F(2));
    finish_writer(&mut app);
    screen(&mut app);
    let original = geometry(&app);
    let text = app.notes()[0].text().to_owned();
    click(&mut app, (70, 10));
    assert_eq!(app.focus, Focus::Paper);
    app.paste("must not edit", area());
    assert_eq!(app.notes()[0].text(), text);
    key(&mut app, KeyCode::PageDown);
    let scroll = app.writer.paper_scroll;
    assert!(scroll > 0);
    key(&mut app, KeyCode::Char('p'));
    key(&mut app, KeyCode::Char('p'));
    assert_eq!(app.writer.paper_scroll, scroll);
    assert_eq!(geometry(&app), original);
    click(&mut app, (70, 10));
    let end = key(&mut app, KeyCode::End);
    assert!(end.contains("What should we keep, cut, or reshape?"));
    key(&mut app, KeyCode::F(2));
    key(&mut app, KeyCode::Char('p'));
    let camera = (app.canvas.state.viewport_x, app.canvas.state.viewport_y);
    app.tick(Duration::from_secs(1), area());
    assert!(!app.paper_open);
    assert_eq!(app.focus, Focus::Card);
    assert_eq!(
        (app.canvas.state.viewport_x, app.canvas.state.viewport_y),
        camera
    );
}

#[test]
fn right_clicking_a_different_note_commits_the_current_editor_before_reselection() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    app.paste("Second", area());
    key(&mut app, KeyCode::Esc);
    let first = point(&app, 0);
    click(&mut app, first);
    key(&mut app, KeyCode::Enter);
    ctrl(&mut app, 'u');
    app.paste("First", area());
    let second = point(&app, 1);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), second);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), second);
    assert!(!app.editing());
    assert_eq!(app.notes()[0].text(), "First");
    assert_eq!(app.notes()[1].text(), "Second");
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::F(2));
    assert_eq!(app.sent_note(), Some("Second"));
}

#[test]
fn no_actions_write_files_even_with_a_real_destination_and_deleted_source_paper_survives() {
    let mut app = example_app();
    screen(&mut app);
    let directory = tempfile::tempdir().unwrap();
    app.canvas.state.path = directory.path().join("scratch.canvas");
    key(&mut app, KeyCode::F(2));
    let sent = app.sent_note().unwrap().to_owned();
    key(&mut app, KeyCode::Delete);
    assert_eq!(app.source_status(), "Source removed / draft retained");
    assert_eq!(app.sent_note(), Some(sent.as_str()));
    key(&mut app, KeyCode::Char('n'));
    app.paste("New", area());
    ctrl(&mut app, 's');
    assert_eq!(app.notice, Some("Unsaved prototype: saving is disabled."));
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'z');
    ctrl(&mut app, 'y');
    app.canvas.state.save().unwrap();
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

fn menu_item_point(app: &Scratchpad, label: &str, right_edge: bool) -> (u16, u16) {
    let menu = app.canvas.state.context_menu.as_ref().unwrap();
    let row = menu
        .items
        .iter()
        .position(|item| item.label == label)
        .unwrap() as u16;
    let rect = menu.rect(Rect::new(
        0,
        0,
        app.canvas.area.width,
        app.canvas.area.height,
    ));
    (
        app.canvas.area.x
            + if right_edge {
                rect.right() - 1
            } else {
                rect.x + 2
            },
        app.canvas.area.y + rect.y + row,
    )
}

#[test]
fn right_click_delete_removes_only_the_clicked_note_and_supports_undo_redo() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    app.paste("Second note", area());
    key(&mut app, KeyCode::Esc);
    let first = app.notes()[0].id().to_owned();
    let second = app.notes()[1].id().to_owned();
    app.canvas.state.selection.add(first.clone());
    let at = point(&app, 0);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), at);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), at);
    let delete = menu_item_point(&app, "Delete Node", false);
    click(&mut app, delete);
    assert_eq!(app.notes().len(), 1);
    assert_eq!(app.notes()[0].id(), second);
    assert!(app.canvas.state.context_menu.is_none());
    ctrl(&mut app, 'z');
    assert_eq!(app.notes().len(), 2);
    assert_eq!(app.notes()[0].id(), first);
    assert_eq!(app.notes()[0].text(), EXAMPLE);
    ctrl(&mut app, 'y');
    assert_eq!(app.notes().len(), 1);
    assert_eq!(app.notes()[0].id(), second);
}

#[test]
fn right_click_delete_commits_another_notes_live_editor_before_deleting() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    app.paste("Second note / live edits", area());
    assert!(app.editing());
    let first = point(&app, 0);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Right), first);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Right), first);
    assert!(!app.editing());
    let delete = menu_item_point(&app, "Delete Node", false);
    click(&mut app, delete);
    assert_eq!(app.notes().len(), 1);
    assert_eq!(app.notes()[0].text(), "Second note / live edits");
    ctrl(&mut app, 'z');
    assert_eq!(app.notes().len(), 2);
    assert_eq!(app.notes()[0].text(), EXAMPLE);
    assert_eq!(app.notes()[1].text(), "Second note / live edits");
}

#[test]
fn right_click_delete_over_the_inspector_is_visible_clickable_and_retains_the_paper() {
    for (width, height) in [(80, 24), (100, 30)] {
        let mut app = example_app();
        let size = Rect::new(0, 0, width, height);
        snapshot(width, height, &mut app, false).unwrap();
        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE), size);
        finish_writer(&mut app);
        snapshot(width, height, &mut app, false).unwrap();
        let paper = app.paper();
        let start = point(&app, 0);
        let end = (inspector(app.canvas.area).x - 5, start.1);
        for (kind, at) in [
            (MouseEventKind::Down(MouseButton::Left), start),
            (MouseEventKind::Drag(MouseButton::Left), end),
            (MouseEventKind::Up(MouseButton::Left), end),
            (MouseEventKind::Down(MouseButton::Right), end),
            (MouseEventKind::Up(MouseButton::Right), end),
        ] {
            app.handle_mouse(
                MouseEvent {
                    kind,
                    column: at.0,
                    row: at.1,
                    modifiers: KeyModifiers::NONE,
                },
                size,
            );
            snapshot(width, height, &mut app, false).unwrap();
        }
        let visible = snapshot(width, height, &mut app, false).unwrap();
        assert!(
            visible.contains("Delete note"),
            "Menu was covered by inspector:\n{visible}"
        );
        let at = menu_item_point(&app, "Delete Node", true);
        assert!(inspector(app.canvas.area).contains(at.into()));
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            app.handle_mouse(
                MouseEvent {
                    kind,
                    column: at.0,
                    row: at.1,
                    modifiers: KeyModifiers::NONE,
                },
                size,
            );
            snapshot(width, height, &mut app, false).unwrap();
        }
        assert!(app.notes().is_empty());
        assert_eq!(app.source_status(), "Source removed / draft retained");
        assert_eq!(app.paper(), paper);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            size,
        );
        assert_eq!(app.notes().len(), 1);
        assert_eq!(app.source_status(), "Draft / unconfirmed");
        assert_eq!(app.paper(), paper);
    }
}

#[test]
fn problem_first_and_plan_first_examples_share_a_goal_without_accepting_a_solution() {
    for (width, height) in [(80, 24), (100, 30)] {
        for (input, approach) in [
            (EXAMPLE, "No approach chosen."),
            (
                response::PLAN_EXAMPLE,
                "A graph connecting decisions to work is one candidate.",
            ),
        ] {
            let mut app = Scratchpad::default();
            let size = Rect::new(0, 0, width, height);
            snapshot(width, height, &mut app, false).unwrap();
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), size);
            app.paste(input, size);
            app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE), size);
            finish_writer(&mut app);
            let paper = app.paper();
            for text in [
                "# Investigation draft",
                "Simulated sample / provisional. Not researched or accepted.",
                "## What you want to change",
                "Make the reasons behind important decisions accessible near the work they affect.",
                "## Why it matters",
                "## What we should investigate",
                "## Possible approaches / not accepted",
                approach,
                "What should we keep, cut, or reshape?",
                input,
            ] {
                assert!(paper.contains(text), "Missing {text}:\n{paper}");
            }
            assert_eq!(app.notes()[0].text(), input);
            assert_eq!(app.sent_note(), Some(input));
            assert_eq!(app.focus, Focus::Card);
            assert!(!app.writer.drafting());
            assert!(app.canvas.state.data.edges.is_empty());
            assert!(!paper.contains("confirmed for intake"));
            let visible = snapshot(width, height, &mut app, false).unwrap();
            assert!(visible.contains("Working paper / sample"));
            app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), size);
            app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), size);
            assert!(
                snapshot(width, height, &mut app, false)
                    .unwrap()
                    .contains("What should we keep, cut, or reshape?")
            );
        }
    }
}

#[test]
fn selected_thoughts_include_live_edits_once_in_note_order_and_exclude_other_notes() {
    let mut app = Scratchpad::default();
    screen(&mut app);
    let (plan, reason) = response::PLAN_EXAMPLE.split_once("\n\n").unwrap();
    key(&mut app, KeyCode::Enter);
    app.paste(plan, area());
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('n'));
    app.paste(reason, area());
    let first = app.notes()[0].id().to_owned();
    let second = app.notes()[1].id().to_owned();
    assert_eq!(app.notes()[1].text(), "");
    app.canvas.state.selection.add(first.clone());
    app.canvas.state.selection.add(second.clone());
    let positions = geometry(&app);
    key(&mut app, KeyCode::F(2));
    finish_writer(&mut app);
    assert_eq!(app.sources.len(), 2);
    assert_eq!(app.sources[0].id, first);
    assert_eq!(app.sources[1].id, second);
    assert_eq!(app.sent_note(), Some(response::PLAN_EXAMPLE));
    assert_eq!(app.paper_label, "Working paper / sample");
    assert_eq!(app.paper().matches(plan).count(), 1);
    assert_eq!(app.paper().matches(reason).count(), 1);
    assert_eq!(geometry(&app), positions);
    assert_eq!(app.notes()[0].text(), plan);
    assert_eq!(app.notes()[1].text(), reason);
    assert!(screen(&mut app).contains("From 2 notes / Draft / unconfirmed"));
    let submitted = app.paper();
    key(&mut app, KeyCode::Char('n'));
    app.paste("Unrelated idea", area());
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.source_status(), "Draft / unconfirmed");
    app.canvas
        .state
        .selection
        .replace_set([first, second].into_iter().collect(), None);
    key(&mut app, KeyCode::F(2));
    finish_writer(&mut app);
    assert_eq!(app.paper(), submitted);
    assert!(!app.paper().contains("Unrelated idea"));
}

#[test]
fn multi_note_sources_track_changes_and_removal_without_rewriting_the_draft() {
    let mut app = example_app();
    screen(&mut app);
    let first = app.notes()[0].id().to_owned();
    key(&mut app, KeyCode::Char('n'));
    app.paste("A boundary", area());
    app.canvas.state.selection.add(first.clone());
    key(&mut app, KeyCode::F(2));
    finish_writer(&mut app);
    let submitted = app.paper();
    let sent = app.sent_note().unwrap().to_owned();
    app.canvas.state.selection.select_only(first);
    key(&mut app, KeyCode::Enter);
    app.paste(" A correction.", area());
    assert_eq!(app.source_status(), "Unsent changes");
    assert_eq!(app.paper(), submitted);
    assert_eq!(app.sent_note(), Some(sent.as_str()));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Delete);
    assert_eq!(app.source_status(), "Source removed / draft retained");
    assert_eq!(app.paper(), submitted);
    ctrl(&mut app, 'z');
    assert_eq!(app.source_status(), "Unsent changes");
}

#[test]
fn arbitrary_thoughts_get_an_honest_template_and_stale_selections_are_rejected() {
    let mut app = Scratchpad::default();
    screen(&mut app);
    key(&mut app, KeyCode::Enter);
    app.paste("I want better garden tools.", area());
    key(&mut app, KeyCode::F(2));
    finish_writer(&mut app);
    let paper = app.paper();
    assert!(paper.contains("Simulated template; no interpretation generated"));
    assert!(paper.contains("I want better garden tools."));
    assert!(!paper.contains("Make the reasons behind important decisions"));
    app.canvas.state.selection.add("missing-note".into());
    key(&mut app, KeyCode::F(2));
    assert_eq!(
        app.notice,
        Some("Selected notes are unavailable. Select them again.")
    );
    assert_eq!(app.paper(), paper);
}

#[test]
fn editing_limits_modal_help_stale_resize_and_outside_releases_remain_safe() {
    let mut app = example_app();
    screen(&mut app);
    key(&mut app, KeyCode::Char('n'));
    app.paste("a\r\nb\x1b\t", area());
    assert_eq!(app.canvas.text(), "a\nb");
    ctrl(&mut app, 'u');
    app.paste(&"z".repeat(4096), area());
    key(&mut app, KeyCode::Char('!'));
    assert_eq!(app.canvas.text().len(), 4096);
    assert_eq!(
        app.notice,
        Some("Card limit: 4096 bytes. Nothing inserted.")
    );
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('?'));
    let original = geometry(&app);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), (8, 12));
    app.paste("not editing", area());
    assert_eq!(geometry(&app), original);
    key(&mut app, KeyCode::Esc);
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 8,
            row: 12,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, 180, 60),
    );
    assert!(!app.canvas.captured());
    mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), (8, 12));
    snapshot(60, 20, &mut app, false).unwrap();
    screen(&mut app);
    let camera = app.canvas.state.viewport_x;
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Middle),
        (15, 12),
    );
    assert_eq!(app.canvas.state.viewport_x, camera);
}
