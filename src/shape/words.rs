//! Shape's in-memory editor: textarea motions, selection and wrapping, with original line endings
//! retained separately. It owns no save, folder or agent policy.

use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    widgets::{Paragraph, Wrap},
};
use ratatui_textarea::{CursorMove, TextArea, WrapMode};

#[derive(Clone)]
struct State {
    raw: String,
    editor: TextArea<'static>,
}

/// An editable box whose displayed newlines do not normalize the operator's stored words.
pub(super) struct Words {
    state: State,
    undo: Vec<State>,
    redo: Vec<State>,
    yank: String,
}

impl Words {
    pub(super) fn new(raw: &str) -> Self {
        let mut editor = TextArea::default();
        editor.set_max_histories(0);
        editor.set_wrap_mode(WrapMode::WordOrGlyph);
        editor.insert_str(display(raw));
        Self {
            state: State {
                raw: raw.into(),
                editor,
            },
            undo: Vec::new(),
            redo: Vec::new(),
            yank: String::new(),
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.state.raw
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.text().is_empty()
    }

    pub(super) fn clear(&mut self) {
        *self = Self::new("");
    }

    fn offset(&self, (row, col): (usize, usize)) -> usize {
        self.state.editor.lines()[..row]
            .iter()
            .map(|l| l.chars().count() + 1)
            .sum::<usize>()
            + col
    }

    fn range(&self) -> (usize, usize) {
        let (start, end) = self.state.editor.selection_range().unwrap_or_else(|| {
            let cursor = self.state.editor.cursor();
            ((cursor.0, cursor.1), (cursor.0, cursor.1))
        });
        (self.offset(start), self.offset(end))
    }

    fn remember(&mut self, before: State) {
        if before.raw != self.state.raw {
            if self.undo.len() == 50 {
                self.undo.remove(0);
            }
            self.undo.push(before);
            self.redo.clear();
        }
    }

    /// Insert at the caret, replacing the selection; CR, CRLF and LF remain distinct in storage.
    pub(super) fn insert(&mut self, raw: &str) {
        let before = self.state.clone();
        let (start, end) = self.range();
        let start = raw_offset(&before.raw, start);
        let end = raw_offset(&before.raw, end);
        self.state.editor.insert_str(display(raw));
        self.state.raw.replace_range(start..end, raw);
        self.resync(start + raw.len());
        self.remember(before);
    }

    pub(super) fn key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
        {
            match key.code {
                KeyCode::Char('u') => {
                    if let Some(previous) = self.undo.pop() {
                        self.redo.push(std::mem::replace(&mut self.state, previous));
                    }
                    return;
                }
                KeyCode::Char('r') => {
                    if let Some(next) = self.redo.pop() {
                        self.undo.push(std::mem::replace(&mut self.state, next));
                    }
                    return;
                }
                KeyCode::Char('c' | 'x') => {
                    let (start, end) = self.range();
                    if start != end {
                        self.yank = self.text()
                            [raw_offset(self.text(), start)..raw_offset(self.text(), end)]
                            .into();
                    }
                    if key.code == KeyCode::Char('c') {
                        return;
                    }
                }
                KeyCode::Char('y') => {
                    self.insert(&self.yank.clone());
                    return;
                }
                _ => {}
            }
        }
        let before = self.state.clone();
        let (start, end) = self.range();
        let old: Vec<char> = display(&before.raw).chars().collect();
        if self.state.editor.input(key) {
            let new: Vec<char> = self.state.editor.lines().join("\n").chars().collect();
            let cursor = self.state.editor.cursor();
            let cursor = self.offset((cursor.0, cursor.1));
            // Bound the diff by the selection/caret: identical neighbouring newlines must not
            // move an edit onto a different original CR or CRLF.
            let prefix = old
                .iter()
                .zip(&new)
                .take(start.min(cursor))
                .take_while(|(a, b)| a == b)
                .count();
            let suffix = old[prefix..]
                .iter()
                .rev()
                .zip(new[prefix..].iter().rev())
                .take((old.len() - end).min(new.len() - cursor))
                .take_while(|(a, b)| a == b)
                .count();
            let replacement: String = new[prefix..new.len() - suffix].iter().collect();
            let from = raw_offset(&before.raw, prefix);
            let to = raw_offset(&before.raw, old.len() - suffix);
            if from < to && display(&before.raw[from..to]) == self.state.editor.yank_text() {
                self.yank = before.raw[from..to].into();
            }
            self.state.raw.replace_range(from..to, &replacement);
            self.resync(from + replacement.len());
            self.remember(before);
        }
    }

    // An edit can bring a CR and LF together across its boundary. Their raw bytes stay, but
    // they now display as one newline, so rebuild the projection and follow the raw caret.
    fn resync(&mut self, mut cursor: usize) {
        if display(self.text()) == self.state.editor.lines().join("\n") {
            return;
        }
        if self.text()[..cursor].ends_with('\r') && self.text()[cursor..].starts_with('\n') {
            cursor += 1;
        }
        let prefix = display(&self.text()[..cursor]);
        let row = prefix.matches('\n').count();
        let col = prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count();
        self.state.editor = Self::new(self.text()).state.editor;
        self.state.editor.move_cursor(CursorMove::Top);
        self.state.editor.move_cursor(CursorMove::Head);
        for _ in 0..row {
            self.state.editor.move_cursor(CursorMove::Down);
        }
        for _ in 0..col {
            self.state.editor.move_cursor(CursorMove::Forward);
        }
    }

    pub(super) fn rows(&self, width: u16) -> u16 {
        Paragraph::new(display(self.text()))
            .wrap(Wrap { trim: false })
            .line_count(width)
            .max(1)
            .min(u16::MAX as usize) as u16
    }

    pub(super) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette, calm: bool) {
        let style = if calm { palette.muted } else { palette.ink };
        let editor = &mut self.state.editor;
        editor.set_style(style);
        editor.set_cursor_line_style(style);
        editor.set_cursor_style(if calm {
            style
        } else {
            style.add_modifier(Modifier::REVERSED)
        });
        editor.set_selection_style(style.add_modifier(Modifier::REVERSED));
        frame.render_widget(&*editor, area);
    }
}

fn display(raw: &str) -> String {
    raw.replace("\r\n", "\n").replace('\r', "\n")
}

/// A character offset in the display mapped back to raw bytes; CRLF is one displayed newline.
fn raw_offset(raw: &str, index: usize) -> usize {
    let mut chars = raw.char_indices().peekable();
    for _ in 0..index {
        if let Some((_, '\r')) = chars.next()
            && chars.peek().is_some_and(|(_, c)| *c == '\n')
        {
            chars.next();
        }
    }
    chars.peek().map_or(raw.len(), |(offset, _)| *offset)
}

#[cfg(test)]
mod tests;
