//! The extracted cards, on a pinstar canvas: the cards' positions and the mouse gestures that
//! move and select them. Cards are the person's own excerpts; nothing here reads or changes
//! the reading.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use pinstar::{PinstarState, data::CanvasData, handle_pinstar_mouse};
use ratatui::layout::Rect;

pub(super) struct Canvas {
    pub state: PinstarState,
    pub area: Rect,
    captured: bool,
    captured_button: MouseButton,
}

impl Canvas {
    pub(super) fn new() -> Self {
        let mut state = PinstarState::in_memory(CanvasData {
            nodes: vec![],
            edges: vec![],
            orientation: Default::default(),
        });
        state.settings.show_hints = false;
        state.show_grid = false;
        Self {
            state,
            area: Rect::default(),
            captured: false,
            captured_button: MouseButton::Left,
        }
    }

    pub(super) fn selected_id(&self) -> Option<&str> {
        self.state.selection.primary.as_deref()
    }

    pub(super) fn finish_edit(&mut self) {
        self.cancel_gesture();
        if self.state.floating_editor.is_some() {
            self.state.toggle_editor();
        }
        self.state.resizing_node_id = None;
        self.captured = false;
    }

    pub(super) fn mouse(&mut self, mut event: MouseEvent) {
        let inside = self.area.contains((event.column, event.row).into());
        if !inside && !self.captured {
            return;
        }
        if self.state.floating_editor.is_some()
            && event.kind == MouseEventKind::Down(MouseButton::Right)
        {
            self.finish_edit();
        }
        if inside
            && event.kind == MouseEventKind::Down(MouseButton::Left)
            && event.modifiers.contains(KeyModifiers::SHIFT)
            && self.state.context_menu.is_none()
            && self.state.resizing_node_id.is_none()
        {
            self.finish_edit();
            let (x, y) = self.state.screen_to_canvas(
                event.column - self.area.x,
                event.row - self.area.y,
                Rect::new(0, 0, self.area.width, self.area.height),
            );
            if let Some(id) = self.state.node_at(x, y) {
                let mut selected = self.state.selection.all();
                let primary = if selected.remove(&id) {
                    self.state
                        .selection
                        .primary
                        .clone()
                        .filter(|primary| primary != &id)
                        .or_else(|| {
                            self.state
                                .data
                                .nodes
                                .iter()
                                .find(|node| selected.contains(node.id()))
                                .map(|node| node.id().to_owned())
                        })
                } else {
                    selected.insert(id.clone());
                    self.state.selection.primary.clone().or(Some(id))
                };
                if let Some(id) = &primary {
                    selected.remove(id);
                }
                self.state.selection.replace_set(selected, primary);
            }
            return;
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
        if let Some(menu) = &mut self.state.context_menu {
            menu.items.retain(|item| {
                matches!(item.label, "Add Text Node" | "Resize Node" | "Delete Node")
            });
            menu.selected = menu.selected.min(menu.items.len().saturating_sub(1));
        }
        if matches!(event.kind, MouseEventKind::Up(_)) {
            self.captured = false;
        }
    }

    pub(super) fn cancel_gesture(&mut self) {
        if self.captured {
            let primary = self.state.selection.primary.clone();
            let selected = self.state.selection.extra.clone();
            self.mouse(MouseEvent {
                kind: MouseEventKind::Up(self.captured_button),
                column: self.area.x,
                row: self.area.y,
                modifiers: KeyModifiers::NONE,
            });
            self.state.selection.replace_set(selected, primary);
        }
        self.state.last_click = None;
        self.state.context_menu = None;
        self.state.right_down_screen = None;
        self.state.marquee.clear();
    }
}
