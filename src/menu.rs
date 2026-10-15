//! The home menu: the operator's seven places, in pipeline order.
//!
//! think · tend · shape · approve · watch · taste · measure. Five are touches, two are views. Only think,
//! tend, shape and measure are built; the others say so plainly. Nothing here starts or steers an agent.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    Palette, measure,
    shape::{Leave, Shape},
    tend::{self, Discovery, Source, Tend},
    think::brain_dump::{self, BrainDump, Exit},
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

/// Name, one-line purpose, and whether the place exists yet.
pub const PLACES: [(&str, &str, bool); 7] = [
    ("think", "Make your intention a clear goal.", true),
    (
        "tend",
        "Choose a saved goal and copy a prompt to continue.",
        true,
    ),
    ("shape", "See where a saved goal's seed stands.", true),
    (
        "approve",
        "Decide whether the plan is ready. Not built yet.",
        false,
    ),
    ("watch", "Follow the work’s progress. Not built yet.", false),
    (
        "taste",
        "Decide whether the result meets your goal. Not built yet.",
        false,
    ),
    (
        "measure",
        "See recorded work and attention, when available.",
        true,
    ),
];

const FIRST_ROW: u16 = 4;
const ROW_GAP: u16 = 2;

/// The menu and whichever place is open in it. It routes input to the open place and owns what
/// outlives a visit: the key bar preference, receipts to print on exit, and the one session
/// scan tend reads from. A place returns here by its own exit; nothing opens by itself.
pub struct Home {
    /// The operator chose to leave Tinkery; the terminal loop stops after this tick.
    pub quit: bool,
    make_dump: Box<dyn Fn() -> BrainDump>,
    sessions_root: Option<PathBuf>,
    substrate_dir: PathBuf,
    link_root: Result<PathBuf, String>,
    selected: usize,
    view: View,
    help: bool,
    /// The muted key bar preference lasts the session, across visits to think.
    key_bar: bool,
    receipts: Vec<String>,
    seed_me: Option<PathBuf>,
    discovery: Discovery,
    record_root: Result<PathBuf, String>,
    history_reader: tend::history::Reader,
}

enum View {
    Menu,
    Think(Box<BrainDump>),
    Tend(Box<Tend>),
    /// Shape, and the tend it was opened from, restored as it was when Shape is left.
    Shape(Box<Shape>, Option<Box<Tend>>),
    Measure(measure::Reading),
    Soon(usize),
}

impl Home {
    /// A menu on its first row. `make_dump` makes a fresh think each time think is opened;
    /// tend reads the selected sessions root or shows its refusal without scanning. Links use
    /// the separate captured `link_root` (or its refusal); measure uses `substrate_dir`.
    pub fn new(
        make_dump: Box<dyn Fn() -> BrainDump>,
        sessions_root: Result<PathBuf, String>,
        substrate_dir: PathBuf,
        link_root: Result<PathBuf, String>,
    ) -> Self {
        let (sessions_root, discovery) = match sessions_root {
            Ok(root) => (Some(root), Discovery::new(Arc::new(tend::scan))),
            Err(reason) => (None, Discovery::unavailable(reason)),
        };
        Self {
            quit: false,
            make_dump,
            sessions_root,
            substrate_dir,
            link_root,
            selected: 0,
            view: View::Menu,
            help: false,
            key_bar: true,
            receipts: Vec::new(),
            seed_me: None,
            discovery,
            record_root: crate::think_record::default_root(std::env::var_os("HOME").as_deref()),
            history_reader: tend::history::Reader::default(),
        }
    }

    /// Replace the reader for a selected root; never override a root refusal.
    pub fn with_discovery_source(mut self, source: Source) -> Self {
        if self.sessions_root.is_some() {
            self.discovery = Discovery::new(source);
        }
        self
    }

    /// The Seed Me skill tend's continuation prompt names.
    pub fn with_seed_me(mut self, skill: PathBuf) -> Self {
        self.seed_me = Some(skill);
        self
    }

    /// Select the existing Think records to read. Configuring a root creates and reads nothing.
    pub fn with_record_root(mut self, root: PathBuf) -> Self {
        self.record_root = Ok(root);
        self
    }

    /// The menu think returns to after saving a goal when it was launched on its own. The
    /// receipt and captured links destination carry over; `open_tend` opens the saved session.
    pub fn from_receipt(
        mut dump: BrainDump,
        session: &Path,
        open_tend: bool,
        link_root: Result<PathBuf, String>,
    ) -> Self {
        let (default_root, substrate) = Self::default_paths();
        let root = session
            .parent()
            .map(|path| Ok(path.to_path_buf()))
            .unwrap_or(default_root);
        let mut home = Self::new(dump.fresh_factory(), root, substrate, link_root);
        if let Some(root) = dump.record_root() {
            home.record_root = Ok(root.to_path_buf());
        }
        home.key_bar = dump.key_bar_on();
        home.seed_me = dump.saved().map(|r| r.skill.clone());
        if let Some(receipt) = dump.finish_handoff() {
            home.receipts.push(receipt);
        }
        if open_tend {
            home.open_tend_session(session);
        }
        home
    }

    /// Checked default sessions root and the unchanged installed tink-substrate location.
    pub fn default_paths() -> (Result<PathBuf, String>, PathBuf) {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let substrate = std::env::var_os("TINK_SUBSTRATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude/skills/tink-substrate"));
        (
            crate::seed_me::default_root(Some(home.as_os_str())),
            substrate,
        )
    }

    /// Advances time: keeps the session scan moving and tend's wait notice current. Never
    /// blocks; a slow scan shows as waiting, not as a frozen screen.
    pub fn tick(&mut self) {
        self.tick_at(Instant::now());
    }

    /// [`Home::tick`] with the clock as an input, so tests never sleep.
    fn tick_at(&mut self, now: Instant) {
        if !self.discovery.requested() {
            self.request_sessions();
        }
        self.discovery.poll();
        self.sync_tend();
        self.sync_shape();
        self.update_stall(now);
        if let Some(result) = self.history_reader.poll()
            && let View::Tend(tend) = &mut self.view
            && let Some(history) = &mut tend.history
        {
            history.finish(result);
        }
        let mut tend_request = None;
        if let View::Think(dump) = &mut self.view {
            dump.tick();
            if let Some(receipt) = dump.saved() {
                self.seed_me = Some(receipt.skill.clone());
            }
            if let Exit::Tend(path) = dump.exit() {
                tend_request = Some(path.clone());
            }
            if *dump.exit() != Exit::Stay {
                self.key_bar = dump.key_bar_on();
                if let Some(receipt) = dump.finish_handoff() {
                    self.receipts.push(receipt);
                }
                self.view = View::Menu;
                // Back from think: a goal may have been saved, so read again.
                self.request_sessions();
            }
        }
        if let Some(path) = tend_request {
            self.open_tend_session(&path);
        }
    }

    /// Show the newest landed read in tend. While a saved session's detail is open
    /// nothing is replaced under the reader; the list catches up when they go back.
    fn sync_tend(&mut self) {
        let View::Tend(tend) = &mut self.view else {
            return;
        };
        tend.current = self.discovery.fresh();
        let Some(landed) = self.discovery.landed() else {
            return;
        };
        let version = self.discovery.version();
        if tend.seen == version || ((tend.detail || tend.is_linking()) && tend.want.is_none()) {
            return;
        }
        tend.apply(&landed.outcome, self.discovery.fresh(), version);
    }

    /// Hand Shape's list the sessions once a read asked for after it opened has landed, and let
    /// an open session's read finish.
    fn sync_shape(&mut self) {
        let View::Shape(shape, _) = &mut self.view else {
            return;
        };
        shape.tick();
        if shape.wants_sessions()
            && self.discovery.fresh()
            && let Some(landed) = self.discovery.landed()
        {
            shape.offer(match &landed.outcome {
                Ok(found) => Ok(&found.sessions),
                Err(error) => Err(error.clone()),
            });
        }
    }

    /// Let tend say when the read it waits on is taking a while. A check on the tick that
    /// already runs: no timer, no new worker.
    fn update_stall(&mut self, now: Instant) {
        if let View::Tend(tend) = &mut self.view {
            tend.note_wait(
                self.discovery
                    .running_since()
                    .map(|since| now.saturating_duration_since(since)),
            );
        }
    }

    /// Read once and wait a bounded time, for the non-interactive `--snapshot` view only.
    pub fn discover_for_snapshot(&mut self, wait: Duration) {
        if self.sessions_root.is_none() {
            return;
        }
        self.request_sessions();
        let deadline = Instant::now() + wait;
        while !self.discovery.poll() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn request_sessions(&mut self) {
        if let Some(root) = &self.sessions_root {
            self.discovery.request(root);
        }
    }

    fn open_tend_session(&mut self, path: &Path) {
        self.open(1);
        if let View::Tend(tend) = &mut self.view {
            // One stat of a path the handoff just wrote; matching is otherwise by identity.
            tend.want = Some((path.to_path_buf(), path.canonicalize().ok()));
            tend.seen = 0;
        }
        self.sync_tend();
    }

    /// Receipts from confirmed goals, printed after the terminal is restored.
    pub fn finish(&mut self) -> Option<String> {
        if let View::Think(dump) = &mut self.view
            && let Some(receipt) = dump.finish_handoff()
        {
            self.receipts.push(receipt);
        }
        (!self.receipts.is_empty()).then(|| self.receipts.join("\n"))
    }

    /// Bracketed paste. Only think takes text; elsewhere a paste does nothing.
    pub fn paste(&mut self, text: &str) {
        if let View::Think(dump) = &mut self.view {
            dump.paste(text);
        }
    }

    /// Text the open place asked to copy, taken once. The terminal loop does the copying and
    /// reports back through [`Home::copy_result`].
    pub fn take_copy_request(&mut self) -> Option<String> {
        match &mut self.view {
            View::Think(dump) => dump.take_copy_request(),
            View::Tend(tend) => tend.copy.take(),
            _ => None,
        }
    }

    /// Whether the last copy reached the clipboard, so the place can say so.
    pub fn copy_result(&mut self, sent: bool) {
        match &mut self.view {
            View::Think(dump) => dump.copy_result(sent),
            View::Tend(tend) => {
                tend.notice = if sent {
                    "Copied."
                } else {
                    "Couldn't copy; try again."
                }
                .into()
            }
            _ => {}
        }
    }

    /// Opens the place at `index` in [`PLACES`]. Opening tend asks for a fresh scan; an
    /// unbuilt place says so. Opening never starts an agent.
    pub fn open(&mut self, index: usize) {
        self.history_reader.cancel();
        self.selected = index;
        self.help = false;
        self.view = match index {
            0 => View::Think(Box::new((self.make_dump)().with_key_bar(self.key_bar))),
            1 => {
                // Deliberate reopening reads again; the last result shows meanwhile.
                self.request_sessions();
                View::Tend(Box::new(Tend::new(self.link_root.clone())))
            }
            2 => {
                self.request_sessions();
                View::Shape(Box::new(Shape::list()), None)
            }
            6 => View::Measure(measure::Reading::new(&self.substrate_dir)),
            other => View::Soon(other),
        };
        self.sync_tend();
        self.sync_shape();
    }

    /// One key press, routed to the open place or the menu. Releases are ignored.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if let View::Think(dump) = &mut self.view {
            dump.handle_key(key);
            return;
        }
        if matches!(self.view, View::Tend(_)) {
            let shifted_view_key = key.modifiers == KeyModifiers::SHIFT
                && matches!(
                    key.code,
                    KeyCode::Char('H' | 'J' | 'K' | 'L' | '+' | '=' | '-')
                );
            if key.modifiers != KeyModifiers::NONE && !shifted_view_key
                || key.kind == KeyEventKind::Repeat
                    && matches!(
                        key.code,
                        KeyCode::Char('c' | 'r' | 'l' | 's')
                            | KeyCode::F(1)
                            | KeyCode::F(10)
                            | KeyCode::Enter
                    )
            {
                return;
            }
            if let View::Tend(tend) = &mut self.view {
                tend.release_pointer();
            }
            if key.code == KeyCode::F(10) {
                self.history_reader.cancel();
                self.view = View::Menu;
                self.help = false;
                return;
            }
            if matches!(key.code, KeyCode::F(1) | KeyCode::Char('?')) {
                self.help = !self.help;
                if let View::Tend(tend) = &mut self.view {
                    tend.how = false;
                    tend.help_scroll = 0;
                    if let Some(history) = &mut tend.history {
                        history.reset_help();
                    }
                }
                return;
            }
            if self.help {
                if let View::Tend(tend) = &mut self.view {
                    if let Some(history) = &mut tend.history {
                        match key.code {
                            KeyCode::Char('h') => {
                                tend.how = !tend.how;
                                history.reset_help();
                            }
                            KeyCode::Esc if tend.how => {
                                tend.how = false;
                                history.reset_help();
                            }
                            KeyCode::Esc => self.help = false,
                            code => history.scroll(code, true, tend.how),
                        }
                        return;
                    }
                    match key.code {
                        KeyCode::Char('h') => {
                            tend.how = !tend.how;
                            tend.help_scroll = 0;
                        }
                        KeyCode::Esc if tend.how => {
                            tend.how = false;
                            tend.help_scroll = 0;
                        }
                        KeyCode::Esc => self.help = false,
                        KeyCode::Down | KeyCode::PageDown => {
                            tend.help_scroll = tend.help_scroll.saturating_add(5)
                        }
                        KeyCode::Up | KeyCode::PageUp => {
                            tend.help_scroll = tend.help_scroll.saturating_sub(5)
                        }
                        _ => {}
                    }
                }
                return;
            }
            if let View::Tend(tend) = &mut self.view {
                if let Some(history) = &mut tend.history {
                    if key.code == KeyCode::Esc {
                        self.history_reader.cancel();
                        tend.history = None;
                    } else {
                        history.scroll(key.code, false, false);
                    }
                    return;
                }
                if key.code == KeyCode::Char('r')
                    && tend.detail
                    && let Some(session) = tend.current()
                {
                    let mut history = tend::history::View::waiting();
                    match &self.record_root {
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
                    tend.history = Some(history);
                    return;
                }
            }
        }
        if matches!(self.view, View::Shape(..)) {
            self.shape_key(key);
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') || key.code == KeyCode::F(10) {
            self.quit = true;
            return;
        }
        if let View::Measure(reading) = &mut self.view
            && reading.failed()
        {
            if reading.key(key, &mut self.help) {
                self.help = false;
                self.view = View::Menu;
            }
            return;
        }
        if matches!(key.code, KeyCode::Char('?') | KeyCode::F(1)) {
            self.help = !self.help;
            return;
        }
        if self.help && key.code == KeyCode::Esc {
            self.help = false;
            return;
        }
        if self.help && matches!(self.view, View::Menu) {
            if key.code == KeyCode::Char('q') {
                self.quit = true;
            }
            return;
        }
        match &mut self.view {
            View::Menu => match key.code {
                KeyCode::Char(c @ '1'..='7') => self.open(c as usize - '1' as usize),
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected = (self.selected + 1) % PLACES.len()
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected = (self.selected + PLACES.len() - 1) % PLACES.len()
                }
                KeyCode::Enter => self.open(self.selected),
                KeyCode::Char('q') => self.quit = true,
                _ => {}
            },
            View::Tend(tend) => match key.code {
                KeyCode::Char('l') => tend.start_link(),
                KeyCode::Esc if tend.is_linking() => tend.cancel_link(),
                KeyCode::Enter if tend.is_linking() => tend.commit_link(),
                KeyCode::Esc if tend.detail => {
                    tend.detail = false;
                    tend.scroll = 0;
                    tend.notice.clear();
                }
                KeyCode::Esc => self.view = View::Menu,
                KeyCode::Char('c') if tend.detail => tend.copy_prompt(self.seed_me.as_deref()),
                KeyCode::Char('s')
                    if tend.detail && tend.current().is_some_and(|s| s.stage.can_copy()) =>
                {
                    let shape =
                        Shape::session(self.seed_me.as_deref(), &tend.current().unwrap().path);
                    let View::Tend(tend) = std::mem::replace(&mut self.view, View::Menu) else {
                        unreachable!()
                    };
                    self.view = View::Shape(Box::new(shape), Some(tend));
                }
                KeyCode::Down | KeyCode::Char('j') if tend.detail => {
                    tend.scroll = tend.scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') if tend.detail => {
                    tend.scroll = tend.scroll.saturating_sub(1)
                }
                KeyCode::PageDown if tend.detail => tend.scroll = tend.scroll.saturating_add(5),
                KeyCode::PageUp if tend.detail => tend.scroll = tend.scroll.saturating_sub(5),
                KeyCode::Right | KeyCode::Down | KeyCode::Char('j') if tend.garden_showing() => {
                    tend.step_selection(true)
                }
                KeyCode::Left | KeyCode::Up | KeyCode::Char('k') if tend.garden_showing() => {
                    tend.step_selection(false)
                }
                KeyCode::Char(c @ ('+' | '=' | '-' | 'H' | 'J' | 'K' | 'L'))
                    if tend.garden_showing() =>
                {
                    tend.adjust_view(c);
                }
                KeyCode::Enter if tend.garden_showing() => {
                    tend.detail = true;
                    tend.scroll = 0;
                    tend.notice.clear();
                }
                _ => {}
            },
            View::Measure(reading) => {
                if reading.key(key, &mut self.help) {
                    self.view = View::Menu;
                }
            }
            View::Soon(_) => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter) {
                    self.view = View::Menu;
                }
            }
            View::Think(_) | View::Shape(..) => {}
        }
    }

    /// Shape's keys: F10 for the menu, F1 or ? for help, Esc steps back (to the tend it came
    /// from, as it was). Held keys don't repeat an open. Nothing here writes or runs.
    fn shape_key(&mut self, key: KeyEvent) {
        if key.modifiers != KeyModifiers::NONE
            || key.kind == KeyEventKind::Repeat && matches!(key.code, KeyCode::Enter)
        {
            return;
        }
        match key.code {
            KeyCode::F(10) => {
                self.view = View::Menu;
                self.help = false;
            }
            KeyCode::F(1) | KeyCode::Char('?') => self.help = !self.help,
            KeyCode::Esc if self.help => self.help = false,
            _ if self.help => {}
            _ => {
                let View::Shape(shape, _) = &mut self.view else {
                    return;
                };
                if shape.key(key, self.seed_me.as_deref()) == Leave::Back {
                    let View::Shape(_, back) = std::mem::replace(&mut self.view, View::Menu) else {
                        unreachable!()
                    };
                    if let Some(tend) = back {
                        self.view = View::Tend(tend);
                        self.sync_tend();
                    }
                }
            }
        }
    }

    /// One mouse event in a terminal of size `area`. Routes deliberate presses to the open place.
    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        self.handle_mouse_at(event, area, Instant::now());
    }

    /// [`Home::handle_mouse`] with the clock as an input, so double-click tests never sleep.
    fn handle_mouse_at(&mut self, event: MouseEvent, area: Rect, now: Instant) {
        if let View::Think(dump) = &mut self.view {
            dump.handle_mouse(event, area);
            return;
        }
        if let View::Tend(tend) = &mut self.view {
            if !self.help {
                tend.pointer(event, body(area), now);
            }
            return;
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        if !matches!(self.view, View::Menu) {
            return;
        }
        if event.row == 1 && event.column + 3 >= area.width {
            self.help = !self.help;
            return;
        }
        if self.help {
            return;
        }
        if let Some(index) =
            (0..PLACES.len()).find(|i| event.row == FIRST_ROW + *i as u16 * ROW_GAP)
        {
            self.open(index);
        }
    }

    /// Memory only: an exact count of active sessions once a complete, current read has
    /// landed. Unknown, partial or failed reads show nothing, never zero.
    pub fn count(&self, index: usize) -> Option<String> {
        if index != 1 {
            return None;
        }
        self.discovery
            .exact_active()
            .filter(|active| *active > 0)
            .map(|active| active.to_string())
    }
}

/// Paints the menu or the open place. Painting never waits on the session scan.
pub fn render(frame: &mut Frame, home: &mut Home, palette: Palette) {
    if let View::Think(dump) = &mut home.view {
        brain_dump::render(frame, dump, palette);
        return;
    }
    let area = frame.area();
    frame.render_widget(Paragraph::new("").style(palette.ink), area);
    let title = match &home.view {
        View::Menu => "tinkery".to_string(),
        View::Tend(_) => "tinkery › tend".into(),
        View::Shape(..) => "tinkery › shape".into(),
        View::Measure(_) => "tinkery › measure".into(),
        View::Soon(i) => format!("tinkery › {}", PLACES[*i].0),
        View::Think(_) => unreachable!(),
    };
    frame.render_widget(
        Paragraph::new(title).style(palette.ink),
        Rect::new(2, 1, area.width.saturating_sub(6), 1),
    );
    if area.width >= 4 {
        frame.render_widget(
            Paragraph::new("?").style(palette.muted),
            Rect::new(area.width - 3, 1, 1, 1),
        );
    }
    let body = body(area);
    if let View::Tend(tend) = &mut home.view
        && let Some(history) = &mut tend.history
    {
        history.render(frame, body, palette, home.help, tend.how);
        return;
    }
    if let View::Measure(reading) = &mut home.view
        && reading.failed()
    {
        reading.render(frame, body, palette, home.help);
        return;
    }
    if home.help && !matches!(home.view, View::Menu) {
        if let View::Shape(shape, _) = &home.view {
            frame.render_widget(
                Paragraph::new(shape.help_text())
                    .wrap(Wrap { trim: false })
                    .style(palette.ink),
                body,
            );
            return;
        }
        if let View::Tend(tend) = &mut home.view {
            let p = Paragraph::new(tend.help_text(home.seed_me.as_deref()))
                .wrap(Wrap { trim: false })
                .style(palette.ink);
            let max = p
                .line_count(body.width)
                .saturating_sub(body.height as usize) as u16;
            tend.help_scroll = tend.help_scroll.min(max);
            frame.render_widget(p.scroll((tend.help_scroll, 0)), body);
            return;
        }
        let help = "Right now\n1–7 or ↑↓ Enter opens a place · ? help · q quits\n\nPlaces\nthink   brain dump → readings → one question → confirm a goal\ntend    your saved goals and seeds; copy a prompt to continue\nshape   where a saved goal's seed stands (read-only)\nmeasure the ledger report (read-only)\napprove, watch, taste: not built yet\n\nInside think, F10 returns here (it asks first if you'd lose work).\nEsc goes back from tend, shape, measure and unbuilt places.";
        frame.render_widget(
            Paragraph::new(help)
                .style(palette.ink)
                .wrap(Wrap { trim: false }),
            body,
        );
        return;
    }
    match &mut home.view {
        View::Menu => {
            for (i, (name, _, built)) in PLACES.iter().enumerate() {
                let y = FIRST_ROW + i as u16 * ROW_GAP;
                if y >= area.height.saturating_sub(2) {
                    break;
                }
                let style = if i == home.selected {
                    palette.jade
                } else if *built {
                    palette.ink
                } else {
                    palette.muted
                };
                let mut spans = vec![Span::styled(format!("{}  {name}", i + 1), style)];
                if let Some(count) = home.count(i) {
                    spans.push(Span::styled(
                        format!("{:>width$}", count, width = 15 - name.len()),
                        palette.muted,
                    ));
                }
                frame.render_widget(
                    Paragraph::new(Line::from(spans)),
                    Rect::new(2, y, area.width.saturating_sub(4), 1),
                );
            }
            let note = home
                .receipts
                .last()
                .map(|r| r.lines().next().unwrap_or("").to_string());
            let line = note.unwrap_or_else(|| PLACES[home.selected].1.to_string());
            frame.render_widget(
                Paragraph::new(line).style(palette.muted),
                Rect::new(
                    5,
                    FIRST_ROW + PLACES.len() as u16 * ROW_GAP + 1,
                    area.width.saturating_sub(7),
                    1,
                ),
            );
            if home.help {
                render_menu_help(frame, palette);
            } else {
                frame.render_widget(
                    Paragraph::new("↑↓ move   enter open   ? help   q quit").style(palette.muted),
                    Rect::new(
                        2,
                        area.height.saturating_sub(2),
                        area.width.saturating_sub(4),
                        1,
                    ),
                );
            }
        }
        View::Tend(tend) => tend::render(frame, tend, body, palette),
        View::Shape(shape, _) => shape.render(frame, body, palette),
        View::Measure(reading) => reading.render(frame, body, palette, home.help),
        View::Soon(i) => frame.render_widget(
            Paragraph::new(format!("{}\n\nNot built yet. Esc goes back.", PLACES[*i].1))
                .style(palette.muted),
            body,
        ),
        View::Think(_) => {}
    }
}

fn body(area: Rect) -> Rect {
    Rect::new(
        2,
        3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    )
}

fn render_menu_help(frame: &mut Frame, palette: Palette) {
    let area = frame.area();
    let width = 38.min(area.width.saturating_sub(4));
    let panel = Rect::new(area.width.saturating_sub(width + 2), 3, width, 13).intersection(area);
    let action = |key: &str, label: &str| {
        Line::from(vec![
            Span::styled(format!("{key:<8}"), palette.ink.bold()),
            Span::styled(label.to_owned(), palette.muted),
        ])
    };
    let lines = vec![
        Line::default(),
        Line::styled("Move around", palette.ink.bold()),
        Line::default(),
        action("↑ ↓", "Choose a place"),
        action("Enter", "Open it"),
        action("1–7", "Open directly"),
        Line::default(),
        action("q", "Quit Tinkery"),
        Line::default(),
        Line::default(),
        action("Esc", "Close help"),
    ];
    let block = Block::default()
        .title(Span::styled(" Help ", palette.jade))
        .borders(Borders::ALL)
        .border_style(palette.muted)
        .padding(Padding::new(1, 1, 0, 0));
    frame.render_widget(Clear, panel);
    frame.render_widget(Paragraph::new(lines).style(palette.ink).block(block), panel);
}

/// The screen as plain text at `width`×`height`, for `--snapshot` and tests. No terminal mode,
/// no provider calls.
pub fn snapshot(
    width: u16,
    height: u16,
    home: &mut Home,
    no_color: bool,
) -> Result<String, std::convert::Infallible> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, home, Palette::new(no_color)))?;
    let buffer = terminal.backend().buffer();
    Ok((0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod measure_tests;
#[cfg(test)]
mod menu_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tend::{Stage, load_sessions};
    use serde_json::Value;
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn home(root: &Path) -> Home {
        Home::new(
            Box::new(BrainDump::default),
            Ok(root.to_path_buf()),
            root.join("no-substrate"),
            Ok(root.join("TEST-links")),
        )
    }

    /// Tick until the read that was asked for has landed (bounded).
    fn settle(h: &mut Home) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            h.tick();
            if h.discovery.requested() && !h.discovery.running() && h.discovery.fresh() {
                return;
            }
            assert!(Instant::now() < deadline, "discovery did not settle");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn session(root: &Path, id: &str, status: &str, updated: &str, seed: Option<&str>) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("ledger.json"),
            serde_json::json!({"status": status, "goal": format!("goal {id}"), "updated_at": updated}).to_string(),
        )
        .unwrap();
        if let Some(first) = seed {
            fs::write(dir.join("seed-contract.md"), format!("{first}\n# Seed\n")).unwrap();
        }
    }

    #[test]
    fn home_shows_seven_places_and_only_counts_what_exists() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "a", "active", "2026-10-09T10:00:00Z", None);
        let mut h = home(tmp.path());
        settle(&mut h);
        let screen = snapshot(100, 30, &mut h, true).unwrap();
        for name in [
            "think", "tend", "shape", "approve", "watch", "taste", "measure",
        ] {
            assert!(screen.contains(name), "{name} missing:\n{screen}");
        }
        assert!(screen.contains("tinkery"));
        assert_eq!(h.count(1), Some("1".into()));
        assert_eq!(h.count(0), None);
    }

    #[test]
    fn home_key_bar_shows_only_immediate_controls_without_replacing_the_description() {
        let tmp = tempfile::tempdir().unwrap();
        let bar = "↑↓ move   enter open   ? help   q quit";
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let mut h = home(tmp.path());
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                assert_eq!(screen.lines().last().unwrap().trim(), bar, "{screen}");
                assert_eq!(screen.matches(bar).count(), 1);
                assert_eq!(
                    screen.lines().nth(19).unwrap(),
                    format!("     {}", PLACES[0].1)
                );
                h.handle_key(key(KeyCode::Down));
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                assert_eq!(
                    screen.lines().nth(19).unwrap(),
                    format!("     {}", PLACES[1].1)
                );
                assert!(screen.lines().nth(height as usize - 3).unwrap().is_empty());
                assert_eq!(screen.lines().last().unwrap().trim(), bar);
                h.handle_key(key(KeyCode::F(1)));
                assert!(
                    !snapshot(width, height, &mut h, no_color)
                        .unwrap()
                        .contains(bar)
                );
                h.handle_key(key(KeyCode::Esc));
                h.handle_key(key(KeyCode::Char('4')));
                assert!(
                    !snapshot(width, height, &mut h, no_color)
                        .unwrap()
                        .contains(bar)
                );
            }
        }
    }

    #[test]
    fn menu_help_is_a_bounded_overlay_with_the_menu_still_visible() {
        let tmp = tempfile::tempdir().unwrap();
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let mut h = home(tmp.path());
                h.handle_key(key(KeyCode::F(1)));
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                let rows: Vec<_> = screen.split('\n').collect();
                for (index, (name, _, _)) in PLACES.iter().enumerate() {
                    assert!(rows[4 + index * 2].starts_with(&format!("  {}  {name}", index + 1)));
                }
                assert_eq!(rows[19], format!("     {}", PLACES[0].1));
                assert!(screen.contains("Choose a place") && screen.contains("Esc     Close help"));
                let panel_content = |row: &str| {
                    row.chars()
                        .skip(width as usize - 38)
                        .take(34)
                        .collect::<String>()
                };
                assert!(panel_content(rows[4]).trim().is_empty());
                assert!(panel_content(rows[5]).starts_with("Move around"));
                assert!(!screen.contains("Leave"));
                assert_eq!(
                    rows[3].chars().position(|c| c == '┌'),
                    Some(width as usize - 40)
                );
                assert!(rows[15].contains('└'));
                assert_eq!(rows[16], "  7  measure");
                assert!(!screen.contains("↑↓ move   enter open"));
                assert!(!screen.contains("Inside think"));
                h.handle_key(key(KeyCode::Esc));
                let closed = snapshot(width, height, &mut h, no_color).unwrap();
                assert!(!closed.contains('┌') && closed.contains("↑↓ move   enter open"));
            }
        }
    }

    #[test]
    fn menu_help_uses_jade_for_place_bold_keys_and_quiet_labels_and_border() {
        use ratatui::style::{Color, Modifier};
        let tmp = tempfile::tempdir().unwrap();
        for no_color in [false, true] {
            let mut h = home(tmp.path());
            h.handle_key(key(KeyCode::F(1)));
            let palette = Palette::new(no_color);
            let backend = ratatui::backend::TestBackend::new(100, 30);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| render(frame, &mut h, palette))
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(62, 3)].symbol(), "H");
            assert_eq!(buffer[(62, 3)].fg, palette.jade.fg.unwrap_or(Color::Reset));
            assert!(buffer[(62, 3)].modifier.contains(Modifier::BOLD));
            for (x, y) in [(62, 5), (62, 7), (62, 8), (62, 9), (62, 11), (62, 14)] {
                assert_eq!(buffer[(x, y)].fg, palette.ink.fg.unwrap_or(Color::Reset));
                assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
            }
            for (x, y) in [(70, 7), (70, 8), (70, 9), (70, 11), (70, 14), (60, 6)] {
                assert_eq!(buffer[(x, y)].fg, palette.muted.fg.unwrap_or(Color::Reset));
                assert!(!buffer[(x, y)].modifier.contains(Modifier::BOLD));
                assert_eq!(buffer[(x, y)].modifier.contains(Modifier::DIM), no_color);
            }
        }
    }

    #[test]
    fn menu_help_blocks_underlying_keys_paste_and_clicks_but_keeps_close_and_quit() {
        let tmp = tempfile::tempdir().unwrap();
        for code in [
            KeyCode::Char('1'),
            KeyCode::Char('2'),
            KeyCode::Char('3'),
            KeyCode::Char('4'),
            KeyCode::Char('5'),
            KeyCode::Char('6'),
            KeyCode::Char('7'),
            KeyCode::Enter,
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Char('j'),
            KeyCode::Char('k'),
        ] {
            let mut h = home(tmp.path());
            h.handle_key(key(KeyCode::F(1)));
            h.handle_key(key(code));
            h.paste("1\nq");
            h.handle_mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 5,
                    row: 6,
                    modifiers: KeyModifiers::NONE,
                },
                Rect::new(0, 0, 80, 24),
            );
            assert!(h.help && matches!(h.view, View::Menu) && h.selected == 0 && !h.quit);
        }
        for close in [KeyCode::Esc, KeyCode::F(1), KeyCode::Char('?')] {
            let mut h = home(tmp.path());
            h.handle_key(key(KeyCode::F(1)));
            h.handle_key(key(close));
            assert!(!h.help && matches!(h.view, View::Menu) && !h.quit);
        }
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::F(1)));
        h.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 77,
                row: 1,
                modifiers: KeyModifiers::NONE,
            },
            Rect::new(0, 0, 80, 24),
        );
        assert!(!h.help && matches!(h.view, View::Menu));
        for quit in [
            key(KeyCode::Char('q')),
            key(KeyCode::F(10)),
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            let mut h = home(tmp.path());
            h.handle_key(key(KeyCode::F(1)));
            h.handle_key(quit);
            assert!(h.quit);
        }
    }

    #[test]
    fn menu_descriptions_distinguish_outputs_without_granting_permission() {
        let tmp = tempfile::tempdir().unwrap();
        let descriptions = [
            "Make your intention a clear goal.",
            "Choose a saved goal and copy a prompt to continue.",
            "See where a saved goal's seed stands.",
            "Decide whether the plan is ready. Not built yet.",
            "Follow the work’s progress. Not built yet.",
            "Decide whether the result meets your goal. Not built yet.",
            "See recorded work and attention, when available.",
        ];
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let mut h = home(tmp.path());
                for description in descriptions {
                    let screen = snapshot(width, height, &mut h, no_color).unwrap();
                    assert_eq!(screen.lines().nth(19).unwrap().trim(), description);
                    assert_eq!(screen.matches(description).count(), 1);
                    assert!(matches!(h.view, View::Menu));
                    assert!(!h.quit && h.take_copy_request().is_none());
                    h.handle_key(key(KeyCode::Down));
                }
            }
        }
    }

    #[test]
    fn home_menu_uses_one_grid_and_keeps_the_count_near_its_place() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..10 {
            session(
                tmp.path(),
                &format!("s{i}"),
                "active",
                "2026-10-09T10:00:00Z",
                None,
            );
        }
        let mut h = home(tmp.path());
        settle(&mut h);
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                let rows: Vec<_> = screen.split('\n').collect();
                assert_eq!(rows[1].find("tinkery"), Some(2));
                assert_eq!(rows[4], "  1  think");
                assert_eq!(rows[6], "  2  tend         10");
                assert_eq!(rows[8], "  3  shape");
                assert_eq!(rows[16], "  7  measure");
                assert!(rows[17].is_empty() && rows[18].is_empty());
                assert_eq!(rows[19], format!("     {}", PLACES[0].1));
                assert_eq!(rows[height as usize - 2].find('↑'), Some(2));
                assert!(
                    rows[height as usize - 1].is_empty(),
                    "footer needs a blank row below"
                );
            }
        }
    }

    #[test]
    fn s_in_tend_detail_opens_shape_and_esc_restores_the_same_detail() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "a", "active", "2026-10-09T10:00:00Z", None);
        let mut h = home(tmp.path());
        h.open(1);
        settle(&mut h);
        h.handle_key(key(KeyCode::Char('s')));
        assert!(matches!(h.view, View::Tend(_)), "the garden has no s");
        h.handle_key(key(KeyCode::Enter));
        let mut held = key(KeyCode::Char('s'));
        held.kind = KeyEventKind::Repeat;
        h.handle_key(held);
        assert!(matches!(h.view, View::Tend(_)), "a held key opens nothing");
        h.handle_key(key(KeyCode::Char('s')));
        assert!(matches!(h.view, View::Shape(..)));
        let screen = snapshot(100, 30, &mut h, true).unwrap();
        assert!(screen.contains("tinkery › shape"), "{screen}");
        assert!(screen.contains("isn't set up here"), "{screen}");
        h.handle_key(key(KeyCode::Esc));
        let View::Tend(tend) = &h.view else { panic!() };
        assert!(tend.detail);
        assert!(tend.current().unwrap().path.ends_with("a"));
        fs::write(tmp.path().join("a/ledger.json"), r#"{"status": "active"}"#).unwrap();
        h.handle_key(key(KeyCode::Esc));
        h.open(1);
        settle(&mut h);
        h.handle_key(key(KeyCode::Enter));
        h.handle_key(key(KeyCode::Char('s')));
        assert!(
            matches!(h.view, View::Tend(_)),
            "no s when the stage is unknown"
        );
    }

    #[test]
    fn digits_open_places_and_esc_returns_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('4')));
        assert!(
            snapshot(100, 30, &mut h, true)
                .unwrap()
                .contains("Not built yet")
        );
        h.handle_key(key(KeyCode::Esc));
        assert!(matches!(h.view, View::Menu));
        h.handle_key(key(KeyCode::Char('1')));
        assert!(matches!(h.view, View::Think(_)));
    }

    #[test]
    fn leaving_think_returns_home_not_out_of_the_app() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.open(0);
        if let View::Think(dump) = &mut h.view {
            dump.handle_key(key(KeyCode::F(10)));
        }
        h.tick();
        assert!(matches!(h.view, View::Menu));
        assert!(!h.quit);
    }

    #[test]
    fn key_bar_preference_lasts_across_visits_to_think() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.open(0);
        if let View::Think(dump) = &mut h.view {
            assert!(dump.key_bar_on(), "on by default");
            dump.handle_key(key(KeyCode::F(1)));
            dump.handle_key(key(KeyCode::Char('b')));
            dump.handle_key(key(KeyCode::F(10)));
        }
        h.tick();
        assert!(matches!(h.view, View::Menu));
        h.open(0);
        let View::Think(dump) = &h.view else { panic!() };
        assert!(!dump.key_bar_on(), "turned off stays off");
    }

    #[test]
    fn tend_reads_stage_from_seed_me_files_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "old", "active", "2026-10-01T00:00:00Z", None);
        session(
            tmp.path(),
            "new",
            "active",
            "2026-10-09T00:00:00Z",
            Some("status: confirmed for intake"),
        );
        session(
            tmp.path(),
            "draft",
            "active",
            "2026-10-05T00:00:00Z",
            Some("status: draft"),
        );
        fs::create_dir_all(tmp.path().join("broken")).unwrap();
        fs::write(tmp.path().join("broken/ledger.json"), "not json").unwrap();
        let sessions = load_sessions(tmp.path()).unwrap();
        let got: Vec<(&str, &str)> = sessions
            .iter()
            .map(|s| (s.goal.as_str(), s.stage.label()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("goal new", "seed ready"),
                ("goal draft", "seed drafted"),
                ("goal old", "goal saved")
            ]
        );
    }

    #[test]
    fn receipt_tend_request_selects_exact_session_and_missing_is_explicit() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "old", "active", "2026-10-01T00:00:00Z", None);
        session(tmp.path(), "new", "active", "2026-10-10T00:00:00Z", None);
        session(tmp.path(), "ended", "stopped", "2026-10-11T00:00:00Z", None);
        let path = tmp.path().join("old");
        let before = fs::read(path.join("ledger.json")).unwrap();
        let mut h = home(tmp.path());
        h.open(0);
        let View::Think(dump) = &mut h.view else {
            panic!()
        };
        dump.set_saved(crate::seed_me::Receipt::test(path.clone()));
        h.handle_key(key(KeyCode::Char('s')));
        h.tick();
        settle(&mut h);
        let View::Tend(tend) = &h.view else { panic!() };
        assert_eq!(tend.selected, 1);
        assert_eq!(
            tend.sessions[tend.selected].path,
            path.canonicalize().unwrap()
        );
        assert!(tend.detail && tend.error.is_none());
        assert_eq!(fs::read(path.join("ledger.json")).unwrap(), before);
        #[cfg(unix)]
        {
            let alias = tmp.path().join("alias-root");
            std::os::unix::fs::symlink(tmp.path(), &alias).unwrap();
            h.sessions_root = Some(alias);
            h.open_tend_session(&path.canonicalize().unwrap());
            settle(&mut h);
            let View::Tend(tend) = &h.view else { panic!() };
            assert_eq!(tend.selected, 1);
            assert_eq!(
                tend.sessions[tend.selected].path.canonicalize().unwrap(),
                path.canonicalize().unwrap()
            );
            h.sessions_root = Some(tmp.path().to_path_buf());
        }
        h.open_tend_session(&tmp.path().join("missing"));
        settle(&mut h);
        let View::Tend(tend) = &h.view else { panic!() };
        assert!(tend.sessions.is_empty());
        assert!(
            tend.error
                .as_ref()
                .unwrap()
                .contains("Saved session is unavailable")
        );
        h.sessions_root = Some(tmp.path().join("not-a-directory"));
        fs::write(h.sessions_root.as_ref().unwrap(), "not a directory").unwrap();
        h.open_tend_session(&path);
        settle(&mut h);
        let View::Tend(tend) = &h.view else { panic!() };
        assert!(
            tend.error.is_some() && tend.sessions.is_empty(),
            "{:?} {:?} {} {}",
            tend.error,
            tend.want,
            tend.pending,
            tend.sessions.len()
        );
    }

    #[test]
    fn tend_garden_derives_each_state_and_only_shows_goal_lines_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        for (id, status, date, seed) in [
            ("saved", "active", "2026-10-10T00:00:00Z", None),
            (
                "drafted",
                "active",
                "2026-10-09T00:00:00Z",
                Some("status: draft"),
            ),
            (
                "ready",
                "active",
                "2026-10-08T00:00:00Z",
                Some("status: confirmed for intake"),
            ),
            (
                "ended",
                "stopped",
                "2026-10-07T00:00:00Z",
                Some("status: confirmed for intake"),
            ),
        ] {
            session(tmp.path(), id, status, date, seed);
            let path = tmp.path().join(id).join("ledger.json");
            let mut ledger: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            ledger["goal"] = format!(
                "{id} {}\nWhy: never on list\nDone when: separate in detail",
                "a long goal line ".repeat(20)
            )
            .into();
            fs::write(path, ledger.to_string()).unwrap();
        }
        let loaded = load_sessions(tmp.path()).unwrap();
        assert_eq!(
            loaded.iter().map(|s| s.stage).collect::<Vec<_>>(),
            vec![
                Stage::GoalSaved,
                Stage::SeedDrafted,
                Stage::SeedReady,
                Stage::Ended
            ]
        );
        for (w, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let mut h = home(tmp.path());
            h.open(1);
            settle(&mut h);
            let screen = snapshot(w, height, &mut h, false).unwrap();
            assert!(
                !screen.contains("Why:") && !screen.contains("Done when:") && !screen.contains("/")
            );
            assert!(!screen.contains("ended a long goal"), "{screen}");
            assert!(screen.contains("1 ended not shown"), "{screen}");
            for id in ["saved", "drafted", "ready"] {
                assert!(screen.contains(id), "{screen}");
            }
            for (id, label) in [
                ("saved", "goal saved"),
                ("drafted", "seed drafted"),
                ("ready", "seed ready"),
            ] {
                let selected = snapshot(w, height, &mut h, false).unwrap();
                let line = selected.lines().nth(height as usize - 3).unwrap();
                assert!(
                    line.trim_start().starts_with(id)
                        && line.contains(label)
                        && line.contains('…')
                        && line.contains(" · "),
                    "{selected}"
                );
                h.handle_key(key(KeyCode::Char('j')));
            }
            assert_eq!(
                screen.lines().last().unwrap().trim(),
                "↑↓←→ move   enter open   l link   esc back"
            );
        }
    }

    #[test]
    fn tend_detail_preserves_receipt_block_and_copy_is_exact_repeatable_and_manual() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "saved", "active", "2026-10-11T00:00:00Z", None);
        let path = tmp.path().join("saved").canonicalize().unwrap();
        let goal = "I can read Café 👩‍💻 in a calm dark theme.\nWhy: Night work.\nDone when: The block stays readable.\nMust: Keep every line.\nMust not: Run agents.";
        let mut ledger: Value =
            serde_json::from_slice(&fs::read(path.join("ledger.json")).unwrap()).unwrap();
        ledger["goal"] = goal.into();
        fs::write(path.join("ledger.json"), ledger.to_string()).unwrap();
        let before = fs::read(path.join("ledger.json")).unwrap();
        let mut receipt = crate::seed_me::Receipt::test(path.clone());
        receipt.goal = goal.into();
        for (w, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let mut h = home(tmp.path()).with_seed_me(receipt.skill.clone());
            h.open_tend_session(&path);
            settle(&mut h);
            let screen = snapshot(w, height, &mut h, false).unwrap();
            let normalized = screen.split_whitespace().collect::<Vec<_>>().join(" ");
            for line in receipt.goal.lines() {
                assert!(normalized.contains(line), "{screen}");
            }
            let View::Tend(tend) = &h.view else { panic!() };
            assert_eq!(tend.current().unwrap().goal, receipt.goal);
            for label in ["Why:", "Done when:", "Must:", "Must not:"] {
                assert!(
                    screen
                        .lines()
                        .any(|line| line.trim_start().starts_with(label)),
                    "{screen}"
                );
            }
            assert!(
                !screen.contains("Saved.")
                    && !screen.contains("any harness")
                    && !screen.contains("/")
            );
            assert!(screen.contains("Next: shape it into a seed with Seed Me."));
            assert_eq!(
                screen.lines().last().unwrap().trim(),
                "c copy prompt   s shape   r history   esc back   F10 menu"
            );
            for _ in 0..2 {
                h.handle_key(key(KeyCode::Char('c')));
                assert_eq!(
                    h.take_copy_request().unwrap(),
                    receipt.continuation_prompt()
                );
                assert!(h.take_copy_request().is_none());
                h.copy_result(true);
                assert!(
                    snapshot(w, height, &mut h, false)
                        .unwrap()
                        .contains("Copied.")
                );
            }
            h.copy_result(false);
            assert!(
                snapshot(w, height, &mut h, false)
                    .unwrap()
                    .contains("Couldn't copy; try again.")
            );
            h.handle_key(key(KeyCode::F(1)));
            assert!(
                !snapshot(w, height, &mut h, false)
                    .unwrap()
                    .contains(path.to_str().unwrap())
            );
            h.handle_key(key(KeyCode::Char('h')));
            let View::Tend(tend) = &h.view else { panic!() };
            assert!(
                tend.help_text(h.seed_me.as_deref())
                    .contains(path.to_str().unwrap())
            );
            h.handle_key(key(KeyCode::Esc));
            h.handle_key(key(KeyCode::Esc));
            h.handle_key(key(KeyCode::Esc));
            let View::Tend(tend) = &h.view else { panic!() };
            assert!(!tend.detail);
            h.handle_key(key(KeyCode::Enter));
            h.handle_key(key(KeyCode::F(10)));
            assert!(matches!(h.view, View::Menu) && !h.quit);
            assert_eq!(fs::read(path.join("ledger.json")).unwrap(), before);
        }
    }

    #[test]
    fn tend_next_action_follows_status_and_ended_sessions_cannot_copy() {
        let tmp = tempfile::tempdir().unwrap();
        for (id, status, seed, next) in [
            (
                "draft",
                "active",
                Some("status: draft"),
                Some("Next: confirm the seed draft with Seed Me."),
            ),
            (
                "ready",
                "active",
                Some("status: confirmed for intake"),
                Some("Next: the seed is ready for intake."),
            ),
            ("stopped", "stopped", None, None),
            (
                "completed",
                "completed",
                Some("status: confirmed for intake"),
                None,
            ),
        ] {
            session(tmp.path(), id, status, "2026-10-11T00:00:00Z", seed);
            let mut h = home(tmp.path()).with_seed_me("/TEST/SKILL.md".into());
            h.open_tend_session(&tmp.path().join(id));
            settle(&mut h);
            let screen = snapshot(100, 30, &mut h, false).unwrap();
            if let Some(next) = next {
                assert!(screen.contains(next));
            } else {
                assert!(!screen.contains("Next:") && !screen.contains("c copy prompt"));
            }
            h.handle_key(key(KeyCode::Char('c')));
            assert_eq!(h.take_copy_request().is_some(), next.is_some());
        }
        assert_eq!(
            Stage::from_files("active", Some("status: simulated")),
            Stage::Simulated
        );
        assert!(!Stage::Simulated.can_copy());
        assert_eq!(
            Stage::from_files("active", Some("not a status")),
            Stage::Unknown
        );
        assert!(!Stage::Unknown.can_copy());
    }

    #[test]
    fn q_quits_from_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('q')));
        assert!(h.quit);
    }
}
