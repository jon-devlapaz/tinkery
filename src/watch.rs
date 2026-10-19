//! Watch: the runs in the one tink-sdlc checkout the operator named, read through its own
//! `status --json` (API 1) only when Watch opens or the operator presses `r`. Runs waiting on a
//! person come first; the rest get one quiet line. It never runs verify, mark, decide, stage or
//! any write, discovers no checkout, polls nothing, and refuses an interface or state it doesn't
//! know rather than guess.

mod run;
#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use run::{DEADLINE, Read, SCRIPT};
use serde_json::Value;

/// What tink-sdlc says about a run's verification, from API 1. Anything else is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Blocked,
    Running,
    NotRun,
    Current,
    Stale,
    Failed,
    Interrupted,
    Invalid,
}

impl State {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "blocked" => Self::Blocked,
            "running" => Self::Running,
            "not-run" => Self::NotRun,
            "current" => Self::Current,
            "stale" => Self::Stale,
            "failed" => Self::Failed,
            "interrupted" => Self::Interrupted,
            "invalid" => Self::Invalid,
            _ => return None,
        })
    }

    /// Whether a person is owed something: a gate decision (tink-sdlc reports `blocked` only
    /// while a gate isn't approved), or a failed, interrupted or unreadable run.
    fn needs_you(self) -> bool {
        matches!(
            self,
            Self::Blocked | Self::Failed | Self::Interrupted | Self::Invalid
        )
    }

    fn words(self) -> &'static str {
        match self {
            Self::Blocked => "waiting on a decision",
            Self::Running => "running",
            Self::NotRun => "not verified yet",
            Self::Current => "verified",
            Self::Stale => "verification out of date",
            Self::Failed => "verification failed",
            Self::Interrupted => "verification interrupted",
            Self::Invalid => "can't be read",
        }
    }
}

/// One run as the list summary reports it.
#[derive(Debug, Clone, PartialEq)]
struct Run {
    slug: String,
    state: State,
    /// tink-sdlc's next action, or the reason an invalid run can't be read.
    next: String,
    passed: u64,
    total: u64,
}

/// One run's detail, from `status <slug> --json`.
#[derive(Debug, Clone, PartialEq)]
struct Detail {
    state: State,
    next: String,
    gates: Vec<(u64, String)>,
    passed: usize,
    proven: usize,
    attested: usize,
    errors: Vec<String>,
}

/// Refuse anything but tink-sdlc API 1.
fn interface(value: &Value) -> Result<(), String> {
    let protocol = value["protocol"].as_str();
    let version = value["api_version"].as_u64();
    if protocol == Some("tink-sdlc") && version == Some(1) && value["api_version"].is_u64() {
        return Ok(());
    }
    Err(format!(
        "The checkout answered with an interface Watch doesn't read (protocol {}, API {}). Watch reads tink-sdlc API 1.",
        protocol.unwrap_or("missing"),
        value["api_version"]
    ))
}

fn state(value: &Value) -> Result<State, String> {
    let word = value["verification_status"].as_str().unwrap_or("missing");
    State::parse(word).ok_or_else(|| {
        format!("tink-sdlc reported a state Watch doesn't know ({word}); nothing is shown rather than a guess.")
    })
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("A run is missing {key}; nothing is shown rather than a guess."))
}

/// The list summary, in tink-sdlc's order.
fn parse_list(output: &str) -> Result<Vec<Run>, String> {
    let value: Value =
        serde_json::from_str(output).map_err(|e| format!("status didn't print JSON: {e}"))?;
    interface(&value)?;
    let runs = value["runs"]
        .as_array()
        .ok_or("status printed no runs list.")?;
    runs.iter()
        .map(|run| {
            let state = state(run)?;
            let count = |key| {
                run["checklist_summary"][key]
                    .as_u64()
                    .ok_or("A run is missing its checklist summary.")
            };
            Ok(Run {
                slug: text(run, "slug")?,
                next: text(
                    run,
                    if state == State::Invalid {
                        "error"
                    } else {
                        "next_action"
                    },
                )?,
                passed: count("passed")?,
                total: count("total")?,
                state,
            })
        })
        .collect()
}

/// One run's detail.
fn parse_detail(output: &str) -> Result<Detail, String> {
    let value: Value =
        serde_json::from_str(output).map_err(|e| format!("status didn't print JSON: {e}"))?;
    interface(&value)?;
    let run = &value["run"];
    let gates = run["gates"]
        .as_array()
        .ok_or("The run has no gates list.")?
        .iter()
        .map(|gate| {
            let stage = gate["stage"].as_u64().ok_or("A gate has no stage.")?;
            let status = gate["status"].as_str().unwrap_or("missing");
            if !matches!(
                status,
                "approved" | "pending" | "stale" | "changes-requested"
            ) {
                return Err(format!("A gate has a state Watch doesn't know ({status})."));
            }
            Ok((stage, status.to_owned()))
        })
        .collect::<Result<_, String>>()?;
    let items = run["checklist"]
        .as_array()
        .ok_or("The run has no checklist.")?;
    let count = |key: &str, word: &str| {
        items
            .iter()
            .filter(|i| i[key].as_str() == Some(word))
            .count()
    };
    Ok(Detail {
        state: state(run)?,
        next: text(run, "next_action")?,
        gates,
        passed: count("status", "passed"),
        proven: count("proof", "automated"),
        attested: count("proof", "attested"),
        errors: run["errors"]
            .as_array()
            .map(|e| {
                e.iter()
                    .filter_map(|e| e.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Runs that need a person first, then the rest, each group in tink-sdlc's order.
fn ordered(runs: &[Run]) -> Vec<&Run> {
    let (mut needs, quiet): (Vec<&Run>, Vec<&Run>) =
        runs.iter().partition(|run| run.state.needs_you());
    needs.extend(quiet);
    needs
}

/// A read that hasn't landed, or what it found and when.
enum Load<T> {
    Reading(Read),
    Done(Result<T, String>, Instant),
}

impl<T> Load<T> {
    fn start(root: &Path, args: Vec<String>, deadline: Duration) -> Self {
        Self::Reading(Read::start(root, args, deadline))
    }

    fn poll(&mut self, parse: fn(&str) -> Result<T, String>) {
        if let Self::Reading(read) = self
            && let Some(result) = read.poll()
        {
            *self = Self::Done(result.and_then(|out| parse(&out)), Instant::now());
        }
    }
}

/// Watch's screen: the list, at most one run's detail, and the read behind each.
pub(crate) struct Watch {
    root: Option<PathBuf>,
    list: Option<Load<Vec<Run>>>,
    selected: usize,
    detail: Option<(String, Load<Detail>)>,
    deadline: Duration,
}

impl Watch {
    /// Watch over `root`, reading its runs now; without a root it only says how to name one.
    pub(crate) fn open(root: Option<PathBuf>) -> Self {
        Self::with_deadline(root, DEADLINE)
    }

    fn with_deadline(root: Option<PathBuf>, deadline: Duration) -> Self {
        let mut watch = Self {
            root,
            list: None,
            selected: 0,
            detail: None,
            deadline,
        };
        watch.read_list();
        watch
    }

    fn read_list(&mut self) {
        let Some(root) = &self.root else { return };
        self.list = Some(if root.join(SCRIPT).is_file() {
            Load::start(root, vec!["status".into(), "--json".into()], self.deadline)
        } else {
            Load::Done(
                Err(format!(
                    "No tink-sdlc in {}: {SCRIPT} is missing.",
                    root.display()
                )),
                Instant::now(),
            )
        });
    }

    /// Collect any finished read; never blocks.
    pub(crate) fn poll(&mut self) {
        if let Some(list) = &mut self.list {
            list.poll(parse_list);
        }
        if let Some((_, detail)) = &mut self.detail {
            detail.poll(parse_detail);
        }
    }

    fn runs(&self) -> Option<&[Run]> {
        match &self.list {
            Some(Load::Done(Ok(runs), _)) => Some(runs),
            _ => None,
        }
    }

    /// One key. Returns true when the operator leaves Watch; leaving drops and stops any read.
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat
            && matches!(key.code, KeyCode::Char('r') | KeyCode::Enter)
        {
            return false;
        }
        if let Some((slug, _)) = &self.detail {
            match key.code {
                KeyCode::Esc => self.detail = None,
                KeyCode::Char('q') => return true,
                KeyCode::Char('r') => self.read_detail(slug.clone()),
                _ => {}
            }
            return false;
        }
        let count = self.runs().map_or(0, <[Run]>::len);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Char('r') => self.read_list(),
            KeyCode::Down | KeyCode::Char('j') if count > 0 => {
                self.selected = (self.selected + 1).min(count - 1)
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Enter => {
                if let Some(run) = self
                    .runs()
                    .and_then(|runs| ordered(runs).get(self.selected).map(|r| r.slug.clone()))
                {
                    self.read_detail(run);
                }
            }
            _ => {}
        }
        false
    }

    fn read_detail(&mut self, slug: String) {
        if let Some(root) = &self.root {
            let read = Load::start(
                root,
                vec!["status".into(), slug.clone(), "--json".into()],
                self.deadline,
            );
            self.detail = Some((slug, read));
        }
    }

    /// Watch's help: what it reads, when, and what it never does.
    pub(crate) fn help_text(&self) -> &'static str {
        concat!(
            "Right now\n↑↓ choose · Enter details · r read again · Esc back · ? help\n\n",
            "What Watch does\nIt reads the runs in the tink-sdlc checkout you named with --sdlc-root, ",
            "by running that checkout's own `python3 _system/scripts/sdlc.py status --json` when you open ",
            "Watch or press r. It never runs verify, mark, decide or stage, and changes nothing.\n\n",
            "Needs you lists runs waiting on a decision, or whose verification failed, was interrupted ",
            "or can't be read. Everything else gets one quiet line."
        )
    }

    /// Paint the list or the open detail into `area`.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect, palette: Palette) {
        let keys = if self.detail.is_some() {
            "esc back   r read again   ? help"
        } else {
            "↑↓ choose   enter details   r read again   esc back   ? help"
        };
        let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
        frame.render_widget(
            Paragraph::new(keys).style(palette.muted),
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
        );
        let Some(root) = &self.root else {
            frame.render_widget(
                Paragraph::new("Watch reads the runs in one tink-sdlc checkout you name.\n\nStart Tinkery with --sdlc-root PATH to choose it. Nothing is discovered or run until you do.")
                    .wrap(Wrap { trim: false })
                    .style(palette.ink),
                body,
            );
            return;
        };
        if let Some((slug, detail)) = &self.detail {
            let text = detail_text(slug, detail, palette);
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), body);
            return;
        }
        let mut lines = vec![
            Line::styled(format!("Runs in {}", root.display()), palette.ink),
            Line::styled(
                source(&["status", "--json"], self.list.as_ref()),
                palette.muted,
            ),
            Line::default(),
        ];
        let mut selected_line = 0;
        match &self.list {
            None | Some(Load::Reading(_)) => {
                lines.push(Line::styled("Reading runs…", palette.muted))
            }
            Some(Load::Done(Err(why), _)) => {
                lines.push(Line::styled("! Couldn't read the runs.", palette.rust));
                lines.push(Line::styled(why.clone(), palette.ink));
            }
            Some(Load::Done(Ok(runs), _)) if runs.is_empty() => lines.push(Line::styled(
                "No runs in this checkout yet. tink-sdlc creates them with sdlc.py new.",
                palette.ink,
            )),
            Some(Load::Done(Ok(runs), _)) => {
                let rows = ordered(runs);
                let waiting = rows.iter().filter(|run| run.state.needs_you()).count();
                lines.push(Line::styled("Needs you", palette.ink.bold()));
                if waiting == 0 {
                    lines.push(Line::styled(
                        "  Nothing needs you right now.",
                        palette.muted,
                    ));
                }
                for (index, run) in rows.iter().enumerate() {
                    if index == waiting {
                        lines.push(Line::default());
                        lines.push(Line::styled("Quiet", palette.ink.bold()));
                    }
                    let mark = if index == self.selected { "› " } else { "  " };
                    let name = if index == self.selected {
                        palette.jade
                    } else {
                        palette.ink
                    };
                    if index == self.selected {
                        selected_line = lines.len();
                    }
                    if run.state.needs_you() {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{mark}{}", run.slug), name),
                            Span::styled(format!("  {}", run.state.words()), palette.ink),
                        ]));
                        lines.push(Line::styled(format!("    {}", run.next), palette.muted));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{mark}{}", run.slug), name),
                            Span::styled(
                                format!(
                                    "  {} · {}/{} checks passed",
                                    run.state.words(),
                                    run.passed,
                                    run.total
                                ),
                                palette.muted,
                            ),
                        ]));
                    }
                }
            }
        }
        let paragraph = Paragraph::new(Text::from(lines));
        // A refusal is read whole; run rows stay one line each so the selection can scroll into view.
        if matches!(self.list, Some(Load::Done(Err(_), _))) {
            frame.render_widget(paragraph.wrap(Wrap { trim: false }), body);
            return;
        }
        let scroll = (selected_line + 2).saturating_sub(body.height as usize) as u16;
        frame.render_widget(paragraph.scroll((scroll, 0)), body);
    }
}

/// Where the shown words came from, and how fresh they are, to the minute.
fn source<T>(args: &[&str], load: Option<&Load<T>>) -> String {
    let fresh = match load {
        Some(Load::Done(_, at)) => match at.elapsed().as_secs() / 60 {
            0 => " · read just now".to_string(),
            1 => " · read 1 min ago".to_string(),
            minutes => format!(" · read {minutes} min ago"),
        },
        _ => String::new(),
    };
    format!("From python3 {SCRIPT} {}{fresh}", args.join(" "))
}

fn detail_text<'a>(slug: &str, detail: &Load<Detail>, palette: Palette) -> Text<'a> {
    let mut lines = vec![
        Line::styled(slug.to_owned(), palette.jade),
        Line::styled(
            source(&["status", slug, "--json"], Some(detail)),
            palette.muted,
        ),
        Line::default(),
    ];
    match detail {
        Load::Reading(_) => lines.push(Line::styled("Reading this run…", palette.muted)),
        Load::Done(Err(why), _) => {
            lines.push(Line::styled("! Couldn't read this run.", palette.rust));
            lines.push(Line::styled(why.clone(), palette.ink));
        }
        Load::Done(Ok(d), _) => {
            lines.push(Line::styled(d.state.words(), palette.ink.bold()));
            lines.push(Line::styled(format!("Next: {}", d.next), palette.ink));
            lines.push(Line::default());
            for (stage, status) in &d.gates {
                lines.push(Line::styled(
                    format!("Stage {stage} gate: {status}"),
                    palette.ink,
                ));
            }
            lines.push(Line::styled(
                format!(
                    "Checklist: {}/{} passed · {} with a check, {} attested",
                    d.passed,
                    d.proven + d.attested,
                    d.proven,
                    d.attested
                ),
                palette.ink,
            ));
            for error in &d.errors {
                lines.push(Line::styled(format!("! {error}"), palette.rust));
            }
        }
    }
    Text::from(lines)
}
