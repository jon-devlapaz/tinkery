//! The home menu: the operator's six places, in pipeline order.
//!
//! think · shape · approve · watch · taste · measure. Four are touches, two are views. Only think, shape and
//! measure are built; the others say so plainly. Nothing here starts or steers an agent.

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    Palette,
    shaping::brain_dump::{self, BrainDump},
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

/// Name, one-line purpose, and whether the place exists yet.
pub const PLACES: [(&str, &str, bool); 6] = [
    (
        "think",
        "Brain-dump something and shape it into a goal you can confirm.",
        true,
    ),
    (
        "shape",
        "Your Seed Me sessions: confirmed goals and seeds in progress.",
        true,
    ),
    (
        "approve",
        "Approve a sequence of slices once. Not built yet.",
        false,
    ),
    ("watch", "What is running now. Not built yet.", false),
    (
        "taste",
        "What is waiting for your review. Not built yet.",
        false,
    ),
    (
        "measure",
        "The ledger: useful changes per hour of your attention.",
        true,
    ),
];
mod discovery;
mod shape;
use discovery::Discovery;
pub use discovery::{Found, Outcome, Source, scan};
use shape::Shape;
pub use shape::{Session, Stage};

const FIRST_ROW: u16 = 4;
const ROW_GAP: u16 = 2;

pub struct Home {
    pub quit: bool,
    make_dump: Box<dyn Fn() -> BrainDump>,
    sessions_root: PathBuf,
    substrate_dir: PathBuf,
    selected: usize,
    view: View,
    help: bool,
    /// The muted key bar preference lasts the session, across visits to think.
    key_bar: bool,
    receipts: Vec<String>,
    seed_me: Option<PathBuf>,
    discovery: Discovery,
}

enum View {
    Menu,
    Think(Box<BrainDump>),
    Shape(Shape),
    Measure(Reading),
    Soon(usize),
}

struct Reading {
    body: String,
    scroll: u16,
}

impl Home {
    pub fn new(
        make_dump: Box<dyn Fn() -> BrainDump>,
        sessions_root: PathBuf,
        substrate_dir: PathBuf,
    ) -> Self {
        Self {
            quit: false,
            make_dump,
            sessions_root,
            substrate_dir,
            selected: 0,
            view: View::Menu,
            help: false,
            key_bar: true,
            receipts: Vec::new(),
            seed_me: None,
            discovery: Discovery::new(Arc::new(scan)),
        }
    }

    /// Replace the blocking read (tests hold it open to prove paint and input never wait).
    pub fn with_discovery_source(mut self, source: Source) -> Self {
        self.discovery = Discovery::new(source);
        self
    }

    pub fn with_seed_me(mut self, skill: PathBuf) -> Self {
        self.seed_me = Some(skill);
        self
    }

    pub fn from_receipt(mut dump: BrainDump, session: &Path, open_shape: bool) -> Self {
        let (default_root, substrate) = Self::default_paths();
        let root = session
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(default_root);
        let mut home = Self::new(dump.fresh_factory(), root, substrate);
        home.key_bar = dump.key_bar_on();
        home.seed_me = dump.receipt.as_ref().map(|r| r.skill.clone());
        if let Some(receipt) = dump.finish_handoff() {
            home.receipts.push(receipt);
        }
        if open_shape {
            home.open_shape_session(session);
        }
        home
    }

    /// Default Seed Me session location and installed tink-substrate package.
    pub fn default_paths() -> (PathBuf, PathBuf) {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let substrate = std::env::var_os("TINK_SUBSTRATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude/skills/tink-substrate"));
        (home.join(".local/share/seed-me/sessions"), substrate)
    }

    pub fn tick(&mut self) {
        if !self.discovery.requested() {
            self.discovery.request(&self.sessions_root);
        }
        self.discovery.poll();
        self.sync_shape();
        let mut shape_request = None;
        if let View::Think(dump) = &mut self.view {
            dump.tick();
            shape_request = dump.take_shape_request();
            if let Some(receipt) = &dump.receipt {
                self.seed_me = Some(receipt.skill.clone());
            }
            if dump.quit || shape_request.is_some() {
                self.key_bar = dump.key_bar_on();
                if let Some(receipt) = dump.finish_handoff() {
                    self.receipts.push(receipt);
                }
                self.view = View::Menu;
                // Back from think: a goal may have been saved, so read again.
                self.discovery.request(&self.sessions_root);
            }
        }
        if let Some(path) = shape_request {
            self.open_shape_session(&path);
        }
    }

    /// Show the newest landed read in shape. While a saved session's detail is open
    /// nothing is replaced under the reader; the list catches up when they go back.
    fn sync_shape(&mut self) {
        let View::Shape(shape) = &mut self.view else {
            return;
        };
        let Some(landed) = self.discovery.landed() else {
            return;
        };
        let version = self.discovery.version();
        if shape.seen == version || (shape.detail && shape.want.is_none()) {
            return;
        }
        shape.apply(&landed.outcome, self.discovery.fresh(), version);
    }

    /// Read once and wait a bounded time, for the non-interactive `--snapshot` view only.
    pub fn discover_for_snapshot(&mut self, wait: Duration) {
        self.discovery.request(&self.sessions_root);
        let deadline = Instant::now() + wait;
        while !self.discovery.poll() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn open_shape_session(&mut self, path: &Path) {
        self.open(1);
        if let View::Shape(shape) = &mut self.view {
            // One stat of a path the handoff just wrote; matching is otherwise by identity.
            shape.want = Some((path.to_path_buf(), path.canonicalize().ok()));
            shape.seen = 0;
        }
        self.sync_shape();
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

    pub fn paste(&mut self, text: &str) {
        if let View::Think(dump) = &mut self.view {
            dump.paste(text);
        }
    }

    pub fn take_copy_request(&mut self) -> Option<String> {
        match &mut self.view {
            View::Think(dump) => dump.take_copy_request(),
            View::Shape(shape) => shape.copy.take(),
            _ => None,
        }
    }

    pub fn copy_result(&mut self, sent: bool) {
        match &mut self.view {
            View::Think(dump) => dump.copy_result(sent),
            View::Shape(shape) => {
                shape.notice = if sent {
                    "Copied."
                } else {
                    "Couldn't copy; try again."
                }
                .into()
            }
            _ => {}
        }
    }

    pub fn open(&mut self, index: usize) {
        self.selected = index;
        self.help = false;
        self.view = match index {
            0 => View::Think(Box::new((self.make_dump)().with_key_bar(self.key_bar))),
            1 => {
                // Deliberate reopening reads again; the last result shows meanwhile.
                self.discovery.request(&self.sessions_root);
                View::Shape(Shape::new())
            }
            5 => View::Measure(Reading {
                body: ledger_report(&self.substrate_dir),
                scroll: 0,
            }),
            other => View::Soon(other),
        };
        self.sync_shape();
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if let View::Think(dump) = &mut self.view {
            dump.handle_key(key);
            return;
        }
        if matches!(self.view, View::Shape(_)) {
            if key.modifiers != KeyModifiers::NONE
                || key.kind == KeyEventKind::Repeat
                    && matches!(
                        key.code,
                        KeyCode::Char('c') | KeyCode::F(1) | KeyCode::F(10) | KeyCode::Enter
                    )
            {
                return;
            }
            if key.code == KeyCode::F(10) {
                self.view = View::Menu;
                self.help = false;
                return;
            }
            if matches!(key.code, KeyCode::F(1) | KeyCode::Char('?')) {
                self.help = !self.help;
                if let View::Shape(shape) = &mut self.view {
                    shape.how = false;
                    shape.help_scroll = 0;
                }
                return;
            }
            if self.help {
                if let View::Shape(shape) = &mut self.view {
                    match key.code {
                        KeyCode::Char('h') => {
                            shape.how = !shape.how;
                            shape.help_scroll = 0;
                        }
                        KeyCode::Esc if shape.how => {
                            shape.how = false;
                            shape.help_scroll = 0;
                        }
                        KeyCode::Esc => self.help = false,
                        KeyCode::Down | KeyCode::PageDown => {
                            shape.help_scroll = shape.help_scroll.saturating_add(5)
                        }
                        KeyCode::Up | KeyCode::PageUp => {
                            shape.help_scroll = shape.help_scroll.saturating_sub(5)
                        }
                        _ => {}
                    }
                }
                return;
            }
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') || key.code == KeyCode::F(10) {
            self.quit = true;
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
        match &mut self.view {
            View::Menu => match key.code {
                KeyCode::Char(c @ '1'..='6') => self.open(c as usize - '1' as usize),
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
            View::Shape(shape) => match key.code {
                KeyCode::Esc if shape.detail => {
                    shape.detail = false;
                    shape.scroll = 0;
                    shape.notice.clear();
                }
                KeyCode::Esc => self.view = View::Menu,
                KeyCode::Char('c') if shape.detail => shape.copy_prompt(self.seed_me.as_deref()),
                KeyCode::Down | KeyCode::Char('j') if shape.detail => {
                    shape.scroll = shape.scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') if shape.detail => {
                    shape.scroll = shape.scroll.saturating_sub(1)
                }
                KeyCode::PageDown if shape.detail => shape.scroll = shape.scroll.saturating_add(5),
                KeyCode::PageUp if shape.detail => shape.scroll = shape.scroll.saturating_sub(5),
                KeyCode::Down | KeyCode::Char('j') if !shape.sessions.is_empty() => {
                    shape.selected = (shape.selected + 1).min(shape.sessions.len() - 1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    shape.selected = shape.selected.saturating_sub(1)
                }
                KeyCode::Enter if !shape.sessions.is_empty() => {
                    shape.detail = true;
                    shape.scroll = 0;
                    shape.notice.clear();
                }
                _ => {}
            },
            View::Measure(reading) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.view = View::Menu,
                KeyCode::Down | KeyCode::Char('j') => {
                    reading.scroll = reading.scroll.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    reading.scroll = reading.scroll.saturating_sub(1)
                }
                KeyCode::PageDown => reading.scroll = reading.scroll.saturating_add(10),
                KeyCode::PageUp => reading.scroll = reading.scroll.saturating_sub(10),
                _ => {}
            },
            View::Soon(_) => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter) {
                    self.view = View::Menu;
                }
            }
            View::Think(_) => {}
        }
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        if let View::Think(dump) = &mut self.view {
            dump.handle_mouse(event, area);
            return;
        }
        if !matches!(self.view, View::Menu) || event.kind != MouseEventKind::Down(MouseButton::Left)
        {
            return;
        }
        if event.row == 1 && event.column + 3 >= area.width {
            self.help = !self.help;
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

/// Seed Me sessions, newest first (blocking). Unreadable records are left out here;
/// [`scan`] reports how many.
pub fn load_sessions(root: &Path) -> Result<Vec<Session>, String> {
    scan(root).map(|found| found.sessions)
}

/// The ledger's text report from the installed tink-substrate. Unknown stays unknown.
pub fn ledger_report(substrate_dir: &Path) -> String {
    match Command::new("python3")
        .args(["-B", "-m", "tink_substrate", "ledger", "report"])
        .current_dir(substrate_dir)
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        Ok(out) => format!(
            "Ledger unknown: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("report failed")
        ),
        Err(error) => format!(
            "Ledger unknown: cannot run tink-substrate in {} ({error})",
            substrate_dir.display()
        ),
    }
}

pub fn render(frame: &mut Frame, home: &mut Home, palette: Palette) {
    if let View::Think(dump) = &mut home.view {
        brain_dump::render(frame, dump, palette);
        return;
    }
    let area = frame.area();
    frame.render_widget(Paragraph::new("").style(palette.ink), area);
    let title = match &home.view {
        View::Menu => "tinkery".to_string(),
        View::Shape(_) => "tinkery › shape".into(),
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
    let body = Rect::new(
        2,
        3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    );
    if home.help {
        if let View::Shape(shape) = &mut home.view {
            let p = Paragraph::new(shape.help_text(home.seed_me.as_deref()))
                .wrap(Wrap { trim: false })
                .style(palette.ink);
            let max = p
                .line_count(body.width)
                .saturating_sub(body.height as usize) as u16;
            shape.help_scroll = shape.help_scroll.min(max);
            frame.render_widget(p.scroll((shape.help_scroll, 0)), body);
            return;
        }
        let help = "Right now\n1–6 or ↑↓ Enter opens a place · ? help · q quits\n\nPlaces\nthink   brain dump → readings → one question → confirm a goal\nshape   your saved goals and seeds; copy a prompt to continue\nmeasure the ledger report (read-only)\napprove, watch, taste: not built yet\n\nInside think, F10 returns here (it asks first if you'd lose work).\nEsc goes back from shape, measure and unbuilt places.";
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
                let mut spans = vec![Span::styled(format!("     {}  {name}", i + 1), style)];
                if let Some(count) = home.count(i) {
                    spans.push(Span::styled(
                        format!("{:>width$}", count, width = 30 - name.len()),
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
                Paragraph::new(line)
                    .style(palette.muted)
                    .alignment(ratatui::layout::Alignment::Center),
                Rect::new(0, area.height.saturating_sub(2), area.width, 1),
            );
        }
        View::Shape(shape) => shape::render(frame, shape, body, palette),
        View::Measure(reading) => frame.render_widget(
            Paragraph::new(reading.body.as_str())
                .style(palette.ink)
                .scroll((reading.scroll, 0)),
            body,
        ),
        View::Soon(i) => frame.render_widget(
            Paragraph::new(format!("{}\n\nNot built yet. Esc goes back.", PLACES[*i].1))
                .style(palette.muted),
            body,
        ),
        View::Think(_) => {}
    }
}

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
mod menu_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn home(root: &Path) -> Home {
        Home::new(
            Box::new(BrainDump::default),
            root.to_path_buf(),
            root.join("no-substrate"),
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
    fn home_shows_six_places_and_only_counts_what_exists() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "a", "active", "2026-10-09T10:00:00Z", None);
        let mut h = home(tmp.path());
        settle(&mut h);
        let screen = snapshot(100, 30, &mut h, true).unwrap();
        for name in ["think", "shape", "approve", "watch", "taste", "measure"] {
            assert!(screen.contains(name), "{name} missing:\n{screen}");
        }
        assert!(screen.contains("tinkery"));
        assert_eq!(h.count(1), Some("1".into()));
        assert_eq!(h.count(0), None);
    }

    #[test]
    fn digits_open_places_and_esc_returns_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('3')));
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
            dump.quit = true;
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
    fn shape_reads_stage_from_seed_me_files_newest_first() {
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
    fn receipt_shape_request_selects_exact_session_and_missing_is_explicit() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "old", "active", "2026-10-01T00:00:00Z", None);
        session(tmp.path(), "new", "active", "2026-10-10T00:00:00Z", None);
        let path = tmp.path().join("old");
        let before = fs::read(path.join("ledger.json")).unwrap();
        let mut h = home(tmp.path());
        h.open(0);
        let View::Think(dump) = &mut h.view else {
            panic!()
        };
        dump.receipt = Some(crate::shaping::brain_dump::handoff::Receipt::test(
            path.clone(),
        ));
        h.handle_key(key(KeyCode::Char('s')));
        h.tick();
        settle(&mut h);
        let View::Shape(shape) = &h.view else {
            panic!()
        };
        assert_eq!(shape.selected, 1);
        assert_eq!(
            shape.sessions[shape.selected].path,
            path.canonicalize().unwrap()
        );
        assert!(shape.detail && shape.error.is_none());
        assert_eq!(fs::read(path.join("ledger.json")).unwrap(), before);
        #[cfg(unix)]
        {
            let alias = tmp.path().join("alias-root");
            std::os::unix::fs::symlink(tmp.path(), &alias).unwrap();
            h.sessions_root = alias;
            h.open_shape_session(&path.canonicalize().unwrap());
            settle(&mut h);
            let View::Shape(shape) = &h.view else {
                panic!()
            };
            assert_eq!(shape.selected, 1);
            assert_eq!(
                shape.sessions[shape.selected].path.canonicalize().unwrap(),
                path.canonicalize().unwrap()
            );
            h.sessions_root = tmp.path().to_path_buf();
        }
        h.open_shape_session(&tmp.path().join("missing"));
        settle(&mut h);
        let View::Shape(shape) = &h.view else {
            panic!()
        };
        assert!(shape.sessions.is_empty());
        assert!(
            shape
                .error
                .as_ref()
                .unwrap()
                .contains("Saved session is unavailable")
        );
        h.sessions_root = tmp.path().join("not-a-directory");
        fs::write(&h.sessions_root, "not a directory").unwrap();
        h.open_shape_session(&path);
        settle(&mut h);
        let View::Shape(shape) = &h.view else {
            panic!()
        };
        assert!(
            shape.error.is_some() && shape.sessions.is_empty(),
            "{:?} {:?} {} {}",
            shape.error,
            shape.want,
            shape.pending,
            shape.sessions.len()
        );
    }

    #[test]
    fn shape_rows_derive_each_state_and_only_show_goal_lines_newest_first() {
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
            for (row, label) in ["goal saved", "seed drafted", "seed ready", "ended"]
                .into_iter()
                .enumerate()
            {
                assert!(
                    screen.lines().nth(row + 3).unwrap().contains(label),
                    "{screen}"
                );
                assert!(screen.lines().nth(row + 3).unwrap().contains('…'));
                assert!(screen.lines().nth(row + 3).unwrap().contains(" · "));
            }
            assert_eq!(
                screen.lines().last().unwrap().trim(),
                "↑↓ move   enter open   esc back"
            );
        }
    }

    #[test]
    fn shape_detail_preserves_receipt_block_and_copy_is_exact_repeatable_and_manual() {
        let tmp = tempfile::tempdir().unwrap();
        session(tmp.path(), "saved", "active", "2026-10-11T00:00:00Z", None);
        let path = tmp.path().join("saved").canonicalize().unwrap();
        let goal = "I can read Café 👩‍💻 in a calm dark theme.\nWhy: Night work.\nDone when: The block stays readable.\nMust: Keep every line.\nMust not: Run agents.";
        let mut ledger: Value =
            serde_json::from_slice(&fs::read(path.join("ledger.json")).unwrap()).unwrap();
        ledger["goal"] = goal.into();
        fs::write(path.join("ledger.json"), ledger.to_string()).unwrap();
        let before = fs::read(path.join("ledger.json")).unwrap();
        let mut receipt = brain_dump::handoff::Receipt::test(path.clone());
        receipt.goal = goal.into();
        for (w, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let mut h = home(tmp.path()).with_seed_me(receipt.skill.clone());
            h.open_shape_session(&path);
            settle(&mut h);
            let screen = snapshot(w, height, &mut h, false).unwrap();
            let normalized = screen.split_whitespace().collect::<Vec<_>>().join(" ");
            for line in receipt.goal.lines() {
                assert!(normalized.contains(line), "{screen}");
            }
            let View::Shape(shape) = &h.view else {
                panic!()
            };
            assert_eq!(shape.current().unwrap().goal, receipt.goal);
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
                "c copy prompt   esc back   10 menu"
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
            let View::Shape(shape) = &h.view else {
                panic!()
            };
            assert!(
                shape
                    .help_text(h.seed_me.as_deref())
                    .contains(path.to_str().unwrap())
            );
            h.handle_key(key(KeyCode::Esc));
            h.handle_key(key(KeyCode::Esc));
            h.handle_key(key(KeyCode::Esc));
            let View::Shape(shape) = &h.view else {
                panic!()
            };
            assert!(!shape.detail);
            h.handle_key(key(KeyCode::Enter));
            h.handle_key(key(KeyCode::F(10)));
            assert!(matches!(h.view, View::Menu) && !h.quit);
            assert_eq!(fs::read(path.join("ledger.json")).unwrap(), before);
        }
    }

    #[test]
    fn shape_next_action_follows_status_and_ended_sessions_cannot_copy() {
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
            h.open_shape_session(&tmp.path().join(id));
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
    fn missing_ledger_is_unknown_not_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(ledger_report(&tmp.path().join("missing")).starts_with("Ledger unknown"));
    }

    #[test]
    fn q_quits_from_home() {
        let tmp = tempfile::tempdir().unwrap();
        let mut h = home(tmp.path());
        h.handle_key(key(KeyCode::Char('q')));
        assert!(h.quit);
    }
}
