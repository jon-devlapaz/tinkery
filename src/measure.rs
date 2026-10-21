//! Measure's honest unknown state; no source reads, writes or provider calls.

use crate::Palette;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

pub(crate) fn render(frame: &mut Frame, area: Rect, palette: Palette) {
    frame.render_widget(
        Paragraph::new("Recorded work and attention are unknown.").style(palette.ink),
        Rect {
            height: area.height.saturating_sub(2),
            ..area
        },
    );
    frame.render_widget(
        Paragraph::new("esc back   ? help").style(palette.muted),
        Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
    );
}
