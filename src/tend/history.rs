//! One requested history at a time, and its read-only page. Late reads cannot reopen a view.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
};

use crate::{
    Palette,
    think_record::{self, Event, History, Unavailable},
};
use crossterm::event::KeyCode;
use ratatui::{Frame, layout::Rect, text::Line, widgets::Paragraph};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

type Outcome = Result<History, Unavailable>;
type Source = Arc<dyn Fn(&Request) -> Outcome + Send + Sync>;

struct Request {
    root: PathBuf,
    session: PathBuf,
    goal: String,
}

/// One worker survives visits. Requests coalesce; cancelling never starts another read.
pub(crate) struct Reader {
    source: Source,
    epoch: u64,
    wanted: Option<Request>,
    running: Option<(u64, Receiver<Outcome>)>,
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            source: Arc::new(|r| think_record::read(&r.root, &r.session, &r.goal)),
            epoch: 0,
            wanted: None,
            running: None,
        }
    }
}

impl Reader {
    /// Ask only after an explicit history key. A pending read is never duplicated.
    pub(crate) fn request(&mut self, root: PathBuf, session: PathBuf, goal: String) {
        self.epoch += 1;
        self.wanted = Some(Request {
            root,
            session,
            goal,
        });
        self.start();
    }

    /// Forget the requested view; its worker may finish, but its result cannot be shown.
    pub(crate) fn cancel(&mut self) {
        self.epoch += 1;
        self.wanted = None;
    }

    fn start(&mut self) {
        if self.running.is_some() {
            return;
        }
        let Some(request) = self.wanted.take() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let source = self.source.clone();
        let _ = std::thread::Builder::new()
            .name("think-history".into())
            .spawn(move || {
                let _ = tx.send(source(&request));
            });
        self.running = Some((self.epoch, rx));
    }

    /// Poll without waiting. Only the newest requested result can land.
    pub(crate) fn poll(&mut self) -> Option<Outcome> {
        let (epoch, rx) = self.running.as_ref()?;
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(Unavailable {
                message: "The history read stopped unexpectedly.".into(),
                details: "Close history and open it again to retry.".into(),
            }),
        };
        let current = *epoch == self.epoch;
        self.running = None;
        self.start();
        current.then_some(result)
    }
}

/// A frozen reading, with separate scroll positions for history and its two help layers.
pub(crate) struct View {
    failed: bool,
    body: Page,
    help: Page,
    details: Page,
}

impl View {
    pub(crate) fn waiting() -> Self {
        Self {
            failed: false,
            body: Page::plain("Reading Think history…"),
            help: Page::plain(concat!(
                "Read how this goal took shape.\n\n↑↓ / j k or PgUp/PgDn scroll.\n",
                "Home / End moves to the start or end.\nF10 returns to the menu.\n\n",
                "Opening history changes nothing and starts no agents."
            )),
            details: Page::plain("The local history read is still pending."),
        }
    }

    pub(crate) fn finish(&mut self, result: Outcome) {
        self.failed = result.is_err();
        match result {
            Ok(history) => {
                let mut blocks = Vec::new();
                for turn in history.turns {
                    match turn.event {
                        Event::Words { text, .. } => add(&mut blocks, "Your words", text),
                        Event::Asked { question, .. } | Event::ScopeAsked { question, .. } => {
                            add(&mut blocks, "Question", question)
                        }
                        Event::Answered { answer, .. } | Event::ScopeAnswered { answer, .. } => {
                            add(&mut blocks, "Your answer", answer)
                        }
                        Event::Skipped { question, .. } => {
                            add(&mut blocks, "Skipped question", question)
                        }
                        Event::Restored { question, .. } => {
                            add(&mut blocks, "Question restored", question)
                        }
                    }
                }
                add(&mut blocks, "Confirmed goal", history.goal);
                self.body = Page::new(blocks);
                self.details = Page::plain(&history.details);
            }
            Err(error) => {
                self.body = Page::plain(&error.message);
                self.details = Page::plain(&error.details);
            }
        }
    }

    fn page(&mut self, help: bool, how: bool) -> &mut Page {
        if !help {
            &mut self.body
        } else if how {
            &mut self.details
        } else {
            &mut self.help
        }
    }

    pub(crate) fn reset_help(&mut self) {
        self.help.offset = 0;
        self.details.offset = 0;
    }

    pub(crate) fn scroll(&mut self, key: KeyCode, help: bool, how: bool) {
        let page = self.page(help, how);
        page.offset = match key {
            KeyCode::Down | KeyCode::Char('j') => page.offset.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => page.offset.saturating_sub(1),
            KeyCode::PageDown => page.offset.saturating_add(5),
            KeyCode::PageUp => page.offset.saturating_sub(5),
            KeyCode::Home => 0,
            KeyCode::End => usize::MAX,
            _ => page.offset,
        };
    }

    pub(crate) fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        palette: Palette,
        help: bool,
        how: bool,
    ) {
        let title = if !help {
            if self.failed {
                "Think history unavailable"
            } else {
                "Think history"
            }
        } else if how {
            "How it works"
        } else {
            "Right now"
        };
        frame.render_widget(
            Paragraph::new(title).style(palette.jade),
            Rect {
                height: 1.min(area.height),
                ..area
            },
        );
        let content = Rect::new(
            area.x,
            area.y.saturating_add(2),
            area.width,
            area.height.saturating_sub(2),
        );
        let scrolls = self.page(help, how).render(frame, content, palette);
        let keys = if !help {
            if scrolls {
                "↑↓ scroll   esc back   ? help"
            } else {
                "esc back   ? help"
            }
        } else if how {
            "esc help   ?/F1 close"
        } else {
            "esc close   h how it works"
        };
        frame.render_widget(
            Paragraph::new(keys).style(palette.muted),
            Rect::new(area.x, frame.area().height.saturating_sub(1), area.width, 1),
        );
    }
}

fn add(blocks: &mut Vec<(String, bool)>, label: &str, text: String) {
    if !blocks.is_empty() {
        blocks.push((String::new(), false));
    }
    blocks.push((label.into(), true));
    blocks.push((text, false));
}

// Keep stored words unchanged. Escape terminal controls only in the display copy.
fn display(text: &str) -> String {
    text.chars().flat_map(|c| {
        if (c.is_control() && c != '\n') || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            c.escape_default().collect::<Vec<_>>()
        } else { vec![c] }
    }).collect()
}

/// Wrapped rows use usize, and only the visible rows reach the terminal renderer.
struct Page {
    source: Vec<(String, bool)>,
    rows: Vec<(String, bool)>,
    width: u16,
    offset: usize,
}

impl Page {
    fn plain(text: &str) -> Self {
        Self::new(vec![(text.into(), false)])
    }

    fn new(source: Vec<(String, bool)>) -> Self {
        Self {
            source: source
                .into_iter()
                .map(|(s, muted)| (display(&s), muted))
                .collect(),
            rows: vec![],
            width: 0,
            offset: 0,
        }
    }

    fn layout(&mut self, width: u16) {
        self.width = width;
        self.rows.clear();
        for (text, muted) in &self.source {
            for line in text.split('\n') {
                let mut row = String::new();
                let mut used = 0;
                for grapheme in line.graphemes(true) {
                    let cells = grapheme.width();
                    if used + cells > usize::from(width) && !row.is_empty() {
                        self.rows.push((std::mem::take(&mut row), *muted));
                        used = 0;
                    }
                    row.push_str(grapheme);
                    used += cells;
                }
                self.rows.push((row, *muted));
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) -> bool {
        if area.is_empty() {
            return false;
        }
        if self.width != area.width {
            self.layout(area.width);
        }
        self.offset = self
            .offset
            .min(self.rows.len().saturating_sub(usize::from(area.height)));
        let lines: Vec<_> = self
            .rows
            .iter()
            .skip(self.offset)
            .take(usize::from(area.height))
            .map(|(text, muted)| {
                Line::styled(
                    text.as_str(),
                    if *muted { palette.muted } else { palette.ink },
                )
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
        self.rows.len() > usize::from(area.height)
    }
}

#[cfg(test)]
mod tests;
