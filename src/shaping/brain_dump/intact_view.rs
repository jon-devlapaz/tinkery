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
    pub fn extract_range(&mut self, source: usize, start: usize, end: usize) -> Result<(), String> {
        if self.fragments.len() >= 32 {
            return Err("Extraction limit: 32 cards. Nothing extracted.".into());
        }
        let original = self
            .sources
            .iter()
            .find(|s| s.id == source)
            .ok_or("Unknown original")?;
        let text = original.text.clone();
        if !grapheme_boundary(&text, start) || !grapheme_boundary(&text, end) {
            return Err("Selection cuts a grapheme; nothing extracted".into());
        }
        text.get(start..end)
            .filter(|s| !s.trim().is_empty())
            .ok_or("Select a nonempty exact source range first")?;
        if self
            .fragments
            .iter()
            .any(|f| f.source == source && f.start == start && f.end == end)
        {
            return Err("That exact selection is already extracted.".into());
        }
        self.add_fragment(source, &text, start, end);
        self.notice = "Extracted by you; original intact. e shows cards; nothing confirmed.".into();
        Ok(())
    }
    pub(super) fn handle_source_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if !self.sources.is_empty()
            && ctrl
            && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
        {
            self.source_view = if key.code == KeyCode::PageDown {
                (self.source_view + 1) % self.sources.len()
            } else {
                (self.source_view + self.sources.len() - 1) % self.sources.len()
            };
            self.source_scroll = 0;
            self.source_cursor = 0;
            self.selection = None;
            self.drag_anchor = None;
            self.show_extractions = false;
            return true;
        }
        if ctrl && key.code == KeyCode::Char('e') {
            let result = if let (Some(source), Some((a, b))) =
                (self.sources.get(self.source_view), self.selection)
            {
                self.extract_range(source.id, a.min(b), a.max(b))
            } else {
                Err("Select text on the intact dump, then Ctrl-E extracts it.".into())
            };
            if let Err(error) = result {
                self.notice = error;
            }
            return true;
        }
        if !self.board_focus || self.sources.is_empty() {
            return false;
        }
        if key.code == KeyCode::Char('e') {
            if self.fragments.is_empty() {
                self.notice = "No extracted cards. Select source text, then Ctrl-E.".into();
            } else {
                self.show_extractions = !self.show_extractions;
            }
            self.canvas.cancel_gesture();
            self.drag_anchor = None;
            return true;
        }
        if self.show_extractions {
            if key.code == KeyCode::Enter {
                if let Some(fragment) = self
                    .canvas
                    .selected_id()
                    .and_then(|id| self.fragments.iter().find(|f| f.id == id))
                    .cloned()
                {
                    self.source_view = self
                        .sources
                        .iter()
                        .position(|s| s.id == fragment.source)
                        .unwrap();
                    self.source_cursor = fragment.start;
                    self.selection = Some((fragment.start, fragment.end));
                    self.source_scroll = Note::new(&self.sources[self.source_view].text)
                        .wrap(self.source_area.width)
                        .positions
                        .iter()
                        .find(|(i, _, _)| *i == fragment.start)
                        .map_or(0, |(_, row, _)| *row as u16);
                }
                self.show_extractions = false;
                return true;
            }
            return false;
        }
        if key.code == KeyCode::Esc {
            self.selection = None;
            self.drag_anchor = None;
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
            let mut note = Note::new(&self.sources[self.source_view].text);
            note.cursor = self.source_cursor;
            let anchor = self.selection.map_or(self.source_cursor, |(a, _)| a);
            match key.code {
                KeyCode::Left => note.left(),
                KeyCode::Right => note.right(),
                KeyCode::Up => note.move_row(false, self.source_area.width),
                KeyCode::Down => note.move_row(true, self.source_area.width),
                KeyCode::Home => note.home(),
                KeyCode::End => note.end(),
                _ => {}
            }
            self.source_cursor = note.cursor;
            self.selection = key
                .modifiers
                .contains(KeyModifiers::SHIFT)
                .then_some((anchor, note.cursor));
            let row = note.wrap(self.source_area.width).cursor.0;
            if row < self.source_scroll {
                self.source_scroll = row;
            } else if row >= self.source_scroll + self.source_area.height {
                self.source_scroll = row.saturating_sub(self.source_area.height.saturating_sub(1));
            }
            return true;
        }
        false
    }
    pub(super) fn handle_source_mouse(&mut self, event: MouseEvent) -> bool {
        if self.sources.is_empty() || self.show_extractions {
            return false;
        }
        let area = self.source_area;
        let inside = area.contains((event.column, event.row).into());
        if !inside && self.drag_anchor.is_none() {
            return false;
        }
        match event.kind {
            MouseEventKind::ScrollDown if inside => {
                self.source_scroll = self.source_scroll.saturating_add(3)
            }
            MouseEventKind::ScrollUp if inside => {
                self.source_scroll = self.source_scroll.saturating_sub(3)
            }
            MouseEventKind::Down(MouseButton::Left) if inside => {
                let index = Note::new(&self.sources[self.source_view].text)
                    .wrap(area.width)
                    .index_at(
                        event.row - area.y + self.source_scroll,
                        event.column - area.x,
                    );
                self.board_focus = true;
                self.source_cursor = index;
                self.selection = None;
                self.drag_anchor = Some(index);
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
                if self.drag_anchor.is_some() =>
            {
                let row = event.row.clamp(area.y, area.bottom().saturating_sub(1)) - area.y
                    + self.source_scroll;
                let col = event.column.clamp(area.x, area.right().saturating_sub(1)) - area.x;
                let end = Note::new(&self.sources[self.source_view].text)
                    .wrap(area.width)
                    .index_at(row, col);
                self.selection = Some((self.drag_anchor.unwrap(), end));
                self.source_cursor = end;
                if matches!(event.kind, MouseEventKind::Up(_)) {
                    self.drag_anchor = None;
                }
            }
            _ => return false,
        }
        true
    }
}

pub(super) fn render_source(frame: &mut Frame, app: &mut BrainDump, area: Rect, palette: Palette) {
    let Some(source) = app.sources.get(app.source_view) else {
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
                if app.source_scroll
                    >= wrapped
                        .lines
                        .len()
                        .saturating_sub(usize::from(inner.height)) as u16
                {
                    "↑ more"
                } else if app.source_scroll > 0 {
                    "↕ more"
                } else {
                    "↓ more"
                },
            )
            .style(palette.muted),
            Rect::new(area.x, area.bottom() - 1, area.width, 1),
        );
    }
    app.source_area = inner;
    let max = wrapped
        .lines
        .len()
        .saturating_sub(usize::from(inner.height)) as u16;
    app.source_scroll = app.source_scroll.min(max);
    frame.render_widget(
        Paragraph::new(wrapped.lines.join("\n"))
            .style(palette.ink)
            .scroll((app.source_scroll, 0)),
        inner,
    );
    // P10: the original stays plain ink; only your own selection is drawn. The exact-quote
    // check still runs; evidence is shown on demand later (C13), never painted by default.
    if app.board_focus
        && !app.original
        && !app.help
        && let Some((_, row, col)) = wrapped
            .positions
            .iter()
            .find(|(i, _, _)| *i == app.source_cursor)
        && let Some(row) = row.checked_sub(usize::from(app.source_scroll))
        && row < usize::from(inner.height)
        && *col < usize::from(inner.width)
    {
        frame.set_cursor_position((inner.x + *col as u16, inner.y + row as u16));
    }
    for (index, grapheme) in source.text.grapheme_indices(true) {
        let Some((_, row, col)) = wrapped.positions.iter().find(|(i, _, _)| *i == index) else {
            continue;
        };
        let Some(row) = row.checked_sub(usize::from(app.source_scroll)) else {
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
            .selection
            .is_some_and(|(a, b)| (a.min(b)..a.max(b)).contains(&index))
        {
            style = style.add_modifier(Modifier::REVERSED);
        }
        cell.set_style(style);
    }
}
