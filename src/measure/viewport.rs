//! A machine-sized diagnostic viewport; only visible source segments reach Ratatui's u16 scroll API.

use std::{ops::Range, sync::Arc};

use crate::Palette;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;

// Even a single enormous logical line must fit a local Paragraph scroll offset.
const SEGMENT_GRAPHEMES: usize = 16_384;
const SEGMENT_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub(super) struct Viewport {
    text: Arc<str>,
    width: u16,
    segments: Vec<Segment>,
    cursor: usize,
    complete: bool,
}

struct Segment {
    source: Range<usize>,
    rows: Range<usize>,
}

impl Viewport {
    pub(super) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        palette: Palette,
        text: impl Into<Arc<str>>,
        offset: &mut usize,
    ) {
        if area.is_empty() {
            return;
        }
        frame.render_widget(Paragraph::new("").style(palette.ink), area);
        let text = text.into();
        if self.width != area.width || (!Arc::ptr_eq(&self.text, &text) && self.text != text) {
            self.text = text;
            self.width = area.width;
            self.cursor = 0;
            self.segments.clear();
            self.complete = false;
        }
        self.advance();
        let rows = self.segments.last().map_or(0, |s| s.rows.end);
        if self.complete {
            *offset = (*offset).min(rows.saturating_sub(usize::from(area.height)));
        }
        let displayed = (*offset).min(rows.saturating_sub(usize::from(area.height)));
        let first = self.segments.partition_point(|s| s.rows.end <= displayed);
        let mut remaining = area;
        for segment in &self.segments[first..] {
            let local = displayed.saturating_sub(segment.rows.start);
            let height = (segment.rows.len() - local).min(usize::from(remaining.height)) as u16;
            let visible = Rect {
                height,
                ..remaining
            };
            let local =
                u16::try_from(local).expect("diagnostic segment exceeds local scroll range");
            let paragraph = Paragraph::new(&self.text[segment.source.clone()])
                .style(palette.ink)
                .scroll((local, 0));
            frame.render_widget(paragraph.wrap(Wrap { trim: false }), visible);
            remaining.y += height;
            remaining.height -= height;
            if remaining.height == 0 {
                break;
            }
        }
        if !self.ready() {
            frame.render_widget(
                Paragraph::new("Preparing report…").style(palette.muted),
                Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
            );
        }
    }

    pub(super) fn ready(&self) -> bool {
        self.complete
    }

    fn advance(&mut self) {
        let mut budget = SEGMENT_BYTES;
        while self.cursor < self.text.len() && budget > 0 {
            let tail = &self.text[self.cursor..];
            let mut bound = tail.len().min(SEGMENT_BYTES - 1);
            while !tail.is_char_boundary(bound) {
                bound -= 1;
            }
            let newline = tail[..bound].rfind('\n');
            let end = newline.unwrap_or({
                if bound == tail.len() {
                    bound
                } else {
                    usize::MAX
                }
            });
            let terminated = end < tail.len() && tail.as_bytes()[end] == b'\n';
            let chunk = if end != usize::MAX {
                end + usize::from(terminated)
            } else {
                tail.grapheme_indices(true)
                    .enumerate()
                    .find(|(index, (byte, _))| *index >= SEGMENT_GRAPHEMES || *byte >= bound)
                    .map_or(tail.len(), |(_, (byte, _))| byte)
            };
            let rows = wrapped_rows(&tail[..chunk], self.width);
            add_segment(&mut self.segments, self.cursor..self.cursor + chunk, rows);
            self.cursor += chunk;
            if !terminated && self.text.as_bytes().get(self.cursor) == Some(&b'\n') {
                self.cursor += 1;
            }
            budget = budget.saturating_sub(chunk.max(1));
        }
        self.complete = self.cursor == self.text.len();
    }
}

fn wrapped_rows(text: &str, width: u16) -> usize {
    Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .line_count(width)
}

fn add_segment(segments: &mut Vec<Segment>, source: Range<usize>, rows: usize) {
    let first = segments.last().map_or(0, |s| s.rows.end);
    segments.push(Segment {
        source,
        rows: first..first + rows,
    });
}
