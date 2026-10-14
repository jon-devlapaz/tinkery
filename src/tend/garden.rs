//! The saved-goal garden: deterministic pages, hollow rings, labels and cell hit-testing.

use super::{Tend, clipped, records, row};
use crate::Palette;
use ratatui::{
    Frame,
    layout::{Alignment, Position, Rect},
    style::Color,
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Paragraph,
        canvas::{Canvas, Points},
    },
};
use unicode_width::UnicodeWidthStr;

const RING: [(u16, u16); 16] = [
    (2, 1),
    (3, 1),
    (4, 1),
    (5, 1),
    (1, 2),
    (6, 2),
    (1, 3),
    (6, 3),
    (1, 4),
    (6, 4),
    (1, 5),
    (6, 5),
    (2, 6),
    (3, 6),
    (4, 6),
    (5, 6),
];

#[derive(Debug, PartialEq, Eq)]
struct Layout {
    area: Rect,
    columns: usize,
    cell_width: u16,
    visible_rows: usize,
    page: usize,
    pages: usize,
    start: usize,
    end: usize,
}
impl Layout {
    fn new(body: Rect, count: usize, selected: usize) -> Self {
        let area = Rect::new(body.x, body.y, body.width, body.height.saturating_sub(3));
        let columns = (usize::from(area.width) * 2 / 20).saturating_sub(1) / 2;
        let columns = columns.max(1);
        let cell_width = (usize::from(area.width) * 2 / (columns * 2 + 1)) as u16;
        let visible_rows = (usize::from(area.height) / 4).max(1);
        let capacity = columns * visible_rows;
        let page = selected / columns / visible_rows;
        let start = page * capacity;
        Self {
            area,
            columns,
            cell_width,
            visible_rows,
            page,
            pages: count.div_ceil(capacity),
            start,
            end: (start + capacity).min(count),
        }
    }
    fn cell(&self, index: usize) -> Rect {
        let local = index - self.start;
        let row = local / self.columns;
        let shift = if (index / self.columns) % 2 == 1 {
            self.cell_width / 2
        } else {
            0
        };
        Rect::new(
            self.area.x + (local % self.columns) as u16 * self.cell_width + shift,
            self.area.y + row as u16 * 4,
            self.cell_width,
            3,
        )
    }
    fn hit(&self, column: u16, row: u16) -> Option<usize> {
        (self.start..self.end).find(|&index| self.cell(index).contains(Position::new(column, row)))
    }
}

pub(super) fn hit(tend: &Tend, column: u16, row: u16, body: Rect) -> Option<usize> {
    Layout::new(body, tend.sessions.len(), tend.selected).hit(column, row)
}

pub(super) fn render(
    frame: &mut Frame,
    tend: &mut Tend,
    body: Rect,
    palette: Palette,
    now: Option<u64>,
) {
    let layout = Layout::new(body, tend.sessions.len(), tend.selected);
    tend.columns = layout.columns;
    let top = f64::from(layout.area.height) * 4.0 - 1.0;
    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .background_color(palette.ink.bg.unwrap_or(Color::Reset))
        .x_bounds([0.0, f64::from(layout.area.width) * 2.0 - 1.0])
        .y_bounds([0.0, top])
        .paint(|ctx| {
            for index in layout.start..layout.end {
                let cell = layout.cell(index);
                let style = if index == tend.selected {
                    palette.jade
                } else {
                    palette.ink
                };
                let left = f64::from(cell.x - layout.area.x + cell.width / 2 - 2) * 2.0;
                let first = top - f64::from(cell.y - layout.area.y + 1) * 4.0;
                let coords = RING.map(|(x, y)| (left + f64::from(x), first - f64::from(y)));
                ctx.draw(&Points {
                    coords: &coords,
                    color: style.fg.unwrap_or(Color::Reset),
                });
            }
        });
    frame.render_widget(canvas, layout.area);
    for index in layout.start..layout.end {
        let cell = layout.cell(index);
        let selected = index == tend.selected;
        let prefix = if selected { "› " } else { "" };
        let label = format!(
            "{prefix}{}",
            clipped(
                tend.sessions[index].goal.lines().next().unwrap_or(""),
                usize::from(cell.width.saturating_sub(2)).saturating_sub(prefix.width()),
            )
        );
        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(if selected {
                    palette.jade
                } else {
                    palette.muted
                }),
            Rect::new(cell.x, cell.y, cell.width, 1),
        );
    }
    let width = usize::from(body.width);
    let page_note = if layout.pages > 1 {
        format!(" · page {} of {}", layout.page + 1, layout.pages)
    } else {
        String::new()
    };
    let (goal, status) = row(
        &tend.sessions[tend.selected],
        width.saturating_sub(page_note.width()),
        now,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(goal, palette.ink),
            Span::styled(format!("{status}{page_note}"), palette.muted),
        ])),
        Rect::new(body.x, body.bottom().saturating_sub(2), body.width, 1),
    );
    if tend.unreadable > 0 {
        frame.render_widget(
            Paragraph::new(format!(
                "{} couldn't be read; this view may be missing sessions.",
                records(tend.unreadable),
            ))
            .style(palette.muted),
            Rect::new(body.x, body.bottom().saturating_sub(3), body.width, 1),
        );
    }
}

#[cfg(test)]
mod tests;
