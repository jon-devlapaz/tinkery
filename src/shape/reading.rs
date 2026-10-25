//! Shape's styled reading stays reachable beyond terminal-sized row counts. Layout and visible
//! rows agree; only a viewport reaches the renderer, and source words are never changed.

use ratatui::{
    Frame,
    buffer::CellWidth,
    layout::Rect,
    text::{Line, Span, StyledGrapheme, Text},
    widgets::Paragraph,
};
use std::collections::VecDeque;

/// Width-wrapped display rows, independent of the terminal's bounded coordinates.
pub(super) struct Reading(Vec<Line<'static>>);

impl Reading {
    pub(super) fn new(text: Text<'static>, width: u16) -> Self {
        let mut rows = Vec::new();
        if width > 0 {
            for mut line in text.lines {
                // Printable ASCII shorter than the viewport cannot wrap or regroup.
                // Keep these owned rows instead of rebuilding every glyph on each draw.
                if line.spans.iter().all(|span| {
                    span.content
                        .bytes()
                        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
                }) && line.width() < usize::from(width)
                {
                    line.style = text.style.patch(line.style);
                    line.alignment = line.alignment.or(text.alignment);
                    rows.push(line);
                    continue;
                }
                let graphemes = line.spans.iter().enumerate().flat_map(|(index, span)| {
                    span.styled_graphemes(text.style.patch(line.style))
                        .map(move |grapheme| (index, grapheme))
                });
                wrap(graphemes, width, |graphemes| {
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut used = 0;
                    let mut previous_span = None;
                    for (index, grapheme) in graphemes {
                        let cells = usize::from(grapheme.symbol.cell_width());
                        // A mixed-width word can overrun Ratatui's pending row at narrow widths.
                        // Keep every fitting grapheme, rather than truncate that display row.
                        if used + cells > usize::from(width) && !spans.is_empty() {
                            let mut row = Line::from(std::mem::take(&mut spans));
                            row.alignment = line.alignment.or(text.alignment);
                            rows.push(row);
                            used = 0;
                            previous_span = None;
                        }
                        used += cells;
                        if let Some(last) = spans.last_mut()
                            && previous_span == Some(index)
                        {
                            last.content.to_mut().push_str(grapheme.symbol);
                        } else {
                            spans.push(Span::styled(grapheme.symbol.to_owned(), grapheme.style));
                        }
                        previous_span = Some(index);
                    }
                    let mut row = Line::from(spans);
                    row.alignment = line.alignment.or(text.alignment);
                    rows.push(row);
                });
            }
        }
        Self(rows)
    }

    pub(super) fn rows(&self) -> usize {
        self.0.len()
    }

    pub(super) fn render(&self, frame: &mut Frame, area: Rect, offset: usize) {
        let visible: Vec<_> = self
            .0
            .iter()
            .skip(offset)
            .take(usize::from(area.height))
            .cloned()
            .collect();
        frame.render_widget(Paragraph::new(visible), area);
    }
}

// The trim:false word-boundary rules below are adapted from ratatui-widgets 0.3.2's
// private WordWrapper. Keep its wrapping while removing the renderer's u16 scroll limit.
//
// The MIT License (MIT)
// Copyright (c) 2016-2022 Florian Dehau
// Copyright (c) 2023-2025 The Ratatui Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
fn wrap<'a>(
    graphemes: impl Iterator<Item = (usize, StyledGrapheme<'a>)>,
    width: u16,
    mut emit: impl FnMut(&[(usize, StyledGrapheme<'a>)]),
) {
    let width = usize::from(width);
    let mut line = Vec::new();
    let mut word = Vec::new();
    let mut spaces: VecDeque<(usize, StyledGrapheme<'a>)> = VecDeque::new();
    let (mut line_width, mut word_width, mut space_width) = (0, 0, 0);
    let mut previous_word = false;
    let mut emitted = false;
    for grapheme in graphemes {
        let whitespace = grapheme.1.is_whitespace();
        let cells = usize::from(grapheme.1.symbol.cell_width());
        if cells > width {
            continue;
        }
        if previous_word && whitespace
            || line.is_empty() && word_width + space_width + cells > width
        {
            line.extend(spaces.drain(..));
            line.append(&mut word);
            line_width += space_width + word_width;
            space_width = 0;
            word_width = 0;
        }
        if line_width >= width || cells > 0 && line_width + space_width + word_width >= width {
            emit(&line);
            emitted = true;
            line.clear();
            let mut remaining = width.saturating_sub(line_width);
            line_width = 0;
            while let Some(space) = spaces.front() {
                let cells = usize::from(space.1.symbol.cell_width());
                if cells > remaining {
                    break;
                }
                space_width -= cells;
                remaining -= cells;
                spaces.pop_front();
            }
            if whitespace && spaces.is_empty() {
                continue;
            }
        }
        if whitespace {
            space_width += cells;
            spaces.push_back(grapheme);
        } else {
            word_width += cells;
            word.push(grapheme);
        }
        previous_word = !whitespace;
    }
    line.extend(spaces);
    line.append(&mut word);
    if !line.is_empty() || !emitted {
        emit(&line);
    }
}

#[cfg(test)]
mod tests;
