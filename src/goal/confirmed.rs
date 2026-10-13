//! One rendering of a confirmed goal, shared by think's review and receipt and shape's detail,
//! so the person sees the same words, wrapping and scroll behaviour in each place. It draws only;
//! saving and reading are elsewhere.

use crate::Palette;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Paragraph, Wrap},
};

/// The goal block as a wrapped paragraph, line breaks kept, in ink.
pub fn goal_block(body: String, palette: Palette) -> Paragraph<'static> {
    Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .style(palette.ink)
}

/// What a saved-goal view shows; the caller owns the state, this only borrows it to draw.
pub struct SavedGoal<'a> {
    /// The composed goal block, exactly as saved.
    pub goal: &'a str,
    /// What to do next (copying the continuation prompt), pinned under the goal.
    pub next: Option<&'a str>,
    /// A muted notice at the bottom, wrapped whole so it is never clipped, or empty.
    pub notice: &'a str,
    /// Show "Saved." above the goal: true on think's receipt, false in review and shape.
    pub saved: bool,
}

/// Draws `view` in `area`. The goal scrolls when it doesn't fit, and `scroll` is clamped to
/// the last useful row, so a resize never leaves the goal scrolled out of sight. `next` and
/// the notice never scroll away.
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
    let wrapped = |text: &str| {
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .line_count(area.width) as u16
    };
    let next_rows = view.next.map_or(0, wrapped);
    let notice_rows = wrapped(view.notice).max(1);
    let rows = Layout::vertical([
        Constraint::Length(if view.saved { 2 } else { 0 }),
        Constraint::Min(1),
        Constraint::Length(next_rows + 1 + notice_rows),
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
            Paragraph::new(view.notice)
                .wrap(Wrap { trim: false })
                .style(palette.muted),
            Rect::new(
                rows[2].x,
                rows[2].bottom().saturating_sub(notice_rows),
                rows[2].width,
                notice_rows.min(rows[2].height),
            ),
        );
    }
}
