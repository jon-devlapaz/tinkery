//! The home menu: the operator's seven places, in pipeline order.
//!
//! think · tend · shape · approve · watch · taste · measure. Each place has its own boundary.
//! Named worker actions use the shared SDLC boundary; navigation never starts work.

mod place;
mod retry;
mod visits;

pub use place::{PLACES, Place};

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    Palette, approve,
    data_roots::DataRoots,
    launch::Launch,
    measure, sdlc, seed_me,
    shape::Shape,
    taste::Taste,
    tend::{self, Discovery, Source, Tend},
    think::brain_dump::{self, BrainDump},
    watch::Watch,
    worker,
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

const FIRST_ROW: u16 = 4;
const ROW_GAP: u16 = 2;

/// The one contract every open place keeps with the menu. A place owns its keys, help and
/// reads, and answers each input or poll with an [`Outcome`]; the menu owns where that leads.
/// No method starts an agent: a place opens another only through [`Outcome::Open`].
trait Visit {
    /// One press or repeat; releases never arrive.
    fn key(&mut self, key: KeyEvent, kept: &mut Kept) -> Outcome;
    /// Literal words; pasting never submits, saves or launches.
    fn paste(&mut self, _text: &str) {}
    /// One mouse event in a terminal of size `area`.
    fn mouse(&mut self, _event: MouseEvent, _area: Rect, _now: Instant, _kept: &Kept) -> Outcome {
        Outcome::Stay
    }
    /// Advance this place's own reads without blocking, including the menu's session scan.
    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        Outcome::Stay
    }
    /// Paint the whole screen.
    fn render(&mut self, frame: &mut Frame, palette: Palette, kept: &Kept);
    /// The place above this one closed and this one shows again.
    fn resumed(&mut self, _kept: &mut Kept) {}
    /// This place leaves the screen for good; hand back what outlives the visit.
    fn close(self: Box<Self>, _kept: &mut Kept) {}
    /// Work this place admitted is still in flight; quitting waits for it.
    fn busy(&self) -> bool {
        false
    }
    /// Text the operator asked to copy, taken once.
    fn take_copy(&mut self) -> Option<String> {
        None
    }
    /// Whether the last copy reached the clipboard.
    fn copy_result(&mut self, _sent: bool) {}
    /// The Think reading this frame shows, if one awaits successful output.
    fn presentation(&self) -> Option<brain_dump::Presentation> {
        None
    }
    /// Successful terminal output of `presentation`.
    fn presented(&mut self, _presentation: brain_dump::Presentation) {}
}

/// What a place asks of the menu.
enum Outcome {
    Stay,
    /// Open the launch's Model panel.
    Model,
    /// Close this place and show the one it was opened over, or the menu.
    Back,
    /// Close every place and show the menu.
    Home,
    /// Leave Tinkery; a busy worker finishes first.
    Quit,
    /// Navigation the operator asked for, or the next step of a finished save.
    Open(Box<Open>),
}

fn to(open: Open) -> Outcome {
    Outcome::Open(Box::new(open))
}

/// Where navigation leads. Each one only builds a place; none starts work.
enum Open {
    /// A place from the menu, fresh.
    Place(Place),
    /// A fresh Think holding Tend's unsent thought.
    Words(String),
    /// A fresh Think offering Taste's rejection draft.
    Draft(crate::think_record::DraftSuggestion),
    /// Tend on this saved session.
    TendSession(PathBuf),
    /// Shape on this session, over the place that asked.
    Shape(PathBuf),
    /// Shape on the session a retry just saved, in place of everything open.
    Saved(PathBuf),
    /// Approve on Shape's ready seed, over Shape.
    Seed(seed_me::ready::SeedTarget),
    /// Taste's retry review of one rejection, over Taste.
    Retry(crate::taste::Rejected),
    /// The worker panel reviewing this start; a busy panel stays as it is.
    Review(worker::Review),
    /// The worker panel for an existing run; a busy panel stays as it is.
    Run {
        root: PathBuf,
        slug: String,
        id: String,
    },
    /// The worker panel as it was left.
    Worker,
    /// Watch over a run's checkout.
    Watch(PathBuf),
    /// Taste over a run's checkout.
    Taste(PathBuf),
}

/// What outlives a visit and what places read from the launch.
struct Kept {
    /// The one resolved launch every place opens from.
    launch: Launch,
    sessions_root: Option<PathBuf>,
    link_root: Result<PathBuf, String>,
    record_root: Result<PathBuf, String>,
    /// The one session scan Tend, Shape, Approve and the menu read from.
    discovery: Discovery,
    /// One verifier across Approve visits.
    approve_reader: approve::Reader,
    /// The muted key bar preference lasts the session, across visits to think.
    key_bar: bool,
    receipts: Vec<seed_me::ExitReceipt>,
    thought_saved: Option<(String, bool)>,
    /// The worker panel while it is not on screen.
    worker: Option<worker::Panel>,
    quit_after_worker: bool,
    /// Think was launched on its own and nothing has left it yet: no session scan runs.
    alone: bool,
}

impl Kept {
    fn request_sessions(&mut self) {
        self.discovery.scope_to_repo(
            self.launch
                .repo
                .as_ref()
                .map(|repo| repo.root().to_path_buf()),
            self.record_root.clone(),
        );
        if let Some(root) = &self.sessions_root {
            self.discovery.request(root);
        }
    }

    /// `shape` for this launch: its drafting agent, its hold and its attribution.
    fn shape(&self, shape: Shape) -> Shape {
        let launch = &self.launch;
        shape.launched(
            launch.shape_agent.clone(),
            &launch.confirmation,
            launch.agent,
        )
    }

    /// A fresh Think visit, keeping the key bar choice and saving thoughts to Tend's store.
    fn think(&self) -> BrainDump {
        self.launch
            .think()
            .with_key_bar(self.key_bar)
            .with_thoughts(self.link_root.clone(), self.launch.repo.clone())
    }

    fn approve_watch_root(&self) -> Option<PathBuf> {
        self.launch.sdlc_root.clone().or_else(|| {
            let root = self.launch.repo.as_ref()?.root();
            sdlc::installed(root).then(|| root.to_path_buf())
        })
    }

    /// Share the declaration and relay snapshot without granting permission on navigation.
    fn worker_gate(&self) -> worker::Gate {
        worker::Gate::new(self.launch.agent, self.launch.worker_authorization.clone())
    }
}

/// An open place and the menu selection to restore when it closes, if it changes one.
struct Entry {
    visit: Box<dyn Visit>,
    back_to: Option<Place>,
}

/// The menu and the stack of places open over it. It routes input to the top place and owns
/// what outlives a visit: the key bar preference, receipts to print on exit, the worker panel
/// and the one session scan. A place returns here by its own exit; nothing opens by itself.
pub struct Home {
    /// The operator chose to leave Tinkery; the terminal loop stops after this tick.
    pub quit: bool,
    kept: Kept,
    stack: Vec<Entry>,
    selected: Place,
    /// The menu's own help overlay.
    help: bool,
    /// `m` on the menu, or a place, asked for the Model panel, which the launch owns.
    model_request: bool,
}

impl Home {
    /// A menu on its first row, for `launch`. Each Think visit is a fresh one from the launch;
    /// tend reads the sessions root or shows its refusal without scanning. Think records and
    /// Tend's stores use the launch's other roots (or their refusals).
    pub fn new(launch: Launch) -> Self {
        let DataRoots {
            sessions,
            records,
            tend,
        } = launch.roots.clone();
        let (sessions_root, discovery) = match sessions {
            Ok(root) => (Some(root), Discovery::new(Arc::new(tend::scan))),
            Err(reason) => (None, Discovery::unavailable(reason)),
        };
        Self {
            quit: false,
            kept: Kept {
                launch,
                sessions_root,
                link_root: tend,
                record_root: records,
                discovery,
                approve_reader: approve::Reader::default(),
                key_bar: true,
                receipts: Vec::new(),
                thought_saved: None,
                worker: None,
                quit_after_worker: false,
                alone: false,
            },
            stack: Vec::new(),
            selected: Place::Think,
            help: false,
            model_request: false,
        }
    }

    /// Think launched on its own. Leaving it without a saved goal quits Tinkery; a saved goal
    /// shows the menu for the same `launch`, or Tend on that goal. No session scan runs before.
    pub fn alone(launch: Launch) -> Self {
        let dump = launch.think();
        let mut home = Self::new(launch);
        home.kept.alone = true;
        home.push(Box::new(dump), None);
        home
    }

    /// Whether Think still runs alone, as [`Home::alone`] opened it.
    pub fn is_alone(&self) -> bool {
        self.kept.alone
    }

    /// Replace the reader for a selected root; never override a root refusal.
    pub fn with_discovery_source(mut self, source: Source) -> Self {
        if self.kept.sessions_root.is_some() {
            self.kept.discovery = Discovery::new(source);
        }
        self
    }

    /// Advances time: keeps the session scan moving and tend's wait notice current. Never
    /// blocks; a slow scan shows as waiting, not as a frozen screen.
    pub fn tick(&mut self) {
        self.tick_at(Instant::now());
    }

    /// [`Home::tick`] with the clock as an input, so tests never sleep.
    fn tick_at(&mut self, now: Instant) {
        if let Some(panel) = &mut self.kept.worker {
            panel.poll();
        }
        if !self.kept.alone {
            if !self.kept.discovery.requested() {
                self.kept.request_sessions();
            }
            self.kept.discovery.poll();
        }
        self.kept.approve_reader.poll(None);
        if let Some(top) = self.stack.last_mut() {
            let outcome = top.visit.poll(&mut self.kept, now);
            self.apply(outcome);
        }
        if self.kept.quit_after_worker && !self.busy() {
            self.quit = true;
        }
    }

    /// Read once and wait a bounded time, for the non-interactive `--snapshot` view only.
    pub fn discover_for_snapshot(&mut self, wait: Duration) {
        if self.kept.sessions_root.is_none() {
            return;
        }
        self.kept.request_sessions();
        let deadline = Instant::now() + wait;
        while !self.kept.discovery.poll() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Receipts from confirmed goals, printed after the terminal is restored.
    pub fn finish(&mut self) -> Option<String> {
        self.clear();
        let receipts = &self.kept.receipts;
        (!receipts.is_empty()).then(|| {
            receipts
                .iter()
                .map(seed_me::ExitReceipt::text)
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Paste literal words into the active editor. Pasting never submits, saves or launches.
    pub fn paste(&mut self, text: &str) {
        if let Some(top) = self.stack.last_mut() {
            top.visit.paste(text);
        }
    }

    /// Text the open place asked to copy, taken once. The terminal loop does the copying and
    /// reports back through [`Home::copy_result`].
    pub fn take_copy_request(&mut self) -> Option<String> {
        self.stack.last_mut()?.visit.take_copy()
    }

    /// Whether `m` on the menu, or F1's Model and responses row in Think or Shape, asked for the
    /// Model panel since the last ask. Asking clears it.
    pub fn take_model_request(&mut self) -> bool {
        std::mem::take(&mut self.model_request)
    }

    /// Whether the last copy reached the clipboard, so the place can say so.
    pub fn copy_result(&mut self, sent: bool) {
        if let Some(top) = self.stack.last_mut() {
            top.visit.copy_result(sent);
        }
    }

    /// Opens a place. Opening tend asks for a fresh scan; opening never starts an agent.
    pub fn open(&mut self, place: Place) {
        if self.quit {
            return;
        }
        self.clear();
        self.selected = place;
        let kept = &mut self.kept;
        let visit: Box<dyn Visit> = match place {
            Place::Think => Box::new(kept.think()),
            Place::Tend => Box::new(visits::tend(kept)),
            Place::Shape => {
                kept.request_sessions();
                let mut shaping = visits::Shaping::new(kept.shape(Shape::list()));
                shaping.poll(kept, Instant::now());
                Box::new(shaping)
            }
            Place::Approve => Box::new(visits::Approving::new(
                approve::Approve::open(kept.approve_watch_root()),
                kept,
            )),
            Place::Watch => Box::new(visits::Helped::new(
                Watch::open(kept.approve_watch_root())
                    .with_codex_report(kept.launch.codex_report.clone()),
            )),
            Place::Taste => Box::new(visits::Helped::new(Taste::open(
                kept.launch.sdlc_root.clone(),
                kept.launch.agent,
            ))),
            Place::Measure => Box::new(visits::Helped::new(measure::Measure::open(
                kept.launch.repo.clone(),
            ))),
        };
        self.push(visit, None);
    }

    fn open_tend_session(&mut self, path: &Path) {
        if self.quit {
            return;
        }
        self.clear();
        self.selected = Place::Tend;
        let mut tend = visits::tend(&mut self.kept);
        tend.open_session(path);
        tend.offer(&self.kept.discovery);
        self.push(Box::new(tend), None);
    }

    fn push(&mut self, visit: Box<dyn Visit>, back_to: Option<Place>) {
        self.help = false;
        self.stack.push(Entry { visit, back_to });
    }

    /// Close every open place, top first.
    fn clear(&mut self) {
        while let Some(entry) = self.stack.pop() {
            entry.visit.close(&mut self.kept);
        }
    }

    fn busy(&self) -> bool {
        self.kept.worker.as_ref().is_some_and(worker::Panel::busy)
            || self.stack.iter().any(|entry| entry.visit.busy())
    }

    /// Show the worker panel, in place of everything open.
    fn show_worker(&mut self) {
        self.clear();
        if let Some(panel) = self.kept.worker.take() {
            self.push(Box::new(panel), None);
        }
    }

    fn apply(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Stay => {}
            Outcome::Model => self.model_request = true,
            Outcome::Back => {
                if let Some(entry) = self.stack.pop() {
                    entry.visit.close(&mut self.kept);
                    if let Some(place) = entry.back_to {
                        self.selected = place;
                    }
                }
                if let Some(top) = self.stack.last_mut() {
                    top.visit.resumed(&mut self.kept);
                }
            }
            Outcome::Home => self.clear(),
            Outcome::Quit if self.busy() => {
                self.kept.quit_after_worker = true;
                self.show_worker();
                self.help = false;
            }
            Outcome::Quit => self.quit = true,
            Outcome::Open(open) => self.go(*open),
        }
    }

    fn go(&mut self, open: Open) {
        let kept = &mut self.kept;
        match open {
            Open::Place(place) => self.open(place),
            Open::Words(words) => {
                let mut dump = kept.think();
                dump.paste(&words);
                self.clear();
                self.push(Box::new(dump), None);
            }
            Open::Draft(draft) => {
                let mut dump = kept.think();
                dump.offer_draft(draft);
                self.clear();
                self.selected = Place::Think;
                self.push(Box::new(dump), None);
            }
            Open::TendSession(path) => self.open_tend_session(&path),
            Open::Shape(path) => {
                let shape = kept.shape(Shape::session(&path));
                let visit = visits::Shaping::new(shape);
                self.push(Box::new(visit), None);
            }
            Open::Saved(path) => {
                self.clear();
                let kept = &mut self.kept;
                let shape = kept.shape(Shape::session(&path));
                kept.request_sessions();
                let visit = visits::Shaping::new(shape);
                self.push(Box::new(visit), None);
            }
            Open::Seed(target) => {
                let view = approve::Approve::open(kept.approve_watch_root()).for_seed(target);
                let visit = visits::Approving::new(view, kept);
                let back_to = Some(std::mem::replace(&mut self.selected, Place::Approve));
                self.push(Box::new(visit), back_to);
            }
            Open::Retry(rejected) => {
                self.selected = Place::Shape;
                self.push(Box::new(retry::Review::new(rejected)), Some(Place::Taste));
            }
            Open::Review(review) => {
                self.clear();
                if !self.busy() {
                    let kept = &mut self.kept;
                    kept.worker = Some(worker::Panel::review(
                        review,
                        kept.record_root.clone(),
                        kept.launch.worker.clone(),
                        kept.worker_gate(),
                    ));
                }
                self.show_worker();
            }
            Open::Run { root, slug, id } => {
                self.clear();
                if !self.busy() {
                    let gate = self.kept.worker_gate();
                    self.kept.worker = Some(worker::Panel::existing(root, slug, id, gate));
                }
                self.show_worker();
            }
            Open::Worker => self.show_worker(),
            Open::Watch(root) => {
                self.clear();
                self.push(Box::new(visits::Helped::new(Watch::open(Some(root)))), None);
            }
            Open::Taste(root) => {
                self.clear();
                let taste = Taste::open(Some(root), self.kept.launch.agent);
                self.push(Box::new(visits::Helped::new(taste)), None);
            }
        }
    }

    /// One key press, routed to the open place or the menu. Releases are ignored.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        let ctrl_c =
            key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
        let outcome = if ctrl_c && self.busy() {
            Outcome::Quit
        } else if let Some(top) = self.stack.last_mut() {
            top.visit.key(key, &mut self.kept)
        } else {
            self.menu_key(key)
        };
        self.apply(outcome);
    }

    fn menu_key(&mut self, key: KeyEvent) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') || key.code == KeyCode::F(10) {
            return Outcome::Quit;
        }
        if matches!(key.code, KeyCode::Char('?') | KeyCode::F(1)) {
            self.help = !self.help;
            return Outcome::Stay;
        }
        if self.help {
            if key.code == KeyCode::Esc {
                self.help = false;
            } else if key.code == KeyCode::Char('q') {
                return Outcome::Quit;
            }
            return Outcome::Stay;
        }
        match key.code {
            KeyCode::Char(c @ '1'..='7') => to(Open::Place(PLACES[c as usize - '1' as usize])),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Up | KeyCode::Char('k') => {
                let index = PLACES
                    .iter()
                    .position(|place| *place == self.selected)
                    .unwrap();
                let step = if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
                    1
                } else {
                    PLACES.len() - 1
                };
                self.selected = PLACES[(index + step) % PLACES.len()];
                Outcome::Stay
            }
            KeyCode::Enter => to(Open::Place(self.selected)),
            KeyCode::Char('q') => Outcome::Quit,
            KeyCode::Char('m') => Outcome::Model,
            KeyCode::Char('b')
                if self.kept.worker.is_some()
                    && key.kind == KeyEventKind::Press
                    && key.modifiers == KeyModifiers::NONE =>
            {
                to(Open::Worker)
            }
            _ => Outcome::Stay,
        }
    }

    /// One mouse event in a terminal of size `area`. Routes deliberate presses to the open place.
    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        self.handle_mouse_at(event, area, Instant::now());
    }

    /// [`Home::handle_mouse`] with the clock as an input, so double-click tests never sleep.
    fn handle_mouse_at(&mut self, event: MouseEvent, area: Rect, now: Instant) {
        if let Some(top) = self.stack.last_mut() {
            let outcome = top.visit.mouse(event, area, now, &self.kept);
            self.apply(outcome);
            return;
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        if event.row == 1 && event.column + 3 >= area.width {
            self.help = !self.help;
            return;
        }
        if self.help {
            return;
        }
        if let Some((_, place)) = PLACES
            .iter()
            .enumerate()
            .find(|(i, _)| event.row == FIRST_ROW + *i as u16 * ROW_GAP)
        {
            self.open(*place);
        }
    }

    /// The Think reading associated with this frame, if one awaits successful output.
    pub fn presentation(&self) -> Option<brain_dump::Presentation> {
        self.stack.last()?.visit.presentation()
    }

    /// Acknowledge successful terminal output to the open Think visit only.
    pub fn presented(&mut self, presentation: brain_dump::Presentation) {
        if let Some(top) = self.stack.last_mut() {
            top.visit.presented(presentation);
        }
    }

    /// Memory only: an exact count of active sessions once a complete, current read has
    /// landed. Unknown, partial or failed reads show nothing, never zero.
    pub fn count(&self, place: Place) -> Option<String> {
        if place != Place::Tend {
            return None;
        }
        self.kept
            .discovery
            .exact_active()
            .filter(|active| *active > 0)
            .map(|active| active.to_string())
    }
}

/// Paints the menu or the open place. Painting never waits on the session scan.
pub fn render(frame: &mut Frame, home: &mut Home, palette: Palette) {
    if let Some(top) = home.stack.last_mut() {
        top.visit.render(frame, palette, &home.kept);
        return;
    }
    let area = frame.area();
    let mut title = vec![Span::styled("tinkery", palette.ink)];
    if let Some(repo) = &home.kept.launch.repo {
        title.push(Span::styled(format!("  in {}", repo.name()), palette.muted));
    }
    chrome(frame, Line::from(title), palette);
    for (i, place) in PLACES.iter().enumerate() {
        let (name, description) = place.label();
        let y = FIRST_ROW + i as u16 * ROW_GAP;
        if y >= area.height.saturating_sub(2) {
            break;
        }
        let style = if *place == home.selected {
            palette.jade
        } else {
            palette.ink
        };
        let mut spans = vec![Span::styled(format!("{}  {name}", i + 1), style)];
        spans.push(Span::styled(
            format!(
                "{:>width$}  {description}",
                home.count(*place).unwrap_or_default(),
                width = 15 - name.len()
            ),
            palette.muted,
        ));
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(2, y, area.width.saturating_sub(4), 1),
        );
    }
    if let Some(receipt) = home.kept.receipts.last() {
        frame.render_widget(
            Paragraph::new(receipt.label()).style(palette.muted),
            Rect::new(
                5,
                FIRST_ROW + PLACES.len() as u16 * ROW_GAP + 1,
                area.width.saturating_sub(7),
                1,
            ),
        );
    }
    if home.help {
        render_menu_help(frame, palette);
    } else {
        frame.render_widget(
            Paragraph::new(if home.kept.worker.is_some() {
                "↑↓ move   enter open   b worker   ? help   q quit"
            } else {
                "↑↓ move   enter open   ? help   q quit"
            })
            .style(palette.muted),
            Rect::new(
                2,
                area.height.saturating_sub(2),
                area.width.saturating_sub(4),
                1,
            ),
        );
    }
}

/// The shared frame of every screen but Think: the title row, the theme switch and the `?`
/// control. Returns the body below it.
fn chrome(frame: &mut Frame, title: Line, palette: Palette) -> Rect {
    let area = frame.area();
    frame.render_widget(Paragraph::new("").style(palette.ink), area);
    frame.render_widget(
        Paragraph::new(title),
        Rect::new(
            2,
            1,
            crate::header_end(area, area.width.saturating_sub(4)).saturating_sub(2),
            1,
        ),
    );
    crate::render_theme_switch(frame, palette);
    if area.width >= 4 {
        frame.render_widget(
            Paragraph::new("?").style(palette.muted),
            Rect::new(area.width - 3, 1, 1, 1),
        );
    }
    body(area)
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
    let panel = Rect::new(area.width.saturating_sub(width + 2), 3, width, 14).intersection(area);
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
        action("m", "Model and responses"),
        if palette.theme.is_some() {
            action("Ctrl-T", "Dark or light")
        } else {
            Line::default()
        },
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
    use super::menu_tests::{at, how_it_works, in_detail, selected_line, think_saved};
    use super::*;
    use crate::tend::{Stage, load_sessions};
    use serde_json::Value;
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn home(root: &Path) -> Home {
        Home::new(Launch::practice(DataRoots {
            sessions: Ok(root.to_path_buf()),
            tend: Ok(root.join("TEST-links")),
            ..DataRoots::from_home(None)
        }))
    }

    /// Tick until the read that was asked for has landed (bounded).
    fn settle(h: &mut Home) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            h.tick();
            if h.kept.discovery.requested()
                && !h.kept.discovery.running()
                && h.kept.discovery.fresh()
            {
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
        assert_eq!(h.count(Place::Tend), Some("1".into()));
        assert_eq!(h.count(Place::Think), None);
    }

    #[test]
    fn home_key_bar_keeps_inline_descriptions_and_receipt_space() {
        let tmp = tempfile::tempdir().unwrap();
        let bar = "↑↓ move   enter open   ? help   q quit";
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let mut h = home(tmp.path());
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                assert_eq!(screen.lines().last().unwrap().trim(), bar, "{screen}");
                assert_eq!(screen.matches(bar).count(), 1);
                assert!(screen.lines().nth(19).unwrap().is_empty());
                h.handle_key(key(KeyCode::Down));
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                assert!(screen.lines().nth(19).unwrap().is_empty());
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
                for (index, place) in PLACES.iter().enumerate() {
                    let (name, _) = place.label();
                    assert!(rows[4 + index * 2].starts_with(&format!("  {}  {name}", index + 1)));
                }
                assert!(rows[19].is_empty());
                assert_eq!(screen.contains("Browse recent pull requests"), width >= 100);
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
                assert!(rows[16].starts_with("  7  measure") && rows[16].contains('└'));
                assert!(panel_content(rows[12]).starts_with("m       Model and responses"));
                assert!(!screen.contains("↑↓ move   enter open"));
                assert!(!screen.contains("Inside think"));
                h.handle_key(key(KeyCode::Esc));
                let closed = snapshot(width, height, &mut h, no_color).unwrap();
                assert!(closed.contains("Browse recent pull requests"));
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
            let find = |needle: &str| -> (u16, u16) {
                let want: Vec<String> = needle.chars().map(String::from).collect();
                (0..buffer.area.height)
                    .find_map(|y| {
                        let row: Vec<&str> = (0..buffer.area.width)
                            .map(|x| buffer[(x, y)].symbol())
                            .collect();
                        row.windows(want.len())
                            .position(|w| w.iter().zip(&want).all(|(a, b)| a == b))
                            .map(|x| (x as u16, y))
                    })
                    .unwrap_or_else(|| panic!("{needle:?} is not on screen"))
            };
            let title = find("Help");
            assert_eq!(buffer[title].fg, palette.jade.fg.unwrap_or(Color::Reset));
            assert!(buffer[title].modifier.contains(Modifier::BOLD));
            for key in [
                "Move around",
                "↑ ↓     Choose",
                "Enter   Open it",
                "1–7     Open",
                "q       Quit",
                "Esc     Close help",
            ] {
                let at = find(key);
                assert_eq!(
                    buffer[at].fg,
                    palette.ink.fg.unwrap_or(Color::Reset),
                    "{key}"
                );
                assert!(buffer[at].modifier.contains(Modifier::BOLD), "{key}");
            }
            let border = find("Move around");
            let mut quiet = vec![(border.0 - 2, border.1)];
            for label in [
                "Choose a place",
                "Open it",
                "Open directly",
                "Quit Tinkery",
                "Close help",
            ] {
                quiet.push(find(label));
            }
            for at in quiet {
                assert_eq!(buffer[at].fg, palette.muted.fg.unwrap_or(Color::Reset));
                assert!(!buffer[at].modifier.contains(Modifier::BOLD));
                assert_eq!(buffer[at].modifier.contains(Modifier::DIM), no_color);
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
            assert!(h.help && at(&mut h) == "menu" && h.selected == Place::Think && !h.quit);
        }
        for close in [KeyCode::Esc, KeyCode::F(1), KeyCode::Char('?')] {
            let mut h = home(tmp.path());
            h.handle_key(key(KeyCode::F(1)));
            h.handle_key(key(close));
            assert!(!h.help && at(&mut h) == "menu" && !h.quit);
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
        assert!(!h.help && at(&mut h) == "menu");
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
            "Clarify a goal or save a thought",
            "Your goals and thoughts",
            "Prepare a brief",
            "Show work to approve using the project's status script",
            "Read runs with the project's status script",
            "Review a result",
            "Browse recent pull requests",
        ];
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let mut h = home(tmp.path());
                for _ in 0..PLACES.len() {
                    let screen = snapshot(width, height, &mut h, no_color).unwrap();
                    for (index, description) in descriptions.iter().enumerate() {
                        assert_eq!(
                            screen.lines().nth(4 + index * 2).unwrap().find(description),
                            Some(22)
                        );
                        assert_eq!(screen.matches(description).count(), 1);
                    }
                    assert!(screen.lines().nth(19).unwrap().is_empty());
                    assert!(at(&mut h) == "menu");
                    assert!(!h.quit && h.take_copy_request().is_none());
                    h.handle_key(key(KeyCode::Down));
                }
                let saved = tmp.path().join("TEST-saved");
                let mut h = home(tmp.path());
                think_saved(&mut h, &saved);
                h.handle_key(key(KeyCode::Esc));
                h.tick();
                let screen = snapshot(width, height, &mut h, no_color).unwrap();
                assert_eq!(screen.lines().nth(19).unwrap().trim(), "Saved.");
                for description in descriptions {
                    assert_eq!(screen.matches(description).count(), 1);
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
                assert!(rows[4].starts_with("  1  think"));
                assert!(rows[6].starts_with("  2  tend         10"));
                assert_eq!(rows[6].find("Your goals and thoughts"), Some(22));
                assert!(rows[8].starts_with("  3  shape"));
                assert!(rows[16].starts_with("  7  measure"));
                assert!(rows[17].is_empty() && rows[18].is_empty());
                assert!(rows[19].is_empty());
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
        h.open(Place::Tend);
        settle(&mut h);
        h.handle_key(key(KeyCode::Char('s')));
        assert!(at(&mut h) == "tend", "the garden has no s");
        h.handle_key(key(KeyCode::Enter));
        let mut held = key(KeyCode::Char('s'));
        held.kind = KeyEventKind::Repeat;
        h.handle_key(held);
        assert!(at(&mut h) == "tend", "a held key opens nothing");
        h.handle_key(key(KeyCode::Char('s')));
        assert!(at(&mut h) == "shape");
        let screen = snapshot(100, 30, &mut h, true).unwrap();
        assert!(screen.contains("tinkery › shape"), "{screen}");
        assert!(screen.contains("Checking this Tink."), "{screen}");
        // A terminal may report ? with Shift; it still opens and closes help.
        let shifted = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        h.handle_key(shifted);
        let help = |h: &mut Home| snapshot(100, 30, h, true).unwrap().contains("F1 close");
        assert!(help(&mut h), "Shift+? opens help");
        h.handle_key(shifted);
        assert!(!help(&mut h), "Shift+? closes help");
        h.handle_key(key(KeyCode::Esc));
        assert!(in_detail(&mut h));
        settle(&mut h);
        assert!(snapshot(100, 30, &mut h, true).unwrap().contains("goal a"));
        fs::write(tmp.path().join("a/ledger.json"), r#"{"status": "active"}"#).unwrap();
        h.handle_key(key(KeyCode::Esc));
        h.open(Place::Tend);
        settle(&mut h);
        h.handle_key(key(KeyCode::Enter));
        h.handle_key(key(KeyCode::Char('s')));
        assert!(at(&mut h) == "tend", "no s when the stage is unknown");
    }

    #[test]
    fn returning_from_shape_rereads_the_same_detail_and_garden_stage() {
        for (seed, stage, next) in [
            (
                "status: draft",
                Stage::SeedDrafted,
                "confirm the draft in Shape",
            ),
            (
                "status: confirmed for intake",
                Stage::SeedUnchecked,
                "check the Tink in Shape",
            ),
        ] {
            let tmp = tempfile::tempdir().unwrap();
            session(tmp.path(), "a", "active", "2026-10-09T10:00:00Z", None);
            let mut h = home(tmp.path());
            h.open(Place::Tend);
            settle(&mut h);
            h.handle_key(key(KeyCode::Enter));
            h.handle_key(key(KeyCode::Char('s')));
            assert!(at(&mut h) == "shape");
            session(
                tmp.path(),
                "a",
                "active",
                "2026-10-09T11:00:00Z",
                Some(seed),
            );
            // A newer goal must not steal selection while the same detail refreshes.
            session(tmp.path(), "b", "active", "2026-10-09T12:00:00Z", None);
            h.handle_key(key(KeyCode::Esc));
            settle(&mut h);
            assert!(in_detail(&mut h));
            let screen = snapshot(80, 24, &mut h, true).unwrap();
            assert!(
                screen.contains("goal a") && !screen.contains("goal b"),
                "{screen}"
            );
            assert_eq!(screen.contains("s shape"), stage.can_shape(), "{screen}");
            assert!(screen.contains(next), "{screen}");
            h.handle_key(key(KeyCode::Esc));
            let screen = snapshot(80, 24, &mut h, true).unwrap();
            assert!(screen.contains(stage.label()), "{screen}");
            assert!(h.take_copy_request().is_none());
        }
    }

    #[test]
    fn digits_open_places_and_esc_returns_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('4')));
        assert!(at(&mut h) == "approve");
        let shown = snapshot(100, 30, &mut h, true).unwrap();
        assert!(shown.contains("tinkery › approve") && shown.contains("esc back"));
        assert!(h.take_copy_request().is_none());
        h.handle_key(key(KeyCode::Esc));
        assert!(at(&mut h) == "menu");
        h.handle_key(key(KeyCode::Char('1')));
        assert!(at(&mut h) == "think");
    }

    #[test]
    fn leaving_think_returns_home_not_out_of_the_app() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.open(Place::Think);
        h.handle_key(key(KeyCode::Esc));
        h.tick();
        assert!(at(&mut h) == "menu");
        assert!(!h.quit);
    }

    #[test]
    fn key_bar_preference_lasts_across_visits_to_think() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.open(Place::Think);
        let on = snapshot(100, 30, &mut h, true).unwrap();
        for code in [
            KeyCode::F(1),
            KeyCode::Char('b'),
            KeyCode::Esc,
            KeyCode::Esc,
        ] {
            h.handle_key(key(code));
        }
        h.tick();
        assert!(at(&mut h) == "menu");
        h.open(Place::Think);
        assert_ne!(
            snapshot(100, 30, &mut h, true).unwrap(),
            on,
            "turned off stays off"
        );
        let mut fresh = home(tmp.path());
        fresh.open(Place::Think);
        assert_eq!(
            snapshot(100, 30, &mut fresh, true).unwrap(),
            on,
            "on by default"
        );
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
                ("goal new", "Tink needs checking"),
                ("goal draft", "Tink drafted"),
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
        think_saved(&mut h, &path);
        h.handle_key(key(KeyCode::Char('s')));
        h.tick();
        settle(&mut h);
        let frame = snapshot(100, 30, &mut h, true).unwrap();
        assert!(
            in_detail(&mut h) && !frame.contains("unavailable"),
            "{frame}"
        );
        assert!(frame.contains("goal old"), "{frame}");
        h.handle_key(key(KeyCode::Esc));
        assert!(
            selected_line(&mut h).starts_with("goal old "),
            "selection follows the path"
        );
        h.handle_key(key(KeyCode::Enter));
        assert_eq!(fs::read(path.join("ledger.json")).unwrap(), before);
        #[cfg(unix)]
        {
            let alias = tmp.path().join("alias-root");
            std::os::unix::fs::symlink(tmp.path(), &alias).unwrap();
            h.kept.sessions_root = Some(alias);
            h.open_tend_session(&path.canonicalize().unwrap());
            settle(&mut h);
            assert!(in_detail(&mut h));
            assert!(
                snapshot(100, 30, &mut h, true)
                    .unwrap()
                    .contains("goal old")
            );
            h.kept.sessions_root = Some(tmp.path().to_path_buf());
        }
        h.open_tend_session(&tmp.path().join("missing"));
        settle(&mut h);
        assert!(how_it_works(&mut h).contains("Saved session is unavailable"));
        assert!(!in_detail(&mut h));
        h.kept.sessions_root = Some(tmp.path().join("not-a-directory"));
        fs::write(h.kept.sessions_root.as_ref().unwrap(), "not a directory").unwrap();
        h.open_tend_session(&path);
        settle(&mut h);
        let frame = snapshot(100, 30, &mut h, true).unwrap();
        assert!(frame.contains("Saved session unavailable."), "{frame}");
        assert!(!in_detail(&mut h));
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
                Stage::SeedUnchecked,
                Stage::Stopped
            ]
        );
        for (w, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let mut h = home(tmp.path());
            h.open(Place::Tend);
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
                ("drafted", "Tink drafted"),
                ("ready", "Tink needs checking"),
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
            let mut h = home(tmp.path());
            h.open_tend_session(&path);
            settle(&mut h);
            let screen = snapshot(w, height, &mut h, false).unwrap();
            let normalized = screen.split_whitespace().collect::<Vec<_>>().join(" ");
            for line in receipt.goal.lines() {
                assert!(normalized.contains(line), "{screen}");
            }
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
            assert!(screen.contains("Next: shape it into a Tink in Shape."));
            assert_eq!(
                screen.lines().last().unwrap().trim(),
                "s shape   r history   esc back   F10 menu"
            );
            h.handle_key(key(KeyCode::Char('c')));
            assert!(h.take_copy_request().is_none());
            h.handle_key(key(KeyCode::F(1)));
            assert!(
                !snapshot(w, height, &mut h, false)
                    .unwrap()
                    .contains(path.to_str().unwrap())
            );
            h.handle_key(key(KeyCode::F(1)));
            assert!(how_it_works(&mut h).contains(path.to_str().unwrap()));
            h.handle_key(key(KeyCode::Esc));
            assert!(!in_detail(&mut h));
            h.handle_key(key(KeyCode::Enter));
            h.handle_key(key(KeyCode::F(10)));
            assert!(at(&mut h) == "menu" && !h.quit);
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
                Some("Next: confirm the draft in Shape."),
            ),
            (
                "ready",
                "active",
                Some("status: confirmed for intake"),
                Some("Next: check the Tink in Shape."),
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
            let mut h = home(tmp.path());
            h.open_tend_session(&tmp.path().join(id));
            settle(&mut h);
            let screen = snapshot(100, 30, &mut h, false).unwrap();
            if let Some(next) = next {
                assert!(screen.contains(next));
            } else {
                assert!(!screen.contains("Next:") && !screen.contains("c copy prompt"));
            }
            h.handle_key(key(KeyCode::Char('c')));
            assert!(h.take_copy_request().is_none());
        }
        assert_eq!(
            Stage::from_files("active", Some("status: simulated")),
            Stage::Simulated
        );
        assert!(!Stage::Simulated.can_shape());
        assert_eq!(
            Stage::from_files("active", Some("not a status")),
            Stage::Unknown
        );
        assert!(!Stage::Unknown.can_shape());
    }

    #[test]
    fn q_quits_from_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('q')));
        assert!(h.quit);
    }
}
