//! The original pane: the person's words exactly as sent, one original at a time, with caret,
//! selection and extraction into cards. Read-only: nothing here edits a source.

use super::*;
use ratatui::style::Modifier;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn title(text: &str) -> String {
    short_title(text, 32)
}
pub(super) fn short_title(text: &str, width: u16) -> String {
    let line = text
        .lines()
        .find(|s| !s.trim().is_empty())
        .unwrap_or(text)
        .trim();
    if UnicodeWidthStr::width(line) <= usize::from(width) {
        return line.into();
    }
    let mut end = 0;
    let mut cells = 0;
    for (i, g) in line.grapheme_indices(true) {
        let w = UnicodeWidthStr::width(g);
        if cells + w > usize::from(width.saturating_sub(1)) {
            break;
        }
        cells += w;
        end = i + g.len();
    }
    let prefix = &line[..end];
    format!(
        "{}…",
        prefix
            .rfind(char::is_whitespace)
            .filter(|i| *i > 0)
            .map_or(prefix, |i| &prefix[..i])
            .trim_end()
    )
}

impl BrainDump {
    /// Pulls `start..end` of source `source` out into a card; the original is untouched.
    /// Refused when the range is blank or cuts a character, or after 32 cards.
    pub fn extract_range(&mut self, source: usize, start: usize, end: usize) -> Result<(), String> {
        self.conversation.extract(source, start, end)?;
        self.place_card();
        self.notice =
            "Extracted by you; original intact. F7 shows cards; nothing confirmed.".into();
        Ok(())
    }
    pub(super) fn handle_source_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if !self.conversation.sources().is_empty()
            && ctrl
            && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
        {
            self.screen.source_view = if key.code == KeyCode::PageDown {
                (self.screen.source_view + 1) % self.conversation.sources().len()
            } else {
                (self.screen.source_view + self.conversation.sources().len() - 1)
                    % self.conversation.sources().len()
            };
            self.screen.source_scroll = 0;
            self.screen.source_cursor = 0;
            self.screen.selection = None;
            self.screen.drag_anchor = None;
            self.screen.show_extractions = false;
            return true;
        }
        if ctrl && key.code == KeyCode::Char('e') {
            let result = if let (Some(source), Some((a, b))) = (
                self.conversation.sources().get(self.screen.source_view),
                self.screen.selection,
            ) {
                self.extract_range(source.id, a.min(b), a.max(b))
            } else {
                Err("Select text on the intact dump, then Ctrl-E extracts it.".into())
            };
            if let Err(error) = result {
                self.refuse(error);
            }
            return true;
        }
        if ctrl && key.code == KeyCode::Char('x') {
            self.toggle_cards();
            return true;
        }
        if !self.screen.board_focus || self.conversation.sources().is_empty() {
            return false;
        }
        if self.screen.show_extractions {
            if key.code == KeyCode::Enter {
                if let Some(fragment) = self
                    .screen
                    .canvas
                    .selected_id()
                    .and_then(|id| self.conversation.fragments().iter().find(|f| f.id == id))
                    .cloned()
                {
                    self.screen.source_view = self
                        .conversation
                        .sources()
                        .iter()
                        .position(|s| s.id == fragment.source)
                        .unwrap();
                    self.screen.source_cursor = fragment.start;
                    self.screen.selection = Some((fragment.start, fragment.end));
                    self.screen.source_scroll =
                        Note::new(&self.conversation.sources()[self.screen.source_view].text)
                            .wrap(self.screen.source_area.width)
                            .positions
                            .iter()
                            .find(|(i, _, _)| *i == fragment.start)
                            .map_or(0, |(_, row, _)| *row as u16);
                }
                self.screen.show_extractions = false;
                return true;
            }
            return false;
        }
        if key.code == KeyCode::Esc {
            self.screen.selection = None;
            self.screen.drag_anchor = None;
            return true;
        }
        if matches!(
            key.code,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
        ) {
            let mut note = Note::new(&self.conversation.sources()[self.screen.source_view].text);
            note.cursor = self.screen.source_cursor;
            let anchor = self
                .screen
                .selection
                .map_or(self.screen.source_cursor, |(a, _)| a);
            match key.code {
                KeyCode::Left => note.left(),
                KeyCode::Right => note.right(),
                KeyCode::Up => note.move_row(false, self.screen.source_area.width),
                KeyCode::Down => note.move_row(true, self.screen.source_area.width),
                KeyCode::Home => note.home(),
                KeyCode::End => note.end(),
                _ => {}
            }
            self.screen.source_cursor = note.cursor;
            self.screen.selection = key
                .modifiers
                .contains(KeyModifiers::SHIFT)
                .then_some((anchor, note.cursor));
            let row = note.wrap(self.screen.source_area.width).cursor.0;
            if row < self.screen.source_scroll {
                self.screen.source_scroll = row;
            } else if row >= self.screen.source_scroll + self.screen.source_area.height {
                self.screen.source_scroll =
                    row.saturating_sub(self.screen.source_area.height.saturating_sub(1));
            }
            return true;
        }
        false
    }
    pub(super) fn handle_source_mouse(&mut self, event: MouseEvent) -> bool {
        if self.conversation.sources().is_empty() || self.screen.show_extractions {
            return false;
        }
        let area = self.screen.source_area;
        let inside = area.contains((event.column, event.row).into());
        if !inside && self.screen.drag_anchor.is_none() {
            return false;
        }
        match event.kind {
            MouseEventKind::ScrollDown if inside => {
                self.screen.source_scroll = self.screen.source_scroll.saturating_add(3)
            }
            MouseEventKind::ScrollUp if inside => {
                self.screen.source_scroll = self.screen.source_scroll.saturating_sub(3)
            }
            MouseEventKind::Down(MouseButton::Left) if inside => {
                let index = Note::new(&self.conversation.sources()[self.screen.source_view].text)
                    .wrap(area.width)
                    .index_at(
                        event.row - area.y + self.screen.source_scroll,
                        event.column - area.x,
                    );
                self.screen.board_focus = true;
                self.input.collapse();
                self.screen.input_drag = false;
                self.screen.source_cursor = index;
                self.screen.selection = None;
                self.screen.drag_anchor = Some(index);
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
                if self.screen.drag_anchor.is_some() =>
            {
                let row = event.row.clamp(area.y, area.bottom().saturating_sub(1)) - area.y
                    + self.screen.source_scroll;
                let col = event.column.clamp(area.x, area.right().saturating_sub(1)) - area.x;
                let end = Note::new(&self.conversation.sources()[self.screen.source_view].text)
                    .wrap(area.width)
                    .index_at(row, col);
                self.screen.selection = Some((self.screen.drag_anchor.unwrap(), end));
                self.screen.source_cursor = end;
                if matches!(event.kind, MouseEventKind::Up(_)) {
                    self.screen.drag_anchor = None;
                }
            }
            _ => return false,
        }
        true
    }
}

pub(super) fn render_source(frame: &mut Frame, app: &mut BrainDump, area: Rect, palette: Palette) {
    let Some(source) = app.conversation.sources().get(app.screen.source_view) else {
        return;
    };
    let wrapped = Note::new(&source.text).wrap(area.width);
    let clipped = wrapped.lines.len() > usize::from(area.height);
    let inner = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(u16::from(clipped)),
    );
    if clipped {
        frame.render_widget(
            Paragraph::new(
                if app.screen.source_scroll
                    >= wrapped
                        .lines
                        .len()
                        .saturating_sub(usize::from(inner.height)) as u16
                {
                    "↑ more"
                } else if app.screen.source_scroll > 0 {
                    "↕ more"
                } else {
                    "↓ more"
                },
            )
            .style(palette.muted),
            Rect::new(area.x, area.bottom() - 1, area.width, 1),
        );
    }
    app.screen.source_area = inner;
    let max = wrapped
        .lines
        .len()
        .saturating_sub(usize::from(inner.height)) as u16;
    app.screen.source_scroll = app.screen.source_scroll.min(max);
    frame.render_widget(
        Paragraph::new(wrapped.lines.join("\n"))
            .style(palette.ink)
            .scroll((app.screen.source_scroll, 0)),
        inner,
    );
    // P10: the original stays plain ink; only your own selection is drawn. The exact-quote
    // check still runs; evidence is shown on demand later (C13), never painted by default.
    if app.screen.board_focus
        && !app.screen.original
        && !app.screen.help
        && let Some((_, row, col)) = wrapped
            .positions
            .iter()
            .find(|(i, _, _)| *i == app.screen.source_cursor)
        && let Some(row) = row.checked_sub(usize::from(app.screen.source_scroll))
        && row < usize::from(inner.height)
        && *col < usize::from(inner.width)
    {
        frame.set_cursor_position((inner.x + *col as u16, inner.y + row as u16));
    }
    for (index, grapheme) in source.text.grapheme_indices(true) {
        let Some((_, row, col)) = wrapped.positions.iter().find(|(i, _, _)| *i == index) else {
            continue;
        };
        let Some(row) = row.checked_sub(usize::from(app.screen.source_scroll)) else {
            continue;
        };
        if row >= usize::from(inner.height) || *col >= usize::from(inner.width) || grapheme == "\n"
        {
            continue;
        }
        let cell = &mut frame.buffer_mut()[(inner.x + *col as u16, inner.y + row as u16)];
        if cell.symbol() != grapheme {
            continue;
        }
        let mut style = palette.ink;
        if app
            .screen
            .selection
            .is_some_and(|(a, b)| (a.min(b)..a.max(b)).contains(&index))
        {
            style = style.add_modifier(Modifier::REVERSED);
        }
        cell.set_style(style);
    }
}
