//! Prepared success-report clipping; frames visit visible glyphs only, never invisible runs or hidden rows.

use std::sync::Arc;

use crate::Palette;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) struct Plain {
    text: Arc<str>,
    glyphs: Vec<Glyph>,
    rows: Vec<u32>,
}

struct Glyph {
    start: u32,
    bytes: u16,
    width: u16,
}

impl Plain {
    // Prepared off-thread after the raw-output and per-grapheme display checks.
    // Byte offsets fit u32 under the 16 MiB capture limit; a cluster fits 4096 bytes.
    pub(super) fn new(body: &str) -> Self {
        let mut glyphs = Vec::new();
        let mut rows = Vec::new();
        let mut start = 0;
        for line in body.split_terminator('\n') {
            rows.push(glyphs.len() as u32);
            for (byte, cluster) in line.grapheme_indices(true) {
                let width = cluster.width();
                if width != 0 {
                    glyphs.push(Glyph {
                        start: (start + byte) as u32,
                        bytes: cluster.len() as u16,
                        width: width as u16,
                    });
                }
            }
            start += line.len() + 1;
        }
        Self {
            text: Arc::from(body),
            glyphs,
            rows,
        }
    }

    pub(super) fn render(&self, frame: &mut Frame, area: Rect, palette: Palette, offset: usize) {
        frame.render_widget(Paragraph::new("").style(palette.ink), area);
        for (row, index) in (offset..self.rows.len())
            .take(usize::from(area.height))
            .enumerate()
        {
            let start = self.rows[index] as usize;
            let end = self
                .rows
                .get(index + 1)
                .map_or(self.glyphs.len(), |n| *n as usize);
            let mut x = 0;
            for glyph in &self.glyphs[start..end] {
                if glyph.width > area.width {
                    continue;
                }
                if x + glyph.width > area.width {
                    break;
                }
                let start = glyph.start as usize;
                frame.buffer_mut()[(area.x + x, area.y + row as u16)]
                    .set_symbol(&self.text[start..start + glyph.bytes as usize])
                    .set_style(palette.ink);
                x += glyph.width;
            }
        }
    }
}
