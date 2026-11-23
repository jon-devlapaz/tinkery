//! The extracted cards, on a pinstar canvas: the cards' positions and the mouse gestures that
//! move and select them. Cards are the person's own excerpts; nothing here reads or changes
//! the reading.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use pinstar::{
    PinstarState,
    data::{CanvasData, CanvasNode, TextNode},
    handle_pinstar_mouse,
};
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

    /// Places card `index` (counting from 0) clear of the others. The first card also
    /// centres the view.
    pub(super) fn place(&mut self, index: usize, id: String, title: String, text: String) {
        if index == 0 {
            self.state.zoom = 0.1;
            self.state.viewport_x = (f64::from(self.area.width) / 2.0 - 1.0) / 0.1;
            self.state.viewport_y = (f64::from(self.area.height) / 2.0 - 1.0) / 0.1;
        }
        let x = (index % 2) as f64 * 200.0;
        let mut y = (index / 2) as f64 * 80.0;
        while self.state.data.nodes.iter().any(|n| {
            let (nx, ny) = n.pos();
            let (w, h) = n.size();
            x < nx + w && x + 190.0 > nx && y < ny + h && y + 70.0 > ny
        }) {
            y += 80.0;
        }
        self.state.data.nodes.push(CanvasNode::Text(TextNode {
            id,
            title: Some(title),
            text,
            x,
            y,
            width: 190.0,
            height: 70.0,
            color: None,
            shape: Default::default(),
        }));
    }
    /// Each card's id and canvas position.
    pub(super) fn positions(&self) -> Vec<(String, f64, f64)> {
        self.state
            .data
            .nodes
            .iter()
            .map(|n| {
                let (x, y) = n.pos();
                (n.id().to_owned(), x, y)
            })
            .collect()
    }
    /// Each card with its left, top, right and bottom edges in terminal cells, for a canvas
    /// drawn in `area`. Edges can lie outside `area`.
    pub(super) fn cards(&self, area: Rect) -> impl Iterator<Item = (&CanvasNode, [f64; 4])> {
        let z = self.state.zoom;
        self.state.data.nodes.iter().map(move |node| {
            let (x, y) = node.pos();
            let (w, h) = node.size();
            let x =
                (x - self.state.viewport_x) * z + f64::from(area.x) + f64::from(area.width) / 2.0;
            let y =
                (y - self.state.viewport_y) * z + f64::from(area.y) + f64::from(area.height) / 2.0;
            (node, [x, y, x + w * z, y + h * z])
        })
    }
    /// The card is selected.
    pub(super) fn is_selected(&self, id: &str) -> bool {
        self.state.selection.is_selected(&id.to_owned())
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
