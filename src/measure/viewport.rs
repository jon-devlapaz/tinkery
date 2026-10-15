//! A machine-sized diagnostic viewport; only visible source segments reach Ratatui's u16 scroll API.

use std::ops::Range;

use crate::Palette;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;

// Even a single enormous logical line must fit a local Paragraph scroll offset.
const SEGMENT_GRAPHEMES: usize = 16_384;

#[derive(Default)]
pub(super) struct Viewport {
    text: String,
    width: u16,
    segments: Vec<Segment>,
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
        text: String,
        offset: &mut usize,
    ) {
        if area.is_empty() {
            return;
        }
        frame.render_widget(Paragraph::new("").style(palette.ink), area);
        if self.width != area.width || self.text != text {
            self.layout(text, area.width);
        }
        let rows = self.segments.last().map_or(0, |s| s.rows.end);
        *offset = (*offset).min(rows.saturating_sub(usize::from(area.height)));
        let first = self.segments.partition_point(|s| s.rows.end <= *offset);
        let mut remaining = area;
        for segment in &self.segments[first..] {
            let local = offset.saturating_sub(segment.rows.start);
            let height = (segment.rows.len() - local).min(usize::from(remaining.height)) as u16;
            let visible = Rect {
                height,
                ..remaining
            };
            let local =
                u16::try_from(local).expect("diagnostic segment exceeds local scroll range");
            frame.render_widget(
                Paragraph::new(&self.text[segment.source.clone()])
                    .style(palette.ink)
                    .wrap(Wrap { trim: false })
                    .scroll((local, 0)),
                visible,
            );
            remaining.y += height;
            remaining.height -= height;
            if remaining.height == 0 {
                break;
            }
        }
    }

    fn layout(&mut self, text: String, width: u16) {
        self.text = text;
        self.width = width;
        self.segments.clear();
        let mut start = 0;
        for raw in self.text.split_inclusive('\n') {
            let line = raw.strip_suffix('\n').unwrap_or(raw);
            let rows = wrapped_rows(line, width);
            if rows <= usize::from(u16::MAX) {
                add_segment(&mut self.segments, start..start + line.len(), rows);
                start += raw.len();
                continue;
            }
            let mut chunk = start;
            for (byte, _) in line
                .grapheme_indices(true)
                .step_by(SEGMENT_GRAPHEMES)
                .skip(1)
            {
                let rows = wrapped_rows(&self.text[chunk..start + byte], width);
                add_segment(&mut self.segments, chunk..start + byte, rows);
                chunk = start + byte;
            }
            let rows = wrapped_rows(&self.text[chunk..start + line.len()], width);
            add_segment(&mut self.segments, chunk..start + line.len(), rows);
            start += raw.len();
        }
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
