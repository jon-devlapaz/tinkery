//! Bounded discovery failures in Shape's scrollable F1; paths are display-escaped, never changed.

use super::*;
use crate::seed_me::{MAX_READ_FAILURES, ReadFailure};

impl Shape {
    /// Retain only the bounded diagnostics belonging to the just-offered successful list read.
    pub(crate) fn offer_failures(&mut self, failures: &[ReadFailure]) {
        if matches!(self.list, Some(Some(Ok(_)))) {
            self.failures = failures
                .iter()
                .take(MAX_READ_FAILURES.min(self.unreadable))
                .cloned()
                .collect();
        }
    }

    /// Show the retained failures and disclose omitted ones without inventing a reason.
    pub(super) fn discovery_help(&self) -> String {
        let mut text = format!(
            "Unreadable session records: {}.\nSome session ledgers could not be read. \
             The list may be missing goals.\nInspect the files below, then Esc to close help and Esc to return to the menu; reopen Shape to read again.\n",
            self.unreadable
        );
        for failure in &self.failures {
            text.push_str(&format!(
                "\n{}\n{}\n",
                display_line(&failure.path.to_string_lossy()),
                display_line(&failure.reason)
            ));
        }
        let more = self.unreadable.saturating_sub(self.failures.len());
        if more > 0 {
            let records = if more == 1 { "record" } else { "records" };
            text.push_str(&format!("\n{more} more unreadable {records} not listed.\n"));
        }
        text
    }

    /// A newly opened help view starts at its first row, without changing the discovery read.
    pub(crate) fn reset_help(&mut self) {
        self.help_scroll = 0;
    }

    /// Scroll only help; navigation and scrolling never launch a read or a step.
    pub(crate) fn scroll_help(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down => self.help_scroll = self.help_scroll.saturating_add(1),
            KeyCode::Up => self.help_scroll = self.help_scroll.saturating_sub(1),
            KeyCode::PageDown => self.help_scroll = self.help_scroll.saturating_add(8),
            KeyCode::PageUp => self.help_scroll = self.help_scroll.saturating_sub(8),
            KeyCode::Home => self.help_scroll = 0,
            KeyCode::End => self.help_scroll = u16::MAX,
            _ => {}
        }
    }

    /// Wrapped, scrollable F1 with a fixed close instruction, including capped failure lists.
    pub(crate) fn render_help(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
        let text = self.help_text();
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        let max = paragraph
            .line_count(body.width)
            .saturating_sub(usize::from(body.height));
        self.help_scroll = self.help_scroll.min(max.min(usize::from(u16::MAX)) as u16);
        frame.render_widget(paragraph.scroll((self.help_scroll, 0)), body);
        frame.render_widget(
            Paragraph::new("PgUp/PgDn scroll · F1 close · Esc back").style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}

fn display_line(text: &str) -> String {
    text.chars().flat_map(|c| {
        if c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            c.escape_default().collect::<Vec<_>>()
        } else { vec![c] }
    }).collect()
}

#[cfg(test)]
mod tests;
