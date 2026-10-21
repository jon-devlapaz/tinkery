//! Approve prepares manual intake from verified seeds, one at a time. It never approves,
//! creates a run or changes a session. The menu supplies discovery; one retained worker verifies
//! seeds off the paint path, and abandoned visits cannot receive its results.

#[cfg(test)]
mod tests;

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use crate::{
    Palette, sdlc,
    seed_me::{
        Session, Stage,
        ready::{self, ReadySeed},
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};

#[derive(Default)]
struct Found {
    seeds: Vec<ReadySeed>,
    unavailable: usize,
    reasons: Vec<String>,
    start_unknown: Option<String>,
    changed: usize,
}

/// One verifier across visits. Cancellation stops between sessions; an in-flight helper keeps
/// its existing timeout. Reopening cannot spawn a replacement until the old worker returns.
#[derive(Default)]
pub(crate) struct Reader {
    running: Option<(mpsc::Receiver<Found>, Arc<AtomicBool>)>,
}

impl Reader {
    /// Stop publication and further session reads without blocking terminal input.
    pub(crate) fn cancel(&mut self) {
        if let Some((_, stop)) = &self.running {
            stop.store(true, Ordering::SeqCst);
        }
    }

    /// Land only the current visit's result, or reap an abandoned worker without showing it.
    pub(crate) fn poll(&mut self, view: Option<&mut Approve>) {
        let Some((rx, stop)) = &self.running else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(found) => Ok(found),
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err(
                "Reading seeds stopped unexpectedly. Return to the menu and reopen Approve.".into(),
            ),
        };
        let cancelled = stop.load(Ordering::SeqCst);
        self.running = None;
        if !cancelled && let Some(view) = view {
            view.found = Some(result);
        }
    }

    fn start(&mut self, sessions: Vec<Session>, root: Option<PathBuf>, mut found: Found) -> bool {
        if self.running.is_some() {
            return false;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let runs = root
                .as_deref()
                .ok_or("Start Tinkery with --sdlc-root PATH to check.".into())
                .and_then(|root| sdlc::seeds(root, &cancelled));
            if let Err(reason) = &runs {
                found.start_unknown = Some(reason.clone());
            }
            // Discovery owns chronological sorting. Reverse its newest-first order, not timestamps.
            for session in sessions.into_iter().rev() {
                if cancelled.load(Ordering::SeqCst) {
                    return;
                }
                if !matches!(session.stage, Stage::SeedUnchecked | Stage::Ended) {
                    continue;
                }
                if let Ok(runs) = &runs {
                    match sdlc::started(runs, &ready::path(&session.path)) {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(reason) => {
                            found.changed += 1;
                            if found.reasons.len() < crate::seed_me::MAX_READ_FAILURES {
                                found
                                    .reasons
                                    .push(format!("{}: {reason}", session.path.display()));
                            }
                        }
                    }
                }
                match ready::read(&session.path, root.as_deref()) {
                    Ok(Some(seed)) => found.seeds.push(seed),
                    Ok(None) => {}
                    Err(reason) => {
                        found.unavailable += 1;
                        if found.reasons.len() < crate::seed_me::MAX_READ_FAILURES {
                            found
                                .reasons
                                .push(format!("{}: {reason}", session.path.display()));
                        }
                    }
                }
            }
            let _ = tx.send(found);
        });
        self.running = Some((rx, stop));
        true
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// A visit shows only verified seeds. Navigation and copying use the same immutable reading.
pub(crate) struct Approve {
    root: Option<PathBuf>,
    waiting: bool,
    found: Option<Result<Found, String>>,
    selected: usize,
    scroll: u16,
    page: u16,
    help_scroll: u16,
    copy: Option<String>,
    notice: &'static str,
}

impl Approve {
    /// Wait for the menu's fresh session scan. Starts no helper and reads no files.
    pub(crate) fn open(root: Option<PathBuf>) -> Self {
        Self {
            root,
            waiting: true,
            found: None,
            selected: 0,
            scroll: 0,
            page: 1,
            help_scroll: 0,
            copy: None,
            notice: "",
        }
    }

    /// Whether this visit still needs the menu's fresh discovery result.
    pub(crate) fn wants_sessions(&self) -> bool {
        self.waiting
    }

    /// Hand off the discovered sessions without reaching into another phase. Missing discovery
    /// is unavailable, never an empty ready list. At most one verifier runs.
    pub(crate) fn offer(
        &mut self,
        sessions: Result<Vec<Session>, String>,
        unavailable: usize,
        reasons: Vec<String>,
        reader: &mut Reader,
    ) {
        if !self.waiting {
            return;
        }
        self.waiting = false;
        let result = sessions.and_then(|sessions| {
            if reader.start(
                sessions,
                self.root.clone(),
                Found {
                    unavailable,
                    reasons,
                    ..Found::default()
                },
            ) {
                Ok(())
            } else {
                Err(
                    "The previous seed read is finishing. Return to the menu and reopen Approve."
                        .into(),
                )
            }
        });
        if let Err(reason) = result {
            self.found = Some(Err(reason));
        }
    }

    fn seeds(&self) -> &[ReadySeed] {
        match &self.found {
            Some(Ok(found)) => &found.seeds,
            _ => &[],
        }
    }

    /// Take one explicit copy request. The terminal owns OSC 52, as it does for other places.
    pub(crate) fn take_copy_request(&mut self) -> Option<String> {
        self.copy.take()
    }

    /// Show clipboard transport feedback, not a claim of host clipboard verification.
    pub(crate) fn copy_result(&mut self, sent: bool) {
        self.notice = if sent {
            "Copied."
        } else {
            "Couldn't copy; try again."
        };
    }

    /// One local key; true leaves for the menu. Help never leaks copy or selection keys.
    pub(crate) fn key(&mut self, key: KeyEvent, help: &mut bool) -> bool {
        if key.kind == KeyEventKind::Repeat
            && matches!(
                key.code,
                KeyCode::Char('c' | '?' | 'n' | 'p') | KeyCode::F(1)
            )
        {
            return false;
        }
        if key.code == KeyCode::F(10) {
            return true;
        }
        if matches!(key.code, KeyCode::F(1) | KeyCode::Char('?')) {
            *help = !*help;
            self.help_scroll = 0;
            return false;
        }
        if *help {
            if key.code == KeyCode::Esc {
                *help = false;
            } else {
                scroll(&mut self.help_scroll, key.code, self.page);
            }
            return false;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Char('n') | KeyCode::Right if self.selected + 1 < self.seeds().len() => {
                self.selected += 1;
                self.scroll = 0;
                self.notice = "";
            }
            KeyCode::Char('p') | KeyCode::Left if self.selected > 0 => {
                self.selected -= 1;
                self.scroll = 0;
                self.notice = "";
            }
            KeyCode::Char('c') => {
                self.copy = self
                    .seeds()
                    .get(self.selected)
                    .and_then(|s| s.commands.clone());
            }
            code => scroll(&mut self.scroll, code, self.page),
        }
        false
    }

    /// Draw one seed's unchanged goal, acceptance section and manual commands; keep navigation
    /// visible while long words, paths and errors wrap and scroll. Painting never reads sources.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette, help: bool) {
        let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
        self.page = body.height;
        let mut lines = Vec::new();
        let mut keys = "↑↓ scroll   esc back   ? help";
        if help {
            keys = "↑↓ scroll   esc close help";
            lines.push(Line::styled("Right now", palette.jade));
            lines.push(Line::raw(
                "n/p choose seed · ↑↓ or PgUp/PgDn read · c copy command",
            ));
            lines.push(Line::raw(
                "Esc returns to the menu. F10 returns to the menu even from help.",
            ));
            lines.push(Line::default());
            lines.push(Line::raw("Copying starts nothing and grants no approval. Run creation and intake happen outside Tinkery; tink-sdlc keeps its definition approval gate."));
            lines.push(Line::raw("Return to the menu and reopen Approve for a fresh read. Seeds are oldest first by the session update time."));
            lines.push(Line::raw(
                "Seeds already recorded by a run move to Watch. Only the original location and unchanged contents establish a match.",
            ));
            if let Some(Ok(found)) = &self.found {
                for reason in &found.reasons {
                    lines.push(Line::raw(display(reason)));
                }
            }
        } else {
            let start_unknown = if self.root.is_none() {
                Some("Start Tinkery with --sdlc-root PATH to check.")
            } else {
                match &self.found {
                    Some(Ok(found)) => found.start_unknown.as_deref(),
                    _ => None,
                }
            };
            if let Some(reason) = start_unknown {
                lines.push(Line::styled(
                    "Can't tell which seeds have started; none are hidden.",
                    palette.muted,
                ));
                lines.push(Line::raw(display(reason)));
            }
            match &self.found {
                None => lines.push(Line::raw(if self.waiting {
                    "Reading sessions…"
                } else {
                    "Verifying seeds…"
                })),
                Some(Err(reason)) => {
                    lines.push(Line::raw("Ready seeds unavailable."));
                    lines.push(Line::raw(display(reason)));
                }
                Some(Ok(found)) => {
                    if found.changed > 0 {
                        lines.push(Line::styled(format!("{} seeds have an unknown start: the original is changed or unavailable. Check in Shape; F1 shows why.", found.changed), palette.muted));
                    }
                    if found.unavailable > 0 {
                        lines.push(Line::styled(format!("{} sessions couldn't be read; this list may be incomplete. F1 shows why.", found.unavailable), palette.muted));
                    }
                    if let Some(seed) = found.seeds.get(self.selected) {
                        keys = if seed.commands.is_some() {
                            "n/p seed   ↑↓ read   c copy command   esc back   ? help"
                        } else {
                            "n/p seed   ↑↓ read   esc back   ? help"
                        };
                        lines.push(Line::styled(
                            format!(
                                "Seed {} of {} · {}",
                                self.selected + 1,
                                found.seeds.len(),
                                seed.revision
                            ),
                            palette.jade,
                        ));
                        lines.push(Line::default());
                        for line in seed.goal.split('\n') {
                            lines.push(Line::raw(display(line)));
                        }
                        lines.push(Line::default());
                        for line in seed.acceptance.split('\n') {
                            lines.push(Line::raw(display(line)));
                        }
                        lines.push(Line::styled(
                            "Manual handoff · not approval to implement",
                            palette.muted,
                        ));
                        lines.push(Line::raw(
                            "Paste this command to create the titled run and import its checks for definition review. Copying runs nothing."
                        ));
                        if let Some(commands) = &seed.commands {
                            for line in commands.lines() {
                                lines.push(Line::raw(line.to_owned()));
                            }
                        } else {
                            lines.push(Line::raw("Cannot copy: name a checkout with --sdlc-root PATH; checkout, seed, title and executable must be safe for one shell line."));
                        }
                    } else {
                        lines.push(Line::raw(
                            if found.unavailable == 0
                                && start_unknown.is_none()
                                && found.changed == 0
                            {
                                "No verified seeds are waiting."
                            } else {
                                "No verified seeds could be listed."
                            },
                        ));
                        lines.push(Line::raw(
                            "Confirm a seed in Shape, or open Watch for started runs.",
                        ));
                    }
                }
            }
        }
        let position = if help {
            &mut self.help_scroll
        } else {
            &mut self.scroll
        };
        let paragraph = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        *position = (*position).min(
            paragraph
                .line_count(body.width)
                .saturating_sub(body.height as usize)
                .min(u16::MAX as usize) as u16,
        );
        frame.render_widget(paragraph.scroll((*position, 0)), body);
        frame.render_widget(
            Paragraph::new(self.notice).style(palette.muted),
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(2),
                area.width,
                1,
            ),
        );
        frame.render_widget(
            Paragraph::new(keys).style(palette.muted),
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
        );
    }
}

fn display(text: &str) -> String {
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

fn scroll(position: &mut u16, code: KeyCode, page: u16) {
    match code {
        KeyCode::Down | KeyCode::Char('j') => *position = position.saturating_add(1),
        KeyCode::Up | KeyCode::Char('k') => *position = position.saturating_sub(1),
        KeyCode::PageDown => *position = position.saturating_add(crate::page(page)),
        KeyCode::PageUp => *position = position.saturating_sub(crate::page(page)),
        KeyCode::Home => *position = 0,
        KeyCode::End => *position = u16::MAX,
        _ => {}
    }
}
