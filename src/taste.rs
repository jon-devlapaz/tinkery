//! Taste reviews one exact result against its unchanged saved goal. Only explicit refresh reads
//! checkout status and GitHub; only o opens a PR. Copying hands review back externally without
//! recording a verdict, posting, merging, signing gates or launching an agent.

mod evidence;
mod links;
mod next;
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
    pause: Option<next::Pause>,
    draft: Option<crate::think_record::DraftSuggestion>,
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
            pause: None,
            draft: None,
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
        self.pause = None;
        self.draft = None;
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
                KeyCode::Char('r' | 'o' | 'a' | 'c' | 'l' | 't') | KeyCode::Enter
            )
        {
            return false;
        }
        match key.code {
            KeyCode::Esc if self.pause.is_some() => {
                self.pause = None;
                self.scroll = 0;
            }
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
            KeyCode::Char('l')
                if self.detail && self.reading.is_none() && self.opening.is_none() =>
            {
                if let Some(row) = self.rows.get(self.selected) {
                    match handoff(row, 'a') {
                        Ok(_) => {
                            self.pause = Some(next::suggest(row));
                            self.scroll = 0;
                            self.copy = None;
                            self.notice.clear();
                        }
                        Err(why) => self.notice = why.into(),
                    }
                }
            }
            KeyCode::Char('t') => {
                if let Some(pause) = &self.pause {
                    self.draft = Some(pause.draft.clone());
                }
            }
            KeyCode::Char(action @ ('a' | 'c')) => {
                self.pause = None;
                self.draft = None;
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
        self.pause = None;
        self.draft = None;
    }

    /// Take the deliberately requested unsent draft; neither copying nor acceptance opens Think.
    pub(crate) fn take_draft(&mut self) -> Option<crate::think_record::DraftSuggestion> {
        self.draft.take()
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
            "Right now\nq goes back to the menu · F10 Home · ? / Esc close help\n",
            "Enter reviews · arrows/PgUp/PgDn/Home/End scroll · n/p result\n",
            "o opens PR · a/c copy accept/correct requests, never decisions\n\n",
            "What Taste reads\nOnly r reads the named checkout's API 1 status and explicit\n",
            "--taste-result PRs with gh, using your login. Missing/conflicting links\n",
            "stay unknown; prose supplies none. No writes, polling or agent launch.\n",
            "Passes need current, exact-head evidence; review is separate.\n",
            "External handoffs must recheck head, inputs, receipts and owed gates;\n",
            "copying cannot enforce that binding, record verdicts or merge.\n\n",
            "After external acceptance\nIn review, l reports you accepted this result externally (not verified).\n",
            "One deterministic guess follows; t opens an unsent Think draft.\n",
            "Only F2 sends. Copying or green checks never imply acceptance.\n",
            "Esc, correction, selection and rereading discard the guess.\n",
            "No verdict, gate or permission is recorded; you merge externally."
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
        let actions = if self.pause.is_some() {
            "t edit in Think (unsent)  c copy correct  r reread\narrows scroll  Esc back  ? help"
        } else if self.detail {
            "o open PR  a copy accept  c copy correct  l accepted externally\nn/p result  arrows scroll  r reread  ? help  Esc back"
        } else {
            "o open PR  a copy accept  c copy correct  r reread\nn/p result  enter review  ? help  Esc back"
        };
        frame.render_widget(Paragraph::new(actions).style(palette.muted), footer);
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
        let guess_lines = if self.detail {
            if let Some(pause) = &self.pause {
                let count = pause.lines.len();
                lines.splice(0..0, pause.lines.clone());
                count
            } else {
                0
            }
        } else {
            0
        };
        let styled: Vec<_> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                Line::styled(
                    safe(line),
                    if index < guess_lines {
                        palette.ink.add_modifier(ratatui::style::Modifier::ITALIC)
                    } else {
                        palette.ink
                    },
                )
            })
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
