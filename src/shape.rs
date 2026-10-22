//! Shape: where a saved goal's seed stands, and working through its questions to a seed.
//!
//! Opened on one session (from tend's detail) or as a list of goals waiting for a seed (from the
//! menu). Incomplete lists retain the unreadable count and never claim there are no waiting goals.
//! It shows the confirmed goal's first line, then one thing: the current question, the seed
//! draft, a ready seed, or an honest unknown or failure, with one next step. `F5` shows the whole
//! goal exactly as recorded, settled decisions, open questions and their waits, and current findings, without writing. Opening,
//! scrolling and typing write nothing and call no model. Only a key that names it saves an answer
//! and starts the agent step (`F2`, `F11` defer, `F12` delegate), or confirms a seed with the
//! operator's own words (`F3`); see [`step`]. A flagged node comes first: `F6 keep with reason`
//! revalidates it without changing its answer; eligible operator decisions also offer revision.
//! The ledger alone owns these rules. Revision saves then looks, asking recorded dependents before
//! any model call; keeping calls no model. Before a step, Shape shows its folder, and `F4` names
//! another for this run. Ending a session for good takes the same key twice: `F6` completes one
//! whose seed is ready, `F8` stops one still being shaped; Esc between them keeps it open. Each
//! finished step or ending is followed by a fresh read, so the screen shows what the ledger holds.
//! `F3` waits until the whole draft has been on screen. Under an agent launch declaration Shape
//! only reads: Tinkery would record its every change as the operator's.

pub use step::Agent;

mod discovery_help;
mod forms;
mod step;
mod words;

use crate::seed_me::ledger::AnswerAction;
use crate::{
    Palette,
    seed_me::{
        Session, Stage,
        answering::{self, Assumption, Ending, NOTHING_FOUND, OBJECTION, PROBE, Refusal},
        shaping::{self, Shaped, Shaping},
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use forms::Form;
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
use words::Words;

/// The widest Shape's open view gets, so lines stay readable on a wide terminal.
const WIDTH: u16 = 76;
/// Why an agent launch without named authorization only reads.
const HELD: &str =
    "Agent operation needs --agent-confirmation FILE. Shape only reads; Esc goes back.";
/// F3 before the whole seed draft has been on screen.
const READ_FIRST: &str = "PgDn to 'End of seed' first. Nothing was confirmed.";
/// Refusals about the typed words; changing the words clears them.
const WORD_REFUSALS: [&str; 4] = [
    "Type your answer first; nothing was saved.",
    "Put your confirming words on one line; nothing was confirmed.",
    "Type your words first; nothing was confirmed.",
    "Type why you're keeping this answer first; nothing was saved.",
];

/// Shape's screen: an optional list of goals waiting for a seed and, at most, one open session.
pub struct Shape {
    /// `None` when opened on one session; otherwise the list, pending until a read lands.
    list: Option<Option<Result<Vec<Session>, String>>>,
    /// Session records discovery could not read; they may hide goals from the list.
    unreadable: usize,
    failures: Vec<crate::seed_me::ReadFailure>,
    help_scroll: u16,
    selected: usize,
    open: Option<Opened>,
    scroll: u16,
    /// Rows of reading on screen at the last draw, for PgUp/PgDn.
    page: u16,
    agent: Option<Arc<Agent>>,
    /// An agent launch without named confirmation context: Shape only reads.
    held: bool,
    copy: Option<String>,
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
    confirmation: Option<crate::think_record::GoalConfirmation>,
    session: PathBuf,
    reading: Reading,
    /// The operator's words being typed: an answer, or the words that confirm a seed.
    typed: Words,
    /// A deferral or revision form; cancelling restores the untouched answer/reason editor.
    form: Option<Form>,
    /// Optional unsaved teach-back; cancelling restores the confirmation words untouched.
    teach_back: Option<Words>,
    /// What the last key or step did, when it needs saying, and its own next step if any.
    notice: Option<(String, Option<String>)>,
    running: Option<Running>,
    /// What the running step was asked to do.
    doing: Doing,
    /// The folder the next step reads.
    folder: Folder,
    /// A folder being typed after `F4`, until F4 uses it or Esc keeps the one shown.
    choosing: Option<Words>,
    /// The "So far" page's scroll while `F5` shows it.
    so_far: Option<u16>,
    /// An ending asked for once, waiting for the same key again or Esc.
    asking: Option<Ending>,
    /// What the step that asked the current question assumed instead of asking; shown with
    /// that question only.
    assumed: Vec<Assumption>,
    /// After `F7`, waiting for the number of the assumption to object to, or Esc.
    objecting: bool,
    /// A hard-to-undo question's suggestion its step didn't record, until `F9` asks for it.
    withheld: Option<String>,
    /// Whether `F9` showed a hard-to-undo question's options and suggestion.
    revealed: bool,
    /// A change without a model (an ending or an objection) the ledger is carrying out.
    writing: Option<(Write, mpsc::Receiver<Result<(), Refusal>>)>,
    /// The exact draft revision whose last line has been on screen; layout changes don't revoke it.
    read_end: Option<Shaped>,
}

/// A change Shape asks of the ledger without a model.
#[derive(Clone)]
enum Write {
    End(Ending),
    Object(String),
    Keep {
        id: String,
        reason: String,
    },
    /// Record the withheld suggestion on question `id`.
    Recommend {
        id: String,
        text: String,
    },
}

/// What a running step was asked to do, so the screen can say where it is.
#[derive(Clone, Copy)]
enum Doing {
    Answer,
    Defer,
    Delegate,
    Revise,
    Look,
    TeachBack,
    Confirm,
}

impl Doing {
    fn saved(self) -> &'static str {
        match self {
            Self::Defer => "Your deferral is saved. ",
            Self::Delegate => "Your delegated answer is saved. ",
            Self::Revise => "Your revised answer is saved. ",
            Self::TeachBack => "Your teach-back is saved. ",
            _ => "Your answer is saved. ",
        }
    }
}

/// Which folder the next step reads: not known until a read with an agent lands, then where
/// Think says the goal was made, or the operator's pick.
enum Folder {
    Unknown,
    Known(PathBuf),
}

/// A read of the session, with the folder a step would read when there's an agent. Both land
/// together, so a step is never offered before its folder is shown.
#[expect(
    clippy::large_enum_variant,
    reason = "Shape holds one reading at a time; boxing would only add indirection"
)]
enum Reading {
    NotStarted,
    Waiting(mpsc::Receiver<(Result<Shaping, String>, Option<PathBuf>)>),
    Done(Result<Shaping, String>),
}

impl Shape {
    /// Shape opened on one session; its first [`Shape::tick`] reads the ledger off the paint
    /// path, or it says plainly that Shape isn't configured.
    pub fn session(session: &Path) -> Self {
        Self {
            list: None,
            unreadable: 0,
            failures: Vec::new(),
            help_scroll: 0,
            selected: 0,
            open: Some(Opened::start(session)),
            scroll: 0,
            page: 1,
            agent: None,
            held: false,
            copy: None,
        }
    }

    /// The same Shape, able to answer and confirm through `agent`. Without one, it cannot save
    /// answers, confirm seeds or call a model; verified intake copying and ending remain available.
    pub fn with_agent(mut self, agent: Option<Arc<Agent>>) -> Self {
        self.agent = agent.filter(|_| !self.held);
        self
    }

    /// The same Shape without the named context required for agent operation: it only reads.
    /// Answering, confirming and ending are held, and explain what is missing.
    pub fn held(mut self, held: bool) -> Self {
        self.held = held;
        if held {
            self.agent = None;
        }
        self
    }

    /// Take the requested intake line once; the terminal owns OSC 52, never execution.
    pub fn take_copy_request(&mut self) -> Option<String> {
        self.copy.take()
    }

    /// Report the terminal write, not host clipboard acceptance, with retryable feedback.
    pub fn copy_result(&mut self, sent: bool) {
        if let Some(open) = &mut self.open {
            open.notice = Some((
                if sent {
                    "Copied."
                } else {
                    "Couldn't copy; try again."
                }
                .into(),
                None,
            ));
        }
    }

    /// Whether keys now type words (an answer, seed confirmation or folder), so letters aren't
    /// commands. Never while an ending or objection asks, or the helper carries one out.
    pub fn takes_text(&self) -> bool {
        self.agent.is_some()
            && self.open.as_ref().is_some_and(|open| {
                open.so_far.is_none()
                    && open.running.is_none()
                    && open.asking.is_none()
                    && !open.objecting
                    && open.writing.is_none()
                    && (open.choosing.is_some() || open.typing().is_some())
            })
    }

    /// Bracketed paste: the words go into the folder, answer or confirmation exactly as pasted.
    pub fn paste(&mut self, text: &str) {
        if self.takes_text()
            && let Some(open) = &mut self.open
        {
            match (&mut open.choosing, &mut open.form) {
                (Some(folder), _) => folder.insert(text),
                (_, Some(form)) => form.words().insert(text),
                (None, None) if open.teach_back.is_some() => {
                    open.teach_back.as_mut().unwrap().insert(text)
                }
                (None, None) => {
                    let before = open.typed.text().to_owned();
                    open.typed.insert(text);
                    open.edited(&before);
                }
            }
        }
    }

    /// Shape opened from the menu: a list of goals waiting for a seed, pending until [`Shape::offer`].
    pub fn list() -> Self {
        Self {
            list: Some(None),
            unreadable: 0,
            failures: Vec::new(),
            help_scroll: 0,
            selected: 0,
            open: None,
            scroll: 0,
            page: 1,
            agent: None,
            held: false,
            copy: None,
        }
    }

    /// Whether the list still waits on a read of the sessions.
    pub fn wants_sessions(&self) -> bool {
        matches!(self.list, Some(None))
    }

    /// Supply readable sessions and the count of unreadable records, or the root failure.
    /// Only goals waiting for a seed are listed. Failed reads discard any earlier count.
    pub fn offer(&mut self, read: Result<&[Session], String>, unreadable: usize) {
        if self.list.is_some() {
            self.failures.clear();
            self.help_scroll = 0;
            self.unreadable = if read.is_ok() { unreadable } else { 0 };
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
                if matches!(open.doing, Doing::TeachBack) {
                    open.teach_back = None;
                } else {
                    open.typed.clear();
                    open.form = None;
                }
            }
            let asked = match &result {
                Ok(Done::Asked(asked)) => asked.clone(),
                _ => step::Asked::default(),
            };
            open.assumed = asked.assumed;
            open.withheld = asked.withheld;
            open.revealed = false;
            open.notice = said(result, open.doing);
            open.reading = Reading::NotStarted;
        }
        if let Some(open) = &mut self.open
            && let Some((write, receiver)) = &open.writing
        {
            let write = write.clone();
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(Refusal::Refused("Ending stopped unexpectedly.".into())))
                }
            };
            if let Some(result) = result {
                open.writing = None;
                open.notice = Some(match (result, write) {
                    (Ok(()), Write::End(Ending::Completed)) => {
                        ("Session completed. It's read-only now.".into(), None)
                    }
                    (Ok(()), Write::End(Ending::Stopped)) => (
                        "Session stopped. Its open questions stay recorded; it's read-only now."
                            .into(),
                        None,
                    ),
                    (Ok(()), Write::Object(text)) => {
                        open.assumed.retain(|a| a.text != text);
                        (
                            format!(
                                "Not assumed: {text}. It's an open question now, asked before the seed is drafted."
                            ),
                            None,
                        )
                    }
                    (Ok(()), Write::Keep { .. }) => {
                        open.typed.clear();
                        ("Answer kept with your reason. No model ran.".into(), None)
                    }
                    (Ok(()), Write::Recommend { .. }) => {
                        open.withheld = None;
                        ("Here are the options and my suggestion.".into(), None)
                    }
                    (Err(refusal), _) => (refusal.to_string(), None),
                });
                open.reading = Reading::NotStarted;
            }
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
                    if result.as_ref().ok().map(|read| &read.stage) != open.read_end.as_ref() {
                        open.read_end = None;
                    }
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

    /// One key, opening only the selected session when requested. Esc stops a running step, or closes an
    /// open session back to the list, or leaves Shape. Explicit answer, defer and delegate keys
    /// publish then look, as does saving a revision; keep publishes without a model. `F3` confirms only the seed.
    /// `F6` or `F8` pressed twice ends the session, without a model.
    pub fn key(&mut self, key: KeyEvent) -> Leave {
        if key.kind == KeyEventKind::Release
            || key.kind == KeyEventKind::Repeat && matches!(key.code, KeyCode::F(2..=12))
        {
            return Leave::Stay;
        }
        let page = crate::page(self.page);
        if let Some(open) = &mut self.open {
            if let Some(scroll) = &mut open.so_far {
                match key.code {
                    KeyCode::Esc | KeyCode::F(5) => open.so_far = None,
                    KeyCode::Down | KeyCode::Char('j') => *scroll = scroll.saturating_add(1),
                    KeyCode::Up | KeyCode::Char('k') => *scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown => *scroll = scroll.saturating_add(page),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(page),
                    _ => {}
                }
                return Leave::Stay;
            }
            if self.held && matches!(key.code, KeyCode::F(2 | 3 | 6 | 8 | 11 | 12)) {
                open.notice = Some((HELD.into(), None));
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
                    KeyCode::F(4) if key.modifiers.is_empty() => {
                        let typed = open.choosing.take().unwrap();
                        open.notice = open.pick(typed.text());
                    }
                    KeyCode::Esc => open.choosing = None,
                    KeyCode::PageDown => self.scroll = self.scroll.saturating_add(page),
                    KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
                    _ => folder.key(key),
                }
                return Leave::Stay;
            }
            if open.writing.is_some() {
                return Leave::Stay;
            }
            if let Some(form) = &mut open.form {
                match key.code {
                    KeyCode::Esc => {
                        open.form = None;
                        open.notice = Some(("Nothing was changed.".into(), None));
                    }
                    KeyCode::Tab => form.toggle(),
                    KeyCode::PageDown => self.scroll = self.scroll.saturating_add(page),
                    KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
                    code if code == form.key() && key.modifiers.is_empty() => {
                        if form.advance()
                            && let (Some(agent), Some(typing)) = (&self.agent, open.typing())
                        {
                            open.act(typing.act, agent);
                        }
                    }
                    KeyCode::F(_) => {}
                    _ => form.words().key(key),
                }
                return Leave::Stay;
            }
            if open.teach_back.is_some() && key.code == KeyCode::Esc {
                open.teach_back = None;
                open.notice = Some(("Teach-back cancelled. Nothing was saved.".into(), None));
                return Leave::Stay;
            }
            if key.code == KeyCode::F(7)
                && key.modifiers.is_empty()
                && self.agent.is_some()
                && open.teach_back.is_none()
                && open.offers_teach_back()
            {
                open.teach_back = Some(Words::new(open.typed.text()));
                open.notice = None;
                return Leave::Stay;
            }
            if open.objecting {
                match key.code {
                    KeyCode::Esc => {
                        open.objecting = false;
                        open.notice = Some(("Nothing was changed.".into(), None));
                    }
                    KeyCode::Char(c) if key.modifiers.is_empty() => {
                        if let Some(text) = c
                            .to_digit(10)
                            .and_then(|n| (n as usize).checked_sub(1))
                            .and_then(|i| open.assumed.get(i))
                            .map(|a| a.text.clone())
                        {
                            open.object(text);
                        }
                    }
                    _ => {}
                }
                return Leave::Stay;
            }
            if key.code == KeyCode::F(9) && key.modifiers.is_empty() && open.instinct_first() {
                open.reveal();
                return Leave::Stay;
            }
            if key.code == KeyCode::F(7) && key.modifiers.is_empty() && open.offers_objection() {
                open.objecting = true;
                open.notice = Some((
                    "Which assumption do you object to?".into(),
                    Some("Next: press its number; Esc keeps them all.".into()),
                ));
                return Leave::Stay;
            }
            if let Some(asked) = open.asking {
                match key.code {
                    code if code == end_key(asked) && key.modifiers.is_empty() => open.end(asked),
                    KeyCode::Esc => {
                        open.asking = None;
                        open.notice = Some(("Nothing was ended.".into(), None));
                    }
                    _ => {}
                }
                return Leave::Stay;
            }
            if key.modifiers.is_empty()
                && let Some(ending) = open.offers_end().filter(|e| key.code == end_key(*e))
            {
                open.asking = Some(ending);
                open.notice = Some(asked(ending));
                return Leave::Stay;
            }
            if let Some(agent) = &self.agent
                && key.modifiers.is_empty()
            {
                match (&open.reading, key.code) {
                    (
                        Reading::Done(Ok(Shaping {
                            stage: Shaped::Question { id, gate, .. },
                            version: Some(expected),
                            ..
                        })),
                        KeyCode::F(11),
                    ) => {
                        open.form = Some(Form::defer(id.clone(), *expected, gate.clone()));
                        open.notice = None;
                        return Leave::Stay;
                    }
                    (
                        Reading::Done(Ok(Shaping {
                            stage:
                                Shaped::Review {
                                    id,
                                    revisable: true,
                                    ..
                                },
                            version: Some(expected),
                            ..
                        })),
                        KeyCode::F(2),
                    ) => {
                        open.form = Some(Form::revise(id.clone(), *expected));
                        open.notice = None;
                        return Leave::Stay;
                    }
                    (
                        Reading::Done(Ok(Shaping {
                            stage: Shaped::Question { id, .. },
                            version: Some(expected),
                            ..
                        })),
                        KeyCode::F(12),
                    ) => {
                        let act = Act::Respond {
                            id: id.clone(),
                            expected: *expected,
                            action: AnswerAction::Delegate,
                        };
                        open.act(act, agent);
                        return Leave::Stay;
                    }
                    _ => {}
                }
            }
            if key.code == KeyCode::Char('c')
                && key.modifiers.is_empty()
                && let Some(command) = open.intake()
            {
                self.copy = Some(command.to_owned());
                return Leave::Stay;
            }
            if key.code == KeyCode::F(5) && open.so_far_ready() {
                open.so_far = Some(0);
                return Leave::Stay;
            }
            if key.code == KeyCode::F(3)
                && matches!(
                    &open.reading,
                    Reading::Done(Ok(Shaping {
                        stage: Shaped::Review { .. },
                        ..
                    }))
                )
            {
                open.notice = Some((
                    "Review the flagged answer first; nothing was confirmed.".into(),
                    None,
                ));
                return Leave::Stay;
            }
            if let (Some(agent), Some(typing)) = (&self.agent, open.typing()) {
                match key.code {
                    code if code == typing.key && key.modifiers.is_empty() => {
                        open.act(typing.act, agent);
                        return Leave::Stay;
                    }
                    KeyCode::Enter if matches!(typing.act, Act::Confirm(_)) => {
                        open.notice = Some((
                            "F3 confirms the seed; Enter does not add a line.".into(),
                            None,
                        ));
                        return Leave::Stay;
                    }
                    KeyCode::Esc | KeyCode::F(_) | KeyCode::PageDown | KeyCode::PageUp => {}
                    _ => {
                        if let Some(words) = &mut open.teach_back {
                            words.key(key);
                            return Leave::Stay;
                        }
                        let before = open.typed.text().to_owned();
                        open.typed.key(key);
                        open.edited(&before);
                        return Leave::Stay;
                    }
                }
            }
            if key.code == KeyCode::F(4)
                && self.agent.is_some()
                && open.offers_step()
                && let Folder::Known(folder) = &open.folder
            {
                open.choosing = Some(Words::new(&folder.display().to_string()));
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
                    self.unreadable = 0;
                    self.failures.clear();
                    self.help_scroll = 0;
                    self.scroll = 0;
                }
                KeyCode::Esc => return Leave::Back,
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(page),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
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
                self.open = Some(Opened::start(&path));
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
            Some(open) if open.running.is_some() => "Esc stop · F10 menu",
            Some(open) if open.teach_back.is_some() => "Esc cancel teach-back · F10 menu",
            Some(open) if open.form.is_some() => "Esc cancel form · F10 menu",
            Some(open) if open.so_far_ready() => "Esc back · F5 so far · F10 menu",
            _ => "Esc back · F10 menu",
        };
        let (now, how) = match &self.open {
            Some(open) => {
                let failure = match &open.reading {
                    Reading::Done(Err(error)) => format!("What went wrong:\n{error}\n\n"),
                    _ => String::new(),
                };
                let model = agent.map_or_else(
                    || {
                        if self.held {
                            HELD
                        } else {
                            "No model is set up, so Shape only reads."
                        }
                        .into()
                    },
                    |a| {
                        let step = if open.offers_teach_back() {
                            "Each F7 teach-back comparison is one call, without repository access."
                        } else {
                            "Each F2 step is one call, with read-only tools."
                        };
                        format!(
                            "Model calls left this run: {} of {}. {step}\n\
                             Tinkery saves every change; the model never writes the session.",
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
                    _ if open.running.is_some() => "A step is running. Esc stops it.",
                    _ if open.form.is_some() => {
                        "Edit the form. Enter adds a line; Esc cancels without saving."
                    }
                    _ if open.writing.is_some() => "Tinkery is saving a change.",
                    _ if open.objecting => {
                        "Press the number of the assumption you object to; it becomes an open question. Esc keeps them all."
                    }
                    _ if open.asking == Some(Ending::Completed) => {
                        "F6 again ends this session as completed, for good; Esc keeps it open."
                    }
                    _ if open.asking.is_some() => {
                        "F8 again stops this session for good; Esc keeps it open."
                    }
                    _ if open.choosing.is_some() => {
                        "Type the folder the next step should read. Enter adds a line; F4 uses it; Esc keeps the one shown."
                    }
                    (Some(_), Some(t)) if t.key == KeyCode::F(7) => {
                        "Say what will and won't be built in your own words. F7 saves and compares (one model call); Esc cancels. Mismatches are guesses and never block F3."
                    }
                    (Some(_), Some(t)) if t.key == KeyCode::F(6) => {
                        if open.revisable() {
                            "Type why you're keeping this answer. F6 keeps it unchanged with your reason; no model runs. F2 revise opens a replacement answer."
                        } else {
                            "Type why you're keeping this answer. F6 keeps it unchanged with your reason; no model runs."
                        }
                    }
                    (Some(_), Some(t)) if t.key == KeyCode::F(2) => {
                        concat!(
                            "Type your answer. Enter adds a line. F2 saves it and looks into the next question (one model call). ",
                            "F11 defer asks for a reason and revisit condition; F12 delegate takes the grounded suggestion in one press. ",
                            "Both then look into the next. F5 shows so far; F8 stops the session (twice). PgUp/PgDn scroll the reading."
                        )
                    }
                    (Some(_), Some(_)) => {
                        concat!(
                            "Read the seed, type your confirming words on one line, then F3 confirms it. ",
                            "Enter shows guidance without changing your words. It is not approval to build. ",
                            "F7 teach-back optionally saves your words and compares with the draft (one call). ",
                            "Mismatches never block F3. Without a completed teach-back, F3 records: Skipped teach-back."
                        )
                    }
                    _ if open.intake().is_some() => {
                        "c copies the intake command. Replace <run> with your SDLC run. Paste it yourself; copying runs nothing and grants no approval to build."
                    }
                    _ => "Read where this goal's seed stands. ↑↓ or PgUp/PgDn scroll.",
                };
                let form = open
                    .form
                    .as_ref()
                    .map(|f| format!("\n{}", f.description()))
                    .unwrap_or_default();
                (
                    now,
                    format!(
                        "{failure}{model}{reads}{form}\nSession: {}",
                        open.session.display()
                    ),
                )
            }
            None => (
                "↑↓ / j k choose a goal waiting for a seed · Enter opens it.",
                match &self.list {
                    Some(Some(Err(error))) => format!("What went wrong:\n{error}"),
                    Some(Some(Ok(_))) if self.unreadable > 0 => self.discovery_help(),
                    _ => "Goals whose seed is ready, ended or unknown are in tend.".into(),
                },
            ),
        };
        let ends =
            "F6 (seed ready) or F8 (still shaping), pressed twice, ends the session for good.";
        let promise = if self.held {
            "Shape reads and calls no model. Agent operation needs --agent-confirmation FILE; Esc goes back.".to_owned()
        } else if agent.is_some() {
            format!(
                "Opening, scrolling and typing write nothing. F2 saves an answer, F11 saves a deferral, or F12 delegates the suggestion, then looks into the next; \
                 only F3 confirms a seed, with your words. Revision saves and looks, asking recorded dependents before a model call. {ends}"
            )
        } else {
            format!(
                "Shape reads and calls no model. Only {ends} \
                 To answer here, start Tinkery with a model and budget; tend's c copies a prompt to continue elsewhere."
            )
        };
        let editing = if self.takes_text() {
            "\nArrows, Home/End and Ctrl+Left/Right move; Shift selects; Ctrl+W or Alt+Backspace deletes a word.\n\
             Ctrl+U undoes, Ctrl+R redoes; PgUp/PgDn scroll the reading."
        } else {
            ""
        };
        format!("Right now\n{now}{editing}\n{back}\n\nF1 close\n\nHow it works\n{how}\n\n{promise}")
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
            self.page = area.height;
            render_so_far(frame, scroll, shaping, area, palette);
            return;
        }
        let agent = self.agent.is_some();
        let overflow = match &mut self.open {
            Some(open) => {
                let (overflow, page) = render_open(
                    frame,
                    &mut self.scroll,
                    open,
                    agent,
                    self.held,
                    area,
                    palette,
                );
                self.page = page;
                overflow
            }
            None => {
                self.render_list(frame, area, palette);
                false
            }
        };
        let keys = match (&self.open, &self.list) {
            (Some(open), _) if open.running.is_some() => "esc stop".to_owned(),
            (Some(open), _) if open.writing.is_some() => String::new(),
            (Some(open), _) if open.objecting => {
                "1-9 object to that one   esc keep them all".to_owned()
            }
            (
                Some(Opened {
                    asking: Some(ending),
                    ..
                }),
                _,
            ) => match ending {
                Ending::Completed => "F6 end for good   esc keep it".to_owned(),
                Ending::Stopped => "F8 stop for good   esc keep it".to_owned(),
            },
            (Some(open), _) if open.form.is_some() => {
                format!(
                    "{}   esc cancel   F10 menu",
                    open.form.as_ref().unwrap().bar()
                )
            }
            (Some(open), _) if open.choosing.is_some() => {
                format!(
                    "{}enter newline   F4 use folder   esc cancel",
                    if overflow { "pgup/dn read   " } else { "" }
                )
            }
            (Some(open), _) => {
                let action = match (&self.agent, open.typing(), &open.reading) {
                    (Some(_), Some(t), _) if t.key == KeyCode::F(7) => {
                        Some("F7 save + compare".into())
                    }
                    (Some(_), Some(t), _) if t.key == KeyCode::F(6) => Some(
                        if open.revisable() {
                            "F6 keep with reason   F2 revise"
                        } else {
                            "F6 keep with reason"
                        }
                        .to_owned(),
                    ),
                    (Some(_), Some(t), _) if t.key == KeyCode::F(2) => Some(
                        if open.offers_answers() && open.instinct_first() {
                            "F2 save + look   F11 defer"
                        } else if open.offers_answers() {
                            "F2 save + look   F11 defer   F12 delegate"
                        } else if overflow {
                            "F2 save + look"
                        } else {
                            "F2 save answer and look into next"
                        }
                        .to_owned(),
                    ),
                    (
                        Some(_),
                        Some(Typing {
                            act: Act::Confirm(r),
                            ..
                        }),
                        _,
                    ) if open.read_end.is_some() => Some(format!("F3 confirm seed {r}")),
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
                    keys.push(
                        if agent
                            && open.read_end.is_none()
                            && matches!(
                                open.typing(),
                                Some(Typing {
                                    act: Act::Confirm(_),
                                    ..
                                })
                            )
                        {
                            "pgup/dn read to end"
                        } else if open.typing().is_some() && agent {
                            "pgup/dn read"
                        } else {
                            "↑↓ scroll"
                        }
                        .to_owned(),
                    );
                }
                if let Some(action) = action {
                    keys.extend(action.split("   ").map(str::to_owned));
                }
                if agent && open.offers_teach_back() && open.teach_back.is_none() {
                    keys.push("F7 teach-back".to_owned());
                }
                if open.intake().is_some() {
                    keys.push("c copy intake".to_owned());
                }
                if open.instinct_first() {
                    keys.push("F9 show me".to_owned());
                }
                if open.offers_objection() {
                    keys.push("F7 object".to_owned());
                }
                keys.extend(
                    open.offers_end()
                        .filter(|_| {
                            !self.held
                                && !(agent && (open.offers_answers() || open.offers_teach_back()))
                        })
                        .map(|ending| {
                            match ending {
                                Ending::Completed => "F6 end session",
                                Ending::Stopped => "F8 stop",
                            }
                            .to_owned()
                        }),
                );
                if open.so_far_ready()
                    && !(agent && (open.offers_answers() || open.offers_teach_back()))
                {
                    keys.push("F5 so far".to_owned());
                }
                keys.extend([
                    if open.teach_back.is_some() {
                        "esc cancel"
                    } else {
                        "esc back"
                    }
                    .to_owned(),
                    "F10 menu".to_owned(),
                ]);
                // Esc and F10 are never cut off: a full bar closes up before it runs out of room.
                let bar = keys.join("   ");
                if bar.width() > frame.area().width.saturating_sub(4) as usize {
                    let tight = keys.join("  ");
                    if tight.width() > frame.area().width.saturating_sub(4) as usize {
                        tight
                            .replace("pgup/dn read", "pgup/dn")
                            .replace("F2 save + look", "F2 answer")
                    } else {
                        tight
                    }
                } else {
                    bar
                }
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

    fn render_list(&self, frame: &mut Frame, mut area: Rect, palette: Palette) {
        if self.unreadable > 0 && matches!(&self.list, Some(Some(Ok(_)))) {
            let records = if self.unreadable == 1 {
                "record"
            } else {
                "records"
            };
            let notice = format!(
                "{} {records} couldn't be read. F1 shows why.",
                self.unreadable
            );
            let height = area.height.min(2);
            frame.render_widget(
                Paragraph::new(notice)
                    .style(palette.muted)
                    .wrap(Wrap { trim: false }),
                Rect::new(area.x, area.y, area.width, height),
            );
            area.y += height;
            area.height -= height;
        }
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
            Some(Some(Ok(_))) if self.unreadable > 0 => {
                "Nothing can be listed from the readable sessions. Esc goes back."
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
    Respond {
        id: String,
        expected: u64,
        action: AnswerAction,
    },
    Keep {
        id: String,
    },
    /// Save the typed answer to question `id`, read at `expected`, then look into the next.
    Answer {
        id: String,
        expected: u64,
    },
    /// Look into the next question from `expected`.
    Look(u64),
    /// Compare typed teach-back with the exact displayed draft.
    TeachBack {
        revision: String,
        expected: u64,
        seed: String,
    },
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
        if self.teach_back.is_some()
            && let (
                Shaped::Drafted {
                    revision: Some(revision),
                    text: Some(seed),
                },
                Some(expected),
            ) = (&shaping.stage, shaping.version)
        {
            return Some(Typing {
                key: KeyCode::F(7),
                act: Act::TeachBack {
                    revision: revision.clone(),
                    expected,
                    seed: seed.clone(),
                },
                label: "your teach-back",
            });
        }
        if let Some(form) = &self.form {
            return Some(Typing {
                key: form.key(),
                act: Act::Respond {
                    id: form.id.clone(),
                    expected: form.expected,
                    action: form.action(),
                },
                label: form.label(),
            });
        }
        match (&shaping.stage, shaping.version) {
            (Shaped::Review { id, .. }, Some(_)) => Some(Typing {
                key: KeyCode::F(6),
                act: Act::Keep { id: id.clone() },
                label: "your reason",
            }),
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

    fn offers_teach_back(&self) -> bool {
        matches!(
            &self.reading,
            Reading::Done(Ok(Shaping {
                stage: Shaped::Drafted {
                    revision: Some(_),
                    text: Some(_)
                },
                version: Some(_),
                ..
            }))
        )
    }

    fn offers_answers(&self) -> bool {
        matches!(
            &self.reading,
            Reading::Done(Ok(Shaping {
                stage: Shaped::Question { .. },
                ..
            }))
        )
    }

    fn revisable(&self) -> bool {
        matches!(
            &self.reading,
            Reading::Done(Ok(Shaping {
                stage: Shaped::Review {
                    revisable: true,
                    ..
                },
                ..
            }))
        )
    }

    fn intake(&self) -> Option<&str> {
        match &self.reading {
            Reading::Done(Ok(shaping)) if matches!(shaping.stage, Shaped::Ready { .. }) => {
                shaping.intake.as_deref()
            }
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

    /// How this stage can be ended, with the helper configured: completed once the seed is
    /// ready, stopped while it's still being shaped. Never for an ended or unknown session.
    fn offers_end(&self) -> Option<Ending> {
        let Reading::Done(Ok(shaping)) = &self.reading else {
            return None;
        };
        match shaping.stage {
            Shaped::Ready { ended: false, .. } => Some(Ending::Completed),
            Shaped::Question { .. }
            | Shaped::Review { .. }
            | Shaped::Waiting
            | Shaped::Drafted { .. } => Some(Ending::Stopped),
            _ => None,
        }
    }

    /// Whether `F7` can object to an assumption: the question its step asked is still current.
    fn offers_objection(&self) -> bool {
        !self.assumed.is_empty()
            && matches!(
                &self.reading,
                Reading::Done(Ok(Shaping {
                    stage: Shaped::Question { .. },
                    version: Some(_),
                    ..
                }))
            )
    }

    /// Whether the current question is hard to undo and its options and suggestion are still held
    /// back for the operator's instinct.
    fn instinct_first(&self) -> bool {
        !self.revealed
            && matches!(
                &self.reading,
                Reading::Done(Ok(Shaping {
                    stage: Shaped::Question { hard: true, .. },
                    ..
                }))
            )
    }

    /// Show what a hard-to-undo question held back, recording its withheld suggestion first.
    fn reveal(&mut self) {
        self.revealed = true;
        let id = match &self.reading {
            Reading::Done(Ok(Shaping {
                stage: Shaped::Question { id, .. },
                ..
            })) => id.clone(),
            _ => return,
        };
        match self.withheld.clone() {
            Some(text) => self.write(Write::Recommend { id, text }, "Saving my suggestion."),
            _ => {
                self.notice = Some((
                    "Here are the options. No suggestion was kept for this question.".into(),
                    None,
                ))
            }
        }
    }

    /// End the session through the helper off the paint path; [`Shape::tick`] reports it.
    fn end(&mut self, ending: Ending) {
        self.asking = None;
        self.write(Write::End(ending), "Ending the session.");
    }

    /// Object to assuming `text`, from the version shown.
    fn object(&mut self, text: String) {
        self.objecting = false;
        self.write(Write::Object(text), "Saving your objection.");
    }

    /// Carry out `write` through the helper off the paint path; [`Shape::tick`] reports it.
    fn write(&mut self, write: Write, said: &str) {
        let Reading::Done(Ok(shaping)) = &self.reading else {
            return;
        };
        let expected = shaping.version;
        let session = self.session.clone();
        let (sender, receiver) = mpsc::channel();
        let work = write.clone();
        let confirmation = self.confirmation.clone();
        std::thread::spawn(move || {
            let actor = confirmation.as_ref();
            let result = match (work, expected) {
                (Write::Keep { id, reason }, Some(expected)) => {
                    answering::keep(&session, expected, &id, &reason, actor)
                }
                (Write::End(ending), _) => answering::end(&session, ending, actor),
                (Write::Object(text), Some(expected)) => {
                    answering::object(&session, expected, &text, actor).map(|_| ())
                }
                (Write::Recommend { id, text }, Some(expected)) => {
                    answering::recommend(&session, expected, &id, &text, actor).map(|_| ())
                }
                (Write::Keep { .. } | Write::Object(_) | Write::Recommend { .. }, None) => Err(
                    Refusal::Refused("Tinkery read no version to change from.".into()),
                ),
            };
            let _ = sender.send(result);
        });
        self.writing = Some((write, receiver));
        self.notice = Some((said.into(), None));
    }

    /// Use the folder the operator typed for the next steps, or say why not and keep the one shown.
    /// `~/` means the home folder; anything else must be an absolute path to a folder. The exact
    /// text is tried first, then without surrounding whitespace, so a pasted line break still works.
    fn pick(&mut self, typed: &str) -> Option<(String, Option<String>)> {
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let folder = |typed: &str| {
            match typed.strip_prefix("~/") {
                Some(rest) => home().map(|home| home.join(rest)),
                None if typed == "~" => home(),
                None => Some(PathBuf::from(typed)),
            }
            .filter(|p| p.is_absolute())
            .and_then(|p| p.canonicalize().ok())
            .filter(|p| p.is_dir())
        };
        match folder(typed).or_else(|| folder(typed.trim())) {
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

    /// The words changed from `before`: a refusal about the old words no longer applies.
    fn edited(&mut self, before: &str) {
        if self.typed.text() != before
            && self
                .notice
                .as_ref()
                .is_some_and(|(said, _)| WORD_REFUSALS.contains(&said.as_str()))
        {
            self.notice = None;
        }
    }

    /// Start what `act` asks, or say why not. Refuses empty words, an exhausted budget, a folder
    /// not yet known and a seed not yet read to the end before anything is written or called.
    fn act(&mut self, act: Act, agent: &Arc<Agent>) {
        let words = self
            .teach_back
            .as_ref()
            .unwrap_or(&self.typed)
            .text()
            .to_owned();
        let refuse = |said: &str| Some((said.to_owned(), None));
        let folder = match &self.folder {
            Folder::Known(folder) => Some(folder.clone()),
            _ => None,
        };
        self.notice = match &act {
            Act::TeachBack { .. } if words.trim().is_empty() => {
                refuse("Type your teach-back first; nothing was saved.")
            }
            Act::Keep { .. } if words.trim().is_empty() => refuse(WORD_REFUSALS[3]),
            Act::Answer { .. } if words.trim().is_empty() => refuse(WORD_REFUSALS[0]),
            Act::Confirm(_) if self.read_end.is_none() => refuse(READ_FIRST),
            Act::Confirm(_) if words.contains(['\n', '\r']) => refuse(WORD_REFUSALS[1]),
            Act::Confirm(_) if words.trim().is_empty() => refuse(WORD_REFUSALS[2]),
            Act::TeachBack { .. } | Act::Respond { .. } | Act::Answer { .. } | Act::Look(_)
                if agent.remaining() == 0 =>
            {
                said(
                    Err(Failure {
                        answer_saved: false,
                        why: Why::Budget(agent.limit()),
                    }),
                    Doing::Answer,
                )
            }
            Act::Respond { .. } | Act::Answer { .. } | Act::Look(_) if folder.is_none() => {
                refuse("Shape is still finding which folder the step reads; nothing was saved.")
            }
            _ => None,
        };
        let None = &self.notice else {
            return;
        };
        let folder = folder.unwrap_or_default();
        let (work, doing) = match act {
            Act::Respond {
                id,
                expected,
                action,
            } => {
                let doing = match &action {
                    AnswerAction::Defer { .. } => Doing::Defer,
                    AnswerAction::Delegate => Doing::Delegate,
                    AnswerAction::Revise(_) => Doing::Revise,
                };
                (
                    Work::Respond {
                        id,
                        expected,
                        action,
                        folder,
                    },
                    doing,
                )
            }
            Act::Keep { id } => {
                self.write(Write::Keep { id, reason: words }, "Saving your reason.");
                return;
            }
            Act::Answer { id, expected } => (
                Work::Answer {
                    id,
                    words,
                    expected,
                    folder,
                },
                Doing::Answer,
            ),
            Act::TeachBack {
                revision,
                expected,
                seed,
            } => (
                Work::TeachBack {
                    revision,
                    expected,
                    seed,
                    words,
                },
                Doing::TeachBack,
            ),
            Act::Look(expected) => (Work::Look { expected, folder }, Doing::Look),
            Act::Confirm(revision) => (Work::Confirm { revision, words }, Doing::Confirm),
        };
        self.doing = doing;
        self.running = Some(Running::start(work, self.session.clone(), agent.clone()));
    }

    /// Read the session off the paint path and, with an agent, find the folder its step reads.
    fn begin(&mut self, agent: Option<Arc<Agent>>) {
        self.confirmation = agent.as_ref().and_then(|a| a.confirmation.clone());
        let (sender, receiver) = mpsc::channel();
        {
            let session = self.session.clone();
            std::thread::spawn(move || {
                let read = shaping::read(&session);
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

    fn start(session: &Path) -> Self {
        Self {
            session: session.to_path_buf(),
            confirmation: None,
            reading: Reading::NotStarted,
            typed: Words::new(""),
            form: None,
            teach_back: None,
            notice: None,
            running: None,
            doing: Doing::Look,
            folder: Folder::Unknown,
            choosing: None,
            so_far: None,
            asking: None,
            assumed: Vec::new(),
            objecting: false,
            withheld: None,
            revealed: false,
            writing: None,
            read_end: None,
        }
    }
}

/// "So far": the confirmed goal, settled decisions, open questions and current findings.
/// Only the operator's own answers are marked as theirs. It scrolls when it doesn't fit.
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
    if !shaping.open.is_empty() {
        lines.push(Line::default());
        lines.push(Line::styled("Open questions", palette.muted));
        for question in &shaping.open {
            lines.push(Line::styled(question.clone(), palette.ink));
        }
    }
    if let Shaped::Question { found, .. } | Shaped::Review { found, .. } = &shaping.stage
        && !found.is_empty()
    {
        lines.push(Line::default());
        lines.push(Line::styled("Findings for this question", palette.muted));
        for entry in found {
            lines.push(Line::styled(entry.clone(), palette.ink));
            lines.push(Line::default());
        }
    }
    let page = Paragraph::new(lines).wrap(Wrap { trim: false });
    // How to close stays on the page's last row, even when the rest scrolls.
    let rows = page.line_count(area.width) as u16;
    let height = area.height.saturating_sub(2);
    *scroll = (*scroll).min(rows.saturating_sub(height));
    frame.render_widget(page.scroll((*scroll, 0)), Rect { height, ..area });
    let close = if rows > height {
        "↑↓ scroll · F5 close"
    } else {
        "F5 close"
    };
    frame.render_widget(
        Paragraph::new(close).style(palette.ink),
        Rect {
            y: area.y + rows.min(height) + 1,
            height: 1,
            ..area
        }
        .intersection(area),
    );
}

/// What to read (the goal's first line, or the seed draft when one waits, then where the seed
/// stands) comes first and scrolls when it doesn't fit; the words box, what was said and any next
/// step follow it and stay in view. Returns whether the reading scrolls, so the key bar offers
/// scrolling only then.
/// Draws the open session; returns whether the reading overflows and how many of its rows show.
fn render_open(
    frame: &mut Frame,
    scroll: &mut u16,
    open: &mut Opened,
    agent: bool,
    held: bool,
    area: Rect,
    palette: Palette,
) -> (bool, u16) {
    let area = Rect {
        width: area.width.min(WIDTH),
        ..area
    };
    if area.width == 0 || area.height == 0 {
        return (false, 1);
    }
    let (reading, input) = match &open.reading {
        Reading::NotStarted | Reading::Waiting(_) => (
            Text::styled("Reading where this seed stands.", palette.muted),
            Text::default(),
        ),
        Reading::Done(Err(error)) => (
            plain(
                &match error
                    .strip_prefix("Tinkery refused: ")
                    .and_then(|reason| reason.lines().next())
                    .filter(|reason| !reason.trim().is_empty())
                {
                    Some(reason) => format!("Tinkery refused to read this session: {reason}"),
                    None => "Shape couldn't read this session. F1 shows why.".into(),
                },
                "Next: Esc goes back; F1 shows where the session is.",
                palette,
            ),
            Text::default(),
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
            let words = || typing.as_ref().map(|t| t.label);
            let acts = open.intake().is_some()
                || typing.is_some()
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
                        (Doing::Defer, false) => "Saving your deferral.".into(),
                        (Doing::Delegate, false) => "Saving the delegated suggestion.".into(),
                        (Doing::Revise, false) => {
                            "Revising your answer and reopening dependents.".into()
                        }
                        (Doing::Defer | Doing::Delegate | Doing::Revise, true) => format!(
                            "{}Looking in {name} for the next question.",
                            open.doing.saved()
                        ),
                        (Doing::Look, _) => format!("Looking in {name} for the next question."),
                        (Doing::TeachBack, false) => "Saving your teach-back.".into(),
                        (Doing::TeachBack, true) => {
                            "Teach-back saved. Comparing with the draft.".into()
                        }
                        (Doing::Confirm, _) => "Confirming the seed with your words.".to_owned(),
                    };
                    Stand {
                        acts,
                        reads: None,
                        words: words(),
                        notice: None,
                        running: Some(Line::from(vec![
                            Span::styled(said, palette.ink),
                            Span::styled(
                                format!("  {}s", running.elapsed().as_secs()),
                                palette.muted,
                            ),
                        ])),
                        assumed: &open.assumed,
                        instinct: open.instinct_first(),
                    }
                }
                (None, Some(_)) => Stand {
                    acts,
                    reads: None,
                    words: Some("Folder:"),
                    notice: Some((
                        "",
                        Some(format!(
                            "Next: F4 uses this folder; Esc keeps {}.",
                            name.as_deref().unwrap_or_default()
                        )),
                    )),
                    running: None,
                    assumed: &open.assumed,
                    instinct: open.instinct_first(),
                },
                (None, None) => Stand {
                    acts,
                    reads: (agent && open.offers_step()).then(|| match &name {
                        Some(name) => {
                            format!("The step reads files in {name}. F4 picks another folder.")
                        }
                        None => "Finding which folder the step reads.".into(),
                    }),
                    words: words(),
                    notice: open
                        .notice
                        .as_ref()
                        .map(|(said, next)| (said.as_str(), next.clone()))
                        .or(held.then_some((HELD, None))),
                    running: None,
                    assumed: &open.assumed,
                    instinct: open.instinct_first(),
                },
            };
            let (said, mut input) = standing(shaping, stand, palette);
            if let Some(note) = &shaping.confirmation_note {
                input.lines.push(Line::styled(note.clone(), palette.muted));
            }
            let mut reading = top.unwrap_or_default();
            if matches!(shaping.stage, Shaped::Drafted { .. }) {
                reading.lines.push(Line::default());
                reading
                    .lines
                    .push(Line::styled("Before you confirm", palette.muted));
                for line in &shaping.teach_back {
                    reading
                        .lines
                        .extend(Text::styled(line.clone(), palette.ink).lines);
                }
                for line in &shaping.confirmation {
                    reading
                        .lines
                        .extend(Text::styled(line.clone(), palette.ink).lines);
                }
            }
            let (mut reading, input) = if let Some(form) = &open.form {
                reading.lines.extend(said.lines);
                reading.lines.push(Line::default());
                reading
                    .lines
                    .extend(Text::styled(form.description(), palette.ink).lines);
                if !open.notice.as_ref().is_some_and(|(_, next)| next.is_some()) {
                    input
                        .lines
                        .retain(|line| !line.to_string().starts_with("Next:"));
                }
                (reading, input)
            } else {
                if !reading.lines.is_empty() && !said.lines.is_empty() {
                    reading.lines.push(Line::default());
                }
                reading.lines.extend(said.lines);
                (reading, input)
            };
            if matches!(shaping.stage, Shaped::Drafted { text: Some(_), .. }) {
                reading.lines.push(Line::default());
                reading
                    .lines
                    .push(Line::styled("End of seed", palette.muted));
            }
            (reading, input)
        }
    };
    let rows = |text: &Text<'static>| {
        Paragraph::new(text.clone())
            .wrap(Wrap { trim: false })
            .line_count(area.width) as u16
    };
    let label = if open.choosing.is_some() {
        Some("Folder:")
    } else {
        open.typing().filter(|_| agent).map(|t| t.label)
    };
    let split = label.and_then(|label| input.lines.iter().position(|l| l.to_string() == label));
    let (before, after) = match split {
        Some(i) => (
            Text::from(input.lines[..=i].to_vec()),
            Text::from(input.lines[i + 1..].to_vec()),
        ),
        None => (Text::default(), input),
    };
    let editor = split.map(|_| {
        if let Some(folder) = &mut open.choosing {
            folder
        } else if let Some(teach_back) = &mut open.teach_back {
            teach_back
        } else if let Some(form) = &mut open.form {
            form.words()
        } else {
            &mut open.typed
        }
    });
    let editor_rows = editor.as_ref().map_or(0, |words| words.rows(area.width));
    let (read_rows, before_rows, after_rows) = (rows(&reading), rows(&before), rows(&after));
    let input_rows = before_rows
        .saturating_add(editor_rows)
        .saturating_add(after_rows);
    let gap = u16::from(read_rows > 0 && input_rows > 0);
    // Reserve at least half the body for a long reading. The editor scrolls to its caret;
    // labels and consequences stay outside that viewport, even while a step is running.
    let input_height =
        input_rows
            .min(area.height.saturating_sub(
                gap + read_rows.min(area.height / 2).max(u16::from(read_rows > 0)),
            ));
    let height = read_rows.min(area.height.saturating_sub(input_height + gap));
    *scroll = (*scroll).min(read_rows.saturating_sub(height));
    if open.read_end.is_none()
        && *scroll >= read_rows.saturating_sub(height)
        && let Reading::Done(Ok(shaping)) = &open.reading
        && matches!(shaping.stage, Shaped::Drafted { text: Some(_), .. })
    {
        open.read_end = Some(shaping.stage.clone());
    }
    // Reaching the end answers "read to the end first".
    if open.read_end.is_some()
        && open
            .notice
            .as_ref()
            .is_some_and(|(said, _)| said == READ_FIRST)
    {
        open.notice = None;
    }
    frame.render_widget(
        Paragraph::new(reading)
            .wrap(Wrap { trim: false })
            .scroll((*scroll, 0)),
        Rect { height, ..area },
    );
    let mut input_area = Rect {
        y: area.y + height + gap,
        height: input_height,
        ..area
    }
    .intersection(area);
    let after_height = after_rows.min(input_height.saturating_sub(u16::from(editor.is_some())));
    let before_height =
        before_rows.min(input_height.saturating_sub(after_height + u16::from(editor.is_some())));
    frame.render_widget(
        Paragraph::new(before).wrap(Wrap { trim: false }),
        Rect {
            height: before_height,
            ..input_area
        },
    );
    input_area.y += before_height;
    input_area.height = input_height.saturating_sub(before_height + after_height);
    if let Some(editor) = editor {
        editor.render(frame, input_area, palette, open.running.is_some());
        if label == Some("your words") && editor.text().is_empty() && open.read_end.is_some() {
            frame.render_widget(
                Paragraph::new("a line saying you confirm, then F3").style(palette.muted),
                Rect {
                    height: input_area.height.min(1),
                    ..input_area
                },
            );
        }
    }
    input_area.y += input_area.height;
    input_area.height = after_height;
    frame.render_widget(
        Paragraph::new(after)
            .wrap(Wrap { trim: false })
            .scroll((after_rows.saturating_sub(after_height), 0)),
        input_area,
    );
    (read_rows > height, height)
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
    for line in seed.trim_end_matches('\n').split('\n') {
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
/// The key that asks for `ending`, and then carries it out.
fn end_key(ending: Ending) -> KeyCode {
    match ending {
        Ending::Completed => KeyCode::F(6),
        Ending::Stopped => KeyCode::F(8),
    }
}

/// What the first press of an ending key asks, and how to answer it.
fn asked(ending: Ending) -> (String, Option<String>) {
    match ending {
        Ending::Completed => (
            "End this session as completed? It becomes read-only for good, and tend no longer lists it, so copy the intake command first if you need it.".into(),
            Some("Next: F6 again ends it; Esc keeps it open.".into()),
        ),
        Ending::Stopped => (
            "Stop this session? It becomes read-only for good and tend no longer lists it; its open questions stay recorded.".into(),
            Some("Next: F8 again stops it; Esc keeps it open.".into()),
        ),
    }
}

fn said(result: Result<Done, Failure>, doing: Doing) -> Option<(String, Option<String>)> {
    if result == Ok(Done::Compared) {
        return Some((
            "Teach-back compared. Mismatches never block F3.".into(),
            None,
        ));
    }
    let Failure { answer_saved, why } = result.err()?;
    let saved = if answer_saved { doing.saved() } else { "" };
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
            matches!(doing, Doing::Defer | Doing::Revise).then(|| {
                "Next: Esc cancels this form; open it again from the refreshed question.".into()
            }),
        ),
        Why::Budget(limit) => (
            format!(
                "{saved}No model calls are left: this run's budget is {limit}. Nothing {}was saved.",
                if answer_saved { "more " } else { "" }
            ),
            Some("Next: start Tinkery with --shape-budget CALLS to allow model calls.".into()),
        ),
        Why::Model(error) => (
            format!(
                "{saved}{}: {error}",
                if matches!(doing, Doing::TeachBack) {
                    "Comparing your teach-back failed"
                } else {
                    "Looking into the next question failed"
                }
            ),
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
    /// The editor's label, when this stage takes words.
    words: Option<&'a str>,
    /// What the last key or step said, and its own next step if any.
    notice: Option<(&'a str, Option<String>)>,
    /// What a running step is doing; everything else then stays, muted.
    running: Option<Line<'static>>,
    /// Defaults the current question's step assumed instead of asking.
    assumed: &'a [Assumption],
    /// Whether a hard-to-undo question's options and suggestion still wait for the operator's instinct.
    instinct: bool,
}

/// Where the seed stands, in plain words, as what to read and then what to act with (the words box,
/// what was said and the next step). The question's label is muted and its wording is the one
/// thing to act on (jade); its suggestion is a guess (italic). The operator's words are in ink.
/// At most one Next line: none when the key bar already names the one next action and nothing
/// was said, and a notice's own next step replaces the stage's. While a step runs, the question and the words
/// stay, muted, under what the step is doing.
fn standing(shaping: &Shaping, stand: Stand, palette: Palette) -> (Text<'static>, Text<'static>) {
    const CONTINUE: &str = "tend's c copies the prompt to continue.";
    let Stand {
        acts,
        reads,
        words,
        notice,
        running,
        assumed,
        instinct,
    } = stand;
    let calm = running.is_some();
    let mut lines = Vec::new();
    let mut flag = None;
    let next = match &shaping.stage {
        Shaped::Review {
            title,
            answer,
            found,
            ..
        } => {
            lines.push(Line::styled(
                "Current question: review flagged answer",
                palette.muted,
            ));
            lines.push(Line::styled(format!("Keep {title}?"), palette.jade));
            lines.push(Line::styled(
                format!("Settled answer: {answer}"),
                palette.ink,
            ));
            for reason in found {
                lines.push(Line::styled(reason.clone(), palette.ink));
            }
            lines.push(Line::styled(
                "Confirmation is held until each flagged node is reviewed.",
                palette.muted,
            ));
            Some(if acts {
                "Next: type why, then F6 keeps the answer unchanged with your reason.".into()
            } else {
                format!("Next: review this flagged answer; {CONTINUE}")
            })
        }
        Shaped::Question {
            id,
            title,
            ask,
            found,
            suggestion,
            options,
            against,
            ..
        } => {
            lines.push(Line::styled(title.clone(), palette.muted));
            lines.push(Line::styled(
                ask.clone(),
                if calm { palette.muted } else { palette.jade },
            ));
            if !calm {
                lines.push(Line::default());
                // Tinkery's own probe and objection questions ask for the operator's words, not evidence.
                if found.is_empty() && id != PROBE.0 && title != OBJECTION {
                    lines.push(Line::styled(
                        "Nothing found: this question records nothing checked in the repository.",
                        palette.ink,
                    ));
                }
                if let Some(entry) = found.first() {
                    lines.push(Line::styled(format!("What I found: {entry}"), palette.ink));
                }
                if instinct {
                    lines.push(Line::styled(
                        "This is hard to undo, so what's your instinct? F9 shows the options and my suggestion.",
                        palette.ink,
                    ));
                } else {
                    for option in options {
                        lines.push(Line::styled(option.clone(), palette.ink));
                    }
                    if let Some(against) = against {
                        lines.push(Line::styled(
                            format!("Against my suggestion: {against}"),
                            palette.muted,
                        ));
                    }
                    if let Some(suggestion) = suggestion {
                        lines.push(Line::styled(
                            format!("Suggestion: {suggestion}"),
                            palette.ink.italic(),
                        ));
                    }
                }
                if !assumed.is_empty() {
                    lines.push(Line::default());
                    lines.push(Line::styled(
                        "I'll assume these unless you object:",
                        palette.muted,
                    ));
                    for (i, a) in assumed.iter().enumerate() {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}. {}", i + 1, a.text), palette.ink),
                            Span::styled(format!(" (why: {})", a.why), palette.muted),
                        ]));
                    }
                }
            }
            Some(if acts {
                "Next: type your answer, then F2 saves it and looks into the next question.".into()
            } else {
                format!("Next: answer this question; {CONTINUE}")
            })
        }
        Shaped::Waiting => {
            if !calm {
                lines.push(Line::styled(
                    if shaping.success_open {
                        "The saved goal leaves Done when open."
                    } else {
                        "Shape hasn't asked its next question yet."
                    },
                    palette.ink,
                ));
            }
            Some(if acts {
                "Next: F2 looks into the next question.".into()
            } else {
                format!("Next: shape it into a seed; {CONTINUE}")
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
            // Beside the words box, so a long seed can't scroll it away.
            if !calm && text.as_deref().is_some_and(|t| t.contains(NOTHING_FOUND)) {
                flag = Some(Line::styled(
                    "Nothing found: this seed records nothing checked in the repository.",
                    palette.ink,
                ));
            }
            match (acts, revision) {
                (true, _) if words == Some("your teach-back") => {
                    Some("Next: F7 saves your words and compares with the draft.".into())
                }
                // The key bar owns the next action, including while a refusal is shown.
                (true, Some(_)) => None,
                _ => Some(format!("Next: confirm the seed draft in Shape; {CONTINUE}")),
            }
        }
        Shaped::Ready { revision, ended } => {
            lines.push(Line::styled(
                match revision {
                    Some(r) => format!("Seed ready: you confirmed {r}, and it still matches."),
                    None => "Seed ready: what you confirmed still matches.".into(),
                },
                palette.ink,
            ));
            if *ended {
                lines.push(Line::styled(
                    "This session is completed, so nothing in it can change.",
                    palette.muted,
                ));
            }
            Some("Next: the seed is ready for intake.".into())
        }
        Shaped::Unverified { reason } => {
            lines.push(Line::styled(
                format!("The seed says it's confirmed, but Tinkery can't verify it: {reason}."),
                palette.ink,
            ));
            Some(format!("Next: confirm the current revision; {CONTINUE}"))
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
            Some(format!("Next: continue shaping; {CONTINUE}"))
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
    // What the operator acts with stays in view beneath what they read.
    let mut input: Vec<Line<'static>> = flag.into_iter().collect();
    if let Some(label) = words {
        let style = if calm { palette.muted } else { palette.ink };
        gap(&mut input);
        input.push(Line::styled(label.to_owned(), style));
        if acts && !calm && matches!(shaping.stage, Shaped::Drafted { .. }) {
            input.push(Line::default());
            input.push(Line::styled(
                "Confirming the seed is not approval to build.",
                palette.ink,
            ));
        }
    }
    if let Some(running) = running {
        gap(&mut input);
        input.push(running);
        return (Text::from(lines), Text::from(input));
    }
    let next = match notice {
        Some((said, own)) => {
            if !said.is_empty() {
                gap(&mut input);
                input.push(Line::styled(said.to_owned(), palette.ink));
            }
            own.or(next)
        }
        // The key bar already names the one next action, unless something needs saying first.
        None => next.filter(|_| !acts),
    };
    if let Some(next) = next {
        gap(&mut input);
        input.push(Line::styled(next, palette.ink));
    }
    (Text::from(lines), Text::from(input))
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
