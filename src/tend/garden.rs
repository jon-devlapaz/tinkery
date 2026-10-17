//! Tend's garden framing: selected-line needs, read notes and explicit refusals.
//! Graf owns the graph drawing and pointer geometry; this preserves Tend's quiet footer.

use super::{Tend, records};
use crate::Palette;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) fn render(
    frame: &mut Frame,
    tend: &mut Tend,
    body: Rect,
    palette: Palette,
    now: Option<u64>,
) {
    tend.sync_graph(body);
    let selected = format!("{:?}", tend.sessions[tend.selected].path);
    let source_index = (0..tend.sessions.len())
        .find(|&i| tend.source_marked(i))
        .unwrap_or(tend.selected);
    let source = format!("{:?}", tend.sessions[source_index].path);
    let linking = tend.is_linking();
    tend.graph.draw(frame, palette, &selected, &source, linking);
    let (goal, status) = tend.selected_line(usize::from(body.width), now);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(goal, palette.ink),
            Span::styled(status, palette.muted),
        ])),
        Rect::new(body.x, body.bottom().saturating_sub(2), body.width, 1),
    );
    render_note(frame, tend, body, palette);
    frame.render_widget(
        Paragraph::new(&*tend.notice).style(palette.muted),
        Rect::new(body.x, body.bottom().saturating_sub(1), body.width, 1),
    );
}

pub(super) fn render_note(frame: &mut Frame, tend: &Tend, body: Rect, palette: Palette) {
    let mut notes = Vec::new();
    if tend.links.is_err() && tend.notice != super::linking::UNAVAILABLE {
        notes.push("Links couldn't be read. F1 then h for details.".into());
    }
    if tend.ended > 0 && (!tend.sessions.is_empty() || tend.repo_empty_message().is_some()) {
        notes.push(format!("{} ended not shown", tend.ended));
    }
    if tend.unreadable > 0 {
        notes.push(format!(
            "{} couldn't be read; this view may be missing sessions.",
            records(tend.unreadable),
        ));
    }
    frame.render_widget(
        Paragraph::new(notes.join(" · ")).style(palette.muted),
        Rect::new(body.x, body.bottom().saturating_sub(3), body.width, 1),
    );
}

#[cfg(test)]
mod tests;
