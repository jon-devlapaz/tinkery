use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct Note {
    pub text: String,
    pub cursor: usize,
}

impl Note {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: text.len(),
        }
    }

    pub fn insert(&mut self, text: &str) -> Result<(), &'static str> {
        // Paste is data, not terminal control sequences. Keep line breaks, not escapes.
        let clean = Self::sanitize(text);
        if self.text.len().saturating_add(clean.len()) > 4096 {
            return Err("Card limit: 4096 bytes. Nothing inserted.");
        }
        self.text.insert_str(self.cursor, &clean);
        self.cursor += clean.len();
        // Inserting before a combining mark may join it to the inserted grapheme.
        while self.cursor < self.text.len()
            && !self
                .text
                .grapheme_indices(true)
                .any(|(index, _)| index == self.cursor)
        {
            self.cursor += self.text[self.cursor..].chars().next().unwrap().len_utf8();
        }
        Ok(())
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

    pub fn backspace(&mut self) {
        let end = self.cursor;
        self.left();
        self.text.replace_range(self.cursor..end, "");
        self.realign_cursor();
    }

    pub fn delete(&mut self) {
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
