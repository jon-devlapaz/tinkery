//! Shape: where a saved goal's seed stands, read from its Seed Me session.
//!
//! Opened on one session (from tend's detail) or as a list of goals waiting for a seed (from the
//! menu). It shows the confirmed goal exactly as recorded, then one thing: the current question,
//! the seed draft, a ready seed, or an honest unknown or failure, with one next step. Read-only:
//! it writes nothing, copies nothing, calls no model and starts no agent.

use crate::{
    Palette,
    goal::confirmed::goal_block,
    seed_me::{
        Session, Stage,
        shaping::{self, Shaped, Shaping},
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Shape's screen: an optional list of goals waiting for a seed and, at most, one open session.
pub struct Shape {
    /// `None` when opened on one session; otherwise the list, pending until a read lands.
    list: Option<Option<Result<Vec<Session>, String>>>,
    selected: usize,
    open: Option<Opened>,
    scroll: u16,
}

/// What a key asks of whoever holds Shape.
#[derive(Debug, PartialEq, Eq)]
pub enum Leave {
    /// Stay in Shape.
    Stay,
    /// Leave Shape for wherever it was opened from.
    Back,
}

struct Opened {
    session: PathBuf,
    skill: Option<PathBuf>,
    reading: Reading,
}

enum Reading {
    Waiting(mpsc::Receiver<Result<Shaping, String>>),
    Done(Result<Shaping, String>),
}

impl Shape {
    /// Shape opened on one session; starts its read through the helper beside `skill` off the
    /// paint path, or says plainly that Seed Me isn't configured.
    pub fn session(skill: Option<&Path>, session: &Path) -> Self {
        Self {
            list: None,
            selected: 0,
            open: Some(Opened::start(skill, session)),
            scroll: 0,
        }
    }

    /// Shape opened from the menu: a list of goals waiting for a seed, pending until [`Shape::offer`].
    pub fn list() -> Self {
        Self {
            list: Some(None),
            selected: 0,
            open: None,
            scroll: 0,
        }
    }

    /// Whether the list still waits on a read of the sessions.
    pub fn wants_sessions(&self) -> bool {
        matches!(self.list, Some(None))
    }

    /// The sessions a read found, or why it failed. Only goals waiting for a seed are listed.
    pub fn offer(&mut self, read: Result<&[Session], String>) {
        if self.list.is_some() {
            self.list = Some(Some(read.map(|sessions| {
                sessions
                    .iter()
                    .filter(|s| matches!(s.stage, Stage::GoalSaved | Stage::SeedDrafted))
                    .cloned()
                    .collect()
            })));
            self.selected = 0;
        }
    }

    /// Take a finished read, if one landed. Never blocks.
    pub fn tick(&mut self) {
        if let Some(open) = &mut self.open
            && let Reading::Waiting(receiver) = &open.reading
        {
            match receiver.try_recv() {
                Ok(result) => open.reading = Reading::Done(result),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    open.reading = Reading::Done(Err("The read stopped unexpectedly.".into()))
                }
            }
        }
    }

    /// One key, with `skill` for opening a listed session. Esc closes an open session back to the
    /// list, or leaves Shape. Nothing here writes or runs anything.
    pub fn key(&mut self, key: KeyEvent, skill: Option<&Path>) -> Leave {
        if self.open.is_some() {
            match key.code {
                KeyCode::Esc if self.list.is_some() => {
                    self.open = None;
                    self.scroll = 0;
                }
                KeyCode::Esc => return Leave::Back,
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(5),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(5),
                _ => {}
            }
            return Leave::Stay;
        }
        let listed = match &self.list {
            Some(Some(Ok(sessions))) => sessions.as_slice(),
            _ => &[],
        };
        match key.code {
            KeyCode::Esc => return Leave::Back,
            KeyCode::Down | KeyCode::Char('j') if !listed.is_empty() => {
                self.selected = (self.selected + 1).min(listed.len() - 1)
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Enter if !listed.is_empty() => {
                let path = listed[self.selected].path.clone();
                self.open = Some(Opened::start(skill, &path));
                self.scroll = 0;
            }
            _ => {}
        }
        Leave::Stay
    }

    /// Help for what is on screen now: what to do, then how it works with the paths it reads.
    pub fn help_text(&self) -> String {
        let (now, how) = match &self.open {
            Some(open) => {
                let failure = match &open.reading {
                    Reading::Done(Err(error)) => format!("\n\nWhat went wrong:\n{error}"),
                    _ => String::new(),
                };
                let skill = open.skill.as_ref().map_or_else(
                    || "Seed Me skill: not configured".into(),
                    |s| format!("Seed Me skill: {}", s.display()),
                );
                (
                    "Read where this goal's seed stands. ↑↓ / j k or PgUp/PgDn scroll.",
                    format!(
                        "Session: {}\n{skill}\nShape reads it only through Seed Me's helper (read and status).{failure}",
                        open.session.display()
                    ),
                )
            }
            None => (
                "↑↓ / j k choose a goal waiting for a seed · Enter opens it.",
                match &self.list {
                    Some(Some(Err(error))) => format!("What went wrong:\n{error}"),
                    _ => "Goals whose seed is ready, ended or unknown are in tend.".into(),
                },
            ),
        };
        format!(
            "Right now\n{now}\nEsc back · F10 menu\n\nF1 close\n\n\
             Shape only reads. It writes nothing, copies nothing and starts nothing. \
             Answering Seed Me's questions happens with Seed Me: tend's c copies the prompt.\n\n\
             How it works\n{how}"
        )
    }

    /// Draws Shape in `area`, with its key bar on the terminal's last row.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let keys = match (&self.open, &self.list) {
            (Some(_), _) => "↑↓ scroll   esc back   F10 menu",
            (None, Some(Some(Ok(sessions)))) if !sessions.is_empty() => {
                "↑↓ move   enter open   esc back"
            }
            _ => "esc back",
        };
        frame.render_widget(
            Paragraph::new(keys).style(palette.muted),
            Rect::new(
                2,
                frame.area().height.saturating_sub(1),
                frame.area().width.saturating_sub(4),
                1,
            ),
        );
        match &self.open {
            Some(open) => render_open(frame, &mut self.scroll, open, area, palette),
            None => self.render_list(frame, area, palette),
        }
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, palette: Palette) {
        let message = match &self.list {
            Some(Some(Ok(sessions))) if !sessions.is_empty() => {
                let width = area.width as usize;
                let lines: Vec<Line> = sessions
                    .iter()
                    .enumerate()
                    .map(|(i, session)| {
                        let label = session.stage.label();
                        let mark = if i == self.selected { "› " } else { "  " };
                        let budget = width.saturating_sub(label.width() + 4);
                        let title = clipped(session.goal.lines().next().unwrap_or(""), budget);
                        let gap = width.saturating_sub(2 + title.width() + label.width());
                        let style = if i == self.selected {
                            palette.jade
                        } else {
                            palette.ink
                        };
                        Line::from(vec![
                            Span::styled(format!("{mark}{title}{}", " ".repeat(gap)), style),
                            Span::styled(label, palette.muted),
                        ])
                    })
                    .collect();
                let top = (self.selected as u16).saturating_sub(area.height.saturating_sub(1));
                frame.render_widget(Paragraph::new(lines).scroll((top, 0)), area);
                return;
            }
            Some(Some(Ok(_))) => {
                "No goals are waiting for a seed. Confirm a goal in think, then shape it here."
            }
            Some(Some(Err(_))) => "Your sessions couldn't be read. F1 shows why. Esc goes back.",
            _ => "Reading your sessions. Esc goes back.",
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(palette.muted)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

impl Opened {
    fn start(skill: Option<&Path>, session: &Path) -> Self {
        let (sender, receiver) = mpsc::channel();
        if let Some(skill) = skill {
            let (skill, session) = (skill.to_path_buf(), session.to_path_buf());
            std::thread::spawn(move || {
                let _ = sender.send(shaping::read(&skill, &session));
            });
        }
        Self {
            session: session.to_path_buf(),
            skill: skill.map(Path::to_path_buf),
            reading: Reading::Waiting(receiver),
        }
    }
}

/// The goal scrolls; where the seed stands and the next step stay pinned beneath it.
fn render_open(frame: &mut Frame, scroll: &mut u16, open: &Opened, area: Rect, palette: Palette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (goal, stand) = match &open.reading {
        _ if open.skill.is_none() => (
            None,
            plain(
                "Shape reads sessions through Seed Me, which isn't set up here.",
                "Next: start Tinkery with --shape-pi, --model and --seed-me.",
                palette,
            ),
        ),
        Reading::Waiting(_) => (
            None,
            Text::styled("Reading where this seed stands.", palette.muted),
        ),
        Reading::Done(Err(error)) => (
            None,
            plain(
                &match error
                    .strip_prefix("Seed Me helper refused: ")
                    .and_then(|reason| reason.lines().next())
                    .filter(|reason| !reason.trim().is_empty())
                {
                    Some(reason) => format!("Seed Me refused to read this session: {reason}"),
                    None => "Shape couldn't read this session. F1 shows why.".into(),
                },
                "Next: Esc goes back; F1 shows where the session is.",
                palette,
            ),
        ),
        Reading::Done(Ok(shaping)) => (shaping.goal.as_deref(), standing(&shaping.stage, palette)),
    };
    let stand_rows = Paragraph::new(stand.clone())
        .wrap(Wrap { trim: false })
        .line_count(area.width) as u16;
    let rows =
        Layout::vertical([Constraint::Min(1), Constraint::Length(stand_rows + 1)]).split(area);
    let (goal_area, stand_area) = if goal.is_some() {
        (
            rows[0],
            Rect::new(rows[1].x, rows[1].y + 1, rows[1].width, rows[1].height - 1),
        )
    } else {
        (Rect::default(), area)
    };
    if let Some(goal) = goal {
        let block = goal_block(goal.to_owned(), palette);
        let max = block
            .line_count(goal_area.width)
            .saturating_sub(goal_area.height as usize) as u16;
        *scroll = (*scroll).min(max);
        frame.render_widget(block.scroll((*scroll, 0)), goal_area);
    }
    frame.render_widget(Paragraph::new(stand).wrap(Wrap { trim: false }), stand_area);
}

fn plain(said: &str, next: &str, palette: Palette) -> Text<'static> {
    Text::from(vec![
        Line::styled(said.to_owned(), palette.ink),
        Line::styled(next.to_owned(), palette.ink),
    ])
}

/// Where the seed stands, in plain words, ending with one next step. Seed Me's question is the
/// one thing to act on (jade); its suggestion is a guess (italic).
fn standing(stage: &Shaped, palette: Palette) -> Text<'static> {
    const CONTINUE: &str = "tend's c copies the prompt to continue.";
    match stage {
        Shaped::Question {
            title,
            ask,
            found,
            suggestion,
        } => {
            let mut lines = vec![Line::styled(format!("{title}: {ask}"), palette.jade)];
            for (i, entry) in found.iter().enumerate() {
                let label = if i == 0 {
                    "What I found: "
                } else {
                    "              "
                };
                lines.push(Line::styled(format!("{label}{entry}"), palette.ink));
            }
            if let Some(suggestion) = suggestion {
                lines.push(Line::styled(
                    format!("Suggestion: {suggestion}"),
                    palette.ink.italic(),
                ));
            }
            lines.push(Line::default());
            lines.push(Line::styled(
                format!("Next: answer it with Seed Me; {CONTINUE}"),
                palette.ink,
            ));
            Text::from(lines)
        }
        Shaped::Waiting => plain(
            "Seed Me hasn't asked its next question yet.",
            &format!("Next: shape it into a seed with Seed Me; {CONTINUE}"),
            palette,
        ),
        Shaped::Drafted { revision } => plain(
            &match revision {
                Some(r) => format!("Seed draft {r} is waiting for you to confirm it."),
                None => "A seed draft is waiting for you to confirm it.".into(),
            },
            &format!("Next: confirm the seed draft with Seed Me; {CONTINUE}"),
            palette,
        ),
        Shaped::Ready { revision } => plain(
            &match revision {
                Some(r) => format!("Seed ready: you confirmed {r}, and it still matches."),
                None => "Seed ready: what you confirmed still matches.".into(),
            },
            "Next: the seed is ready for intake.",
            palette,
        ),
        Shaped::Unverified { reason } => plain(
            &format!("The seed says it's confirmed, but Seed Me can't verify it: {reason}."),
            &format!("Next: confirm the current revision with Seed Me; {CONTINUE}"),
            palette,
        ),
        Shaped::Practice => plain(
            "This is a practice session's seed, not a real one.",
            "Next: confirm a goal in think to start a real one.",
            palette,
        ),
        Shaped::Ended { status } => plain(
            &format!("This session has ended ({status}). There's nothing left to shape."),
            "Next: Esc goes back.",
            palette,
        ),
        Shaped::Unknown { what } => plain(
            &format!("Shape can't tell where this seed stands. {what}"),
            &format!("Next: continue with Seed Me; {CONTINUE}"),
            palette,
        ),
    }
}

fn clipped(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    let mut result = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        if used + g.width() + 1 > width {
            break;
        }
        result.push_str(g);
        used += g.width();
    }
    if width > 0 {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests;
