//! This repo's recent PRs, not productivity or acceptance. Only an explicit read contacts GitHub;
//! navigation and details use memory, and leaving cancels the read without writing records.

mod read;
#[cfg(test)]
mod tests;

use crate::{Palette, repo::Repo};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum State {
    Open,
    Merged,
    Closed,
}
impl State {
    fn label(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Merged => "merged",
            Self::Closed => "closed-unmerged",
        }
    }
}

#[derive(Debug, Deserialize)]
struct Author {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequest {
    number: u64,
    title: String,
    state: State,
    url: String,
    author: Option<Author>,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    merged_at: Option<String>,
    head_ref_oid: String,
    status_check_rollup: Option<Vec<serde_json::Value>>,
}

struct Report {
    prs: Vec<PullRequest>,
    partial: bool,
}

fn parse(text: &str) -> Result<Report, String> {
    let mut prs: Vec<PullRequest> = serde_json::from_str(text).map_err(|_| {
        "GitHub returned an unreadable PR list. Results unknown; press r to retry.".to_owned()
    })?;
    let mut numbers = std::collections::HashSet::new();
    for pr in &prs {
        let dates = [&pr.created_at, &pr.updated_at]
            .into_iter()
            .chain(pr.closed_at.iter())
            .chain(pr.merged_at.iter());
        if pr.number == 0
            || !numbers.insert(pr.number)
            || dates
                .into_iter()
                .any(|date| OffsetDateTime::parse(date, &Rfc3339).is_err())
            || !pr.url.starts_with("https://")
            || pr.head_ref_oid.is_empty()
            || matches!(pr.state, State::Merged) != pr.merged_at.is_some()
            || matches!(pr.state, State::Open) != pr.closed_at.is_none()
        {
            return Err(
                "GitHub returned inconsistent PR records. Results unknown; press r to retry."
                    .into(),
            );
        }
    }
    let partial = prs.len() > 100;
    prs.truncate(100);
    Ok(Report { prs, partial })
}

fn visible(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// A visit to Measure. No source is read until the operator presses the labelled read key.
pub(crate) struct Measure {
    repo: Option<Repo>,
    read: Option<read::Read>,
    report: Option<Report>,
    notice: String,
    since: String,
    read_at: String,
    selected: usize,
    detail: bool,
    scroll: u16,
}

impl Measure {
    /// Opening only captures the launch repo; no command or network access occurs.
    pub(crate) fn open(repo: Option<Repo>) -> Self {
        Self {
            repo,
            read: None,
            report: None,
            notice: String::new(),
            since: String::new(),
            read_at: String::new(),
            selected: 0,
            detail: false,
            scroll: 0,
        }
    }

    /// Receive one explicit read; late results cannot reopen an exited view.
    pub(crate) fn poll(&mut self) {
        if let Some(result) = self.read.as_ref().and_then(read::Read::poll) {
            self.read = None;
            match result {
                Ok(report) => {
                    self.report = Some(report);
                    self.notice.clear();
                    let now = OffsetDateTime::now_utc();
                    self.read_at =
                        format!("{} {:02}:{:02} UTC", now.date(), now.hour(), now.minute());
                }
                Err(reason) => self.notice = reason,
            }
        }
    }

    /// Route only plain presses; repeats cannot start a read. True returns to the menu.
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::NONE {
            return false;
        }
        if key.kind == KeyEventKind::Repeat && !matches!(key.code, KeyCode::Up | KeyCode::Down) {
            return false;
        }
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc if self.detail => {
                self.detail = false;
                self.scroll = 0;
            }
            KeyCode::Esc => return true,
            KeyCode::Char('r') if !self.detail && self.read.is_none() => {
                if let Some(repo) = &self.repo {
                    self.since = (OffsetDateTime::now_utc() - time::Duration::days(30))
                        .date()
                        .to_string();
                    self.report = None;
                    self.selected = 0;
                    self.scroll = 0;
                    self.notice = "Reading GitHub through gh… Esc cancels and goes back.".into();
                    self.read = Some(read::Read::start(repo.root(), self.since.clone()));
                }
            }
            KeyCode::Enter if self.report.as_ref().is_some_and(|r| !r.prs.is_empty()) => {
                self.detail = true;
                self.scroll = 0;
            }
            KeyCode::Down if self.detail => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up if self.detail => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => {
                if let Some(report) = &self.report {
                    self.selected = (self.selected + 1).min(report.prs.len().saturating_sub(1));
                }
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            _ => {}
        }
        false
    }

    /// Render memory only. Details never launch a browser or another GitHub read.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette, help: bool) {
        let footer = if help {
            "esc close help   q menu"
        } else if self.detail {
            "↑↓ scroll   esc list   ? help"
        } else if self.repo.is_none() || self.read.is_some() {
            "esc back   ? help"
        } else if self.report.is_some() {
            "↑↓ select   enter details   r refresh GitHub   esc back   ? help"
        } else {
            "r read GitHub   esc back   ? help"
        };
        let content = Rect {
            height: area.height.saturating_sub(2),
            ..area
        };
        frame.render_widget(
            Paragraph::new(footer).style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
        if help {
            frame.render_widget(Paragraph::new(concat!(
                "Measure — this repo's pull requests\n\n",
                "r reads GitHub using gh and your login, only from the list.\n",
                "Opening, moving, details and resizing run no commands.\n",
                "All states; updated since the UTC date 30 days before the read.\n",
                "Newest updated first; at most 100 PRs. A capped read says partial.\n",
                "Checks are GitHub's reported PR-head checks, not merge checks or acceptance.\n",
                "No effort, usefulness or productivity is inferred. Nothing is saved.\n\n",
                "Esc closes help, then details, then returns to the menu.\n",
                "q goes back to the menu. F10 or Ctrl-C quits Tinkery.\n",
                "Leaving cancels the read; r retries failures. Reads stop after 30 seconds."
            )).style(palette.ink).wrap(Wrap { trim: false }), content);
            return;
        }
        let Some(repo) = &self.repo else {
            frame.render_widget(Paragraph::new("No launch repository.\nStart Tinkery inside a Git repo to read its pull requests.")
                .style(palette.ink).wrap(Wrap { trim: false }), content);
            return;
        };
        if self.detail {
            let pr = &self.report.as_ref().unwrap().prs[self.selected];
            let author = pr.author.as_ref().map_or("unknown", |a| &a.login);
            let mut lines = vec![
                format!("#{} · {}", pr.number, pr.state.label()),
                visible(&pr.title),
                String::new(),
                format!("Author: {}", visible(author)),
                format!("Opened: {}", pr.created_at),
                format!("Updated: {}", pr.updated_at),
                format!(
                    "Closed: {}",
                    pr.closed_at.as_deref().unwrap_or("not closed")
                ),
                format!(
                    "Merged: {}",
                    pr.merged_at.as_deref().unwrap_or("not merged")
                ),
                visible(&pr.url),
                String::new(),
                format!("Reported PR-head checks · {}", visible(&pr.head_ref_oid)),
                "Not merge-commit checks or operator acceptance.".into(),
            ];
            match &pr.status_check_rollup {
                Some(checks) if !checks.is_empty() => {
                    for check in checks {
                        let name = check["name"]
                            .as_str()
                            .or(check["context"].as_str())
                            .unwrap_or("unnamed check");
                        let status = check["conclusion"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .or(check["status"].as_str())
                            .or(check["state"].as_str())
                            .unwrap_or("unknown");
                        lines.push(format!("{}: {}", visible(name), visible(status)));
                    }
                }
                _ => lines.push("Checks unavailable or not recorded; no pass is inferred.".into()),
            }
            let paragraph = Paragraph::new(lines.join("\n"))
                .style(palette.ink)
                .wrap(Wrap { trim: false });
            self.scroll = self.scroll.min(
                paragraph
                    .line_count(content.width)
                    .saturating_sub(content.height as usize)
                    .min(u16::MAX as usize) as u16,
            );
            frame.render_widget(paragraph.scroll((self.scroll, 0)), content);
            return;
        }
        let mut lines = vec![Line::from(format!(
            "Pull requests · {}",
            visible(&repo.name())
        ))];
        if self.since.is_empty() {
            lines.push(Line::from("Updated in the last 30 days · all states"));
            lines.push(Line::from(""));
            lines.push(Line::from(
                "Read GitHub using gh and your login. Nothing has been read yet.",
            ));
        } else {
            lines.push(Line::from(format!(
                "Updated since {} UTC · newest updated first",
                self.since
            )));
            if let Some(report) = &self.report {
                lines.push(Line::styled(
                    format!("GitHub · read at {}", self.read_at),
                    palette.muted,
                ));
                lines.push(Line::from(if report.partial {
                    "Partial results: first 100 PRs; more match this window.".to_owned()
                } else {
                    format!("{} PRs in this read", report.prs.len())
                }));
                lines.push(Line::from(""));
                let available = content.height.saturating_sub(5).max(1) as usize;
                let start = self.selected.saturating_sub(available - 1);
                if report.prs.is_empty() {
                    lines.push(Line::from("No PRs found in this window."));
                }
                for (index, pr) in report.prs.iter().enumerate().skip(start).take(available) {
                    // State precedes the title so even a narrow terminal retains its meaning.
                    lines.push(Line::from(Span::styled(
                        format!(
                            "{} #{}  {}  {}",
                            if index == self.selected { ">" } else { " " },
                            pr.number,
                            pr.state.label(),
                            visible(&pr.title)
                        ),
                        if index == self.selected {
                            palette.jade
                        } else {
                            palette.ink
                        },
                    )));
                }
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(self.notice.clone()));
            }
        }
        let paragraph = Paragraph::new(lines).style(palette.ink);
        frame.render_widget(
            if self.report.is_none() {
                paragraph.wrap(Wrap { trim: false })
            } else {
                paragraph
            },
            content,
        );
    }
}
