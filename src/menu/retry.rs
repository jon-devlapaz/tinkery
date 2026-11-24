//! Taste hands one reviewed rejection to Shape through the menu. Opening reads the old
//! result. Saving the displayed attempt is separate from drafting, confirmation and starting.

use super::*;
use crate::{
    process::Task,
    taste::{Rejected, Reviewed},
};

/// A review can be cancelled until a save begins; a partial save never repeats automatically.
pub(super) struct Review {
    rejected: Rejected,
    reading: Option<Task<Reviewed>>,
    reviewed: Option<Reviewed>,
    saving: Option<Task<PathBuf>>,
    notice: String,
    scroll: u16,
    page: u16,
    help: bool,
}

impl Review {
    pub(super) fn new(rejected: Rejected) -> Self {
        let source = rejected.clone();
        Self {
            rejected,
            reading: Some(Task::start(move |stop| source.review(stop))),
            reviewed: None,
            saving: None,
            notice: String::new(),
            scroll: 0,
            page: 1,
            help: false,
        }
    }
}

impl Visit for Review {
    /// Nothing but a plain key acts, and nothing acts while a save runs. Esc returns to Taste;
    /// only F2 saves, outside help, for a human, after a clean read.
    fn key(&mut self, key: KeyEvent, kept: &mut Kept) -> Outcome {
        if !key.modifiers.is_empty() || self.saving.is_some() {
            return Outcome::Stay;
        }
        let press = key.kind == KeyEventKind::Press;
        match key.code {
            KeyCode::F(1) | KeyCode::Char('?') if press => {
                self.help = !self.help;
                self.scroll = 0;
            }
            KeyCode::Esc if self.help => self.help = false,
            KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(self.page),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page),
            KeyCode::Esc if press => return Outcome::Back,
            KeyCode::F(10) if press => return Outcome::Home,
            KeyCode::F(2)
                if press && !self.help && !kept.launch.agent && self.notice.is_empty() =>
            {
                if let Some(review) = self.reviewed.clone() {
                    let Some(root) = kept.sessions_root.clone() else {
                        self.notice = "No session storage is configured. Name a --seed-session-root and reopen.".into();
                        return Outcome::Stay;
                    };
                    let source = self.rejected.clone();
                    self.saving = Some(Task::start(move |stop| source.save(&review, &root, stop)));
                }
            }
            _ => {}
        }
        Outcome::Stay
    }

    /// A saved attempt opens Shape on its new session, in place of Taste and this review.
    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        if let Some(result) = self.reading.as_ref().and_then(Task::poll) {
            self.reading = None;
            match result {
                Ok(read) => self.reviewed = Some(read),
                Err(why) => self.notice = why,
            }
        }
        if let Some(result) = self.saving.as_ref().and_then(Task::poll) {
            self.saving = None;
            match result {
                Ok(session) => return to(Open::Saved(session)),
                Err(why) => self.notice = why,
            }
        }
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, kept: &Kept) {
        let area = visits::titled(frame, "tinkery › shape", palette);
        let (human, help) = (!kept.launch.agent, self.help);
        let body = Rect {
            height: area.height.saturating_sub(1),
            ..area
        };
        self.page = crate::page(body.height);
        let mut text = if help {
            "Review the previous goal, rejection, and starting commit. Saving creates a fresh Shape session and isolated checkout. Confirming the new Tink happens later in Shape.".into()
        } else if let Some(r) = &self.reviewed {
            let decisions = if r.prior.decisions.is_empty() {
                "None recorded.".into()
            } else {
                r.prior.decisions.join("\n")
            };
            let heading = if self.rejected.pinned() {
                "Starting commit"
            } else {
                "Earlier rejection did not record a commit.\nCurrent checked commit"
            };
            format!(
                "Confirmed goal\n{}\n\nYour rejection\n{}\n\n{heading}\n{}\n\nPrior decisions (provisional)\n{}\n\nSaving creates a Shape session and detached checkout under {}.",
                r.prior.goal,
                self.rejected.words(),
                self.rejected.head(),
                decisions,
                r.worktrees.display()
            )
        } else if self.reading.is_some() {
            "Reading the rejected result…".into()
        } else {
            String::new()
        };
        if !self.notice.is_empty() {
            text.push('\n');
            text.push_str(&self.notice);
        }
        // Preserve original lines and spaces; never emit terminal controls from source text.
        let lines: Vec<Line> = text
            .split('\n')
            .map(|s| Line::raw(crate::shown_line(s)))
            .collect();
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        let max = paragraph
            .line_count(body.width)
            .saturating_sub(body.height.into());
        self.scroll = self.scroll.min(max.min(usize::from(u16::MAX)) as u16);
        frame.render_widget(paragraph.scroll((self.scroll, 0)), body);
        let bar = if help {
            "? / Esc close help"
        } else if self.saving.is_some() {
            "Saving attempt…"
        } else if !human {
            "Only a human can save this attempt.   Esc cancel"
        } else if self.reviewed.is_some() && self.notice.is_empty() {
            if self.rejected.pinned() {
                "F2 save attempt   ↑↓/PgDn read   Esc cancel"
            } else {
                "F2 use current checked commit and save attempt   ↑↓/PgDn read   Esc cancel"
            }
        } else {
            "↑↓/PgDn read   Esc cancel"
        };
        frame.render_widget(
            Paragraph::new(bar).style(palette.muted),
            Rect::new(area.x, body.bottom(), area.width, 1),
        );
    }
}
