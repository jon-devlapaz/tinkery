use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use pinstar::{
    ActionCtx, PinstarAction, PinstarState, ThemeColors, apply_action,
    data::{CanvasData, CanvasNode, TextNode},
    draw_pinstar_view, handle_pinstar_mouse,
};
use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Style},
};

use super::Note;
use crate::Palette;

const NOTE_ID: &str = "draft-note";

pub(super) struct Canvas {
    pub state: PinstarState,
    pub area: Rect,
    captured: bool,
    captured_button: MouseButton,
    pub spatial: bool,
}

impl Canvas {
    pub fn new(text: &str) -> Self {
        let mut state = PinstarState::in_memory(CanvasData {
            nodes: vec![CanvasNode::Text(TextNode {
                id: NOTE_ID.into(),
                x: 0.0,
                y: 0.0,
                width: 270.0,
                height: 120.0,
                text: text.into(),
                title: Some("Your note".into()),
                color: None,
                shape: Default::default(),
            })],
            edges: vec![],
            orientation: Default::default(),
        });
        state.settings.show_hints = false;
        state.show_grid = false;
        state.selection.select_only(NOTE_ID.into());
        state.center_on_selected();
        Self {
            state,
            area: Rect::default(),
            captured: false,
            captured_button: MouseButton::Left,
            spatial: false,
        }
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.state.selection.primary.as_deref()
    }

    pub fn selected_text(&self) -> Option<String> {
        let id = self.selected_id()?;
        let node = self.state.data.nodes.iter().find(|node| node.id() == id)?;
        Some(self.state.floating_editor.as_ref().map_or_else(
            || node.text().to_owned(),
            |editor| editor.lines().join("\n"),
        ))
    }

    pub fn text(&self) -> String {
        self.selected_text().unwrap_or_default()
    }

    pub fn start_edit(&mut self) {
        self.cancel_gesture();
        self.state.resizing_node_id = None;
        if !self.spatial
            && self.state.selection.primary.is_none()
            && let Some(node) = self.state.data.nodes.first()
        {
            self.state.selection.select_only(node.id().to_owned());
        }
        if self.state.floating_editor.is_none() {
            self.state.toggle_editor();
        }
    }

    pub fn captured(&self) -> bool {
        self.captured
    }

    pub fn finish_edit(&mut self) {
        self.cancel_gesture();
        if self.state.floating_editor.is_some() {
            self.state.toggle_editor();
        }
        self.state.resizing_node_id = None;
        self.captured = false;
    }

    pub fn paste(&mut self, text: &str) -> Result<(), &'static str> {
        let mut clean = Note::new("");
        clean.insert(text)?;
        let Some(editor) = &mut self.state.floating_editor else {
            return Ok(());
        };
        let before = editor.clone();
        editor.insert_str(clean.text);
        if editor.lines().join("\n").len() > 4096 {
            *editor = before;
            return Err("Card limit: 4096 bytes. Nothing inserted.");
        }
        Ok(())
    }

    pub fn edit_key(&mut self, key: KeyEvent) -> Result<(), &'static str> {
        let Some(editor) = &mut self.state.floating_editor else {
            return Ok(());
        };
        let before = editor.clone();
        if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
            editor.select_all();
            editor.cut();
        } else if key.code == KeyCode::Char('z') && key.modifiers.contains(KeyModifiers::CONTROL) {
            editor.undo();
        } else if key.code == KeyCode::Char('y') && key.modifiers.contains(KeyModifiers::CONTROL) {
            editor.redo();
        } else {
            editor.input(key);
        }
        if editor.lines().join("\n").len() > 4096 {
            *editor = before;
            return Err("Card limit: 4096 bytes. Nothing inserted.");
        }
        Ok(())
    }

    pub fn action_key(&mut self, key: KeyEvent) -> bool {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if self.spatial && self.state.context_menu.is_some() {
            let mut running = true;
            pinstar::handle_pinstar_event(
                &mut self.state,
                key,
                &mut running,
                Rect::new(0, 0, self.area.width, self.area.height),
            );
            return true;
        }
        if control && key.code == KeyCode::Char('f') {
            self.state
                .fit_to_view(Rect::new(0, 0, self.area.width, self.area.height));
            return true;
        }
        if self.state.resizing_node_id.is_some()
            && matches!(key.code, KeyCode::Esc | KeyCode::Enter)
        {
            let mut running = true;
            pinstar::handle_pinstar_event(
                &mut self.state,
                key,
                &mut running,
                Rect::new(0, 0, self.area.width, self.area.height),
            );
            return true;
        }
        let action = match key.code {
            KeyCode::Char('j') if control => PinstarAction::ZoomIn,
            KeyCode::Char('k') if control => PinstarAction::ZoomOut,
            KeyCode::Char('g') if control => PinstarAction::ToggleGrid,
            KeyCode::Char('z') if control => PinstarAction::Undo,
            KeyCode::Char('y') if control => PinstarAction::Redo,
            KeyCode::Char('+' | '=') => PinstarAction::ZoomIn,
            KeyCode::Char('-' | '_') => PinstarAction::ZoomOut,
            KeyCode::Char('s') if !control => PinstarAction::ResizeMode,
            KeyCode::Delete if self.spatial => PinstarAction::DeleteNode,
            KeyCode::Left | KeyCode::Char('h') if self.spatial => PinstarAction::MoveLeft,
            KeyCode::Right | KeyCode::Char('l') if self.spatial => PinstarAction::MoveRight,
            KeyCode::Up | KeyCode::Char('k') if self.spatial => PinstarAction::MoveUp,
            KeyCode::Down | KeyCode::Char('j') if self.spatial => PinstarAction::MoveDown,
            _ => return false,
        };
        apply_action(
            &mut self.state,
            action,
            &ActionCtx {
                area: Rect::new(0, 0, self.area.width, self.area.height),
                count: 1,
            },
        );
        true
    }

    pub fn mouse(&mut self, mut event: MouseEvent) -> bool {
        let inside = self.area.contains((event.column, event.row).into());
        if !inside && !self.captured {
            return false;
        }
        // This shaping surface is one draft note, not a graph editor.
        if matches!(
            event.kind,
            MouseEventKind::Down(MouseButton::Right)
                | MouseEventKind::Up(MouseButton::Right)
                | MouseEventKind::Drag(MouseButton::Right)
        ) && !self.spatial
            && self.state.resizing_node_id.is_none()
        {
            return true;
        }
        if self.spatial
            && self.state.floating_editor.is_some()
            && event.kind == MouseEventKind::Down(MouseButton::Right)
        {
            self.finish_edit();
        }
        if let MouseEventKind::Down(button) = event.kind {
            self.captured = true;
            self.captured_button = button;
        }
        event.column = event
            .column
            .saturating_sub(self.area.x)
            .min(self.area.width.saturating_sub(1));
        event.row = event
            .row
            .saturating_sub(self.area.y)
            .min(self.area.height.saturating_sub(1));
        self.state.mouse_pos = Some((event.column, event.row));
        handle_pinstar_mouse(
            &mut self.state,
            event,
            Rect::new(0, 0, self.area.width, self.area.height),
        );
        if self.spatial
            && let Some(menu) = &mut self.state.context_menu
        {
            menu.items.retain(|item| {
                matches!(item.label, "Add Text Node" | "Resize Node" | "Delete Node")
            });
            menu.selected = menu.selected.min(menu.items.len().saturating_sub(1));
        }
        if matches!(event.kind, MouseEventKind::Up(_)) {
            self.captured = false;
        }
        true
    }

    pub fn cancel_gesture(&mut self) {
        if self.captured {
            self.mouse(MouseEvent {
                kind: MouseEventKind::Up(self.captured_button),
                column: self.area.x,
                row: self.area.y,
                modifiers: KeyModifiers::NONE,
            });
        }
        self.state.last_click = None;
        self.state.context_menu = None;
        self.state.right_down_screen = None;
        self.state.marquee.clear();
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        if self.area != area {
            self.cancel_gesture();
        }
        self.area = area;
        let ink = palette.ink.fg.unwrap_or(Color::Reset);
        let bg = palette.ink.bg.unwrap_or(Color::Reset);
        let jade = palette.jade.fg.unwrap_or(Color::Reset);
        let muted = palette.muted.fg.unwrap_or(Color::Reset);
        let theme = ThemeColors {
            accent: jade,
            heading: ink,
            success: jade,
            warning: muted,
            destructive: ink,
            muted,
            text: ink,
            fg: ink,
            bg,
            border: muted,
            tag: jade,
            folder: muted,
            highlight_fg: bg,
            highlight_bg: jade,
            selection_indicator: Some(jade),
        };
        if let Some(editor) = &mut self.state.floating_editor {
            editor.set_cursor_style(if jade == Color::Reset {
                Style::default().add_modifier(ratatui::style::Modifier::REVERSED)
            } else {
                Style::default().fg(bg).bg(jade)
            });
            editor.set_cursor_line_style(Style::default());
        }
        // Render into a bounded surface: upstream handles must never paint over the paper.
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|surface| {
                let mouse = self.state.mouse_pos;
                draw_pinstar_view(surface, &mut self.state, &theme, surface.area(), mouse);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        for y in 0..area.height {
            for x in 0..area.width {
                frame.buffer_mut()[(area.x + x, area.y + y)] = buffer[(x, y)].clone();
            }
        }
    }
}
