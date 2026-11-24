//! Shape: where a saved goal's seed stands, and working through its questions to a seed.
//!
//! Opened on one session (from tend's detail) or as a list of goals waiting for a seed (from the
//! menu). Incomplete lists retain the unreadable count and never claim there are no waiting goals.
//! It shows the confirmed goal's first line, then one thing: the current question, the seed
//! draft, a ready seed, or an honest unknown or failure, with one next step. Opening, scrolling
//! and typing write nothing and call no model. F2 runs the one action the key bar names: save an
//! answer and start the agent step, keep a flagged node with a reason, save a form, or confirm a
//! seed with the operator's own words; see [`step`]. F1 lists every other action allowed now by
//! number ([`choices`]): "So far" (the whole goal exactly as recorded, decisions, open questions
//! and current findings, without writing), defer, delegate, revise, teach-back, the folder a step
//! reads, and ending. The ledger alone owns these rules. Revision saves then looks, asking recorded
//! dependents before any model call; keeping calls no model. Ending a session for good asks once
//! more, then F2 ends it; Esc keeps it open. Each finished step or ending is followed by a fresh
//! read, so the screen shows what the ledger holds. Confirming waits until the whole draft has
//! been on screen. Under an agent launch declaration Shape only reads: Tinkery would record its
//! every change as the operator's.

pub use step::Agent;

pub mod answering;
mod choices;
pub mod command;
mod forms;
mod help;
pub mod shaping;
mod step;
use crate::confirmation::GoalConfirmation;
use crate::words;

use crate::seed_me::ledger::{AnswerAction, CheckMethod};
use crate::{
    Palette,
    seed_me::{ReadFailure, Session, Stage},
};
use answering::{Assumption, Authority, Ending, OBJECTION, PROBE, SAMPLE, UNOBSERVED};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use forms::Form;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use shaping::{Criteria, SeedLine, SeedView, Shaped, Shaping};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};
use step::{CheckEdit, Done, Failure, Reads, RetryFeedback, Running, Steps, Why, Work};
use unicode_width::UnicodeWidthStr;
use words::Words;

/// The widest Shape's open view gets, so lines stay readable on a wide terminal.
const WIDTH: u16 = 76;
/// Why an agent launch without a drafting model only reads: its answers would have no agent attribution.
const HELD: &str = "An agent answers Shape with --shape-pi. Shape only reads; Esc goes back.";
/// The only words that confirm a seed draft, as in Think: anything else may be a correction.
const CONFIRM: &str = "confirm";
/// Confirming before the whole seed draft has been on screen.
const READ_FIRST: &str = "PgDn or Down to 'End of Tink Contract' first. Nothing was confirmed.";
const ANSWER_FIRST: &str = "Type your answer first; nothing was saved.";
const CONFIRM_FIRST: &str = "Type confirm, or F1 Revise the draft. Nothing was confirmed.";
const REASON_FIRST: &str = "Type why you're keeping this answer first; nothing was saved.";
const TEACH_BACK_FIRST: &str = "Type your teach-back first; nothing was saved.";
const CHANGE_FIRST: &str = "Type what should change first; nothing was saved.";
/// A fresh read no longer shows the draft an open teach-back or change box was for.
const STALE_ASIDE: &str = "The draft changed. Your words were not saved.";
const STALE_NEXT: &str = "Next: copy your words, then Esc closes this box.";

/// Shape's screen: an optional list of goals waiting for a seed and, at most, one open session.
pub struct Shape {
    /// `None` when opened on one session; otherwise the list, pending until a read lands.
    list: Option<Option<Result<Vec<Session>, String>>>,
    /// Session records discovery could not read; they may hide goals from the list.
    unreadable: usize,
    failures: Vec<ReadFailure>,
    help_scroll: crate::help::Scroll,
    actions: crate::actions::Page,
    selected: usize,
    open: Option<Opened>,
    scroll: u16,
    /// Rows of reading on screen at the last draw, for PgUp/PgDn.
    page: u16,
    agent: Option<Arc<Agent>>,
    /// An agent launch without --shape-pi: Shape only reads.
    held: bool,
    /// The launch's declaration; every write a session makes is attributed with it.
    confirmation: GoalConfirmation,
    /// F1's Model and responses row asked for the Model panel, which the launch owns.
    model: bool,
    approve: Option<crate::seed_me::ready::SeedTarget>,
}

/// What a key asks of whoever holds Shape.
#[derive(Debug, PartialEq, Eq)]
pub enum Leave {
    /// Stay in Shape.
    Stay,
    /// Leave Shape for wherever it was opened from.
    Back,
}

/// A words box opened from F1 over a seed draft.
enum Aside {
    TeachBack,
    Change,
    RepairCheck(CheckEdit),
}

struct Opened {
    session: PathBuf,
    /// The launch's declaration every write from this session is attributed with.
    confirmation: GoalConfirmation,
    reading: Reading,
    /// The stage's own words: an answer, a reason to keep, or the words that confirm a seed. A
    /// form or aside leaves them untouched.
    typed: Words,
    /// The one thing the session is doing now.
    mode: Mode,
    /// What the last key or step did, when it needs saying.
    notice: Option<Notice>,
    /// The last rejected reply's diagnostic, only for a matching explicit retry in this visit.
    feedback: Option<RetryFeedback>,
    /// The folder the next step reads.
    folder: Folder,
    /// What the step that asked the current question assumed instead of asking; shown with
    /// that question only.
    assumed: Vec<Assumption>,
    /// A hard-to-undo question's suggestion its step didn't record, until F1 asks for it.
    withheld: Option<String>,
    /// Whether F1 showed a hard-to-undo question's options and suggestion.
    revealed: bool,
    /// The exact draft revision whose last line has been on screen; layout changes don't revoke it.
    read_end: Option<Shaped>,
}

/// What an open session is doing. Each mode alone decides its keys, key bar, help line and the
/// words box keys type into.
enum Mode {
    /// Reading, or typing into the editor.
    Idle(Editor),
    /// A model step or confirmation is running; Esc stops it, and its editor stays on screen, muted.
    Running(Running, Work, Editor),
    /// The ledger is carrying out a change without a model; keys do nothing until it lands.
    Writing(Running, Work, Editor),
    /// The "So far" page, at this scroll.
    SoFar(u16),
    /// A folder being typed after choosing it in F1, until F2 uses it or Esc keeps the one shown.
    Choosing(Words),
    /// After choosing to object, waiting for the number of the assumption, or Esc.
    Objecting,
    /// An ending asked for once, waiting for F2 or Esc.
    Asking(Ending),
}

/// The words box an idle, running or saving session shows.
enum Editor {
    /// The stage's own words, [`Opened::typed`].
    Stage,
    /// A deferral or revision form; cancelling restores the untouched stage words.
    Form(Form),
    /// An unsaved box over the draft, with its own words and cancel action.
    Aside(Aside, Words),
}

/// What the last key or step said.
enum Notice {
    /// Its words, and its own next step if any.
    Said(String, Option<String>),
    /// A refusal about the typed words; changing the words clears it.
    Words(&'static str),
    /// Confirming before the draft's end was on screen; reaching the end clears it.
    ReadFirst,
}

impl Notice {
    fn said(said: impl Into<String>) -> Self {
        Self::Said(said.into(), None)
    }

    /// What it says, and its own next step.
    fn parts(&self) -> (&str, Option<String>) {
        match self {
            Self::Said(said, next) => (said, next.clone()),
            Self::Words(said) => (said, None),
            Self::ReadFirst => (READ_FIRST, None),
        }
    }
}

/// What the next step reads: not known until a read with an agent lands, then where Think
/// says the goal was made, or the operator's pick, with Think's answers.
enum Folder {
    Unknown,
    Known(Reads),
}

/// A read of the session, with the folder a step would read when there's an agent. Both land
/// together, so a step is never offered before its folder is shown.
#[expect(
    clippy::large_enum_variant,
    reason = "Shape holds one reading at a time; boxing would only add indirection"
)]
enum Reading {
    NotStarted,
    Waiting(mpsc::Receiver<(Result<Shaping, String>, Option<Reads>)>),
    Done(Result<Shaping, String>),
}

/// Where Shape opens.
pub enum Open<'a> {
    /// A list of goals waiting for a seed, pending until [`Shape::offer`].
    List,
    /// One session; its first [`Shape::tick`] reads the ledger off the paint path.
    Session(&'a Path),
}

impl Shape {
    /// Shape for one launch, opened on `open`. `agent` answers, confirms and drafts; without one,
    /// Shape cannot save answers, confirm seeds or call a model, and ending remains available.
    /// Every write is attributed with the launch's declared `confirmation`. `held` makes Shape
    /// only read: answering, confirming and ending are held, say what is missing, and `agent`
    /// is never used.
    pub fn new(
        open: Open<'_>,
        agent: Option<Arc<Agent>>,
        confirmation: &GoalConfirmation,
        held: bool,
    ) -> Self {
        let (list, open) = match open {
            Open::List => (Some(None), None),
            Open::Session(session) => (None, Some(Opened::start(session, confirmation.clone()))),
        };
        Self {
            list,
            unreadable: 0,
            failures: Vec::new(),
            help_scroll: crate::help::Scroll::default(),
            actions: crate::actions::Page::default(),
            selected: 0,
            open,
            scroll: 0,
            page: 1,
            agent: agent.filter(|_| !held),
            held,
            confirmation: confirmation.clone(),
            model: false,
            approve: None,
        }
    }

    /// Whether F1's Model and responses row was picked since the last ask. Asking clears it.
    pub fn take_model_request(&mut self) -> bool {
        std::mem::take(&mut self.model)
    }

    /// Take the deliberate Ready action's exact seed once; routing starts no write or model.
    pub(crate) fn take_approve_request(&mut self) -> Option<crate::seed_me::ready::SeedTarget> {
        self.approve.take()
    }

    /// Reenter this same session through its normal asynchronous read, keeping words and context.
    pub(crate) fn reread(&mut self) {
        if let Some(open) = &mut self.open {
            open.reading = Reading::NotStarted;
        }
    }

    /// Whether keys now type words (an answer, seed confirmation or folder), so letters aren't
    /// commands. Never while an ending or objection asks, or the ledger carries one out.
    pub fn takes_text(&self) -> bool {
        self.agent.is_some()
            && self.open.as_ref().is_some_and(|open| match open.mode {
                Mode::Choosing(_) => true,
                Mode::Idle(_) => open.typing().is_some(),
                _ => false,
            })
    }

    /// Bracketed paste: the words go into the folder, answer or confirmation exactly as pasted.
    pub fn paste(&mut self, text: &str) {
        if self.takes_text()
            && let Some(open) = &mut self.open
        {
            open.type_into(|words| words.insert(text));
        }
    }

    /// Whether the list still waits on a read of the sessions.
    pub fn wants_sessions(&self) -> bool {
        matches!(self.list, Some(None))
    }

    /// Supply one discovery read: its readable sessions, the count of unreadable records and
    /// their path and reason samples, or the root failure. Only goals waiting for a seed are
    /// listed, and F1 keeps at most `MAX_READ_FAILURES` samples. A failed read discards any
    /// earlier count and samples.
    pub fn offer(&mut self, read: Result<(&[Session], usize, &[ReadFailure]), String>) {
        if self.list.is_some() {
            self.help_scroll.reset();
            (self.unreadable, self.failures) = match &read {
                Ok((_, unreadable, failures)) => (
                    *unreadable,
                    failures
                        .iter()
                        .take(crate::seed_me::MAX_READ_FAILURES.min(*unreadable))
                        .cloned()
                        .collect(),
                ),
                Err(_) => (0, Vec::new()),
            };
            let listed: Result<Vec<Session>, String> = read.map(|(sessions, ..)| {
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
            && let Mode::Running(running, work, _) | Mode::Writing(running, work, _) = &open.mode
            && let Some(result) = running.poll()
        {
            let work = work.clone();
            let editor = open.take_editor();
            let wrote = |said: String| Some(Notice::said(said));
            // A change without a model closes its box only when saved; a step keeps typed words
            // only when nothing of them was saved.
            let (notice, editor) = match (&work, result) {
                (Work::RepairCheck { .. }, Ok(_)) => {
                    open.read_end = None;
                    self.scroll = 0;
                    (
                        wrote(
                            "Check saved. Read the changed draft, then confirm. No model ran."
                                .into(),
                        ),
                        Editor::Stage,
                    )
                }
                (Work::Revisit { .. }, Ok(_)) => {
                    open.typed.clear();
                    (
                        wrote("Deferred prerequisite answered. No model ran.".into()),
                        Editor::Stage,
                    )
                }
                (Work::End(Ending::Completed), Ok(_)) => (
                    wrote("Session completed. It's read-only now.".into()),
                    editor,
                ),
                (Work::End(Ending::Stopped), Ok(_)) => (
                    wrote(
                        "Session stopped. Its open questions stay recorded; it's read-only now."
                            .into(),
                    ),
                    editor,
                ),
                (Work::Object { text, .. }, Ok(_)) => {
                    open.assumed.retain(|a| &a.text != text);
                    (
                        wrote(format!(
                            "Not assumed: {text}. It's an open question now, asked before Shape drafts the Tink."
                        )),
                        editor,
                    )
                }
                (Work::Keep { .. }, Ok(_)) => {
                    open.typed.clear();
                    (
                        wrote("Answer kept with your reason. No model ran.".into()),
                        editor,
                    )
                }
                (Work::Recommend { .. }, Ok(_)) => {
                    open.withheld = None;
                    (
                        wrote("Here are the options and my suggestion.".into()),
                        editor,
                    )
                }
                (
                    Work::RepairCheck { .. }
                    | Work::Revisit { .. }
                    | Work::End(_)
                    | Work::Object { .. }
                    | Work::Keep { .. }
                    | Work::Recommend { .. },
                    Err(failure),
                ) => (
                    wrote(match failure.why {
                        Why::Stale => "Another writer changed this session first.".into(),
                        Why::Refused(reason) => reason,
                        _ => "Ending stopped unexpectedly.".into(),
                    }),
                    editor,
                ),
                (_, result) => {
                    let editor = if result
                        .as_ref()
                        .map_or_else(|failure| failure.answer_saved, |_| true)
                    {
                        if !matches!(work, Work::TeachBack { .. }) {
                            open.typed.clear();
                        }
                        Editor::Stage
                    } else {
                        editor
                    };
                    let asked = match &result {
                        Ok(Done::Asked(asked)) => asked.clone(),
                        _ => step::Asked::default(),
                    };
                    open.assumed = asked.assumed;
                    open.withheld = asked.withheld;
                    open.revealed = false;
                    open.feedback = result
                        .as_ref()
                        .err()
                        .and_then(|failure| failure.feedback.as_deref().cloned());
                    (
                        said(result, &work, self.agent.as_deref().map(Agent::left)),
                        editor,
                    )
                }
            };
            open.notice = notice;
            open.mode = Mode::Idle(editor);
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
                Ok((result, reads)) => {
                    if result.as_ref().ok().map(|read| &read.stage) != open.read_end.as_ref() {
                        open.read_end = None;
                    }
                    open.reading = Reading::Done(result);
                    // The operator's folder stays; Think's answers are as of this read.
                    match (&mut open.folder, reads) {
                        (Folder::Unknown, Some(reads)) => open.folder = Folder::Known(reads),
                        (Folder::Known(known), Some(reads)) => {
                            *known = reads.reading(known.folder.clone());
                        }
                        (_, None) => {}
                    }
                    if !matches!((&open.reading, &open.folder),
                        (Reading::Done(Ok(Shaping { stage: Shaped::Waiting, version: Some(version), .. })),
                            Folder::Known(reads)) if open.feedback.as_ref().is_some_and(|feedback|
                                feedback.applies(&open.session, *version, &reads.folder)))
                    {
                        open.feedback = None;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    open.feedback = None;
                    open.reading = Reading::Done(Err("The read stopped unexpectedly.".into()))
                }
            }
        }
    }

    /// One key, opening only the selected session when requested. Esc stops a running step, or closes an
    /// open session back to the list, or leaves Shape. F2 runs the one action the key bar names:
    /// save an answer and look, keep with a reason, save a form or teach-back, use a folder,
    /// confirm the displayed seed with typed words, or end a session after it asked. Every other
    /// action is a row in F1 ([`Shape::pick`]). No other function key does anything.
    pub fn key(&mut self, key: KeyEvent) -> Leave {
        // In an open session Enter runs the key bar's action; the list keeps Enter to open.
        let key = if self.open.is_some() {
            crate::writing_key(key)
        } else {
            key
        };
        if key.kind == KeyEventKind::Release
            || matches!(key.code, KeyCode::F(n) if n != 2 || key.kind == KeyEventKind::Repeat)
        {
            return Leave::Stay;
        }
        let page = crate::page(self.page);
        if let Some(open) = &mut self.open {
            if let Mode::SoFar(scroll) = &mut open.mode {
                match key.code {
                    KeyCode::Esc => open.mode = Mode::Idle(Editor::Stage),
                    KeyCode::Down | KeyCode::Char('j') => *scroll = scroll.saturating_add(1),
                    KeyCode::Up | KeyCode::Char('k') => *scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown => *scroll = scroll.saturating_add(page),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(page),
                    _ => {}
                }
                return Leave::Stay;
            }
            if self.held && key.code == KeyCode::F(2) {
                open.notice = Some(Notice::said(HELD));
                return Leave::Stay;
            }
            let f2 = key.code == KeyCode::F(2) && key.modifiers.is_empty();
            let stale = open.stale_aside().is_some();
            match &mut open.mode {
                Mode::Running(running, ..) => {
                    if key.code == KeyCode::Esc {
                        running.stop();
                    }
                    return Leave::Stay;
                }
                Mode::Writing(..) => return Leave::Stay,
                Mode::Choosing(folder) => {
                    match key.code {
                        _ if f2 => {
                            let typed = folder.text().to_owned();
                            open.mode = Mode::Idle(Editor::Stage);
                            open.notice = open.pick(&typed);
                        }
                        KeyCode::Esc => open.mode = Mode::Idle(Editor::Stage),
                        KeyCode::PageDown => self.scroll = self.scroll.saturating_add(page),
                        KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
                        _ => open.type_into(|words| words.key(key)),
                    }
                    return Leave::Stay;
                }
                Mode::Objecting => {
                    match key.code {
                        KeyCode::Esc => {
                            open.mode = Mode::Idle(Editor::Stage);
                            open.notice = Some(Notice::said("Nothing was changed."));
                        }
                        KeyCode::Char(c) if key.modifiers.is_empty() => {
                            if let Some(text) = c
                                .to_digit(10)
                                .and_then(|n| (n as usize).checked_sub(1))
                                .and_then(|i| open.assumed.get(i))
                                .map(|a| a.text.clone())
                            {
                                match open.version() {
                                    Some(expected) => open
                                        .write(Work::Object { expected, text }, self.agent.clone()),
                                    None => open.unversioned(),
                                }
                            }
                        }
                        _ => {}
                    }
                    return Leave::Stay;
                }
                Mode::Asking(ending) => {
                    match key.code {
                        _ if f2 => {
                            let ending = *ending;
                            open.write(Work::End(ending), self.agent.clone());
                        }
                        KeyCode::Esc => {
                            open.mode = Mode::Idle(Editor::Stage);
                            open.notice = Some(Notice::said("Nothing was ended."));
                        }
                        _ => {}
                    }
                    return Leave::Stay;
                }
                Mode::Idle(Editor::Form(form)) => {
                    match key.code {
                        KeyCode::Esc => {
                            open.mode = Mode::Idle(Editor::Stage);
                            open.notice = Some(Notice::said("Nothing was changed."));
                        }
                        KeyCode::Tab => form.toggle(),
                        KeyCode::PageDown => self.scroll = self.scroll.saturating_add(page),
                        KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
                        _ if f2 => {
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
                Mode::Idle(Editor::Aside(..)) if f2 && stale => {
                    open.notice = Some(Notice::Said(STALE_ASIDE.into(), Some(STALE_NEXT.into())));
                    return Leave::Stay;
                }
                Mode::Idle(Editor::Aside(aside, _)) if key.code == KeyCode::Esc => {
                    let what = match aside {
                        Aside::TeachBack => "Teach-back",
                        Aside::Change => "Change",
                        Aside::RepairCheck(CheckEdit {
                            method: CheckMethod::Command,
                            ..
                        }) => "Check command",
                        Aside::RepairCheck(CheckEdit {
                            method: CheckMethod::Manual,
                            ..
                        }) => "How to judge",
                    };
                    open.notice = Some(Notice::said(if stale {
                        format!("{what} closed. Its words were not saved.")
                    } else {
                        format!("{what} cancelled. Nothing was saved.")
                    }));
                    open.mode = Mode::Idle(Editor::Stage);
                    return Leave::Stay;
                }
                Mode::Idle(_) | Mode::SoFar(_) => {}
            }
            if let (Some(agent), Some(typing)) = (&self.agent, open.typing()) {
                if f2 {
                    open.act(typing.act, agent);
                    return Leave::Stay;
                }
                // Confirmation words are one line, so Up and Down scroll the seed being read,
                // for terminals that take PgUp/PgDn (#614), and Enter adds no line.
                match (typing.act, key.code) {
                    (_, KeyCode::Esc | KeyCode::F(_) | KeyCode::PageDown | KeyCode::PageUp) => {}
                    (Act::Confirm { .. }, KeyCode::Down) => {
                        self.scroll = self.scroll.saturating_add(1);
                        return Leave::Stay;
                    }
                    (Act::Confirm { .. }, KeyCode::Up) => {
                        self.scroll = self.scroll.saturating_sub(1);
                        return Leave::Stay;
                    }
                    (Act::Confirm { .. }, KeyCode::Enter) => {
                        open.notice = Some(Notice::said(
                            "Enter confirms the Tink. ^J adds no line here.",
                        ));
                        return Leave::Stay;
                    }
                    _ => {
                        open.type_into(|words| words.key(key));
                        return Leave::Stay;
                    }
                }
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
                    self.help_scroll.reset();
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
                self.open = Some(Opened::start(&path, self.confirmation.clone()));
                self.scroll = 0;
            }
            _ => {}
        }
        Leave::Stay
    }

    /// Draws Shape in `area`, with its key bar on the terminal's last row. The "So far" page,
    /// like help, fills the body and has no key bar.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        if let Some(Opened {
            mode: Mode::SoFar(scroll),
            reading: Reading::Done(Ok(shaping)),
            ..
        }) = &mut self.open
        {
            self.page = area.height;
            render_so_far(frame, scroll, shaping, area, palette);
            return;
        }
        let overflow = match &mut self.open {
            Some(open) => {
                let (overflow, page) = render_open(
                    frame,
                    &mut self.scroll,
                    open,
                    self.agent.as_deref(),
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
        let width = frame.area().width.saturating_sub(4);
        let keys = match (&self.open, &self.list) {
            (Some(open), _) => match &open.mode {
                Mode::Running(..) => "esc stop".to_owned(),
                Mode::Writing(..) => String::new(),
                Mode::Objecting => "1-9 object to that one   esc keep them all".to_owned(),
                Mode::Asking(Ending::Completed) => "enter end for good   esc keep it".to_owned(),
                Mode::Asking(Ending::Stopped) => "enter stop for good   esc keep it".to_owned(),
                Mode::Idle(Editor::Form(form)) => format!("{}   esc cancel", form.bar()),
                Mode::Choosing(_) => format!(
                    "{}^J newline   enter use folder   esc cancel",
                    if overflow { "pgup/dn read   " } else { "" }
                ),
                Mode::Idle(_) | Mode::SoFar(_) => self.bar(open, overflow, width),
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
    }

    /// The key bar over an open session's reading or words box, fitted to `width`. Esc is never
    /// cut off: a full bar closes up, then uses each key's short words, before it runs out of room.
    fn bar(&self, open: &Opened, overflow: bool, width: u16) -> String {
        let agent = self.agent.is_some();
        let typing = open.typing();
        let act = typing.as_ref().map(|typing| &typing.act);
        // Each key's words, and shorter words for a crowded bar.
        let mut keys: Vec<(String, Option<&str>)> = Vec::new();
        if overflow {
            keys.push(match act {
                Some(Act::Confirm { .. }) if agent && open.read_end.is_none() => {
                    ("pgup/dn read to end".into(), Some("pgup/dn to end"))
                }
                Some(_) if agent => ("pgup/dn read".into(), Some("pgup/dn")),
                _ => ("↑↓ scroll".into(), None),
            });
        }
        let action = match (agent, act, &open.reading) {
            (true, Some(Act::TeachBack { .. }), _) => Some(("enter save + compare".into(), None)),
            (true, Some(Act::Change { .. }), _) => Some(("enter save + redraft".into(), None)),
            (_, Some(Act::RepairCheck(_)), _) => Some(("enter save check".into(), None)),
            (true, Some(Act::Keep { .. }), _) => Some(("enter keep with reason".into(), None)),
            (true, Some(Act::Answer { .. }), _) => Some(if overflow || open.offers_answers() {
                ("enter save + look".into(), Some("enter answer"))
            } else {
                ("enter save answer and look into next".into(), None)
            }),
            (true, Some(Act::Confirm { revision, .. }), _) if open.read_end.is_some() => {
                Some((format!("enter confirm Tink {revision}"), None))
            }
            (
                true,
                None,
                Reading::Done(Ok(Shaping {
                    stage: Shaped::Waiting,
                    ..
                })),
            ) if open.stale_aside().is_none() => {
                Some(("enter look into the next question".into(), None))
            }
            _ => None,
        };
        keys.extend(action);
        if !self.rows().is_empty() {
            keys.push(("F1 more".into(), None));
        }
        keys.push((
            match open.editor() {
                Editor::Aside(..) => "esc cancel",
                _ => "esc back",
            }
            .into(),
            None,
        ));
        crate::help::fit(&keys, width)
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
                        let title =
                            crate::clipped(session.goal.lines().next().unwrap_or(""), budget);
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
                "No goals are waiting for a Tink. Confirm a goal in think, then shape it here."
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
    RepairCheck(CheckEdit),
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
    /// Save the typed change to seed draft `revision`, read at `expected`, then redraft.
    Change {
        revision: String,
        expected: u64,
    },
    /// Confirm seed `revision` with the typed word `confirm`.
    Confirm {
        revision: String,
        expected: u64,
        seed: String,
    },
}

/// The words box an open stage offers, and the key that acts on them.
struct Typing {
    act: Act,
    label: &'static str,
}

impl Opened {
    /// The words box shown now: a running step or a save keeps the one it started from.
    fn editor(&self) -> &Editor {
        match &self.mode {
            Mode::Idle(editor) | Mode::Running(.., editor) | Mode::Writing(.., editor) => editor,
            _ => &Editor::Stage,
        }
    }

    /// Leave the current mode for an idle stage, keeping the words box it showed.
    fn take_editor(&mut self) -> Editor {
        match std::mem::replace(&mut self.mode, Mode::Idle(Editor::Stage)) {
            Mode::Idle(editor) | Mode::Running(.., editor) | Mode::Writing(.., editor) => editor,
            _ => Editor::Stage,
        }
    }

    /// The words keys type into now: a folder, an aside, a form's field, or the stage's own.
    fn words(&mut self) -> &mut Words {
        match &mut self.mode {
            Mode::Choosing(folder) => folder,
            Mode::Idle(editor) | Mode::Running(.., editor) | Mode::Writing(.., editor) => {
                match editor {
                    Editor::Aside(_, words) => words,
                    Editor::Form(form) => form.words(),
                    Editor::Stage => &mut self.typed,
                }
            }
            _ => &mut self.typed,
        }
    }

    /// Edit the words keys type into; a refusal about them no longer applies once they change.
    fn type_into(&mut self, edit: impl FnOnce(&mut Words)) {
        let words = self.words();
        let before = words.text().to_owned();
        edit(words);
        if self.words().text() != before && matches!(self.notice, Some(Notice::Words(_))) {
            self.notice = None;
        }
    }

    /// The label of an open teach-back or change box whose draft a fresh read no longer shows.
    /// Its words stay on screen, read-only, and never go to the new stage.
    fn stale_aside(&self) -> Option<&'static str> {
        if !matches!(self.reading, Reading::Done(Ok(_))) || self.typing().is_some() {
            return None;
        }
        match self.editor() {
            Editor::Aside(Aside::TeachBack, _) => Some("your teach-back"),
            Editor::Aside(Aside::Change, _) => Some("your change"),
            _ => None,
        }
    }

    /// The words box this stage offers, if any: an answer to the current question, or the words
    /// that confirm a seed draft.
    fn typing(&self) -> Option<Typing> {
        if let Editor::Aside(Aside::RepairCheck(edit), _) = self.editor() {
            return matches!(self.reading, Reading::Done(Ok(_))).then(|| Typing {
                act: Act::RepairCheck(edit.clone()),
                label: if edit.method == CheckMethod::Manual {
                    "your how-to-judge words"
                } else {
                    "your check command"
                },
            });
        }
        let Reading::Done(Ok(shaping)) = &self.reading else {
            return None;
        };
        match (self.editor(), &shaping.stage, shaping.version) {
            (
                Editor::Aside(aside, _),
                Shaped::Drafted {
                    revision: Some(revision),
                    text: Some(seed),
                },
                Some(expected),
            ) => Some(match aside {
                Aside::TeachBack => Typing {
                    act: Act::TeachBack {
                        revision: revision.clone(),
                        expected,
                        seed: seed.clone(),
                    },
                    label: "your teach-back",
                },
                _ => Typing {
                    act: Act::Change {
                        revision: revision.clone(),
                        expected,
                    },
                    label: "your change",
                },
            }),
            (Editor::Form(form), ..) => Some(Typing {
                act: Act::Respond {
                    id: form.id.clone(),
                    expected: form.expected,
                    action: form.action(),
                },
                label: form.label(),
            }),
            (Editor::Aside(..), ..) => None,
            (Editor::Stage, Shaped::Review { id, .. }, Some(_)) => Some(Typing {
                act: Act::Keep { id: id.clone() },
                label: "your reason",
            }),
            (Editor::Stage, Shaped::Question { id, .. }, Some(expected)) => Some(Typing {
                act: Act::Answer {
                    id: id.clone(),
                    expected,
                },
                label: "your answer",
            }),
            (
                Editor::Stage,
                Shaped::Drafted {
                    revision: Some(revision),
                    text: Some(seed),
                },
                Some(expected),
            ) => Some(Typing {
                act: Act::Confirm {
                    revision: revision.clone(),
                    expected,
                    seed: seed.clone(),
                },
                label: "your words",
            }),
            _ => None,
        }
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

    /// Show what hard-to-undo question `id` held back, recording its withheld suggestion first.
    fn reveal(&mut self, id: String, agent: Option<Arc<Agent>>) {
        self.revealed = true;
        match (self.withheld.clone(), self.version()) {
            (Some(text), Some(expected)) => {
                self.write(Work::Recommend { id, expected, text }, agent)
            }
            (Some(_), None) => self.unversioned(),
            _ => {
                self.notice = Some(Notice::said(
                    "Here are the options. No suggestion was kept for this question.",
                ))
            }
        }
    }

    /// The version a change without a model starts from, once a read landed.
    fn version(&self) -> Option<u64> {
        match &self.reading {
            Reading::Done(Ok(shaping)) => shaping.version,
            _ => None,
        }
    }

    /// A read without a ledger version gives a versioned change nothing to start from.
    fn unversioned(&mut self) {
        self.notice = Some(Notice::said("Tinkery read no version to change from."));
    }

    /// Carry out a change without a model off the paint path; [`Shape::tick`] reports it.
    fn write(&mut self, work: Work, agent: Option<Arc<Agent>>) {
        if !matches!(self.reading, Reading::Done(Ok(_))) {
            return;
        }
        let said = doing(&work, false, "");
        let running = Running::start(
            work.clone(),
            self.session.clone(),
            agent,
            self.confirmation.clone(),
        );
        let editor = self.take_editor();
        self.mode = Mode::Writing(running, work, editor);
        self.notice = Some(Notice::said(said));
    }

    /// Use the folder the operator typed for the next steps, or say why not and keep the one shown.
    /// `~/` means the home folder; anything else must be an absolute path to a folder. The exact
    /// text is tried first, then without surrounding whitespace, so a pasted line break still works.
    fn pick(&mut self, typed: &str) -> Option<Notice> {
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
                self.folder = match &self.folder {
                    Folder::Known(previous) if previous.folder == folder => return None,
                    Folder::Known(previous) => Folder::Known(previous.reading(folder)),
                    Folder::Unknown => Folder::Known(Reads::from(folder)),
                };
                self.feedback = None;
                None
            }
            None => Some(Notice::said(format!(
                "There's no folder at {typed}, so the step still reads {}.",
                self.folder_name().unwrap_or_default()
            ))),
        }
    }

    /// The name of the folder the next step reads, once known.
    fn folder_name(&self) -> Option<String> {
        let Folder::Known(Reads { folder, .. }) = &self.folder else {
            return None;
        };
        Some(folder.file_name().map_or_else(
            || folder.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        ))
    }

    /// Start what `act` asks, or say why not. Refuses empty words, an exhausted budget, a folder
    /// not yet known and a seed not yet read to the end before anything is written or called.
    fn act(&mut self, act: Act, agent: &Arc<Agent>) {
        let words = match self.editor() {
            Editor::Aside(_, words) => words,
            _ => &self.typed,
        }
        .text()
        .to_owned();
        let folder = match &self.folder {
            Folder::Known(reads) => Some(reads.clone()),
            _ => None,
        };
        let path = folder
            .clone()
            .unwrap_or_else(|| Reads::from(PathBuf::new()));
        let mut work = match act {
            Act::RepairCheck(edit) => Work::RepairCheck { edit, words },
            Act::Respond {
                id,
                expected,
                action: AnswerAction::Revisit(words),
            } => Work::Revisit {
                id,
                expected,
                words,
            },
            Act::Respond {
                id,
                expected,
                action,
            } => Work::Respond {
                id,
                expected,
                action,
                reads: path,
            },
            Act::Keep { id } => match self.version() {
                Some(expected) => Work::Keep {
                    id,
                    expected,
                    reason: words,
                },
                None if words.trim().is_empty() => {
                    self.notice = Some(Notice::Words(REASON_FIRST));
                    return;
                }
                None => return self.unversioned(),
            },
            Act::Answer { id, expected } => Work::Answer {
                relay: None,
                id,
                words,
                expected,
                reads: path,
            },
            Act::TeachBack {
                revision,
                expected,
                seed,
            } => Work::TeachBack {
                revision,
                expected,
                seed,
                words,
            },
            Act::Change { revision, expected } => Work::Change {
                revision,
                words,
                expected,
                reads: path,
            },
            Act::Look(expected) => Work::Look {
                expected,
                feedback: None,
                reads: path,
            },
            Act::Confirm {
                revision,
                expected,
                seed,
            } => Work::Confirm {
                revision,
                words,
                expected,
                seed,
            },
        };
        let empty = |words: &str| words.trim().is_empty();
        self.notice = match &work {
            Work::TeachBack { words, .. } if empty(words) => Some(Notice::Words(TEACH_BACK_FIRST)),
            Work::Change { words, .. } if empty(words) => Some(Notice::Words(CHANGE_FIRST)),
            Work::Keep { reason, .. } if empty(reason) => Some(Notice::Words(REASON_FIRST)),
            Work::Answer { words, .. } if empty(words) => Some(Notice::Words(ANSWER_FIRST)),
            Work::Confirm { .. } if self.read_end.is_none() => Some(Notice::ReadFirst),
            Work::Confirm { words, .. } if words != CONFIRM => Some(Notice::Words(CONFIRM_FIRST)),
            work if work.calls_model() => {
                match Steps::new(Some(agent), &self.confirmation).admit(work) {
                    Err(failure) => said(Err(failure), work, None),
                    Ok(()) if folder.is_none() && !matches!(work, Work::TeachBack { .. }) => {
                        Some(Notice::said(
                            "Shape is still finding which folder the step reads; nothing was saved.",
                        ))
                    }
                    Ok(()) => None,
                }
            }
            _ => None,
        };
        if self.notice.is_some() {
            return;
        }
        // A retry diagnostic goes only to the explicit look it was kept for.
        let feedback = self.feedback.take();
        if let Work::Look {
            expected,
            reads,
            feedback: slot,
        } = &mut work
        {
            *slot = feedback.filter(|f| f.applies(&self.session, *expected, &reads.folder));
        }
        if work.calls_model() || matches!(work, Work::Confirm { .. }) {
            let running = Running::start(
                work.clone(),
                self.session.clone(),
                Some(agent.clone()),
                self.confirmation.clone(),
            );
            let editor = self.take_editor();
            self.mode = Mode::Running(running, work, editor);
        } else {
            self.write(work, Some(agent.clone()));
        }
    }

    /// Read the session off the paint path and, with an agent, find the folder its step reads.
    fn begin(&mut self, agent: Option<Arc<Agent>>) {
        let (sender, receiver) = mpsc::channel();
        {
            let session = self.session.clone();
            std::thread::spawn(move || {
                let (read, reads) = match shaping::read_with_base(&session) {
                    Ok((read, base)) => {
                        match Steps::new(agent.as_deref(), &GoalConfirmation::default()).reads(
                            &session,
                            read.goal.as_deref(),
                            base.map(|base| base.checkout),
                        ) {
                            Ok(reads) => (Ok(read), reads),
                            Err(why) => (Err(why), None),
                        }
                    }
                    Err(why) => (Err(why), None),
                };
                let _ = sender.send((read, reads));
            });
        }
        self.reading = Reading::Waiting(receiver);
    }

    fn start(session: &Path, confirmation: GoalConfirmation) -> Self {
        Self {
            session: session.to_path_buf(),
            confirmation,
            reading: Reading::NotStarted,
            typed: Words::new(""),
            mode: Mode::Idle(Editor::Stage),
            notice: None,
            feedback: None,
            folder: Folder::Unknown,
            assumed: Vec::new(),
            withheld: None,
            revealed: false,
            read_end: None,
        }
    }
}

/// Refuses an oversized reading before wrapping; source bytes stay untouched.
fn reading_fits(frame: &mut Frame, texts: &[&Text<'_>], area: Rect, palette: Palette) -> bool {
    let bytes: usize = texts
        .iter()
        .flat_map(|text| &text.lines)
        .map(|line| {
            1 + line
                .spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>()
        })
        .sum();
    if bytes < 60_000 {
        return true;
    }
    frame.render_widget(
        Paragraph::new("Too long to show. Nothing was confirmed or changed. Esc goes back.")
            .style(palette.ink)
            .wrap(Wrap { trim: false }),
        area,
    );
    false
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
        let by = match &decided.authority {
            Authority::User => "you",
            Authority::Simulated => "practice",
            other => other.word(),
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
    let text = Text::from(lines);
    if !reading_fits(frame, &[&text], area, palette) {
        return;
    }
    let page = Paragraph::new(text).wrap(Wrap { trim: false });
    // How to close stays on the page's last row, even when the rest scrolls.
    let rows = page.line_count(area.width) as u16;
    let height = area.height.saturating_sub(2);
    *scroll = (*scroll).min(rows.saturating_sub(height));
    frame.render_widget(page.scroll((*scroll, 0)), Rect { height, ..area });
    let close = if rows > height {
        "↑↓ scroll · esc close"
    } else {
        "esc close"
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
/// scrolling only then, and how many of its rows show.
fn render_open(
    frame: &mut Frame,
    scroll: &mut u16,
    open: &mut Opened,
    drafting: Option<&Agent>,
    held: bool,
    area: Rect,
    palette: Palette,
) -> (bool, u16) {
    let agent = drafting.is_some();
    let area = Rect {
        width: area.width.min(WIDTH),
        ..area
    };
    if area.width == 0 || area.height == 0 {
        return (false, 1);
    }
    let rows = |text: &Text<'static>| {
        Paragraph::new(text.clone())
            .wrap(Wrap { trim: false })
            .line_count(area.width) as u16
    };
    let main_draft = !matches!(open.editor(), Editor::Aside(..));
    let confirming = agent
        && !matches!(open.mode, Mode::Choosing(_))
        && matches!(
            open.typing(),
            Some(Typing {
                act: Act::Confirm { .. },
                ..
            })
        );
    let mut composed = compose(open, drafting, held, palette);
    loop {
        let Composed {
            reading,
            before,
            after,
            editor,
            calm,
        } = composed;
        if !reading_fits(frame, &[&reading, &before, &after], area, palette) {
            open.read_end = None;
            return (false, 1);
        }
        let editor_rows = if editor {
            open.words().rows(area.width)
        } else {
            0
        };
        let (read_rows, before_rows, after_rows) = (rows(&reading), rows(&before), rows(&after));
        let input_rows = before_rows
            .saturating_add(editor_rows)
            .saturating_add(after_rows);
        let gap = u16::from(read_rows > 0 && input_rows > 0);
        // Reserve at least half the body for a long reading. The editor scrolls to its caret;
        // labels and consequences stay outside that viewport, even while a step is running.
        let input_height = input_rows
            .min(area.height.saturating_sub(
                gap + read_rows.min(area.height / 2).max(u16::from(read_rows > 0)),
            ));
        let height = read_rows.min(area.height.saturating_sub(input_height + gap));
        *scroll = (*scroll).min(read_rows.saturating_sub(height));
        if height > 0
            && open.read_end.is_none()
            && main_draft
            && *scroll >= read_rows.saturating_sub(height)
            && let Reading::Done(Ok(shaping)) = &open.reading
            && matches!(shaping.stage, Shaped::Drafted { text: Some(_), .. })
        {
            open.read_end = Some(shaping.stage.clone());
            // Reaching the end answers "read to the end first", from this very frame.
            if let Some(Notice::ReadFirst) = open.notice {
                open.notice = None;
                composed = compose(open, drafting, held, palette);
                continue;
            }
        }
        let calm = calm || matches!(open.mode, Mode::Running(..));
        let read_end = open.read_end.is_some();
        let editor = editor.then(|| open.words());
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
        let before_height = before_rows
            .min(input_height.saturating_sub(after_height + u16::from(editor.is_some())));
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
            editor.render(frame, input_area, palette, calm);
            if confirming && editor.text().is_empty() && read_end {
                frame.render_widget(
                    Paragraph::new("type confirm, then Enter").style(palette.muted),
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
        return (read_rows > height, height);
    }
}

/// An open session's screen before layout: what to read, then the lines above and below the words
/// box, and whether there is one.
struct Composed {
    reading: Text<'static>,
    before: Text<'static>,
    after: Text<'static>,
    editor: bool,
    /// Whether the words box only shows words, muted, and takes no keys.
    calm: bool,
}

fn compose(open: &Opened, drafting: Option<&Agent>, held: bool, palette: Palette) -> Composed {
    let agent = drafting.is_some();
    let shaping = match &open.reading {
        Reading::NotStarted | Reading::Waiting(_) => {
            return Composed {
                reading: Text::styled("Checking this Tink.", palette.muted),
                before: Text::default(),
                after: Text::default(),
                editor: false,
                calm: false,
            };
        }
        Reading::Done(Err(error)) => {
            return Composed {
                reading: plain(
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
                before: Text::default(),
                after: Text::default(),
                editor: false,
                calm: false,
            };
        }
        Reading::Done(Ok(shaping)) => shaping,
    };
    let editing_check = match open.editor() {
        Editor::Aside(Aside::RepairCheck(edit), _) => Some(&edit.target),
        _ => None,
    };
    let top = match (&shaping.stage, editing_check) {
        (_, Some(check)) => Some(Text::styled(
            format!("Edit check {}\n\n{}", check.index + 1, check.statement()),
            palette.ink,
        )),
        (
            Shaped::Drafted {
                revision,
                text: Some(_),
            },
            _,
        ) => shaping
            .seed_view()
            .map(|view| seed_view(view, revision.as_deref(), shaping, agent && !held, palette)),
        _ => shaping
            .goal
            .as_deref()
            .map(|goal| Text::styled(goal.lines().next().unwrap_or("").to_owned(), palette.ink)),
    };
    let name = open.folder_name();
    let typing = open.typing().filter(|_| agent);
    let act = typing.as_ref().map(|t| &t.act);
    let stale = open.stale_aside().filter(|_| agent);
    let words = typing.as_ref().map(|t| t.label).or(stale);
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
    let form = matches!(open.editor(), Editor::Form(_));
    let stand = match &open.mode {
        Mode::Running(running, work, _) => {
            let name = name.as_deref().unwrap_or_default();
            let said = doing(work, running.saved(), name);
            Stand {
                acts,
                reads: None,
                words,
                act,
                notice: None,
                stage_next: !form,
                running: Some(Line::from(vec![
                    Span::styled(said, palette.ink),
                    Span::styled(format!("  {}s", running.elapsed().as_secs()), palette.muted),
                ])),
                assumed: &open.assumed,
                instinct: open.instinct_first(),
            }
        }
        Mode::Choosing(_) => Stand {
            acts,
            reads: None,
            words: Some("Folder:"),
            act: None,
            notice: Some((
                "",
                Some(format!(
                    "Next: Enter uses this folder; Esc keeps {}.",
                    name.as_deref().unwrap_or_default()
                )),
            )),
            stage_next: true,
            running: None,
            assumed: &open.assumed,
            instinct: open.instinct_first(),
        },
        _ => Stand {
            acts,
            reads: (agent && open.offers_step()).then(|| match (&name, &open.folder) {
                (Some(name), Folder::Known(reads)) => {
                    let unread = reads
                        .unread()
                        .map(|why| format!(" {why}"))
                        .unwrap_or_default();
                    format!("The step reads files in {name}. F1 picks another folder.{unread}")
                }
                _ => "Finding which folder the step reads.".into(),
            }),
            words,
            act,
            notice: match stale {
                Some(_) => Some((STALE_ASIDE, Some(STALE_NEXT.into()))),
                None => open
                    .notice
                    .as_ref()
                    .map(Notice::parts)
                    .or(held.then_some((HELD, None))),
            },
            stage_next: !form,
            running: None,
            assumed: &open.assumed,
            instinct: open.instinct_first(),
        },
    };
    let (mut said, mut input, mut editor_at) = standing(shaping, stand, palette);
    if !matches!(open.mode, Mode::Running(..))
        && open.offers_answers()
        && let Some(drafting) = drafting.filter(|a| a.remaining() == 1)
    {
        input
            .lines
            .insert(0, Line::styled(drafting.left(), palette.muted));
        editor_at = editor_at.map(|at| at + 1);
    }
    if agent
        && !held
        && matches!(open.mode, Mode::Idle(Editor::Stage))
        && shaping.deferred.is_some()
    {
        said.lines.push(Line::styled(
            "F1 offers an explicit answer; no model runs.",
            palette.muted,
        ));
    }
    if let Some(note) = &shaping.confirmation_note {
        input.lines.push(Line::styled(note.clone(), palette.muted));
    }
    let mut reading = top.unwrap_or_default();
    if editing_check.is_none() && matches!(shaping.stage, Shaped::Drafted { .. }) {
        reading.lines.push(Line::default());
        reading
            .lines
            .push(Line::styled("Before you confirm", palette.muted));
        for line in &shaping.teach_back {
            reading
                .lines
                .extend(Text::styled(line.clone(), palette.ink).lines);
        }
        for line in shaping.review_lines(!shaping.checks.is_empty()) {
            reading.lines.extend(Text::styled(line, palette.ink).lines);
        }
        if let Some(signoff) = &shaping.signoff {
            reading
                .lines
                .push(Line::styled(signoff.clone(), palette.ink));
        }
    }
    if let Editor::Form(form) = open.editor() {
        reading.lines.extend(said.lines);
        reading.lines.push(Line::default());
        reading
            .lines
            .extend(Text::styled(form.description(), palette.ink).lines);
    } else {
        if !reading.lines.is_empty() && !said.lines.is_empty() {
            reading.lines.push(Line::default());
        }
        reading.lines.extend(said.lines);
    }
    if editing_check.is_none() && matches!(shaping.stage, Shaped::Drafted { text: Some(_), .. }) {
        reading.lines.push(Line::default());
        reading
            .lines
            .push(Line::styled("End of Tink Contract", palette.muted));
    }
    let (before, after) = match editor_at {
        Some(at) => {
            let after = input.lines.split_off(at + 1);
            (input, Text::from(after))
        }
        None => (Text::default(), input),
    };
    Composed {
        reading,
        before,
        after,
        editor: editor_at.is_some(),
        calm: stale.is_some(),
    }
}

/// Review the proposed checks first. Stored seed bytes and artifact order do not change.
fn seed_view(
    view: SeedView,
    revision: Option<&str>,
    shaping: &Shaping,
    editable: bool,
    palette: Palette,
) -> Text<'static> {
    let mut lines = vec![
        Line::styled(
            revision.map_or_else(
                || "Tink Contract".to_owned(),
                |r| format!("Tink Contract {r}"),
            ),
            palette.muted,
        ),
        Line::default(),
    ];
    if let Some(criteria) = view.criteria {
        lines.push(Line::styled(
            "Pass/fail checks (proposed)",
            palette.ink.bold(),
        ));
        match criteria {
            Criteria::Readable(readable) => {
                lines.extend(Text::styled(readable, palette.ink).lines);
                lines.push(Line::default());
            }
            Criteria::Checks(checks) => {
                for check in checks {
                    let n = check.index + 1;
                    lines.extend(
                        Text::styled(format!("{n}. {}", check.statement()), palette.ink.bold())
                            .lines,
                    );
                    if let Some(gap) = check.gap() {
                        let edit = if editable {
                            format!("  F1: Edit check {n}.")
                        } else {
                            String::new()
                        };
                        lines.push(Line::styled(
                            format!("{}.{edit}", gap.missing()),
                            palette.ink,
                        ));
                    } else if editable {
                        lines.push(Line::styled(format!("F1: Edit check {n}."), palette.muted));
                    }
                    if let Some(words) = check.words() {
                        let method = match check.method() {
                            CheckMethod::Command => "Command",
                            CheckMethod::Manual => "Manual steps",
                        };
                        lines.extend(Text::styled(format!("{method}: {words}"), palette.ink).lines);
                    }
                    if let Some(cwd) = check.cwd() {
                        lines.push(Line::styled(format!("From: {cwd}"), palette.muted));
                    }
                    if let Some(expected) = check.expected() {
                        lines.extend(Text::styled(format!("Pass: {expected}"), palette.ink).lines);
                    }
                    if let Some(failure) = shaping.failure_case(check) {
                        lines.extend(
                            Text::styled(format!("Must fail: {failure}"), palette.ink).lines,
                        );
                    }
                    if let Some(prerequisite) = check.prerequisite() {
                        lines.extend(
                            Text::styled(
                                format!("Needed before running: {prerequisite}"),
                                palette.muted,
                            )
                            .lines,
                        );
                    }
                    lines.push(Line::default());
                }
            }
        }
    }
    lines.extend(view.prose.into_iter().map(|line| match line {
        SeedLine::Heading(heading) => Line::styled(heading.to_owned(), palette.ink.bold()),
        SeedLine::Words(words) => Line::styled(words.to_owned(), palette.ink),
    }));
    Text::from(lines)
}

/// What choosing an ending asks, and how to answer it.
fn asked(ending: Ending) -> Notice {
    match ending {
        Ending::Completed => Notice::Said(
            "End this session as completed? It becomes read-only for good, and tend no longer lists it; Approve still offers its Tink.".into(),
            Some("Next: Enter ends it; Esc keeps it open.".into()),
        ),
        Ending::Stopped => Notice::Said(
            "Stop this session? It becomes read-only for good and tend no longer lists it; its open questions stay recorded.".into(),
            Some("Next: Enter stops it; Esc keeps it open.".into()),
        ),
    }
}

/// What `work` says while it runs: before the operator's words are `saved`, then after, with
/// `name` the folder a look reads.
fn doing(work: &Work, saved: bool, name: &str) -> String {
    match (work, saved) {
        (Work::Answer { .. }, false) => "Saving your answer.".into(),
        (Work::Answer { .. }, true) => {
            format!("Answer saved. Looking in {name} for the next question.")
        }
        (Work::Respond { action, .. }, false) => match action {
            AnswerAction::Defer { .. } => "Saving your deferral.",
            AnswerAction::Delegate => "Saving the delegated suggestion.",
            AnswerAction::Revise(_) | AnswerAction::Revisit(_) => {
                "Revising your answer and reopening dependents."
            }
        }
        .into(),
        (Work::Respond { .. }, true) => {
            format!(
                "{}Looking in {name} for the next question.",
                self::saved(work)
            )
        }
        (Work::Look { .. }, _) => format!("Looking in {name} for the next question."),
        (Work::TeachBack { .. }, false) => "Saving your teach-back.".into(),
        (Work::TeachBack { .. }, true) => "Teach-back saved. Comparing with the draft.".into(),
        (Work::Change { .. }, false) => "Saving your change.".into(),
        (Work::Change { .. }, true) => format!("Change saved. Redrafting the Tink from {name}."),
        (Work::Confirm { .. }, _) => "Confirming the Tink.".into(),
        (Work::RepairCheck { .. }, _) => "Saving your check. No model runs.".into(),
        (Work::Revisit { .. }, _) => "Saving your answer. No model runs.".into(),
        (Work::Keep { .. }, _) => "Saving your reason.".into(),
        (Work::End(_), _) => "Ending the session.".into(),
        (Work::Object { .. }, _) => "Saving your objection.".into(),
        (Work::Recommend { .. }, _) => "Saving my suggestion.".into(),
    }
}

/// Which of the operator's words a step saved before it stopped short.
fn saved(work: &Work) -> &'static str {
    match work {
        Work::Respond { action, .. } => match action {
            AnswerAction::Defer { .. } => "Your deferral is saved. ",
            AnswerAction::Delegate => "Your delegated answer is saved. ",
            AnswerAction::Revise(_) | AnswerAction::Revisit(_) => "Your revised answer is saved. ",
        },
        Work::TeachBack { .. } => "Your teach-back is saved. ",
        Work::Change { .. } => "Your change is saved. ",
        _ => "Your answer is saved. ",
    }
}

/// `left` is the allowance, added where the operator decides whether to retry (#435).
fn said(result: Result<Done, Failure>, work: &Work, left: Option<String>) -> Option<Notice> {
    if result == Ok(Done::Compared) {
        return Some(Notice::said(
            "Teach-back compared. Mismatches never block confirming.",
        ));
    }
    let Failure {
        answer_saved,
        why,
        feedback,
    } = result.err()?;
    let saved = if answer_saved { saved(work) } else { "" };
    let left = left.map_or_else(String::new, |left| format!(" {left}"));
    let (said, next) = match why {
        Why::Stale if answer_saved => (
            format!(
                "{saved}Another writer changed this session before the next step, so Shape saved nothing more and read it again."
            ),
            None,
        ),
        Why::Stale => (
            "Another writer changed this session first, so nothing was saved. Shape read it again."
                .into(),
            matches!(
                work,
                Work::Respond {
                    action: AnswerAction::Defer { .. }
                        | AnswerAction::Revise(_)
                        | AnswerAction::Revisit(_),
                    ..
                }
            )
            .then(|| {
                "Next: Esc cancels this form; open it again from the refreshed question.".into()
            }),
        ),
        Why::Budget(limit) => (
            format!(
                "{saved}No drafting responses are left: this launch's allowance is {limit}. Nothing {}was saved.",
                if answer_saved { "more " } else { "" }
            ),
            Some("Next: add responses from F1, Model and responses, then send again.".into()),
        ),
        Why::Model(error) => (
            format!(
                "{saved}{}: {error}{left}",
                if matches!(work, Work::TeachBack { .. }) {
                    "Comparing your teach-back failed"
                } else {
                    "Looking into the next question failed"
                }
            ),
            None,
        ),
        // A rule the model's reply broke: the retry hears the full reason; the screen says
        // whose reply it was and the rule, without the ledger's node names (#649).
        Why::Refused(reason) | Why::Rule(answering::Rule { rule: reason, .. })
            if feedback.is_some() =>
        {
            (
                format!(
                    "{saved}The model's reply was refused, so nothing from it was saved: {}.{left}",
                    reason.trim_end_matches('.')
                ),
                None,
            )
        }
        Why::Refused(reason) => (format!("{saved}{reason}{left}"), None),
        Why::Rule(rule) => (format!("{saved}{rule}{left}"), None),
        Why::Stopped => (
            format!(
                "{saved}Stopped before the next step finished. The drafting attempt still counts against this launch's allowance.{left}"
            ),
            None,
        ),
    };
    Some(Notice::Said(said, next))
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
    /// What typed words would do, when the words box is the stage's or an aside's.
    act: Option<&'a Act>,
    /// What the last key or step said, and its own next step if any.
    notice: Option<(&'a str, Option<String>)>,
    /// Whether the stage's own Next line may show; a form's description says what to do instead.
    stage_next: bool,
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
/// stay, muted, under what the step is doing. Also returns the index of the input line the words
/// box follows, when there is one.
fn standing(
    shaping: &Shaping,
    stand: Stand,
    palette: Palette,
) -> (Text<'static>, Text<'static>, Option<usize>) {
    const SETUP: &str = "Next: reopen this goal with Shape set up; F1 shows how.";
    const INSPECT: &str = "Next: check this session's files; F1 shows the path.";
    let Stand {
        acts,
        reads,
        words,
        act,
        notice,
        stage_next,
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
                "Next: type why, then Enter keeps the answer unchanged with your reason.".into()
            } else {
                SETUP.into()
            })
        }
        Shaped::Question {
            id,
            title,
            ask,
            gate,
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
                // Tinkery's own probe and objection questions ask for the operator's words, not evidence.
                let own =
                    [PROBE.0, SAMPLE.0, UNOBSERVED.0].contains(&id.as_str()) || title == OBJECTION;
                if !own && !gate.is_empty() && gate != ask {
                    lines.push(Line::styled(
                        format!("What your answer changes: {gate}"),
                        palette.ink,
                    ));
                }
                lines.push(Line::default());
                if found.is_empty() && !own {
                    lines.push(Line::styled(
                        "Nothing found: this question records nothing checked in the repository.",
                        palette.ink,
                    ));
                }
                if id == PROBE.0 {
                    lines.push(Line::styled(
                        "Answer in your own words, or type skip and send to skip it.",
                        palette.ink,
                    ));
                }
                if id == SAMPLE.0 {
                    lines.push(Line::styled(
                        "Checks are drafted against this. Paste an excerpt, or type no sample and why.",
                        palette.ink,
                    ));
                }
                if id == UNOBSERVED.0 {
                    lines.push(Line::styled(
                        "F1 can stop this session instead.",
                        palette.ink,
                    ));
                }
                if let Some(entry) = found.first() {
                    lines.push(Line::styled(format!("What I found: {entry}"), palette.ink));
                }
                if instinct {
                    lines.push(Line::styled(
                        "This is hard to undo, so what's your instinct? F1 shows the options and my suggestion.",
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
                "Next: type your answer, then Enter saves it and looks into the next question."
                    .into()
            } else {
                SETUP.into()
            })
        }
        Shaped::Waiting => {
            if let Some(target) = &shaping.deferred {
                lines.push(Line::styled(
                    format!("Deferred prerequisite: {}", target.title),
                    palette.ink,
                ));
            }
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
                "Next: Enter looks into the next question.".into()
            } else {
                SETUP.into()
            })
        }
        Shaped::Drafted { revision, text } => {
            // When the draft itself is shown, its header already says this.
            if text.is_none() {
                lines.push(Line::styled(
                    match revision {
                        Some(r) => format!("Tink Contract {r} is waiting for you to confirm it."),
                        None => "A draft Tink Contract is waiting for you to confirm it.".into(),
                    },
                    palette.ink,
                ));
            }
            // Beside the words box, so a long seed can't scroll it away.
            if !calm && shaping.seed_view().is_some_and(|view| view.nothing_found) {
                flag = Some(Line::styled(
                    "Nothing found: this Tink Contract records nothing checked in the repository.",
                    palette.ink,
                ));
            }
            match (acts, revision, act) {
                (true, _, Some(Act::TeachBack { .. })) => {
                    Some("Next: Enter saves your words and compares with the draft.".into())
                }
                (true, _, Some(Act::Change { .. })) => {
                    Some("Next: Enter saves your change and redrafts the Tink.".into())
                }
                // The key bar owns the next action, including while a refusal is shown.
                (true, Some(_), _) => None,
                (false, Some(_), _) => Some(SETUP.into()),
                _ => Some(INSPECT.into()),
            }
        }
        Shaped::Ready { revision, ended } => {
            lines.push(Line::styled(
                match revision {
                    Some(r) => format!("Tink ready: you confirmed {r}, and it still matches."),
                    None => "Tink ready: what you confirmed still matches.".into(),
                },
                palette.ink,
            ));
            if let Some(signoff) = &shaping.signoff {
                lines.push(Line::styled(signoff.clone(), palette.ink));
            }
            if *ended {
                lines.push(Line::styled(
                    "This session is completed, so nothing in it can change.",
                    palette.muted,
                ));
            }
            Some(if shaping.ready_target.is_some() {
                "Next: F1, then 1: Open this Tink in Approve.".into()
            } else {
                "Next: open Approve from the menu to inspect this Tink.".into()
            })
        }
        Shaped::Unverified { reason } => {
            lines.push(Line::styled(
                format!("Tinkery cannot verify this Tink: {reason}."),
                palette.ink,
            ));
            Some(INSPECT.into())
        }
        Shaped::Practice => {
            lines.push(Line::styled("This Tink is for practice only.", palette.ink));
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
                format!("Shape cannot check this Tink. {what}"),
                palette.ink,
            ));
            Some(INSPECT.into())
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
    let mut editor_at = None;
    if let Some(label) = words {
        let style = if calm { palette.muted } else { palette.ink };
        gap(&mut input);
        input.push(Line::styled(label.to_owned(), style));
        editor_at = Some(input.len() - 1);
        if matches!(act, Some(Act::Confirm { .. }))
            && acts
            && !calm
            && matches!(shaping.stage, Shaped::Drafted { .. })
        {
            input.push(Line::default());
            input.push(Line::styled(
                "Confirm the Tink here. Approve the build separately.",
                palette.ink,
            ));
        }
    }
    if let Some(running) = running {
        gap(&mut input);
        input.push(running);
        return (Text::from(lines), Text::from(input), editor_at);
    }
    let next = next.filter(|_| stage_next);
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
    (Text::from(lines), Text::from(input), editor_at)
}

#[cfg(test)]
mod tests;
