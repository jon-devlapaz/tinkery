//! The home menu: the operator's six places, in pipeline order.
//!
//! think · shape · approve · watch · taste · measure. Four are touches, two are views. Only think, shape and
//! measure are built; the others say so plainly. Nothing here starts or steers an agent.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use serde_json::Value;

use crate::{
    Palette,
    shaping::brain_dump::{self, BrainDump},
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
    receipts: Vec<String>,
}

enum View {
    Menu,
    Think(Box<BrainDump>),
    Shape(Shape),
    Measure(Reading),
    Soon(usize),
}

struct Shape {
    sessions: Vec<Session>,
    error: Option<String>,
    selected: usize,
    detail: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub path: PathBuf,
    pub goal: String,
    pub stage: String,
    pub updated: String,
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
            receipts: Vec::new(),
        }
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
        if let View::Think(dump) = &mut self.view {
            dump.tick();
            if dump.quit {
                if let Some(receipt) = dump.finish_handoff() {
                    self.receipts.push(receipt);
                }
                self.view = View::Menu;
            }
        }
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
            _ => None,
        }
    }

    pub fn copy_result(&mut self, sent: bool) {
        if let View::Think(dump) = &mut self.view {
            dump.copy_result(sent);
        }
    }

    pub fn open(&mut self, index: usize) {
        self.selected = index;
        self.help = false;
        self.view = match index {
            0 => View::Think(Box::new((self.make_dump)())),
            1 => View::Shape(match load_sessions(&self.sessions_root) {
                Ok(sessions) => Shape {
                    sessions,
                    error: None,
                    selected: 0,
                    detail: false,
                },
                Err(error) => Shape {
                    sessions: vec![],
                    error: Some(error),
                    selected: 0,
                    detail: false,
                },
            }),
            5 => View::Measure(Reading {
                body: ledger_report(&self.substrate_dir),
                scroll: 0,
            }),
            other => View::Soon(other),
        };
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if let View::Think(dump) = &mut self.view {
            dump.handle_key(key);
            return;
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
                KeyCode::Esc if shape.detail => shape.detail = false,
                KeyCode::Esc | KeyCode::Char('q') => self.view = View::Menu,
                KeyCode::Down | KeyCode::Char('j') if !shape.sessions.is_empty() => {
                    shape.selected = (shape.selected + 1).min(shape.sessions.len() - 1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    shape.selected = shape.selected.saturating_sub(1)
                }
                KeyCode::Enter if !shape.sessions.is_empty() => shape.detail = true,
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

    pub fn count(&self, index: usize) -> Option<String> {
        if index != 1 {
            return None;
        }
        let active = load_sessions(&self.sessions_root)
            .ok()?
            .iter()
            .filter(|s| s.stage != "completed" && s.stage != "stopped")
            .count();
        (active > 0).then(|| active.to_string())
    }
}

/// Seed Me sessions, newest first. Stage comes from Seed Me's own files: the ledger's status and the seed contract's
/// first line. Unreadable sessions are skipped, never guessed.
pub fn load_sessions(root: &Path) -> Result<Vec<Session>, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(format!("Cannot read {}: {error}", root.display())),
    };
    let mut sessions: Vec<Session> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let ledger: Value =
                serde_json::from_str(&fs::read_to_string(path.join("ledger.json")).ok()?).ok()?;
            let status = ledger.get("status")?.as_str()?.to_string();
            let goal = ledger
                .get("goal")
                .and_then(Value::as_str)
                .unwrap_or("(no goal recorded)")
                .to_string();
            let updated = ledger
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let seed = fs::read_to_string(path.join("seed-contract.md")).ok();
            let stage = match (
                status.as_str(),
                seed.as_deref().and_then(|s| s.lines().next()),
            ) {
                ("completed", _) => "completed".into(),
                ("stopped", _) => "stopped".into(),
                (_, Some("status: confirmed for intake")) => "seed confirmed".into(),
                (_, Some("status: simulated")) => "simulated seed".into(),
                (_, Some(_)) => "seed draft".into(),
                (_, None) => "goal confirmed".into(),
            };
            Some(Session {
                path,
                goal,
                stage,
                updated,
            })
        })
        .collect();
    sessions.sort_by(|a, b| b.updated.cmp(&a.updated));
    Ok(sessions)
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
        let help = "Right now\n1–6 or ↑↓ Enter opens a place · ? help · q quits\n\nPlaces\nthink   brain dump → readings → one question → confirm a goal\nshape   your Seed Me sessions (read-only); continue them in any harness\nmeasure the ledger report (read-only)\napprove, watch, taste: not built yet\n\nInside think, F10 or Ctrl-C leaves (it asks first if you'd lose work) and returns here.\nEsc goes back from shape, measure and unbuilt places.";
        frame.render_widget(
            Paragraph::new(help)
                .style(palette.ink)
                .wrap(Wrap { trim: false }),
            body,
        );
        return;
    }
    match &home.view {
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
        View::Shape(shape) => render_shape(frame, shape, body, palette),
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

fn render_shape(frame: &mut Frame, shape: &Shape, area: Rect, palette: Palette) {
    if let Some(error) = &shape.error {
        frame.render_widget(Paragraph::new(error.as_str()).style(palette.ink), area);
        return;
    }
    if shape.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new("No Seed Me sessions yet. Confirm a goal in think to start one.")
                .style(palette.muted),
            area,
        );
        return;
    }
    if shape.detail {
        let s = &shape.sessions[shape.selected];
        let text = Text::from(vec![
            Line::styled(s.goal.clone(), palette.ink),
            Line::from(""),
            Line::styled(s.stage.clone(), palette.jade),
            Line::styled(s.path.display().to_string(), palette.muted),
            Line::from(""),
            Line::styled(
                "Continue with Seed Me in any harness. Esc goes back.",
                palette.muted,
            ),
        ]);
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), area);
        return;
    }
    let width = area.width.saturating_sub(18) as usize;
    let lines: Vec<Line> = shape
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let goal: String = s.goal.chars().take(width.saturating_sub(2)).collect();
            let goal = if s.goal.chars().count() > goal.chars().count() {
                format!("{goal}…")
            } else {
                goal
            };
            let style = if i == shape.selected {
                palette.jade
            } else {
                palette.ink
            };
            Line::from(vec![
                Span::styled(format!("{goal:<width$}"), style),
                Span::styled(format!("  {}", s.stage), palette.muted),
            ])
        })
        .collect();
    let skip = shape
        .selected
        .saturating_sub(area.height.saturating_sub(1) as usize);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((skip as u16, 0)),
        area,
    );
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
mod tests {
    use super::*;

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
            .map(|s| (s.goal.as_str(), s.stage.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("goal new", "seed confirmed"),
                ("goal draft", "seed draft"),
                ("goal old", "goal confirmed")
            ]
        );
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
