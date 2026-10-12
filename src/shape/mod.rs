//! Shape: the saved goals list and one goal's detail, read from Seed Me sessions.
//!
//! Shape only reads. It shows what discovery has landed, says plainly when that is unknown,
//! incomplete or failed, and offers the continuation prompt to copy; it never edits a session,
//! starts an agent or guesses a stage. Everything about Seed Me's files comes from `seed_me`.

pub mod discovery;
pub use discovery::{Discovery, Found, Outcome, Source, scan};

pub use crate::seed_me::{Session, Stage};
use crate::{
    Palette,
    goal::confirmed::{SavedGoal, render_saved_goal},
    seed_me::continuation_prompt,
};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::Paragraph,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// How long a read may be pending before the quiet line says it is still going. A normal
/// read takes milliseconds (about 70 ms for 5,000 sessions on the measured machine), so
/// this only shows when something is actually slow, such as a cold or network disk.
pub(super) const STILL_READING_AFTER: Duration = Duration::from_secs(3);

pub(super) struct Shape {
    pub sessions: Vec<Session>,
    pub error: Option<String>,
    /// Nothing has landed yet: unknown, never "no sessions".
    pub pending: bool,
    /// The read this view waits on has run past [`STILL_READING_AFTER`]. Only the pending
    /// line reads it, and only its wording changes.
    pub still_reading: bool,
    /// The list shown reflects every read asked for. An older empty list is not proof
    /// that nothing was saved since.
    pub current: bool,
    /// A saved session to open in detail once a read that can contain it lands.
    pub want: Option<(PathBuf, Option<PathBuf>)>,
    /// Records the last read could not parse; the list may be missing them.
    pub unreadable: usize,
    /// The discovery version last applied.
    pub seen: u64,
    pub selected: usize,
    pub detail: bool,
    pub scroll: u16,
    pub how: bool,
    pub help_scroll: u16,
    pub notice: String,
    pub copy: Option<String>,
}
impl Shape {
    pub(super) fn new() -> Self {
        Self {
            sessions: vec![],
            error: None,
            pending: true,
            still_reading: false,
            current: false,
            want: None,
            unreadable: 0,
            seen: 0,
            selected: 0,
            detail: false,
            scroll: 0,
            how: false,
            help_scroll: 0,
            notice: String::new(),
            copy: None,
        }
    }
    /// Note how long the read this view waits on has been running (`None`: no worker).
    /// Wording only: it starts nothing and leaves every key as it was.
    pub(super) fn note_wait(&mut self, waited: Option<Duration>) {
        self.still_reading = waited.is_some_and(|waited| waited >= STILL_READING_AFTER);
    }
    /// Take a landed read. Selection follows the session's path, not its position.
    /// A wanted session is opened only from a read that contains it (or that began
    /// after it was asked for); until then the view stays pending.
    pub(super) fn apply(&mut self, outcome: &Outcome, fresh: bool, version: u64) {
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
        self.seen = version;
        self.pending = false;
        match outcome {
            Err(error) => {
                self.error = Some(error.clone());
                self.sessions.clear();
                self.unreadable = 0;
                self.want = None;
            }
            Ok(found) => {
                let keep = self.current().map(|s| s.path.clone());
                self.error = None;
                self.unreadable = found.unreadable;
                self.sessions = found.sessions.clone();
                if let Some(index) = wanted_at {
                    self.selected = index;
                    self.detail = true;
                    self.scroll = 0;
                    self.want = None;
                } else if let Some((path, _)) = self.want.take() {
                    self.error = Some(format!("Saved session is unavailable: {}", path.display()));
                    self.sessions.clear();
                } else {
                    self.selected = keep
                        .and_then(|k| self.sessions.iter().position(|s| s.path == k))
                        .unwrap_or(self.selected)
                        .min(self.sessions.len().saturating_sub(1));
                }
            }
        }
    }
    pub(super) fn current(&self) -> Option<&Session> {
        self.sessions.get(self.selected)
    }
    pub(super) fn copy_prompt(&mut self, skill: Option<&Path>) {
        if let Some(session) = self.current().filter(|s| s.stage.can_copy()) {
            if let Some(skill) = skill {
                self.copy = Some(continuation_prompt(skill, &session.path));
            } else {
                self.notice = "Couldn't copy; try again.".into();
            }
        }
    }
    pub(super) fn help_text(&self, skill: Option<&Path>) -> String {
        if self.how {
            let session = self.current().map_or(String::new(), |s| {
                format!(
                    "\nSession: {}\nStatus: {}",
                    s.path.display(),
                    s.stage.label()
                )
            });
            let skill = skill.map_or("\nSeed Me skill is not configured. Start Tinkery with --shape-pi, --model and --seed-me to copy a continuation prompt.".into(), |s| format!("\nSeed Me skill: {}", s.display()));
            return format!(
                "How it works\nh or Esc returns to Help · PgUp/PgDn scroll\n{session}{skill}\n{}\n\nTinkery reads the saved session files. Copy uses OSC 52; your terminal may require clipboard permission. Paste the prompt yourself. No agent, harness or browser starts. Ended sessions are read-only and have no continuation action.",
                self.error.as_deref().unwrap_or("")
            );
        }
        let copy = if self.detail && self.current().is_some_and(|s| s.stage.can_copy()) {
            "\nc copy prompt to continue in Claude, Codex or Pi"
        } else {
            ""
        };
        format!(
            "Right now\n{}{}\nEsc back · F10 menu\n\nF1 close · h how it works · PgUp/PgDn scroll\n\nNothing runs by itself. A saved goal is not seed or implementation approval.",
            if self.detail {
                "Read the saved goal. ↑↓ / j k or PgUp/PgDn scroll."
            } else {
                "↑↓ / j k move · Enter opens the saved goal."
            },
            copy
        )
    }
}

fn clipped(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        let cells = g.width();
        if used + cells > width - 1 {
            break;
        }
        result.push_str(g);
        used += cells;
    }
    result.push('…');
    result
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

pub(super) fn render(frame: &mut Frame, shape: &mut Shape, area: Rect, palette: Palette) {
    let keys = if shape.detail {
        if shape.current().is_some_and(|s| s.stage.can_copy()) {
            "c copy prompt   esc back   10 menu"
        } else {
            "esc back   10 menu"
        }
    } else if shape.sessions.is_empty() || shape.want.is_some() {
        // Nothing on screen to move to or open: reading, unavailable or empty.
        "esc back"
    } else {
        "↑↓ move   enter open   esc back"
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
    let stale_empty = shape.sessions.is_empty()
        && shape.unreadable == 0
        && shape.error.is_none()
        && !shape.current;
    if shape.pending || shape.want.is_some() || stale_empty {
        let status = if shape.still_reading {
            "Still reading your sessions. Esc goes back."
        } else {
            "Reading your sessions. Esc goes back."
        };
        frame.render_widget(Paragraph::new(status).style(palette.muted), area);
        return;
    }
    if shape.error.is_some() {
        frame.render_widget(
            Paragraph::new("Saved session unavailable. F1 then h for details.").style(palette.ink),
            area,
        );
        return;
    }
    if shape.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new(if shape.unreadable > 0 {
                format!(
                    "{} couldn't be read, so nothing can be listed. Esc goes back.",
                    records(shape.unreadable)
                )
            } else {
                "No Seed Me sessions yet. Confirm a goal in think to start one.".into()
            })
            .style(palette.muted),
            area,
        );
        return;
    }
    if shape.detail {
        let session = &shape.sessions[shape.selected];
        render_saved_goal(
            frame,
            &mut shape.scroll,
            area,
            palette,
            SavedGoal {
                goal: &session.goal,
                next: session.stage.next(),
                notice: &shape.notice,
                saved: false,
            },
        );
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    let lines: Vec<Line> = shape
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let (goal, status) = row(session, usize::from(area.width), now);
            Line::from(vec![
                Span::styled(
                    goal,
                    if i == shape.selected {
                        palette.jade
                    } else {
                        palette.ink
                    },
                ),
                Span::styled(status, palette.muted),
            ])
        })
        .collect();
    let note = shape.unreadable > 0 && area.height > 1;
    let list = Rect::new(area.x, area.y, area.width, area.height - u16::from(note));
    let skip = shape
        .selected
        .saturating_sub(list.height.saturating_sub(1) as usize);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((skip as u16, 0)),
        list,
    );
    if note {
        frame.render_widget(
            Paragraph::new(format!(
                "{} couldn't be read; this list may be missing sessions.",
                records(shape.unreadable)
            ))
            .style(palette.muted),
            Rect::new(area.x, area.bottom() - 1, area.width, 1),
        );
    }
}
fn records(n: usize) -> String {
    if n == 1 {
        "1 saved record".into()
    } else {
        format!("{n} saved records")
    }
}

pub(super) fn timestamp(value: &str) -> Option<u64> {
    let number = |a, b| value.get(a..b)?.parse::<i64>().ok();
    if value.get(4..5)? != "-"
        || value.get(7..8)? != "-"
        || value.get(10..11)? != "T"
        || value.get(13..14)? != ":"
        || value.get(16..17)? != ":"
    {
        return None;
    }
    let (year, month, day, hour, minute, second) = (
        number(0, 4)?,
        number(5, 7)?,
        number(8, 10)?,
        number(11, 13)?,
        number(14, 16)?,
        number(17, 19)?,
    );
    if !(1..=12).contains(&month)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..60).contains(&second)
    {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=days[(month - 1) as usize]).contains(&day) {
        return None;
    }
    let tail = value.get(19..)?;
    let zone = if let Some(fraction) = tail.strip_prefix('.') {
        let n = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return None;
        }
        fraction.get(n..)?
    } else {
        tail
    };
    let offset = if zone == "Z" {
        0
    } else {
        if zone.len() != 6 || zone.get(3..4)? != ":" {
            return None;
        }
        let h = zone.get(1..3)?.parse::<i64>().ok()?;
        let m = zone.get(4..6)?.parse::<i64>().ok()?;
        if !(0..24).contains(&h) || !(0..60).contains(&m) {
            return None;
        }
        match zone.get(..1)? {
            "+" => h * 3600 + m * 60,
            "-" => -(h * 3600 + m * 60),
            _ => return None,
        }
    };
    // Gregorian civil date to days since 1970-01-01.
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let days = era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + day - 1 - 719468;
    u64::try_from(days * 86400 + hour * 3600 + minute * 60 + second - offset).ok()
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

/// Seed Me sessions, newest first (blocking). Unreadable records are left out here;
/// [`scan`] reports how many.
pub fn load_sessions(root: &Path) -> Result<Vec<Session>, String> {
    scan(root).map(|found| found.sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            path: "/TEST/session".into(),
            goal: "界👩‍💻 Café goal with many words\nWhy: never in row".into(),
            stage: Stage::SeedDrafted,
            updated: "2026-10-11T10:00:00Z".into(),
        };
        let (title, status) = row(&s, 40, timestamp("2026-10-11T12:00:00Z"));
        assert!(!title.contains("Why:"));
        assert!(title.trim_end().ends_with('…'));
        assert_eq!(status, "seed drafted · 2h ago");
        assert_eq!(title.width() + status.width(), 40);
        assert!(title.contains("👩‍💻"));
        assert_eq!(row(&s, 40, None).1, "seed drafted · time unknown");
    }
}
