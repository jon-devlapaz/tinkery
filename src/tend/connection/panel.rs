//! Connection review and explicit actions. Background reads keep navigation responsive;
//! closing cancels a request and never authorizes a save or opens Shape.

use super::*;
use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use std::sync::{atomic::Ordering, mpsc};

struct Done {
    input: Result<Input, String>,
    record: Result<Option<Record>, String>,
    receipt: Option<seed_me::Receipt>,
    notice: Option<String>,
}

/// One reviewed proposal. Only a verified explicit save can return a handoff receipt.
pub(crate) struct Panel {
    context: Context,
    input: Result<Input, String>,
    record: Option<Record>,
    notice: Option<String>,
    editor: Option<TextArea<'static>>,
    saving_editor: Option<TextArea<'static>>,
    offset: u16,
    running: Option<mpsc::Receiver<Done>>,
    stop: Arc<AtomicBool>,
    saving: bool,
    receipt: Option<seed_me::Receipt>,
}

impl Drop for Panel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Panel {
    /// Open a read-only disclosure. Missing data is shown as a refusal, never an empty success.
    pub(crate) fn open(context: Context) -> Self {
        let mut panel = Self {
            context,
            input: Err("Reading goals…".into()),
            record: None,
            notice: None,
            editor: None,
            saving_editor: None,
            offset: 0,
            running: None,
            stop: Arc::new(AtomicBool::new(false)),
            saving: false,
            receipt: None,
        };
        panel.start(|_| Ok(None));
        panel
    }

    fn start(
        &mut self,
        action: impl FnOnce(&Context) -> Result<Option<seed_me::Receipt>, String> + Send + 'static,
    ) {
        if self.running.is_some() {
            return;
        }
        let context = self.context.clone();
        let (tx, rx) = mpsc::channel();
        self.running = Some(rx);
        self.notice = None;
        std::thread::spawn(move || {
            let result = action(&context);
            let _ = tx.send(Done {
                input: context.snapshot(),
                record: context.read(),
                receipt: result.as_ref().ok().cloned().flatten(),
                notice: result.err(),
            });
        });
    }

    /// Receive finished work without blocking. A saved result is not seed or execution permission.
    pub(crate) fn poll(&mut self) -> Option<seed_me::Receipt> {
        if let Some(rx) = &self.running {
            match rx.try_recv() {
                Ok(done) => {
                    self.running = None;
                    self.input = done.input;
                    self.notice = done.notice;
                    match done.record {
                        Ok(record) => self.record = record,
                        Err(reason) => {
                            self.input = Err(reason.clone());
                            self.notice = Some(reason);
                            self.record = None;
                        }
                    }
                    if self.notice.is_some()
                        && self.record.as_ref().is_none_or(|r| r.pending.is_none())
                    {
                        self.editor = self.saving_editor.take();
                    } else {
                        self.saving_editor = None;
                    }
                    self.receipt = done.receipt;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.running = None;
                    self.notice = Some(
                        "The connection operation stopped. Reopen it to inspect saved state."
                            .into(),
                    );
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.receipt.take()
    }

    /// Insert literal pasted text only into the active goal editor. Pasting never saves or requests.
    pub(crate) fn paste(&mut self, text: &str) {
        if let Some(editor) = &mut self.editor {
            editor.insert_str(text);
        }
    }

    /// Handle explicit review actions. True returns to Tend without starting anything.
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if matches!(key.code, KeyCode::Esc | KeyCode::F(10)) {
            if self.editor.take().is_some() {
                return false;
            }
            self.stop.store(true, Ordering::Relaxed);
            return true;
        }
        if self.running.is_some() {
            return false;
        }
        if let Some(editor) = &mut self.editor {
            if key.code == KeyCode::F(2) {
                let words = editor.lines().join("\n");
                let Some(record) = self.record.clone() else {
                    return false;
                };
                self.saving_editor = self.editor.take();
                self.saving = true;
                self.start(move |context| {
                    context
                        .save(&record, &words)
                        .map(|(_, receipt)| Some(receipt))
                });
            } else {
                editor.input(key);
            }
            return false;
        }
        if key.modifiers != KeyModifiers::NONE {
            return false;
        }
        match key.code {
            KeyCode::Down | KeyCode::PageDown => self.offset = self.offset.saturating_add(5),
            KeyCode::Up | KeyCode::PageUp => self.offset = self.offset.saturating_sub(5),
            KeyCode::Char('p') => {
                if let Some(record) = &self.record
                    && !record.dismissed
                    && record.pending.is_none()
                    && self.input.as_ref().ok() == Some(&record.input)
                    && let Some(proposal) = &record.proposal
                {
                    self.editor = Some(TextArea::from(proposal.goal.lines()));
                }
            }
            KeyCode::Char('d') => {
                if let Some(record) = self.record.clone() {
                    self.start(move |context| context.dismiss(&record).map(|_| None));
                }
            }
            KeyCode::Char('c') => {
                if let Some(record) = self.record.clone()
                    && record.pending.is_some()
                    && record.saved.is_none()
                {
                    self.start(move |context| context.cancel(&record).map(|_| None));
                }
            }
            KeyCode::Char('o') => {
                if let Some(record) = self.record.clone()
                    && record.saved.is_some()
                {
                    self.saving = true;
                    self.start(move |context| {
                        context.save(&record, "").map(|(_, receipt)| Some(receipt))
                    });
                }
            }
            KeyCode::F(2) => {
                if let Some(record) = self.record.clone()
                    && record.pending.is_some()
                    && (record.saved.is_none() || self.input.as_ref().ok() == Some(&record.input))
                {
                    self.saving = true;
                    self.start(move |context| {
                        context.save(&record, "").map(|(_, receipt)| Some(receipt))
                    });
                } else if let Ok(input) = self.input.clone() {
                    self.saving = false;
                    let stop = self.stop.clone();
                    self.start(move |context| context.explore(&input, &stop).map(|_| None));
                }
            }
            _ => {}
        }
        false
    }

    /// Show full sources and separate provisional wording. Scrolling makes no calls or writes.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let footer = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
        let body = Rect {
            height: area.height.saturating_sub(2),
            ..area
        };
        if let Some(editor) = &self.editor {
            frame.render_widget(
                Paragraph::new(format!(
                    "Confirm this goal in {}",
                    self.context.repo.display()
                ))
                .style(palette.jade),
                Rect { height: 1, ..body },
            );
            let top = if let Some(notice) = &self.notice {
                frame.render_widget(
                    Paragraph::new(crate::tend::history::display(notice))
                        .wrap(Wrap { trim: false })
                        .style(palette.muted),
                    Rect::new(body.x, body.y + 1, body.width, 3),
                );
                5
            } else {
                2
            };
            frame.render_widget(
                editor,
                Rect::new(
                    body.x,
                    body.y + top,
                    body.width,
                    body.height.saturating_sub(top),
                ),
            );
            frame.render_widget(
                Paragraph::new("F2 save exact words and open Shape   esc keep proposal")
                    .style(palette.muted),
                footer,
            );
            return;
        }
        let mut lines = vec![
            Line::styled("Find connections", palette.jade),
            Line::from(Span::raw(format!(
                "Repository: {}",
                self.context.repo.display()
            ))),
        ];
        if let Some(notice) = &self.notice {
            lines.push(Line::from(Span::raw(format!("! {notice}"))));
        }
        let pending = self.record.as_ref().is_some_and(|r| r.pending.is_some());
        let mut keys = "esc back";
        match &self.input {
            Err(reason) => lines.push(Line::from(reason.clone())),
            Ok(input) => {
                lines.push(Line::from(Span::raw(format!(
                    "{} complete goals · model {} · one request",
                    input.sources.len(),
                    input.model
                ))));
                lines.push(Line::from("Starting sends these goals to the configured provider. No tools or external search."));
                if let Some((_, budget)) = &self.context.call {
                    lines.push(Line::from(budget.left()));
                }
                if self.record.as_ref().is_none_or(|r| r.input != *input) && !pending {
                    keys = "F2 find one connection   ↑↓ scroll sources   esc back";
                }
            }
        }
        if let Some(record) = &self.record {
            if let Some(pending) = &record.pending {
                lines.push(Line::from(Span::raw(format!(
                    "Confirmed goal:\n{}",
                    pending.origin.goal
                ))));
                lines.push(Line::from(if record.saved.is_some() {
                    "Goal saved. Opening it starts no drafting."
                } else {
                    "A save is pending. Resume uses the frozen confirmed words."
                }));
                keys = if record.saved.is_some() && self.input.as_ref().ok() == Some(&record.input)
                {
                    "F2 open saved goal   esc back"
                } else if record.saved.is_some() {
                    "F2 find a fresh connection   o open saved goal   esc back"
                } else {
                    "F2 resume saving   c cancel pending save   esc back"
                };
            } else if self.input.as_ref().ok() == Some(&record.input) {
                keys = "↑↓ scroll   esc back";
                if record.dismissed {
                    lines.push(Line::from(
                        "Dismissed. A later changed-input pass can rediscover this idea.",
                    ));
                } else if record.proposal.is_some() {
                    keys = "p prepare this goal   d dismiss   ↑↓ scroll   esc back";
                }
            } else {
                lines.push(Line::from(
                    "This previous result is outdated. It cannot create a new handoff.",
                ));
            }
            if let Some(proposal) = &record.proposal {
                lines.push(Line::styled(
                    "Proposed connection · Tinkery's guess",
                    palette.muted,
                ));
                lines.push(
                    Line::from(Span::raw(proposal.relationship.clone()))
                        .style(palette.ink.add_modifier(ratatui::style::Modifier::ITALIC)),
                );
                for support in &proposal.sources {
                    lines.push(Line::from(Span::raw(format!(
                        "Source {}: {}",
                        support.id, support.quote
                    ))));
                }
                for (label, text) in [
                    ("Proposed goal", &proposal.goal),
                    ("Inspectable result", &proposal.outcome),
                    ("Judgment", &proposal.question),
                ] {
                    lines.push(Line::from(Span::raw(format!("{label}: {text}"))));
                }
                for assumption in &proposal.assumptions {
                    lines.push(Line::from(Span::raw(format!(
                        "Assumption or conflict: {assumption}"
                    ))));
                }
            } else {
                lines.push(Line::from("No useful connection found."));
            }
        }
        if let Ok(input) = &self.input {
            lines.push(Line::styled("Full source goals", palette.muted));
            for source in &input.sources {
                lines.push(Line::from(Span::raw(format!(
                    "{}\n{}\n",
                    source.id, source.goal
                ))));
            }
        }
        if self.running.is_some() {
            keys = if self.saving {
                "Saving…   esc return while save finishes"
            } else {
                "Working…   esc cancel and return"
            };
        }
        let lines = lines
            .into_iter()
            .flat_map(|line| {
                let style = line.style;
                let text = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>();
                crate::tend::history::display(&text)
                    .lines()
                    .map(|part| Line::styled(part.to_owned(), style))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        self.offset = self.offset.min(
            paragraph
                .line_count(body.width)
                .saturating_sub(body.height as usize) as u16,
        );
        frame.render_widget(paragraph.scroll((self.offset, 0)), body);
        frame.render_widget(Paragraph::new(keys).style(palette.muted), footer);
    }
}
