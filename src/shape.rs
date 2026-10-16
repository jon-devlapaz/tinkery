//! Shape: where a saved goal's seed stands, and working through Seed Me's interview to a seed.
//!
//! Opened on one session (from tend's detail) or as a list of goals waiting for a seed (from the
//! menu). It shows the confirmed goal exactly as recorded, then one thing: the current question,
//! the seed draft, a ready seed, or an honest unknown or failure, with one next step. Opening,
//! scrolling and typing write nothing and call no model. Only a key that names it saves an answer
//! and starts the agent step (`F2`), or confirms a seed with the operator's own words (`F3`); see
//! [`step`]. Before a step, Shape shows which folder it will read, and `F4` lets the operator name
//! another for this run. Each finished step is followed by a fresh read, so the screen shows what
//! Seed Me holds.

pub use step::Agent;

mod step;

use crate::{
    Palette,
    goal::confirmed::goal_block,
    seed_me::{
        Session, Stage,
        answering::NOTHING_FOUND,
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
    sync::{Arc, mpsc},
};
use step::{Done, Failure, Running, Why, Work};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
    /// What the running step is doing, in plain words.
    doing: &'static str,
    /// The folder the next step reads.
    folder: Folder,
    /// A folder being typed after `F4`, until Enter uses it or Esc keeps the one shown.
    choosing: Option<String>,
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
            && self
                .open
                .as_ref()
                .is_some_and(|open| open.choosing.is_some() || open.typing().is_some())
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
        format!(
            "Right now\n{now}\nEsc back · F10 menu\n\nF1 close\n\n{promise}\n\nHow it works\n{how}"
        )
    }

    /// Draws Shape in `area`, with its key bar on the terminal's last row.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let keys = match (&self.open, &self.list) {
            (Some(open), _) if open.running.is_some() => "esc stop".to_owned(),
            (Some(open), _) if open.choosing.is_some() => {
                "enter use this folder   esc cancel".to_owned()
            }
            (Some(open), _) => match (&self.agent, open.typing(), &open.reading) {
                (Some(_), Some(t), _) if t.key == KeyCode::F(2) => {
                    "↑↓ scroll   F2 save answer and look into next   esc back   F10 menu".into()
                }
                (
                    Some(_),
                    Some(Typing {
                        act: Act::Confirm(r),
                        ..
                    }),
                    _,
                ) => {
                    format!("↑↓ scroll   F3 confirm seed {r}   esc back   F10 menu")
                }
                (
                    Some(_),
                    None,
                    Reading::Done(Ok(Shaping {
                        stage: Shaped::Waiting,
                        ..
                    })),
                ) => "↑↓ scroll   F2 look into the next question   esc back   F10 menu".into(),
                _ => "↑↓ scroll   esc back   F10 menu".into(),
            },
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
        let agent = self.agent.is_some();
        match &self.open {
            Some(open) => render_open(frame, &mut self.scroll, open, agent, area, palette),
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
                label: "Your answer: ",
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
                label: "Your words: ",
            }),
            _ => None,
        }
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
                "Saving your answer, then looking into the next question. Esc stops the looking.",
            ),
            Act::Look(expected) => (
                Work::Look { expected, folder },
                "Looking into the next question. Esc stops it.",
            ),
            Act::Confirm(revision) => (
                Work::Confirm { revision, words },
                "Confirming the seed with your words.",
            ),
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
            doing: "",
            folder: Folder::Unknown,
            choosing: None,
        }
    }
}

/// The goal (or, when one waits, the seed draft) scrolls; where the seed stands, the operator's
/// words and the next step stay pinned beneath it.
fn render_open(
    frame: &mut Frame,
    scroll: &mut u16,
    open: &Opened,
    agent: bool,
    area: Rect,
    palette: Palette,
) {
    if area.width == 0 || area.height == 0 {
        return;
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
        Reading::Done(Ok(shaping)) if open.running.is_some() => (
            shaping
                .goal
                .as_deref()
                .map(|goal| goal_block(goal.to_owned(), palette)),
            Text::styled(open.doing, palette.muted),
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
                    text: Some(seed), ..
                } => Some(Paragraph::new(seed.clone()).style(palette.ink)),
                _ => shaping
                    .goal
                    .as_deref()
                    .map(|goal| goal_block(goal.to_owned(), palette)),
            };
            let name = open.folder_name();
            let (words, notice) = match &open.choosing {
                Some(typed) => (
                    Some(("Folder: ", typed.as_str())),
                    Some((
                        "",
                        Some(format!(
                            "Next: Enter uses this folder; Esc keeps {}.",
                            name.as_deref().unwrap_or_default()
                        )),
                    )),
                ),
                None => (
                    if agent {
                        open.typing().map(|t| (t.label, open.typed.as_str()))
                    } else {
                        None
                    },
                    open.notice
                        .as_ref()
                        .map(|(said, next)| (said.as_str(), next.clone())),
                ),
            };
            let reads = (agent && open.offers_step()).then(|| match &name {
                Some(name) => format!("The step reads files in {name}. F4 picks another folder."),
                None => "Finding which folder the step reads.".into(),
            });
            (
                top,
                standing(&shaping.stage, agent, reads, words, notice, palette),
            )
        }
    };
    let stand_rows = Paragraph::new(stand.clone())
        .wrap(Wrap { trim: false })
        .line_count(area.width) as u16;
    let rows =
        Layout::vertical([Constraint::Min(1), Constraint::Length(stand_rows + 1)]).split(area);
    let (top_area, stand_area) = if top.is_some() {
        (
            rows[0],
            Rect::new(rows[1].x, rows[1].y + 1, rows[1].width, rows[1].height - 1),
        )
    } else {
        (Rect::default(), area)
    };
    if let Some(block) = top {
        let block = block.wrap(Wrap { trim: false });
        let max = block
            .line_count(top_area.width)
            .saturating_sub(top_area.height as usize) as u16;
        *scroll = (*scroll).min(max);
        frame.render_widget(block.scroll((*scroll, 0)), top_area);
    }
    frame.render_widget(Paragraph::new(stand).wrap(Wrap { trim: false }), stand_area);
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

/// Where the seed stands, in plain words, ending with one next step. Seed Me's question is the
/// one thing to act on (jade); its suggestion is a guess (italic). With an agent, the stages that
/// take words show the operator's words in ink, and a notice from the last key or step comes
/// before the next step, replacing it when it carries its own.
fn standing(
    stage: &Shaped,
    agent: bool,
    reads: Option<String>,
    words: Option<(&str, &str)>,
    notice: Option<(&str, Option<String>)>,
    palette: Palette,
) -> Text<'static> {
    const CONTINUE: &str = "tend's c copies the prompt to continue.";
    let mut lines = Vec::new();
    let next = match stage {
        Shaped::Question {
            title,
            ask,
            found,
            suggestion,
            ..
        } => {
            lines.push(Line::styled(format!("{title}: {ask}"), palette.jade));
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
            if agent {
                "Next: type your answer, then F2 saves it and looks into the next question.".into()
            } else {
                format!("Next: answer it with Seed Me; {CONTINUE}")
            }
        }
        Shaped::Waiting => {
            lines.push(Line::styled(
                "Seed Me hasn't asked its next question yet.",
                palette.ink,
            ));
            if agent {
                "Next: F2 looks into the next question.".into()
            } else {
                format!("Next: shape it into a seed with Seed Me; {CONTINUE}")
            }
        }
        Shaped::Drafted { revision, text } => {
            lines.push(Line::styled(
                match revision {
                    Some(r) => format!("Seed draft {r} is waiting for you to confirm it."),
                    None => "A seed draft is waiting for you to confirm it.".into(),
                },
                palette.ink,
            ));
            if text.as_deref().is_some_and(|t| t.contains(NOTHING_FOUND)) {
                lines.push(Line::styled(
                    "Nothing found: this seed records nothing checked in the repository.",
                    palette.ink,
                ));
            }
            match (agent, revision) {
                (true, Some(r)) => format!(
                    "Next: read the seed, type your own words, then F3 confirms {r}. It is not approval to build."
                ),
                _ => format!("Next: confirm the seed draft with Seed Me; {CONTINUE}"),
            }
        }
        Shaped::Ready { revision } => {
            lines.push(Line::styled(
                match revision {
                    Some(r) => format!("Seed ready: you confirmed {r}, and it still matches."),
                    None => "Seed ready: what you confirmed still matches.".into(),
                },
                palette.ink,
            ));
            "Next: the seed is ready for intake.".into()
        }
        Shaped::Unverified { reason } => {
            lines.push(Line::styled(
                format!("The seed says it's confirmed, but Seed Me can't verify it: {reason}."),
                palette.ink,
            ));
            format!("Next: confirm the current revision with Seed Me; {CONTINUE}")
        }
        Shaped::Practice => {
            lines.push(Line::styled(
                "This is a practice session's seed, not a real one.",
                palette.ink,
            ));
            "Next: confirm a goal in think to start a real one.".into()
        }
        Shaped::Ended { status } => {
            lines.push(Line::styled(
                format!("This session has ended ({status}). There's nothing left to shape."),
                palette.ink,
            ));
            "Next: Esc goes back.".into()
        }
        Shaped::Unknown { what } => {
            lines.push(Line::styled(
                format!("Shape can't tell where this seed stands. {what}"),
                palette.ink,
            ));
            format!("Next: continue with Seed Me; {CONTINUE}")
        }
    };
    if let Some(reads) = reads {
        lines.push(Line::styled(reads, palette.ink));
    }
    if let Some((label, typed)) = words {
        // Each typed line shows as its own row, so a pasted line break is never hidden.
        lines.push(Line::default());
        let typed = format!("{typed}▏");
        for (i, row) in typed.split('\n').enumerate() {
            let lead = if i == 0 {
                label.to_owned()
            } else {
                " ".repeat(label.width())
            };
            lines.push(Line::from(vec![
                Span::styled(lead, palette.muted),
                Span::styled(row.to_owned(), palette.ink),
            ]));
        }
    }
    let noticed = notice.is_some();
    let next = match notice {
        Some((said, own)) => {
            if !said.is_empty() {
                lines.push(Line::default());
                lines.push(Line::styled(said.to_owned(), palette.ink));
            }
            own.unwrap_or(next)
        }
        None => next,
    };
    if matches!(stage, Shaped::Question { .. }) || words.is_some() || noticed {
        lines.push(Line::default());
    }
    lines.push(Line::styled(next, palette.ink));
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
