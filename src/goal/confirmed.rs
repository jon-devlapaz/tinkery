use crate::Palette;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Paragraph, Wrap},
};

pub fn goal_block(body: String, palette: Palette) -> Paragraph<'static> {
    Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .style(palette.ink)
}

pub struct SavedGoal<'a> {
    pub goal: &'a str,
    pub next: Option<&'a str>,
    pub notice: &'a str,
    pub saved: bool,
}

pub fn render_saved_goal(
    frame: &mut Frame,
    scroll: &mut u16,
    area: Rect,
    palette: Palette,
    view: SavedGoal<'_>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let next_rows = view.next.map_or(0, |next| {
        Paragraph::new(next)
            .wrap(Wrap { trim: false })
            .line_count(area.width) as u16
    });
    let rows = Layout::vertical([
        Constraint::Length(if view.saved { 2 } else { 0 }),
        Constraint::Min(1),
        Constraint::Length(next_rows + 2),
    ])
    .split(area);
    if view.saved {
        frame.render_widget(Paragraph::new("Saved.").style(palette.ink), rows[0]);
    }
    let goal = goal_block(view.goal.to_owned(), palette);
    let max = goal
        .line_count(rows[1].width)
        .saturating_sub(rows[1].height as usize) as u16;
    *scroll = (*scroll).min(max);
    frame.render_widget(goal.scroll((*scroll, 0)), rows[1]);
    if let Some(next) = view.next {
        frame.render_widget(
            Paragraph::new(next)
                .wrap(Wrap { trim: false })
                .style(palette.ink),
            rows[2],
        );
    }
    if !view.notice.is_empty() {
        frame.render_widget(
            Paragraph::new(view.notice).style(palette.muted),
            Rect::new(
                rows[2].x,
                rows[2].bottom().saturating_sub(1),
                rows[2].width,
                1,
            ),
        );
    }
}
