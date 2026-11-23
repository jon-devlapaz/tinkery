//! The help panel and key-bar text that Shape uses beside [`crate::actions`]: wrapped help that
//! scrolls under a fixed footer row, and a key bar that closes up, then uses each key's short
//! words, before it cuts anything off. It knows no phase, decides no wording and runs nothing.

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::Palette;

/// Rows PgUp and PgDn move help.
const PAGE: u16 = 8;

/// How far one help panel is scrolled. A fresh panel starts at its first row; scrolling never
/// passes the end of the text last painted.
#[derive(Default)]
pub(crate) struct Scroll(u16);

impl Scroll {
    /// Back to the first row.
    pub(crate) fn reset(&mut self) {
        self.0 = 0;
    }

    /// Move for `code`: a row for Up and Down, a page for PgUp and PgDn, Home and End to either
    /// end. Other keys do nothing.
    pub(crate) fn key(&mut self, code: KeyCode) {
        self.0 = match code {
            KeyCode::Down => self.0.saturating_add(1),
            KeyCode::Up => self.0.saturating_sub(1),
            KeyCode::PageDown => self.0.saturating_add(PAGE),
            KeyCode::PageUp => self.0.saturating_sub(PAGE),
            KeyCode::Home => 0,
            KeyCode::End => u16::MAX,
            _ => self.0,
        };
    }

    /// Paint `text` wrapped over `area` less its last row, scrolled no further than its end,
    /// and `footer` muted in that last row.
    pub(crate) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        (text, footer): (String, String),
        palette: Palette,
    ) {
        let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        let max = paragraph
            .line_count(body.width)
            .saturating_sub(usize::from(body.height));
        self.0 = self.0.min(max.min(usize::from(u16::MAX)) as u16);
        frame.render_widget(paragraph.scroll((self.0, 0)), body);
        frame.render_widget(
            Paragraph::new(footer).style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}

/// `keys` as one bar no wider than `width`: three spaces apart, else two, else two with each
/// key's short words where it has them. The last key is never dropped.
pub(crate) fn fit(keys: &[(String, Option<&str>)], width: u16) -> String {
    let join = |gap: &str, short: bool| {
        keys.iter()
            .map(|(full, brief)| match brief {
                Some(brief) if short => *brief,
                _ => full.as_str(),
            })
            .collect::<Vec<_>>()
            .join(gap)
    };
    [join("   ", false), join("  ", false)]
        .into_iter()
        .find(|bar| bar.width() <= width as usize)
        .unwrap_or_else(|| join("  ", true))
}
