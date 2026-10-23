//! Taste reviews one exact result against its unchanged saved goal. Only explicit refresh reads
//! checkout status and GitHub; only o opens a PR. Copying hands review back externally without
//! recording a verdict, posting, merging, signing gates or launching an agent.

mod evidence;
mod links;
mod read;
/// Explicit canonical run-ID to PR inputs for this launch, never persisted or inferred.
pub use links::ResultLinks;
#[cfg(test)]
mod tests;

use crate::{
    Palette,
    sdlc::{self, DEADLINE, Goal, Read},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};
use read::ResultRow;
use std::{path::PathBuf, process::Command, time::Instant};

/// A read-only visit; snapshots and copy requests die when the operator leaves.
pub(crate) struct Taste {
    root: Option<PathBuf>,
    links: ResultLinks,
    reading: Option<Read<Vec<ResultRow>>>,
    opening: Option<Read<()>>,
    rows: Vec<ResultRow>,
    selected: usize,
    selection: Option<(String, Option<String>)>,
    detail: bool,
    scroll: u16,
    page: u16,
    at: Option<Instant>,
    notice: String,
    copy: Option<String>,
}

impl Taste {
    /// Open without executing helpers or contacting GitHub. r discloses and starts the read.
    pub(crate) fn open(root: Option<PathBuf>, links: ResultLinks) -> Self {
        Self {
            root,
            links,
            reading: None,
            opening: None,
            rows: Vec::new(),
            selected: 0,
            selection: None,
            detail: false,
            scroll: 0,
            page: 1,
            at: None,
            notice: String::new(),
            copy: None,
        }
    }

    fn refresh(&mut self) {
        if self.reading.is_some() || self.opening.is_some() {
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        self.selection = self
            .rows
            .get(self.selected)
            .map(|r| (r.id.clone(), r.url.clone()));
        self.rows.clear();
        self.copy = None;
        self.notice.clear();
        self.at = None;
        self.scroll = 0;
        let links = self.links.clone();
        self.reading = Some(Read::task(move |stop| read::collect(&root, stop, &links)));
    }

    /// Land only this visit's explicit read or navigation result; no automatic retries.
    pub(crate) fn poll(&mut self) {
        if let Some(result) = self.reading.as_ref().and_then(Read::poll) {
            self.reading = None;
            match result {
                Ok(rows) => {
                    self.selected = self
                        .selection
                        .as_ref()
                        .and_then(|key| {
                            rows.iter()
                                .position(|r| (&r.id, &r.url) == (&key.0, &key.1))
                        })
                        .unwrap_or(0);
                    if self.selection.as_ref().is_some_and(|key| {
                        !rows.iter().any(|r| (&r.id, &r.url) == (&key.0, &key.1))
                    }) {
                        self.detail = false;
                        self.notice =
                            "Selected result is no longer in this reading; choose a result.".into();
                    }
                    self.rows = rows;
                    self.at = Some(Instant::now());
                }
                Err(why) => {
                    self.detail = false;
                    self.notice = format!("Couldn't read results: {why}");
                }
            }
        }
        if let Some(result) = self.opening.as_ref().and_then(Read::poll) {
            self.opening = None;
            self.notice = if result.is_ok() {
                "PR sent to browser; no verdict recorded."
            } else {
                "Couldn't open the PR. Check gh/browser setup, then press o to retry."
            }
            .into();
        }
    }

    /// Handle unmodified deliberate actions. Help is isolated by the menu; repeats never open,
    /// reread or copy. true returns Home, dropping and cancelling every owned helper.
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::NONE || key.kind == KeyEventKind::Release {
            return false;
        }
        if key.kind == KeyEventKind::Repeat
            && matches!(
                key.code,
                KeyCode::Char('r' | 'o' | 'a' | 'c') | KeyCode::Enter
            )
        {
            return false;
        }
        match key.code {
            KeyCode::Esc if self.detail => {
                self.detail = false;
                self.scroll = 0;
            }
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Enter if !self.rows.is_empty() => {
                self.detail = true;
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') if self.detail => {
                self.scroll = self.scroll.saturating_add(1)
            }
            KeyCode::Up | KeyCode::Char('k') if self.detail => {
                self.scroll = self.scroll.saturating_sub(1)
            }
            KeyCode::PageDown if self.detail => self.scroll = self.scroll.saturating_add(self.page),
            KeyCode::PageUp if self.detail => self.scroll = self.scroll.saturating_sub(self.page),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End if self.detail => self.scroll = u16::MAX,
            KeyCode::Down | KeyCode::Char('j' | 'n') | KeyCode::Right => self.select(true),
            KeyCode::Up | KeyCode::Char('k' | 'p') | KeyCode::Left => self.select(false),
            KeyCode::Char('o') if self.reading.is_none() && self.opening.is_none() => {
                if let (Some(root), Some(url)) = (
                    &self.root,
                    self.rows.get(self.selected).and_then(|r| r.url.clone()),
                ) {
                    let root = root.clone();
                    self.opening = Some(Read::task(move |stop| {
                        let mut command = Command::new("gh");
                        command.args(["pr", "view", &url, "--web"]);
                        sdlc::execute(&root, command, stop, DEADLINE).map(|_| ())
                    }));
                } else {
                    self.notice =
                        "PR link unknown; relaunch with one --taste-result RUN_ID=PR_URL input."
                            .into();
                }
            }
            KeyCode::Char(action @ ('a' | 'c')) => {
                match self
                    .rows
                    .get(self.selected)
                    .ok_or("Select a result first.")
                    .and_then(|row| handoff(row, action))
                {
                    Ok(text) => self.copy = Some(text),
                    Err(why) => self.notice = why.into(),
                }
            }
            _ => {}
        }
        false
    }

    fn select(&mut self, next: bool) {
        if self.rows.is_empty() {
            return;
        }
        self.selected = if next {
            (self.selected + 1).min(self.rows.len() - 1)
        } else {
            self.selected.saturating_sub(1)
        };
        self.scroll = 0;
        self.notice.clear();
        self.copy = None;
    }

    /// Take a clipboard request once; never writes a review record.
    pub(crate) fn take_copy_request(&mut self) -> Option<String> {
        self.copy.take()
    }

    /// Report transport success only, not verified clipboard contents or acceptance.
    pub(crate) fn copy_result(&mut self, sent: bool) {
        self.notice = if sent {
            "Handoff copied; decide externally. No verdict recorded."
        } else {
            "Clipboard unavailable; try again."
        }
        .into();
    }

    /// Source and authority explanation; the menu blocks all actions under help.
    pub(crate) fn help_text(&self) -> &'static str {
        concat!(
            "Right now\nq goes back to the menu · ? closes help\n",
            "r reads again; o opens the PR; a/c copy accept/correct review requests. Esc goes back.\n\n",
            "What Taste reads\nOnly r runs the named checkout's python3 _system/scripts/sdlc.py status --json, then status <run> --json. ",
            "It uses only --taste-result RUN_ID=PR_URL launch inputs with gh pr view --json. This contacts GitHub using your login. ",
            "Missing or conflicting inputs stay unknown. Handoff prose, slugs and branches never supply PR links; no mapping store is written.\n\n",
            "What evidence means\nThe commit appears once per result. Local passes must match that exact PR head and a current SDLC verification. ",
            "Stale evidence is history. Independent review and human acceptance are separate from checks.\n\n",
            "Hand off deliberately\nCopying records no verdict, signs no gate, posts nothing and merges nothing. ",
            "The handoff asks the external reviewer to recheck current head, inputs and receipts and resolve owed gates before deciding. ",
            "It cannot guarantee external expected-input binding. Navigation launches no agent.\n\n",
            "Read more\nEnter reviews the whole unchanged goal; arrows, PgUp/PgDn and Home/End scroll. n/p change result. F10 returns Home."
        )
    }

    /// Paint the in-memory reading only, with a fixed action bar and scrollable whole goal.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let footer = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(2),
            area.width,
            2,
        );
        frame.render_widget(Paragraph::new("o open PR  a copy accept  c copy correct  r reread\nn/p result  arrows scroll  enter review  ? help  Esc back").style(palette.muted), footer);
        let mut body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(3));
        if self.detail
            && let Some(row) = self.rows.get(self.selected)
        {
            let header = Paragraph::new(Text::from(vec![
                Line::raw(safe(row.url.as_deref().unwrap_or("Result unknown"))),
                Line::raw(format!(
                    "Commit: {}",
                    row.head.as_deref().unwrap_or("unknown")
                )),
                Line::styled(
                    format!(
                        "Read {} min ago (snapshot)",
                        self.at.map_or(0, |at| at.elapsed().as_secs() / 60)
                    ),
                    palette.muted,
                ),
            ]))
            .style(palette.ink)
            .wrap(Wrap { trim: false });
            let height = (header.line_count(body.width) as u16).min(body.height.saturating_sub(1));
            frame.render_widget(header, Rect::new(body.x, body.y, body.width, height));
            body.y += height.saturating_add(1);
            body.height = body.height.saturating_sub(height.saturating_add(1));
        }
        if !self.notice.is_empty() {
            frame.render_widget(
                Paragraph::new(safe(&self.notice)).style(palette.muted),
                Rect::new(
                    area.x,
                    area.y + area.height.saturating_sub(3),
                    area.width,
                    1,
                ),
            );
        }
        let mut lines = Vec::new();
        let mut selected_line = 0;
        if self.root.is_none() {
            lines.push("Start Tinkery with --sdlc-root PATH to choose its run checkout.".into());
            lines.push("Nothing is discovered or run without that choice.".into());
        } else if self.reading.is_some() {
            lines.push("Reading results from the named checkout and GitHub…".into());
        } else if self.at.is_none() && self.notice.is_empty() {
            lines.push("Press r to read results from the named checkout and GitHub.".into());
            lines.push(
                "Runs python3 _system/scripts/sdlc.py status --json and per-run status.".into(),
            );
            lines.push(
                "Reads only --taste-result RUN_ID=PR_URL inputs with gh using your login.".into(),
            );
        } else if self.detail {
            if let Some(row) = self.rows.get(self.selected) {
                lines.extend(result_lines(row).into_iter().skip(2));
            }
        } else if self.at.is_some() && self.rows.is_empty() {
            lines
                .push("No open results in this reading. See Watch for runs; r reads again.".into());
        } else {
            lines.push("Results waiting on you".into());
            for (i, row) in self.rows.iter().enumerate() {
                if i == self.selected {
                    selected_line = lines.len();
                }
                lines.push(format!(
                    "{} {}",
                    if i == self.selected { "›" } else { " " },
                    row.slug
                ));
                lines.push(goal(&row.goal).split('\n').next().unwrap_or("").into());
                lines.push(row.url.clone().unwrap_or_else(|| "Result unknown".into()));
                lines.push(format!(
                    "Commit: {}",
                    row.head.as_deref().unwrap_or("unknown")
                ));
                lines.push(
                    row.items
                        .iter()
                        .map(|i| format!("{}: {}", i.label, i.state))
                        .collect::<Vec<_>>()
                        .join("; "),
                );
                lines.extend(row.gates.iter().map(|g| format!("Owed: {g}")));
                lines.extend(row.gaps.iter().cloned());
                lines.push(String::new());
            }
        }
        if !self.detail
            && let Some(at) = self.at
        {
            lines.push(format!(
                "Read {} min ago; a snapshot, not a freshness guarantee.",
                at.elapsed().as_secs() / 60
            ));
        }
        if !self.notice.is_empty() {
            lines.push(self.notice.clone());
        }
        let styled: Vec<_> = lines
            .iter()
            .map(|line| Line::styled(safe(line), palette.ink))
            .collect();
        let paragraph = Paragraph::new(Text::from(styled.clone())).wrap(Wrap { trim: false });
        self.page = body.height.max(1);
        let max = paragraph
            .line_count(body.width)
            .saturating_sub(body.height as usize)
            .min(u16::MAX as usize) as u16;
        if self.detail {
            self.scroll = self.scroll.min(max);
        } else {
            self.scroll = Paragraph::new(Text::from(styled[..selected_line].to_vec()))
                .wrap(Wrap { trim: false })
                .line_count(body.width)
                .saturating_sub(body.height.saturating_sub(3) as usize)
                .min(max as usize) as u16;
        }
        frame.render_widget(paragraph.scroll((self.scroll, 0)), body);
    }
}

fn goal(goal: &Goal) -> &str {
    match goal {
        Goal::Matched(text) => text,
        Goal::NoSeed => "No confirmed seed was recorded for this run.",
        Goal::Unknown => "Goal unknown: the original seed changed or can't be found.",
    }
}

fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

fn result_lines(row: &ResultRow) -> Vec<String> {
    let mut lines = vec![
        row.url.clone().unwrap_or_else(|| "Result unknown".into()),
        format!("Commit: {}", row.head.as_deref().unwrap_or("unknown")),
        "Goal (unchanged)".into(),
    ];
    lines.extend(goal(&row.goal).split('\n').map(str::to_owned));
    lines.push(String::new());
    lines.push("Conditions and checks".into());
    lines.extend(
        row.items
            .iter()
            .map(|i| format!("{}: {} ({})", i.label, i.state, i.kind)),
    );
    lines.push("Gates owed".into());
    if row.gates.is_empty() {
        lines.push("No source gate owed; acceptance is still external.".into());
    } else {
        lines.extend(row.gates.iter().cloned());
    }
    lines.extend(row.gaps.iter().map(|s| format!("Unresolved: {s}")));
    lines.push("Next: inspect the PR, then copy a review handoff to decide externally.".into());
    lines.push(String::new());
    lines.push(format!("Run: {} ({})", row.slug, row.id));
    lines.push("Evidence (not an operator verdict)".into());
    lines.extend(
        row.items
            .iter()
            .map(|i| format!("{}: {}", i.label, i.evidence)),
    );
    lines
}

fn handoff(row: &ResultRow, action: char) -> Result<String, &'static str> {
    if !matches!(row.goal, Goal::Matched(_)) || row.id.is_empty() {
        return Err("Goal/run identity unknown; repair sources and press r before copying.");
    }
    let (Some(url), Some(head)) = (&row.url, &row.head) else {
        return Err("Result identity unknown; press r after repairing the source.");
    };
    Ok(format!(
        concat!(
            "{} requested: {} @{}; run {} ({}); seed digest {}. Goal (unchanged):\n{}\n",
            "Evidence and unresolved conditions: {}. Gates owed: {}. Source gaps: {}. ",
            "Recheck current PR head, inputs and receipts; resolve gates and inspect this exact result ",
            "before recording any decision externally. Add the operator's requested changes externally. ",
            "No verdict recorded; copying posts nothing and merges nothing."
        ),
        if action == 'a' {
            "Acceptance review"
        } else {
            "Correction"
        },
        url,
        head,
        row.id,
        row.slug,
        row.seed,
        goal(&row.goal),
        row.items
            .iter()
            .map(|i| format!("{}: {} ({}, {})", i.label, i.state, i.kind, i.evidence))
            .collect::<Vec<_>>()
            .join("; "),
        row.gates.join("; "),
        row.gaps.join("; ")
    ))
}
