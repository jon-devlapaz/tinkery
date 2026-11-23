//! Tend: the saved goals and repository thoughts garden, with read-only original detail.
//!
//! Shows what discovery has landed, says plainly when that is unknown, incomplete or failed,
//! and offers explicit needs links and saved Think history; it never edits a session,
//! starts an agent or guesses a stage. Everything about session files comes from `seed_me`.

pub(crate) mod connection;
pub mod discovery;
mod garden;
mod graph;
pub(crate) mod history;
mod linking;
pub mod needs;
pub(crate) mod thought;
pub use discovery::{Discovery, Found, Identities, Outcome, Source, scan};

use crate::clipped;
pub use crate::seed_me::{Session, Stage};
use crate::{
    Palette,
    goal::confirmed::{SavedGoal, render_saved_goal},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use unicode_width::UnicodeWidthStr;

/// How long a read may be pending before the quiet line says it is still going. A normal
/// read takes milliseconds (about 70 ms for 5,000 sessions on the measured machine), so
/// this only shows when something is actually slow, such as a cold or network disk.
pub(super) const STILL_READING_AFTER: Duration = Duration::from_secs(3);

/// Builds the Find connections request when `f` is pressed, or says why it cannot. The menu
/// supplies it because only the menu knows the launch's drafting model and operator.
pub(crate) type Connect = Box<dyn Fn() -> Result<connection::Context, String> + Send>;

/// What Tend asks the menu to do after a key. Tend opens Think or Shape only on the operator's
/// own key, and never starts an agent itself.
pub(crate) enum Next {
    /// Tend keeps the screen.
    Stay,
    /// Back to the menu.
    Menu,
    /// Open Think: with the thought reader's unsent copy, or fresh from `t` (`None`).
    Think(Option<String>),
    /// Open Shape on this saved session.
    Shape(PathBuf),
}

/// The help layer over Tend or its history: none, "Right now", or "How it works".
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Overlay {
    Closed,
    Help,
    How,
}

#[derive(Clone, PartialEq, Eq)]
enum Entry {
    Goal(PathBuf),
    Thought(String),
}

impl Entry {
    fn graph_id(&self) -> String {
        match self {
            Self::Goal(path) => format!("{path:?}"),
            Self::Thought(words) => format!("thought:{words}"),
        }
    }
}

pub(crate) struct Tend {
    sessions: Vec<Session>,
    error: Option<String>,
    /// Nothing has landed yet: unknown, never "no sessions".
    pending: bool,
    /// The read this view waits on has run past [`STILL_READING_AFTER`]. Only the pending
    /// line reads it, and only its wording changes.
    still_reading: bool,
    /// The list shown reflects every read asked for. An older empty list is not proof
    /// that nothing was saved since.
    current: bool,
    /// A saved session to open in detail once a read that can contain it lands.
    want: Option<(PathBuf, Option<PathBuf>)>,
    /// Records the last read could not parse; the list may be missing them.
    unreadable: usize,
    /// Readable ended sessions omitted from the garden.
    ended: usize,
    /// The discovery version last applied.
    seen: u64,
    selected: usize,
    detail: bool,
    scroll: u16,
    /// Rows of goal on screen at the last detail draw, for PgUp/PgDn.
    page: u16,
    overlay: Overlay,
    help_scroll: u16,
    notice: String,
    history: Option<history::View>,
    history_reader: history::Reader,
    records: Result<PathBuf, String>,
    connection: Option<connection::Panel>,
    connect: Connect,
    thoughts: Option<thought::Garden>,
    graph: graph::GraphView,
    last_press: Option<(Entry, Instant)>,
    hidden_thought: Option<Entry>,
    needs: needs::Store,
    links: Result<needs::Links, String>,
    all_sessions: Vec<Session>,
    ids: Identities,
    /// Only exact recorded repo matches; absent outside a repository.
    repo_paths: Option<HashSet<PathBuf>>,
    /// Active goals with unknown repository attribution, not known non-matches.
    repo_unknown: usize,
    show_all: bool,
    /// The source path stays fixed while selection picks the other goal.
    linking: Option<PathBuf>,
    link_error: Option<String>,
}
impl Tend {
    /// Whether automatic discovery is requested from this repository's garden.
    fn connection_scope(&self) -> bool {
        self.repo_paths.is_some() && !self.show_all
    }

    /// A pending view with a captured Tend store folder or refusal. Reads only the saved thoughts
    /// of `repo`, when Tinkery started in one, and no links; writes nothing. History reads
    /// `records` only after `r`; `connect` runs only after `f`.
    pub(crate) fn new(
        link_root: Result<PathBuf, String>,
        repo: Option<&Path>,
        records: Result<PathBuf, String>,
        connect: Connect,
    ) -> Self {
        Self {
            sessions: vec![],
            error: None,
            pending: true,
            still_reading: false,
            current: false,
            want: None,
            unreadable: 0,
            ended: 0,
            seen: 0,
            selected: 0,
            detail: false,
            scroll: 0,
            page: 1,
            overlay: Overlay::Closed,
            help_scroll: 0,
            notice: String::new(),
            history: None,
            history_reader: history::Reader::default(),
            records,
            connection: None,
            connect,
            graph: graph::GraphView::default(),
            last_press: None,
            hidden_thought: None,
            thoughts: repo.map(|repo| thought::Garden::open(link_root.clone(), repo.to_path_buf())),
            needs: needs::Store::new(link_root),
            links: Ok(needs::Links::default()),
            all_sessions: vec![],
            ids: Identities::default(),
            repo_paths: None,
            repo_unknown: 0,
            show_all: false,
            linking: None,
            link_error: None,
        }
    }
    /// Take a landed read. Selection follows the session's path, not its position.
    /// A wanted session is opened only from a read that contains it (or that began
    /// after it was asked for); until then the view stays pending.
    fn apply(&mut self, outcome: &Outcome, fresh: bool, version: u64) {
        let wanted_at = match (&self.want, outcome) {
            (Some((path, canonical)), Ok(found)) => found
                .sessions
                .iter()
                .position(|s| s.path == *path || canonical.as_ref().is_some_and(|c| s.path == *c)),
            _ => None,
        };
        if self.want.is_some() && wanted_at.is_none() && !fresh {
            return;
        }
        self.cancel_link();
        self.links = self.needs.read().into_links();
        if self.notice.is_empty() {
            self.link_error = self.links.as_ref().err().cloned();
        }
        let first_read = self.pending;
        self.seen = version;
        self.pending = false;
        match outcome {
            Err(error) => {
                self.error = Some(error.clone());
                self.sessions.clear();
                self.all_sessions.clear();
                self.ids = Identities::default();
                self.unreadable = 0;
                self.ended = 0;
                self.want = None;
            }
            Ok(found) => {
                let keep = if self.connection_scope() {
                    self.hidden_thought
                        .take()
                        .or_else(|| (!first_read).then(|| self.selected_entry()).flatten())
                } else if first_read {
                    None
                } else {
                    self.selected_entry()
                };
                if let Some(thoughts) = &mut self.thoughts {
                    thoughts.refresh();
                }
                self.error = None;
                self.unreadable = found.unreadable;
                self.all_sessions = found.sessions.clone();
                self.ids = found.identities();
                if let Some(index) = wanted_at
                    && self
                        .repo_paths
                        .as_ref()
                        .is_some_and(|paths| !paths.contains(&found.sessions[index].path))
                {
                    // An explicit receipt/Shape return must still open its exact goal.
                    self.show_all = true;
                }
                self.filter_sessions();
                if let Some(index) = wanted_at {
                    let path = &found.sessions[index].path;
                    let active = self.sessions.iter().position(|s| s.path == *path);
                    self.selected = active.unwrap_or(0);
                    self.detail = active.is_some();
                    self.scroll = 0;
                    self.want = None;
                } else if let Some((path, _)) = self.want.take() {
                    self.error = Some(format!("Saved session is unavailable: {}", path.display()));
                    self.sessions.clear();
                } else {
                    self.select_entry(keep);
                }
            }
        }
        self.prepare_graph();
    }
    fn filter_sessions(&mut self) {
        let visible = |s: &&Session| {
            self.show_all
                || self
                    .repo_paths
                    .as_ref()
                    .is_none_or(|paths| paths.contains(&s.path))
        };
        self.ended = self
            .all_sessions
            .iter()
            .filter(visible)
            .filter(|s| s.stage.ended())
            .count();
        self.sessions = self
            .all_sessions
            .iter()
            .filter(visible)
            .filter(|s| !s.stage.ended())
            .cloned()
            .collect();
    }

    fn repo_empty_message(&self) -> Option<String> {
        if self.repo_paths.is_none() || self.show_all {
            return None;
        }
        if self.repo_unknown > 0 {
            let goals = if self.repo_unknown == 1 {
                "goal has"
            } else {
                "goals have"
            };
            Some(format!(
                "{} {goals} no readable repo record. Press a for all goals.",
                self.repo_unknown
            ))
        } else if self.unreadable == 0 {
            Some("No active goals are recorded for this repo. Press a for all goals.".into())
        } else {
            Some(format!(
                "{} couldn't be read; this repo may have active goals. Press a for all goals.",
                records(self.unreadable)
            ))
        }
    }

    /// Whether the "this repo / all" row sits above the garden.
    fn scope_row(&self) -> bool {
        self.can_toggle_scope() || self.is_linking() && self.repo_paths.is_some()
    }

    fn scope_body(&self, area: Rect) -> Rect {
        if self.scope_row() {
            Rect::new(
                area.x,
                area.y.saturating_add(1),
                area.width,
                area.height.saturating_sub(1),
            )
        } else {
            area
        }
    }

    fn can_toggle_scope(&self) -> bool {
        self.repo_paths.is_some()
            && !self.pending
            && (self.current || !self.sessions.is_empty() || self.unreadable > 0)
            && !self.detail
            && self.want.is_none()
            && self.error.is_none()
            && !self.is_linking()
    }

    /// Change only the visible garden, retaining global needs and exact selection when possible.
    fn toggle_scope(&mut self) {
        if !self.can_toggle_scope() {
            return;
        }
        let mut keep = self.selected_entry();
        if matches!(keep, Some(Entry::Thought(_))) {
            self.hidden_thought = keep.clone();
        } else if self.show_all && self.hidden_thought.is_some() {
            keep = self.hidden_thought.take();
        }
        self.show_all = !self.show_all;
        self.filter_sessions();
        self.prepare_graph();
        self.select_entry(keep);
        self.last_press = None;
        self.graph.release();
        self.notice.clear();
    }

    /// Whether the mixed garden is showing, rather than detail or a discovery state.
    fn garden_showing(&self) -> bool {
        !self.detail
            && !self.pending
            && self.want.is_none()
            && self.error.is_none()
            && self.entry_count() > 0
    }
    /// One mouse event over `body`. Pointer mechanics go to graf; only a timely second press
    /// opens an original, and a press while picking links. Help and readers take no pointer.
    pub(crate) fn pointer(&mut self, event: MouseEvent, body: Rect, now: Instant) {
        if self.connection.is_some()
            || self.thoughts.as_ref().is_some_and(|t| t.active())
            || self.overlay != Overlay::Closed
        {
            return;
        }
        if !self.garden_showing() {
            self.graph.release();
            return;
        }
        let body = self.scope_body(body);
        self.sync_graph(body);
        if matches!(event.kind, MouseEventKind::Drag(_)) {
            self.last_press = None;
        }
        let Some(id) = self.graph.pointer(event) else {
            return;
        };
        let entry = self
            .sessions
            .iter()
            .map(|s| Entry::Goal(s.path.clone()))
            .chain(self.thought_entries().iter().cloned().map(Entry::Thought))
            .find(|entry| entry.graph_id() == id);
        let Some(entry) = entry else { return };
        if self.linking.is_some() {
            if matches!(entry, Entry::Thought(_)) {
                self.graph.release();
                return;
            }
            self.select_entry(Some(entry));
            self.commit_link();
            self.graph.release();
            return;
        }

        let double = self.last_press.as_ref().is_some_and(|(last, at)| {
            *last == entry
                && now
                    .checked_duration_since(*at)
                    .is_some_and(|elapsed| elapsed <= Duration::from_millis(500))
        });
        self.select_entry(Some(entry.clone()));
        self.last_press = Some((entry, now));
        if double {
            self.open_entry();
            self.last_press = None;
            self.graph.release();
        }
    }
    fn current(&self) -> Option<&Session> {
        self.sessions.get(self.selected)
    }
    fn help_text(&self) -> String {
        if self.overlay == Overlay::How {
            let session = self.current().map_or(String::new(), |s| {
                format!(
                    "\nSession: {}\nStatus: {}",
                    s.path.display(),
                    s.stage.label()
                )
            });
            return format!(
                "How it works\nh or Esc returns to Help · PgUp/PgDn scroll\n{session}\n{}\n{}\n{}\n\nNothing starts on its own.",
                self.error.as_deref().unwrap_or(""),
                self.link_error.as_deref().unwrap_or(""),
                self.graph.error.as_deref().unwrap_or("")
            );
        }
        let needs = self.needs_text();
        let needs = if needs.is_empty() {
            String::new()
        } else {
            format!("\n\n{}", needs.trim_start_matches(" · "))
        };
        format!(
            "Right now\n{}\nEsc back · F10 menu\n\n\
             F1 close · h how it works · PgUp/PgDn scroll{needs}\n\n\
             Nothing runs by itself. Confirm the Tink and approve the build separately.",
            self.keys().0
        )
    }

    /// Tend's keys for the state on screen, said once: the help's "Right now" lines and the
    /// key bar come from the same branch.
    fn keys(&self) -> (String, String) {
        let here = self.connection_scope();
        let scope = if self.can_toggle_scope() {
            "\na toggles this repo / all goals"
        } else {
            ""
        };
        let thoughts = if here {
            "\nt think · v read thoughts\ng in the thought reader opens Think, unsent."
        } else {
            ""
        };
        if self.linking.is_some() {
            (
                "Arrows / j k pick · Enter links or unlinks · Esc cancels.".into(),
                format!("↑↓←→ pick   enter {}   esc cancel", self.link_verb()),
            )
        } else if self.detail {
            let read =
                "Read the saved goal. ↑↓ / j k or PgUp/PgDn scroll.\nr opens its Think history.";
            if self.current().is_some_and(|s| s.stage.can_shape()) {
                (
                    format!("{read}\ns shape: answer its questions and confirm its Tink"),
                    "s shape   r history   esc back   F10 menu".into(),
                )
            } else {
                (read.into(), "r history   esc back   F10 menu".into())
            }
        } else if self.garden_showing() {
            let help = concat!(
                "Arrows / j k move · Enter or a double click opens the selected original.\n",
                "l link or unlink\n",
                "Up / Left / k previous · Down / Right / j next (goals, then thoughts).\n",
                "+ / - zoom · H J K L pan · drag a node to move it.\n",
                "Drag empty space to pan · scroll to zoom",
            );
            let bar = if here {
                "↑↓←→ move   enter open   f connections   l link   esc back   t think"
            } else {
                "↑↓←→ move   enter open   l link   esc back"
            };
            (format!("{help}{scope}{thoughts}"), bar.into())
        } else {
            // An empty repo view still offers the way back to all goals.
            let bar = if here {
                "f connections   a toggle goals   esc back   t think"
            } else if self.can_toggle_scope() {
                "a toggle goals   esc back"
            } else {
                "esc back"
            };
            (
                format!("Esc returns to the menu.{scope}{thoughts}"),
                bar.into(),
            )
        }
    }
}

/// Tend's place surface: what the menu hands in and gets back. Tend owns its keys, help,
/// readers and refresh rules; the menu only routes input and opens what [`Next`] names.
impl Tend {
    /// Open the saved session at `path` in detail once a read that can contain it lands.
    pub(crate) fn open_session(&mut self, path: &Path) {
        // One stat of a path the handoff just wrote; matching is otherwise by identity.
        self.want = Some((path.to_path_buf(), path.canonicalize().ok()));
        self.seen = 0;
    }

    /// Back from Shape: the goal that was showing opens again once a fresh read lands.
    pub(crate) fn returned(&mut self) {
        self.want = self.current().map(|s| (s.path.clone(), None));
    }

    /// Show the newest landed read. Detail and link picking stay fixed unless a goal was asked
    /// for again; that request waits for a fresh read.
    pub(crate) fn offer(&mut self, discovery: &Discovery) {
        self.current = discovery.fresh();
        let Some(landed) = discovery.landed() else {
            return;
        };
        let version = discovery.version();
        if self.seen == version
            || ((self.detail || self.is_linking()) && (self.want.is_none() || !discovery.fresh()))
        {
            return;
        }
        self.repo_paths = landed.repo_paths.clone();
        self.repo_unknown = landed.repo_unknown;
        self.apply(&landed.outcome, discovery.fresh(), version);
    }

    /// Advance Tend's own reads without waiting, and say when the session read runs long
    /// (wording only). Returns the receipt of a goal saved from Find connections; the menu
    /// then opens Shape on it.
    pub(crate) fn poll(
        &mut self,
        discovery: &Discovery,
        now: Instant,
    ) -> Option<crate::seed_me::Receipt> {
        if let Some(receipt) = self.connection.as_mut().and_then(|panel| panel.poll()) {
            self.connection = None;
            if let Some(thoughts) = &mut self.thoughts {
                thoughts.clear_save_notice();
            }
            return Some(receipt);
        }
        self.offer(discovery);
        self.still_reading = discovery
            .running_since()
            .is_some_and(|since| now.saturating_duration_since(since) >= STILL_READING_AFTER);
        if let Some(result) = self.history_reader.poll()
            && let Some(history) = &mut self.history
        {
            history.finish(result);
        }
        None
    }

    /// Paste into the Find connections panel only; the thought reader never accepts edits.
    pub(crate) fn paste(&mut self, text: &str) {
        if self.thoughts.as_ref().is_some_and(|t| t.active()) {
            return;
        }
        if let Some(panel) = &mut self.connection {
            panel.paste(text);
        }
    }

    /// One key press. Held or modified keys never open, link, toggle or read; only `f`, `r`,
    /// `s`, `t` and the thought reader's `g` reach past the garden, and none starts an agent.
    pub(crate) fn key(&mut self, key: KeyEvent) -> Next {
        let plain = key.kind == KeyEventKind::Press && key.modifiers == KeyModifiers::NONE;
        let help = self.overlay != Overlay::Closed;
        let garden_key = plain
            && !help
            && self.connection.is_none()
            && !self.detail
            && !self.is_linking()
            && self.connection_scope();
        if let Some(thoughts) = &mut self.thoughts {
            if thoughts.active() {
                if let Some(words) = thoughts.key(key) {
                    return Next::Think(Some(words));
                }
                if !thoughts.active() {
                    self.thought_reader_return();
                }
                return Next::Stay;
            }
            match key.code {
                KeyCode::Char('t') if garden_key => return Next::Think(None),
                KeyCode::Char('v') if garden_key => {
                    thoughts.view();
                    return Next::Stay;
                }
                _ => {}
            }
        }
        if let Some(panel) = &mut self.connection {
            if panel.key(key) {
                self.connection = None;
            }
            return Next::Stay;
        }
        if plain && key.code == KeyCode::Char('f') && !self.detail && !help {
            self.notice = if self.garden_showing() && self.current().is_none() {
                "Find connections uses goals, not thoughts.".into()
            } else if !self.connection_scope() {
                "Find connections is available only in this repository's garden.".into()
            } else {
                match (self.connect)() {
                    Ok(context) => {
                        self.connection = Some(connection::Panel::open(context));
                        return Next::Stay;
                    }
                    Err(reason) => reason,
                }
            };
            return Next::Stay;
        }
        let shifted_view_key = key.modifiers == KeyModifiers::SHIFT
            && matches!(
                key.code,
                KeyCode::Char('H' | 'J' | 'K' | 'L' | '+' | '=' | '-')
            );
        if key.modifiers != KeyModifiers::NONE && !shifted_view_key
            || key.kind == KeyEventKind::Repeat
                && matches!(
                    key.code,
                    KeyCode::Char('a' | 'c' | 'r' | 'l' | 's')
                        | KeyCode::F(1)
                        | KeyCode::F(10)
                        | KeyCode::Enter
                )
        {
            return Next::Stay;
        }
        self.graph.release();
        let how = self.overlay == Overlay::How;
        match key.code {
            KeyCode::F(10) => return Next::Menu,
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.overlay = if help { Overlay::Closed } else { Overlay::Help };
                self.help_scroll = 0;
                if let Some(history) = &mut self.history {
                    history.reset_help();
                }
                return Next::Stay;
            }
            KeyCode::Char('h') | KeyCode::Esc if help => {
                self.overlay = match (key.code, how) {
                    (KeyCode::Esc, false) => Overlay::Closed,
                    (_, false) => Overlay::How,
                    (_, true) => Overlay::Help,
                };
                self.help_scroll = 0;
                if let Some(history) = &mut self.history
                    && self.overlay != Overlay::Closed
                {
                    history.reset_help();
                }
            }
            code if help => match (&mut self.history, code) {
                (Some(history), code) => history.scroll(code, self.overlay),
                (None, KeyCode::Down | KeyCode::PageDown) => {
                    self.help_scroll = self.help_scroll.saturating_add(5)
                }
                (None, KeyCode::Up | KeyCode::PageUp) => {
                    self.help_scroll = self.help_scroll.saturating_sub(5)
                }
                _ => {}
            },
            KeyCode::Esc if self.history.is_some() => {
                self.history_reader.cancel();
                self.history = None;
            }
            code if self.history.is_some() => {
                if let Some(history) = &mut self.history {
                    history.scroll(code, Overlay::Closed);
                }
            }
            KeyCode::Char('r') if self.detail && self.current().is_some() => {
                let session = &self.sessions[self.selected];
                let mut history = history::View::waiting();
                match &self.records {
                    Ok(root) => self.history_reader.request(
                        root.clone(),
                        session.path.clone(),
                        session.goal.clone(),
                    ),
                    Err(reason) => history.finish(Err(crate::think_record::Unavailable {
                        message: "The Think record folder is not configured.".into(),
                        details: reason.clone(),
                    })),
                }
                self.history = Some(history);
            }
            KeyCode::Char('a') => self.toggle_scope(),
            KeyCode::Char('l') => self.start_link(),
            KeyCode::Esc if self.is_linking() => self.cancel_link(),
            KeyCode::Enter if self.is_linking() => self.commit_link(),
            KeyCode::Esc if self.detail => {
                self.detail = false;
                self.want = None;
                self.scroll = 0;
                self.notice.clear();
            }
            KeyCode::Esc => return Next::Menu,
            KeyCode::Char('s')
                if self.detail && self.current().is_some_and(|s| s.stage.can_shape()) =>
            {
                if let Some(thoughts) = &mut self.thoughts {
                    thoughts.clear_save_notice();
                }
                return Next::Shape(self.sessions[self.selected].path.clone());
            }
            KeyCode::Down | KeyCode::Char('j') if self.detail => {
                self.scroll = self.scroll.saturating_add(1)
            }
            KeyCode::Up | KeyCode::Char('k') if self.detail => {
                self.scroll = self.scroll.saturating_sub(1)
            }
            KeyCode::PageDown if self.detail => {
                self.scroll = self.scroll.saturating_add(crate::page(self.page))
            }
            KeyCode::PageUp if self.detail => {
                self.scroll = self.scroll.saturating_sub(crate::page(self.page))
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Char('j') if self.garden_showing() => {
                self.step_selection(true)
            }
            KeyCode::Left | KeyCode::Up | KeyCode::Char('k') if self.garden_showing() => {
                self.step_selection(false)
            }
            KeyCode::Char(c @ ('+' | '=' | '-' | 'H' | 'J' | 'K' | 'L'))
                if self.garden_showing() =>
            {
                self.adjust_view(c)
            }
            KeyCode::Enter if self.garden_showing() => self.open_entry(),
            _ => {}
        }
        Next::Stay
    }

    /// Paint Tend into `body`: an open reader, panel, history or help covers the garden.
    /// Painting never reads files or waits on a worker.
    pub(crate) fn render(&mut self, frame: &mut Frame, body: Rect, palette: Palette) {
        if let Some(thoughts) = &mut self.thoughts
            && thoughts.active()
        {
            thoughts.render(frame, body, palette);
        } else if let Some(panel) = &mut self.connection {
            panel.render(frame, body, palette);
        } else if let Some(history) = &mut self.history {
            history.render(frame, body, palette, self.overlay);
        } else if self.overlay != Overlay::Closed {
            let p = Paragraph::new(self.help_text())
                .wrap(Wrap { trim: false })
                .style(palette.ink);
            let max = p
                .line_count(body.width)
                .saturating_sub(body.height as usize) as u16;
            self.help_scroll = self.help_scroll.min(max);
            frame.render_widget(p.scroll((self.help_scroll, 0)), body);
        } else {
            render_view(frame, self, body, palette);
        }
    }
}

fn row(session: &Session, width: usize, now: Option<u64>) -> (String, String) {
    let suffix = format!(
        "{} · {}",
        session.stage.label(),
        now.map_or_else(
            || "time unknown".into(),
            |now| relative_time(&session.updated, now)
        )
    );
    let budget = width.saturating_sub(suffix.width() + 2);
    let title = clipped(session.goal.lines().next().unwrap_or(""), budget);
    let gap = width.saturating_sub(title.width() + suffix.width());
    (format!("{title}{}", " ".repeat(gap)), suffix)
}

fn render_view(frame: &mut Frame, tend: &mut Tend, area: Rect, palette: Palette) {
    let keys = tend.keys().1;
    frame.render_widget(
        Paragraph::new(keys).style(palette.muted),
        Rect::new(
            2,
            frame.area().height.saturating_sub(1),
            frame.area().width.saturating_sub(4),
            1,
        ),
    );
    if tend.scope_row() {
        frame.render_widget(
            Paragraph::new(if tend.show_all {
                "all · a this repo"
            } else {
                "this repo · a all"
            })
            .style(palette.muted),
            Rect {
                height: 1.min(area.height),
                ..area
            },
        );
    }
    let area = tend.scope_body(area);
    let stale_empty =
        tend.entry_count() == 0 && tend.unreadable == 0 && tend.error.is_none() && !tend.current;
    if tend.pending || tend.want.is_some() || stale_empty {
        let status = if tend.still_reading {
            "Still reading your sessions. Esc goes back."
        } else {
            "Reading your sessions. Esc goes back."
        };
        frame.render_widget(Paragraph::new(status).style(palette.muted), area);
        return;
    }
    if tend.error.is_some() {
        frame.render_widget(
            Paragraph::new("Saved session unavailable. F1 then h for details.").style(palette.ink),
            area,
        );
        return;
    }
    if tend.entry_count() == 0 {
        frame.render_widget(
            Paragraph::new(if let Some(message) = tend.repo_empty_message() {
                message
            } else if tend.unreadable > 0 {
                format!(
                    "{} couldn't be read, so nothing can be listed. Esc goes back.",
                    records(tend.unreadable)
                )
            } else if tend.ended > 0 {
                format!(
                    "No active goals. {} ended not shown. Confirm a goal in think to start one.",
                    tend.ended
                )
            } else {
                "No saved sessions yet. Confirm a goal in think to start one.".into()
            })
            .style(palette.muted)
            .wrap(Wrap { trim: false }),
            area,
        );
        frame.render_widget(
            Paragraph::new(tend.thought_status()).style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(2), area.width, 1),
        );
        if tend.ended > 0 || tend.links.is_err() || tend.repo_unknown > 0 && tend.unreadable > 0 {
            garden::render_note(frame, tend, area, palette);
        }
        return;
    }
    if tend.detail {
        let session = &tend.sessions[tend.selected];
        tend.page = render_saved_goal(
            frame,
            &mut tend.scroll,
            area,
            palette,
            SavedGoal {
                goal: &session.goal,
                next: session.stage.next(),
                notice: &tend.notice,
                saved: false,
            },
        );
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    garden::render(frame, tend, area, palette, now);
}
fn records(n: usize) -> String {
    if n == 1 {
        "1 saved record".into()
    } else {
        format!("{n} saved records")
    }
}

/// A session's `updated` time in Unix seconds; `None` when it is not RFC 3339 or is before 1970.
pub(super) fn timestamp(value: &str) -> Option<u64> {
    u64::try_from(crate::calendar::parse(value)?.unix_timestamp()).ok()
}
fn relative_time(updated: &str, now: u64) -> String {
    let Some(at) = timestamp(updated) else {
        return "time unknown".into();
    };
    let seconds = now.abs_diff(at);
    let label = match seconds {
        0..60 => return if at <= now { "just now" } else { "in a moment" }.into(),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        86400..172800 if at <= now => return "yesterday".into(),
        _ => format!("{}d", seconds / 86400),
    };
    if at <= now {
        format!("{label} ago")
    } else {
        format!("in {label}")
    }
}

/// saved sessions, newest first (blocking). Unreadable records are left out here;
/// [`scan`] reports how many.
pub fn load_sessions(root: &Path) -> Result<Vec<Session>, String> {
    scan(root).map(|found| found.sessions)
}

#[cfg(test)]
mod place_tests;
#[cfg(test)]
mod scope_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_footer_names_f10_for_copyable_and_noncopyable_goals() {
        use ratatui::{Terminal, backend::TestBackend};
        for stage in [Stage::GoalSaved, Stage::Unknown] {
            let mut tend = Tend::new(
                Err("TEST no links".into()),
                None,
                Err("TEST no records".into()),
                Box::new(|| Err("TEST no connections".into())),
            );
            tend.pending = false;
            tend.current = true;
            tend.detail = true;
            tend.sessions.push(Session {
                id: Some("TEST-id".into()),
                path: "/TEST/goal".into(),
                goal: "Synthetic goal".into(),
                stage,
                updated: String::new(),
            });
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
            terminal
                .draw(|frame| tend.render(frame, Rect::new(2, 3, 96, 26), Palette::new(true)))
                .unwrap();
            let footer: String = (0..100)
                .map(|x| terminal.backend().buffer()[(x, 29)].symbol())
                .collect();
            assert!(footer.contains("F10 menu"), "{footer}");
            assert_eq!(footer.contains("s shape"), stage.can_shape());
        }
    }

    #[test]
    fn relative_time_validates_dates_offsets_and_handles_unknown_or_future_times() {
        let now = timestamp("2026-10-11T12:00:00Z").unwrap();
        for (value, expected) in [
            ("2026-10-11T10:00:00.123+00:00", "2h ago"),
            ("2026-10-11T12:00:00+02:00", "2h ago"),
            ("2026-10-10T12:00:00Z", "yesterday"),
            ("2026-10-11T11:58:00Z", "2m ago"),
            ("2026-10-11T13:00:00Z", "in 1h"),
            ("2026-02-30T12:00:00Z", "time unknown"),
            ("not a time", "time unknown"),
        ] {
            assert_eq!(relative_time(value, now), expected);
        }
        assert_eq!(timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert!(timestamp("2024-02-29T00:00:00Z").is_some());
        assert!(timestamp("2025-02-29T00:00:00Z").is_none());
    }
    #[test]
    fn row_only_uses_first_line_and_preserves_status_with_unicode_truncation() {
        let s = Session {
            id: None,
            path: "/TEST/session".into(),
            goal: "界👩‍💻 Café goal with many words\nWhy: never in row".into(),
            stage: Stage::SeedDrafted,
            updated: "2026-10-11T10:00:00Z".into(),
        };
        let (title, status) = row(&s, 40, timestamp("2026-10-11T12:00:00Z"));
        assert!(!title.contains("Why:"));
        assert!(title.trim_end().ends_with('…'));
        assert_eq!(status, "Tink drafted · 2h ago");
        assert_eq!(title.width() + status.width(), 40);
        assert!(title.contains("👩‍💻"));
        assert_eq!(row(&s, 40, None).1, "Tink drafted · time unknown");
    }
}
