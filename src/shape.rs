//! Shape: where a saved goal's seed stands, and working through Seed Me's interview to a seed.
//!
//! Opened on one session (from tend's detail) or as a list of goals waiting for a seed (from the
//! menu). It shows the confirmed goal's first line, then one thing: the current question, the seed
//! draft, a ready seed, or an honest unknown or failure, with one next step. `F5` shows the whole
//! goal exactly as recorded and each settled decision, and writes nothing. Opening,
//! scrolling and typing write nothing and call no model. Only a key that names it saves an answer
//! and starts the agent step (`F2`), or confirms a seed with the operator's own words (`F3`); see
//! [`step`]. Before a step, Shape shows which folder it will read, and `F4` lets the operator name
//! another for this run. Each finished step is followed by a fresh read, so the screen shows what
//! Seed Me holds.

pub use step::Agent;

mod step;

use crate::{
    Palette,
    seed_me::{
        Session, Stage,
        answering::NOTHING_FOUND,
        shaping::{self, Shaped, Shaping},
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};
use step::{Done, Failure, Running, Why, Work};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The widest Shape's open view gets, so lines stay readable on a wide terminal.
const WIDTH: u16 = 76;

/// Shape's screen: an optional list of goals waiting for a seed and, at most, one open session.
pub struct Shape {
    /// `None` when opened on one session; otherwise the list, pending until a read lands.
    list: Option<Option<Result<Vec<Session>, String>>>,
    selected: usize,
    open: Option<Opened>,
    scroll: u16,
    agent: Option<Arc<Agent>>,
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
    /// The operator's words being typed: an answer, or the words that confirm a seed.
    typed: String,
    /// What the last key or step did, when it needs saying, and its own next step if any.
    notice: Option<(String, Option<String>)>,
    running: Option<Running>,
    /// What the running step was asked to do.
    doing: Doing,
    /// The folder the next step reads.
    folder: Folder,
    /// A folder being typed after `F4`, until Enter uses it or Esc keeps the one shown.
    choosing: Option<String>,
    /// The "So far" page's scroll while `F5` shows it.
    so_far: Option<u16>,
}

/// What a running step was asked to do, so the screen can say where it is.
#[derive(Clone, Copy)]
enum Doing {
    Answer,
    Look,
    Confirm,
}

/// Which folder the next step reads: not known until a read with an agent lands, then where
/// Think says the goal was made, or the operator's pick.
enum Folder {
    Unknown,
    Known(PathBuf),
}

/// A read of the session, with the folder a step would read when there's an agent. Both land
/// together, so a step is never offered before its folder is shown.
enum Reading {
    NotStarted,
    Waiting(mpsc::Receiver<(Result<Shaping, String>, Option<PathBuf>)>),
    Done(Result<Shaping, String>),
}

impl Shape {
    /// Shape opened on one session; its first [`Shape::tick`] starts its read through the helper
    /// beside `skill` off the paint path, or it says plainly that Seed Me isn't configured.
    pub fn session(skill: Option<&Path>, session: &Path) -> Self {
        Self {
            list: None,
            selected: 0,
            open: Some(Opened::start(skill, session)),
            scroll: 0,
            agent: None,
        }
    }

    /// The same Shape, able to answer and confirm through `agent`. Without one, Shape only reads.
    pub fn with_agent(mut self, agent: Option<Arc<Agent>>) -> Self {
        self.agent = agent;
        self
    }

    /// Whether keys now type words (an answer, seed confirmation or folder), so letters aren't
    /// commands.
    pub fn takes_text(&self) -> bool {
        self.agent.is_some()
            && self.open.as_ref().is_some_and(|open| {
                open.so_far.is_none() && (open.choosing.is_some() || open.typing().is_some())
            })
    }

    /// Bracketed paste: the words go into the folder, answer or confirmation exactly as pasted.
    pub fn paste(&mut self, text: &str) {
        if self.takes_text()
            && let Some(open) = &mut self.open
        {
            match &mut open.choosing {
                Some(folder) => folder.push_str(text),
                None => open.typed.push_str(text),
            }
        }
    }

    /// Shape opened from the menu: a list of goals waiting for a seed, pending until [`Shape::offer`].
    pub fn list() -> Self {
        Self {
            list: Some(None),
            selected: 0,
            open: None,
            scroll: 0,
            agent: None,
        }
    }

    /// Whether the list still waits on a read of the sessions.
    pub fn wants_sessions(&self) -> bool {
        matches!(self.list, Some(None))
    }

    /// The sessions a read found, or why it failed. Only goals waiting for a seed are listed.
    pub fn offer(&mut self, read: Result<&[Session], String>) {
        if self.list.is_some() {
            let listed: Result<Vec<Session>, String> = read.map(|sessions| {
                sessions
                    .iter()
                    .filter(|s| matches!(s.stage, Stage::GoalSaved | Stage::SeedDrafted))
                    .cloned()
                    .collect()
            });
            let count = listed.as_ref().map_or(0, Vec::len);
            self.selected = self.selected.min(count.saturating_sub(1));
            self.list = Some(Some(listed));
        }
    }

    /// Take a finished read or step, if one landed. A finished step starts a fresh read. Never
    /// blocks.
    pub fn tick(&mut self) {
        if let Some(open) = &mut self.open
            && let Some(result) = open.running.as_ref().and_then(Running::poll)
        {
            open.running = None;
            // Typed words stay only when nothing of them was saved.
            if result
                .as_ref()
                .map_or_else(|failure| failure.answer_saved, |_| true)
            {
                open.typed.clear();
            }
            open.notice = said(result);
            open.reading = Reading::NotStarted;
        }
        if let Some(open) = &mut self.open
            && let Reading::NotStarted = open.reading
        {
            open.begin(self.agent.clone());
        }
        if let Some(open) = &mut self.open
            && let Reading::Waiting(receiver) = &open.reading
        {
            match receiver.try_recv() {
                Ok((result, folder)) => {
                    open.reading = Reading::Done(result);
                    if let (Folder::Unknown, Some(folder)) = (&open.folder, folder) {
                        open.folder = Folder::Known(folder);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    open.reading = Reading::Done(Err("The read stopped unexpectedly.".into()))
                }
            }
        }
    }

    /// One key, with `skill` for opening a listed session. Esc stops a running step, or closes an
    /// open session back to the list, or leaves Shape. Only `F2` (save the answer and look into the
    /// next question, or just look) and `F3` (confirm the seed with the typed words) write or call
    /// a model, and only with an agent.
    pub fn key(&mut self, key: KeyEvent, skill: Option<&Path>) -> Leave {
        if let Some(open) = &mut self.open {
            if let Some(scroll) = &mut open.so_far {
                match key.code {
                    KeyCode::Esc | KeyCode::F(5) => open.so_far = None,
                    KeyCode::Down | KeyCode::Char('j') => *scroll = scroll.saturating_add(1),
                    KeyCode::Up | KeyCode::Char('k') => *scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown => *scroll = scroll.saturating_add(5),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(5),
                    _ => {}
                }
                return Leave::Stay;
            }
            if open.running.is_some() {
                if key.code == KeyCode::Esc
                    && let Some(running) = &open.running
                {
                    running.stop();
                }
                return Leave::Stay;
            }
            if let Some(folder) = &mut open.choosing {
                match key.code {
                    KeyCode::Char(c) => folder.push(c),
                    KeyCode::Backspace => {
                        folder.pop();
                    }
                    KeyCode::Enter => {
                        let typed = open.choosing.take().unwrap_or_default();
                        open.notice = open.pick(&typed);
                    }
                    KeyCode::Esc => open.choosing = None,
                    _ => {}
                }
                return Leave::Stay;
            }
            if key.code == KeyCode::F(5) && open.so_far_ready() {
                open.so_far = Some(0);
                return Leave::Stay;
            }
            if let (Some(agent), Some(typing)) = (&self.agent, open.typing()) {
                match key.code {
                    KeyCode::Char(c) => {
                        open.typed.push(c);
                        return Leave::Stay;
                    }
                    KeyCode::Backspace => {
                        open.typed.pop();
                        return Leave::Stay;
                    }
                    code if code == typing.key => {
                        open.act(typing.act, agent);
                        return Leave::Stay;
                    }
                    _ => {}
                }
            }
            if key.code == KeyCode::F(4)
                && self.agent.is_some()
                && open.offers_step()
                && let Folder::Known(folder) = &open.folder
            {
                open.choosing = Some(folder.display().to_string());
                open.notice = None;
                return Leave::Stay;
            }
            if key.code == KeyCode::F(2)
                && let Some(agent) = &self.agent
                && let Reading::Done(Ok(Shaping {
                    stage: Shaped::Waiting,
                    version: Some(expected),
                    ..
                })) = &open.reading
            {
                let expected = *expected;
                open.act(Act::Look(expected), agent);
                return Leave::Stay;
            }
            match key.code {
                KeyCode::Esc if self.list.is_some() => {
                    // A step may have moved this goal on (a confirmed seed leaves the list).
                    self.open = None;
                    self.list = Some(None);
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
        let agent = self.agent.as_deref();
        let back = match &self.open {
            Some(open) if open.so_far_ready() => "Esc back · F5 so far · F10 menu",
            _ => "Esc back · F10 menu",
        };
        let (now, how) = match &self.open {
            Some(open) => {
                let failure = match &open.reading {
                    Reading::Done(Err(error)) => format!("What went wrong:\n{error}\n\n"),
                    _ => String::new(),
                };
                let skill = open.skill.as_ref().map_or_else(
                    || "Seed Me skill: not configured".into(),
                    |s| format!("Seed Me skill: {}", s.display()),
                );
                let model = agent.map_or_else(
                    || "No model is set up, so Shape only reads.".into(),
                    |a| {
                        format!(
                            "Model calls left this run: {} of {}. Each F2 step is one call, with read-only tools.\n\
                             Seed Me's helper makes every change; the model never calls it.",
                            a.remaining(),
                            a.limit()
                        )
                    },
                );
                let reads = match (agent, &open.folder) {
                    (Some(_), Folder::Known(folder)) => {
                        format!("\nThe next step reads files in: {}", folder.display())
                    }
                    _ => String::new(),
                };
                let now = match (agent, open.typing()) {
                    _ if open.choosing.is_some() => {
                        "Type the folder the next step should read. Enter uses it; Esc keeps the one shown."
                    }
                    (Some(_), Some(t)) if t.key == KeyCode::F(2) => {
                        "Type your answer. F2 saves it and looks into the next question (one model call)."
                    }
                    (Some(_), Some(_)) => {
                        "Read the seed, type your own words, then F3 confirms it. It is not approval to build."
                    }
                    _ => "Read where this goal's seed stands. ↑↓ or PgUp/PgDn scroll.",
                };
                (
                    now,
                    format!(
                        "{failure}Session: {}\n{skill}\n{model}{reads}",
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
        let promise = if agent.is_some() {
            "Opening, scrolling and typing write nothing. Only F2 saves an answer and calls a model; \
             only F3 confirms a seed, with your words."
        } else {
            "Shape only reads. It writes nothing, copies nothing and starts nothing. \
             Answering Seed Me's questions happens with Seed Me: tend's c copies the prompt."
        };
        format!("Right now\n{now}\n{back}\n\nF1 close\n\n{promise}\n\nHow it works\n{how}")
    }

    /// Draws Shape in `area`, with its key bar on the terminal's last row. The "So far" page,
    /// like help, fills the body and has no key bar.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        if let Some(Opened {
            so_far: Some(scroll),
            reading: Reading::Done(Ok(shaping)),
            ..
        }) = &mut self.open
        {
            render_so_far(frame, scroll, shaping, area, palette);
            return;
        }
        let agent = self.agent.is_some();
        let overflow = match &self.open {
            Some(open) => render_open(frame, &mut self.scroll, open, agent, area, palette),
            None => {
                self.render_list(frame, area, palette);
                false
            }
        };
        let keys = match (&self.open, &self.list) {
            (Some(open), _) if open.running.is_some() => "esc stop".to_owned(),
            (Some(open), _) if open.choosing.is_some() => {
                "enter use this folder   esc cancel".to_owned()
            }
            (Some(open), _) => {
                let action = match (&self.agent, open.typing(), &open.reading) {
                    (Some(_), Some(t), _) if t.key == KeyCode::F(2) => {
                        Some("F2 save answer and look into next".to_owned())
                    }
                    (
                        Some(_),
                        Some(Typing {
                            act: Act::Confirm(r),
                            ..
                        }),
                        _,
                    ) => Some(format!("F3 confirm seed {r}")),
                    (
                        Some(_),
                        None,
                        Reading::Done(Ok(Shaping {
                            stage: Shaped::Waiting,
                            ..
                        })),
                    ) => Some("F2 look into the next question".to_owned()),
                    _ => None,
                };
                let mut keys = Vec::new();
                if overflow {
                    keys.push("↑↓ scroll".to_owned());
                }
                keys.extend(action);
                if open.so_far_ready() {
                    keys.push("F5 so far".to_owned());
                }
                keys.extend(["esc back".to_owned(), "F10 menu".to_owned()]);
                keys.join("   ")
            }
            (None, Some(Some(Ok(sessions)))) if !sessions.is_empty() => {
                "↑↓ move   enter open   esc back".into()
            }
            _ => "esc back".into(),
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
                            Span::styled(label.to_owned(), palette.muted),
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

/// What a key asks of an open session.
#[derive(Clone)]
enum Act {
    /// Save the typed answer to question `id`, read at `expected`, then look into the next.
    Answer { id: String, expected: u64 },
    /// Look into the next question from `expected`.
    Look(u64),
    /// Confirm seed `revision` with the typed words.
    Confirm(String),
}

/// The words box an open stage offers, and the key that acts on them.
struct Typing {
    key: KeyCode,
    act: Act,
    label: &'static str,
}

impl Opened {
    /// The words box this stage offers, if any: an answer to the current question, or the words
    /// that confirm a seed draft.
    fn typing(&self) -> Option<Typing> {
        let Reading::Done(Ok(shaping)) = &self.reading else {
            return None;
        };
        match (&shaping.stage, shaping.version) {
            (Shaped::Question { id, .. }, Some(expected)) => Some(Typing {
                key: KeyCode::F(2),
                act: Act::Answer {
                    id: id.clone(),
                    expected,
                },
                label: "your answer",
            }),
            (
                Shaped::Drafted {
                    revision: Some(revision),
                    ..
                },
                _,
            ) => Some(Typing {
                key: KeyCode::F(3),
                act: Act::Confirm(revision.clone()),
                label: "your words",
            }),
            _ => None,
        }
    }

    /// Whether `F5` can show the goal and decisions now: once read, and not while a step runs.
    fn so_far_ready(&self) -> bool {
        matches!(self.reading, Reading::Done(Ok(_))) && self.running.is_none()
    }

    /// Whether this stage offers an agent step (a question to answer, or none asked yet).
    fn offers_step(&self) -> bool {
        matches!(
            &self.reading,
            Reading::Done(Ok(Shaping {
                stage: Shaped::Question { .. } | Shaped::Waiting,
                ..
            }))
        )
    }

    /// Use the folder the operator typed for the next steps, or say why not and keep the one shown.
    /// `~/` means the home folder; anything else must be an absolute path to a folder.
    fn pick(&mut self, typed: &str) -> Option<(String, Option<String>)> {
        let typed = typed.trim();
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let path = match typed.strip_prefix("~/") {
            Some(rest) => home().map(|home| home.join(rest)),
            None if typed == "~" => home(),
            None => Some(PathBuf::from(typed)),
        };
        match path
            .filter(|p| p.is_absolute())
            .and_then(|p| p.canonicalize().ok())
            .filter(|p| p.is_dir())
        {
            Some(folder) => {
                self.folder = Folder::Known(folder);
                None
            }
            None => Some((
                format!(
                    "There's no folder at {typed}, so the step still reads {}.",
                    self.folder_name().unwrap_or_default()
                ),
                None,
            )),
        }
    }

    /// The name of the folder the next step reads, once known.
    fn folder_name(&self) -> Option<String> {
        let Folder::Known(folder) = &self.folder else {
            return None;
        };
        Some(folder.file_name().map_or_else(
            || folder.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        ))
    }

    /// Start what `act` asks, or say why not. Refuses empty words, an exhausted budget and a
    /// folder not yet known before anything is written or called.
    fn act(&mut self, act: Act, agent: &Arc<Agent>) {
        let words = self.typed.clone();
        let refuse = |said: &str| Some((said.to_owned(), None));
        let folder = match &self.folder {
            Folder::Known(folder) => Some(folder.clone()),
            _ => None,
        };
        self.notice = match &act {
            Act::Answer { .. } if words.trim().is_empty() => {
                refuse("Type your answer first; nothing was saved.")
            }
            Act::Confirm(_) if words.trim().is_empty() => {
                refuse("Type your words first; nothing was confirmed.")
            }
            Act::Answer { .. } | Act::Look(_) if agent.remaining() == 0 => said(Err(Failure {
                answer_saved: false,
                why: Why::Budget(agent.limit()),
            })),
            Act::Answer { .. } | Act::Look(_) if folder.is_none() => {
                refuse("Shape is still finding which folder the step reads; nothing was saved.")
            }
            _ => None,
        };
        let (Some(skill), None) = (&self.skill, &self.notice) else {
            return;
        };
        let folder = folder.unwrap_or_default();
        let (work, doing) = match act {
            Act::Answer { id, expected } => (
                Work::Answer {
                    id,
                    words,
                    expected,
                    folder,
                },
                Doing::Answer,
            ),
            Act::Look(expected) => (Work::Look { expected, folder }, Doing::Look),
            Act::Confirm(revision) => (Work::Confirm { revision, words }, Doing::Confirm),
        };
        self.doing = doing;
        self.running = Some(Running::start(
            work,
            skill.clone(),
            self.session.clone(),
            agent.clone(),
        ));
    }

    /// Read the session off the paint path and, with an agent, find the folder its step reads.
    fn begin(&mut self, agent: Option<Arc<Agent>>) {
        let (sender, receiver) = mpsc::channel();
        if let Some(skill) = self.skill.clone() {
            let session = self.session.clone();
            std::thread::spawn(move || {
                let read = shaping::read(&skill, &session);
                let folder = match (&agent, &read) {
                    (
                        Some(agent),
                        Ok(Shaping {
                            goal: Some(goal), ..
                        }),
                    ) => Some(agent.folder(&session, goal)),
                    _ => None,
                };
                let _ = sender.send((read, folder));
            });
        }
        self.reading = Reading::Waiting(receiver);
    }

    fn start(skill: Option<&Path>, session: &Path) -> Self {
        Self {
            session: session.to_path_buf(),
            skill: skill.map(Path::to_path_buf),
            reading: Reading::NotStarted,
            typed: String::new(),
            notice: None,
            running: None,
            doing: Doing::Look,
            folder: Folder::Unknown,
            choosing: None,
            so_far: None,
        }
    }
}

/// "So far": the whole goal exactly as confirmed, then each settled decision, with only the
/// operator's own answers marked as theirs. It scrolls when it doesn't fit.
fn render_so_far(
    frame: &mut Frame,
    scroll: &mut u16,
    shaping: &Shaping,
    area: Rect,
    palette: Palette,
) {
    let mut lines = vec![Line::styled(
        "Your goal, as you confirmed it",
        palette.muted,
    )];
    match &shaping.goal {
        Some(goal) => lines.extend(
            goal.split('\n')
                .map(|line| Line::styled(line.to_owned(), palette.ink)),
        ),
        None => lines.push(Line::styled(
            "This session records no confirmed goal.",
            palette.ink,
        )),
    }
    if !shaping.decided.is_empty() {
        lines.push(Line::default());
        lines.push(Line::styled("Decided", palette.muted));
    }
    for decided in &shaping.decided {
        let by = match decided.authority.as_str() {
            "user" => "you",
            "simulated" => "practice",
            other => other,
        };
        let rows: Vec<&str> = decided.answer.split('\n').collect();
        for (i, row) in rows.iter().enumerate() {
            let mut spans = Vec::new();
            if i == 0 {
                spans.push(Span::styled(format!("{}: ", decided.title), palette.muted));
            }
            spans.push(Span::styled((*row).to_owned(), palette.ink));
            if i + 1 == rows.len() {
                spans.push(Span::styled(format!(" ({by})"), palette.muted));
            }
            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::default());
    lines.push(Line::styled("F5 close", palette.ink));
    let page = Paragraph::new(lines).wrap(Wrap { trim: false });
    let max = page
        .line_count(area.width)
        .saturating_sub(area.height as usize) as u16;
    *scroll = (*scroll).min(max);
    frame.render_widget(page.scroll((*scroll, 0)), area);
}

/// The goal's first line (or, when one waits, the seed draft) comes first and scrolls when it
/// doesn't fit; where the seed stands, the operator's words and any next step follow it and stay
/// in view. Returns whether the top scrolls, so the key bar offers scrolling only then.
fn render_open(
    frame: &mut Frame,
    scroll: &mut u16,
    open: &Opened,
    agent: bool,
    area: Rect,
    palette: Palette,
) -> bool {
    let area = Rect {
        width: area.width.min(WIDTH),
        ..area
    };
    if area.width == 0 || area.height == 0 {
        return false;
    }
    let (top, stand) = match &open.reading {
        _ if open.skill.is_none() => (
            None,
            plain(
                "Shape reads sessions through Seed Me, which isn't set up here.",
                "Next: start Tinkery with --shape-pi, --model and --seed-me.",
                palette,
            ),
        ),
        Reading::NotStarted | Reading::Waiting(_) => (
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
        Reading::Done(Ok(shaping)) => {
            let top = match &shaping.stage {
                Shaped::Drafted {
                    revision,
                    text: Some(seed),
                } => Some(seed_view(seed, revision.as_deref(), palette)),
                _ => shaping.goal.as_deref().map(|goal| {
                    Text::styled(goal.lines().next().unwrap_or("").to_owned(), palette.ink)
                }),
            };
            let name = open.folder_name();
            let typing = open.typing().filter(|_| agent);
            let words = |typed| {
                typing.as_ref().map(|t| Words {
                    label: t.label,
                    typed,
                    own_line: true,
                })
            };
            let acts = typing.is_some()
                || agent
                    && matches!(
                        shaping,
                        Shaping {
                            stage: Shaped::Waiting,
                            version: Some(_),
                            ..
                        }
                    );
            let stand = match (&open.running, &open.choosing) {
                (Some(running), _) => {
                    let name = name.as_deref().unwrap_or_default();
                    let said = match (open.doing, running.saved()) {
                        (Doing::Answer, false) => "Saving your answer.".to_owned(),
                        (Doing::Answer, true) => {
                            format!("Answer saved. Looking in {name} for the next question.")
                        }
                        (Doing::Look, _) => format!("Looking in {name} for the next question."),
                        (Doing::Confirm, _) => "Confirming the seed with your words.".to_owned(),
                    };
                    Stand {
                        acts,
                        reads: None,
                        words: words(open.typed.as_str()),
                        notice: None,
                        running: Some(Line::from(vec![
                            Span::styled(said, palette.ink),
                            Span::styled(
                                format!("  {}s", running.elapsed().as_secs()),
                                palette.muted,
                            ),
                        ])),
                    }
                }
                (None, Some(typed)) => Stand {
                    acts,
                    reads: None,
                    words: Some(Words {
                        label: "Folder: ",
                        typed,
                        own_line: false,
                    }),
                    notice: Some((
                        "",
                        Some(format!(
                            "Next: Enter uses this folder; Esc keeps {}.",
                            name.as_deref().unwrap_or_default()
                        )),
                    )),
                    running: None,
                },
                (None, None) => Stand {
                    acts,
                    reads: (agent && open.offers_step()).then(|| match &name {
                        Some(name) => {
                            format!("The step reads files in {name}. F4 picks another folder.")
                        }
                        None => "Finding which folder the step reads.".into(),
                    }),
                    words: words(open.typed.as_str()),
                    notice: open
                        .notice
                        .as_ref()
                        .map(|(said, next)| (said.as_str(), next.clone())),
                    running: None,
                },
            };
            (top, standing(&shaping.stage, stand, palette))
        }
    };
    let rows = |text: &Text<'static>| {
        Paragraph::new(text.clone())
            .wrap(Wrap { trim: false })
            .line_count(area.width) as u16
    };
    let stand_rows = rows(&stand);
    let Some(top) = top else {
        frame.render_widget(Paragraph::new(stand).wrap(Wrap { trim: false }), area);
        return false;
    };
    // One blank row between the top and where the seed stands.
    let top_rows = rows(&top);
    let height = top_rows.min(area.height.saturating_sub(stand_rows + 1).max(1));
    *scroll = (*scroll).min(top_rows - height);
    frame.render_widget(
        Paragraph::new(top)
            .wrap(Wrap { trim: false })
            .scroll((*scroll, 0)),
        Rect { height, ..area },
    );
    frame.render_widget(
        Paragraph::new(stand).wrap(Wrap { trim: false }),
        Rect {
            y: area.y + height + 1,
            height: area.height.saturating_sub(height + 1),
            ..area
        }
        .intersection(area),
    );
    top_rows > height
}

/// The seed draft for reading: a muted `Seed draft rN` header in place of the file's `status:`
/// and `revision:` lines, and headings bold without their `#` marks. Every other line is shown
/// as written. Display only: the file is never changed.
fn seed_view(seed: &str, revision: Option<&str>, palette: Palette) -> Text<'static> {
    let mut lines = vec![
        Line::styled(
            revision.map_or_else(|| "Seed draft".to_owned(), |r| format!("Seed draft {r}")),
            palette.muted,
        ),
        Line::default(),
    ];
    let mut header = true;
    for line in seed.split('\n') {
        header &= !line.trim().is_empty();
        if header && (line.starts_with("status:") || line.starts_with("revision:")) {
            continue;
        }
        lines.push(
            match line.strip_prefix("## ").or_else(|| line.strip_prefix("# ")) {
                Some(heading) => Line::styled(heading.to_owned(), palette.ink.bold()),
                None => Line::styled(line.to_owned(), palette.ink),
            },
        );
    }
    Text::from(lines)
}

/// What a finished step needs to say, and its own next step when the stage's won't do.
fn said(result: Result<Done, Failure>) -> Option<(String, Option<String>)> {
    let Failure { answer_saved, why } = result.err()?;
    let saved = if answer_saved {
        "Your answer is saved. "
    } else {
        ""
    };
    Some(match why {
        Why::Stale if answer_saved => (
            format!(
                "{saved}Another writer changed this session before the next step, so Shape saved nothing more and read it again."
            ),
            None,
        ),
        Why::Stale => (
            "Another writer changed this session first, so nothing was saved. Shape read it again."
                .into(),
            None,
        ),
        Why::Budget(limit) => (
            format!(
                "{saved}No model calls are left: this run's budget is {limit}. Nothing {}was saved.",
                if answer_saved { "more " } else { "" }
            ),
            Some("Next: start Tinkery with --shape-budget CALLS to allow model calls.".into()),
        ),
        Why::Model(error) => (
            format!("{saved}Looking into the next question failed: {error}"),
            None,
        ),
        Why::Refused(reason) => (format!("{saved}{reason}"), None),
        Why::Stopped => (
            format!(
                "{saved}Stopped before the next step finished. The model call it started still counts against this run's budget."
            ),
            None,
        ),
    })
}

fn plain(said: &str, next: &str, palette: Palette) -> Text<'static> {
    Text::from(vec![
        Line::styled(said.to_owned(), palette.ink),
        Line::styled(next.to_owned(), palette.ink),
    ])
}

/// What [`standing`] shows beside the stage.
struct Stand<'a> {
    /// The key bar names this stage's one next action, so no Next line repeats it.
    acts: bool,
    /// Which folder the next step reads, when a step is offered.
    reads: Option<String>,
    /// The operator's words being typed.
    words: Option<Words<'a>>,
    /// What the last key or step said, and its own next step if any.
    notice: Option<(&'a str, Option<String>)>,
    /// What a running step is doing; everything else then stays, muted.
    running: Option<Line<'static>>,
}

/// A words box: its label, then the typed words, each typed line as its own row.
struct Words<'a> {
    label: &'a str,
    typed: &'a str,
    /// The label on its own line (an answer or seed words), or before the words (a folder).
    own_line: bool,
}

/// Where the seed stands, in plain words. Seed Me's label is muted and its question is the one
/// thing to act on (jade); its suggestion is a guess (italic). The operator's words are in ink.
/// At most one Next line: none when the key bar already names the one next action and nothing
/// was said, and a notice's own next step replaces the stage's. While a step runs, the question and the words
/// stay, muted, under what the step is doing.
fn standing(stage: &Shaped, stand: Stand, palette: Palette) -> Text<'static> {
    const CONTINUE: &str = "tend's c copies the prompt to continue.";
    let Stand {
        acts,
        reads,
        words,
        notice,
        running,
    } = stand;
    let calm = running.is_some();
    let mut lines = Vec::new();
    let next = match stage {
        Shaped::Question {
            title,
            ask,
            found,
            suggestion,
            ..
        } => {
            lines.push(Line::styled(title.clone(), palette.muted));
            lines.push(Line::styled(
                ask.clone(),
                if calm { palette.muted } else { palette.jade },
            ));
            if !calm {
                lines.push(Line::default());
                if found.is_empty() {
                    lines.push(Line::styled(
                        "Nothing found: this question records nothing checked in the repository.",
                        palette.ink,
                    ));
                }
                for (i, entry) in found.iter().enumerate() {
                    // A label per entry, not a hanging indent: wrapped lines return to the margin.
                    let label = if i == 0 {
                        "What I found: "
                    } else {
                        "Also found: "
                    };
                    lines.push(Line::styled(format!("{label}{entry}"), palette.ink));
                }
                if let Some(suggestion) = suggestion {
                    lines.push(Line::styled(
                        format!("Suggestion: {suggestion}"),
                        palette.ink.italic(),
                    ));
                }
            }
            Some(if acts {
                "Next: type your answer, then F2 saves it and looks into the next question.".into()
            } else {
                format!("Next: answer it with Seed Me; {CONTINUE}")
            })
        }
        Shaped::Waiting => {
            if !calm {
                lines.push(Line::styled(
                    "Seed Me hasn't asked its next question yet.",
                    palette.ink,
                ));
            }
            Some(if acts {
                "Next: F2 looks into the next question.".into()
            } else {
                format!("Next: shape it into a seed with Seed Me; {CONTINUE}")
            })
        }
        Shaped::Drafted { revision, text } => {
            // When the draft itself is shown, its header already says this.
            if text.is_none() {
                lines.push(Line::styled(
                    match revision {
                        Some(r) => format!("Seed draft {r} is waiting for you to confirm it."),
                        None => "A seed draft is waiting for you to confirm it.".into(),
                    },
                    palette.ink,
                ));
            }
            if !calm && text.as_deref().is_some_and(|t| t.contains(NOTHING_FOUND)) {
                lines.push(Line::styled(
                    "Nothing found: this seed records nothing checked in the repository.",
                    palette.ink,
                ));
            }
            Some(match (acts, revision) {
                (true, Some(r)) => format!("Next: type your own words, then F3 confirms {r}."),
                _ => format!("Next: confirm the seed draft with Seed Me; {CONTINUE}"),
            })
        }
        Shaped::Ready { revision } => {
            lines.push(Line::styled(
                match revision {
                    Some(r) => format!("Seed ready: you confirmed {r}, and it still matches."),
                    None => "Seed ready: what you confirmed still matches.".into(),
                },
                palette.ink,
            ));
            Some("Next: the seed is ready for intake.".into())
        }
        Shaped::Unverified { reason } => {
            lines.push(Line::styled(
                format!("The seed says it's confirmed, but Seed Me can't verify it: {reason}."),
                palette.ink,
            ));
            Some(format!(
                "Next: confirm the current revision with Seed Me; {CONTINUE}"
            ))
        }
        Shaped::Practice => {
            lines.push(Line::styled(
                "This is a practice session's seed, not a real one.",
                palette.ink,
            ));
            Some("Next: confirm a goal in think to start a real one.".into())
        }
        Shaped::Ended { status } => {
            lines.push(Line::styled(
                format!("This session has ended ({status}). There's nothing left to shape."),
                palette.ink,
            ));
            Some("Next: Esc goes back.".into())
        }
        Shaped::Unknown { what } => {
            lines.push(Line::styled(
                format!("Shape can't tell where this seed stands. {what}"),
                palette.ink,
            ));
            Some(format!("Next: continue with Seed Me; {CONTINUE}"))
        }
    };
    let gap = |lines: &mut Vec<Line<'static>>| {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
    };
    if let Some(reads) = reads {
        gap(&mut lines);
        lines.push(Line::styled(reads, palette.ink));
    }
    let worded = words.is_some();
    if let Some(Words {
        label,
        typed,
        own_line,
    }) = words
    {
        let style = if calm { palette.muted } else { palette.ink };
        gap(&mut lines);
        if own_line {
            lines.push(Line::styled(label.to_owned(), style));
        }
        let typed = if calm {
            typed.to_owned()
        } else {
            format!("{typed}▏")
        };
        for (i, row) in typed.split('\n').enumerate() {
            let lead = match (own_line, i) {
                (true, _) => String::new(),
                (false, 0) => label.to_owned(),
                (false, _) => " ".repeat(label.width()),
            };
            lines.push(Line::from(vec![
                Span::styled(lead, palette.muted),
                Span::styled(row.to_owned(), style),
            ]));
        }
        if acts && !calm && matches!(stage, Shaped::Drafted { .. }) {
            lines.push(Line::default());
            lines.push(Line::styled(
                "Confirming the seed is not approval to build.",
                palette.ink,
            ));
        }
    }
    if let Some(running) = running {
        gap(&mut lines);
        lines.push(running);
        return Text::from(lines);
    }
    let noticed = notice.is_some();
    let next = match notice {
        Some((said, own)) => {
            if !said.is_empty() {
                gap(&mut lines);
                lines.push(Line::styled(said.to_owned(), palette.ink));
            }
            own.or(next)
        }
        // The key bar already names the one next action, unless something needs saying first.
        None => next.filter(|_| !acts),
    };
    if let Some(next) = next {
        if matches!(stage, Shaped::Question { .. }) || worded || noticed {
            gap(&mut lines);
        }
        lines.push(Line::styled(next, palette.ink));
    }
    Text::from(lines)
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
