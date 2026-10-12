use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct Note {
    pub text: String,
    pub cursor: usize,
    /// Fixed end of a selection, as a logical byte offset on a grapheme boundary.
    /// The moving end is `cursor`. Any edit clears it, so it can never outlive the text
    /// it indexes; an anchor equal to the cursor is an empty selection.
    anchor: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

impl Note {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: text.len(),
            anchor: None,
        }
    }

    /// The nonempty selection as ordered logical byte offsets, if any.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor.filter(|a| *a != self.cursor)?;
        let (start, end) = (anchor.min(self.cursor), anchor.max(self.cursor));
        (self.boundary(start) && self.boundary(end)).then_some((start, end))
    }

    fn boundary(&self, index: usize) -> bool {
        index == self.text.len()
            || (index < self.text.len()
                && self.text.grapheme_indices(true).any(|(i, _)| i == index))
    }

    /// Nearest grapheme boundary at or before `index` (clamped to the text).
    fn floor(&self, index: usize) -> usize {
        if index >= self.text.len() {
            return self.text.len();
        }
        self.text
            .grapheme_indices(true)
            .take_while(|(i, _)| *i <= index)
            .last()
            .map_or(0, |(i, _)| i)
    }

    /// Keep the caret and anchor on grapheme boundaries before any edit or move, so a
    /// stale offset can never slice inside a character.
    fn settle(&mut self) {
        self.cursor = self.floor(self.cursor);
        self.anchor = self.anchor.map(|a| self.floor(a));
    }

    pub fn selected_text(&self) -> Option<&str> {
        let (start, end) = self.selection()?;
        self.text.get(start..end)
    }

    pub fn select_all(&mut self) {
        self.anchor = (!self.text.is_empty()).then_some(0);
        self.cursor = self.text.len();
    }

    /// Drop the selection but keep the text and the caret where it is.
    pub fn collapse(&mut self) {
        self.anchor = None;
    }

    /// Start a mouse drag at `index` (a grapheme boundary).
    pub fn begin_drag(&mut self, index: usize) {
        self.cursor = index;
        self.anchor = Some(index);
        self.settle();
    }

    /// Extend a drag started by [`begin_drag`]. False when an edit or a collapse has
    /// since cleared the anchor: the drag is over and must not select anything.
    pub fn drag_to(&mut self, index: usize) -> bool {
        if self.anchor.is_none() {
            return false;
        }
        self.cursor = index;
        self.settle();
        true
    }

    /// Move the caret. With `extend` the selection grows from its fixed end; without,
    /// Left/Right collapse to the selection's edge and other motions just move.
    pub fn navigate(&mut self, motion: Motion, extend: bool, width: u16) {
        self.settle();
        if !extend {
            if let (Some((start, end)), Motion::Left | Motion::Right) = (self.selection(), motion) {
                self.cursor = if motion == Motion::Left { start } else { end };
                self.anchor = None;
                return;
            }
            self.anchor = None;
        }
        let anchor = self.anchor.unwrap_or(self.cursor);
        let before = self.cursor;
        match motion {
            Motion::Left => self.left(),
            Motion::Right => self.right(),
            Motion::Home => self.home(),
            Motion::End => self.end(),
            Motion::Up | Motion::Down => {
                self.move_row(motion == Motion::Down, width);
                // Selecting past the first or last row reaches the edge of the buffer.
                if extend && self.cursor == before {
                    self.cursor = if motion == Motion::Up {
                        0
                    } else {
                        self.text.len()
                    };
                }
            }
        }
        if extend {
            self.anchor = (anchor != self.cursor).then_some(anchor);
        }
    }

    /// Replace `start..end` (grapheme boundaries) with already-sanitized text.
    /// The 4096-byte cap is checked on the proposed result before anything changes.
    fn replace(&mut self, start: usize, end: usize, clean: &str) -> Result<(), &'static str> {
        let (start, end) = (self.floor(start), self.floor(end).max(self.floor(start)));
        if self.text.len() - (end - start) + clean.len() > 4096 {
            return Err("Card limit: 4096 bytes. Nothing inserted.");
        }
        self.text.replace_range(start..end, clean);
        self.cursor = start + clean.len();
        self.anchor = None;
        self.realign_cursor_forward();
        Ok(())
    }

    pub fn insert(&mut self, text: &str) -> Result<(), &'static str> {
        // Paste is data, not terminal control sequences. Keep line breaks, not escapes.
        let clean = Self::sanitize(text);
        if clean.is_empty() {
            return Ok(());
        }
        self.settle();
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        self.replace(start, end, &clean)
    }

    /// Inserting before a combining mark may join it to the inserted grapheme.
    fn realign_cursor_forward(&mut self) {
        while self.cursor < self.text.len()
            && !self
                .text
                .grapheme_indices(true)
                .any(|(index, _)| index == self.cursor)
        {
            self.cursor += self.text[self.cursor..].chars().next().unwrap().len_utf8();
        }
    }

    pub(crate) fn sanitize(text: &str) -> String {
        text.replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect()
    }

    pub fn left(&mut self) {
        self.cursor = self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(i, _)| i);
    }

    pub fn right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor += self.text[self.cursor..]
                .graphemes(true)
                .next()
                .unwrap()
                .len();
        }
    }

    fn delete_selection(&mut self) -> bool {
        self.settle();
        let Some((start, end)) = self.selection() else {
            return false;
        };
        self.text.replace_range(start..end, "");
        self.cursor = start;
        self.anchor = None;
        self.realign_cursor();
        true
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        self.cursor = self.floor(self.cursor);
        self.anchor = None; // an edit (or a no-op at the edge) ends any drag
        let end = self.cursor;
        self.left();
        self.text.replace_range(self.cursor..end, "");
        self.realign_cursor();
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        self.cursor = self.floor(self.cursor);
        self.anchor = None; // an edit (or a no-op at the edge) ends any drag
        let start = self.cursor;
        self.right();
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.realign_cursor();
    }

    fn realign_cursor(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self
                .text
                .grapheme_indices(true)
                .take_while(|(index, _)| *index <= self.cursor)
                .last()
                .map_or(0, |(index, _)| index);
        }
    }

    pub fn home(&mut self) {
        self.cursor = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
    }

    pub fn end(&mut self) {
        self.cursor += self.text[self.cursor..]
            .find('\n')
            .unwrap_or(self.text.len() - self.cursor);
    }

    pub fn move_row(&mut self, down: bool, width: u16) {
        let wrapped = self.wrap(width);
        let current = wrapped
            .positions
            .iter()
            .find(|(index, _, _)| *index == self.cursor)
            .unwrap();
        let row = if down {
            current.1.saturating_add(1)
        } else {
            current.1.saturating_sub(1)
        };
        if let Some((index, _, _)) = wrapped
            .positions
            .iter()
            .filter(|(_, y, _)| *y == row)
            .min_by_key(|(_, _, x)| x.abs_diff(current.2))
        {
            self.cursor = *index;
        }
    }

    pub fn wrap(&self, width: u16) -> Wrapped {
        let width = usize::from(width.max(2));
        let graphemes: Vec<_> = self.text.grapheme_indices(true).collect();
        let mut lines = vec![String::new()];
        let mut positions = Vec::new();
        let (mut row, mut column) = (0usize, 0usize);
        for (i, &(index, grapheme)) in graphemes.iter().enumerate() {
            let whitespace = grapheme.chars().all(char::is_whitespace);
            let word_start = i == 0 || graphemes[i - 1].1.chars().all(char::is_whitespace);
            if word_start && !whitespace && column > 0 {
                let word_width: usize = graphemes[i..]
                    .iter()
                    .take_while(|(_, g)| !g.chars().all(char::is_whitespace))
                    .map(|(_, g)| g.width())
                    .sum();
                if word_width <= width && column + word_width > width {
                    lines.push(String::new());
                    row += 1;
                    column = 0;
                }
            }
            if grapheme != "\n" && column + grapheme.width() > width {
                lines.push(String::new());
                row += 1;
                column = 0;
                if whitespace {
                    positions.push((index, row, column));
                    continue;
                }
            }
            positions.push((index, row, column));
            if grapheme == "\n" {
                lines.push(String::new());
                row += 1;
                column = 0;
            } else {
                lines[row].push_str(grapheme);
                column += grapheme.width();
            }
        }
        if column >= width {
            lines.push(String::new());
            row += 1;
            column = 0;
        }
        positions.push((self.text.len(), row, column));
        let (_, cursor_row, cursor_column) = positions
            .iter()
            .find(|(i, _, _)| *i == self.cursor)
            .unwrap();
        Wrapped {
            cursor: (*cursor_row as u16, *cursor_column as u16),
            lines,
            positions,
        }
    }
}

pub struct Wrapped {
    pub lines: Vec<String>,
    pub cursor: (u16, u16),
    pub(super) positions: Vec<(usize, usize, usize)>,
}

impl Wrapped {
    pub fn index_at(&self, row: u16, column: u16) -> usize {
        self.positions
            .iter()
            .min_by_key(|(_, y, x)| {
                (
                    y.abs_diff(usize::from(row)),
                    x.abs_diff(usize::from(column)),
                )
            })
            .map_or(0, |(index, _, _)| *index)
    }
}
