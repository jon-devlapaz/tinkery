//! The header's model label and the Model panel it opens. The label shows real state: calls left
//! when this launch drafts with a model, else `practice`. The panel changes a draft of the saved
//! setup; `s` saves it, `esc` drops it. Claude Code's models are picked from a list; Pi's are
//! typed. When this launch drafts, its Now row adds calls at once, for this launch only. Nothing
//! here starts a request.
use std::{path::PathBuf, sync::Arc};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

use super::{CALLS, Harness, Setup};
use crate::{Palette, pi::CallBudget};

/// Calls one `enter` on the Now row adds.
const TOP_UP: usize = 5;
/// Why an agent's panel only shows: its allowance is the operator's to give.
const AGENT: &str = "An agent can't save a setup or add calls; the operator does.";

/// This launch's model state: where the setup saves, what was saved, the launch's calls when it
/// drafts, and the panel while it is open.
pub struct Controls {
    path: Option<PathBuf>,
    saved: Result<Option<Setup>, String>,
    budget: Option<Arc<CallBudget>>,
    panel: Option<Panel>,
    agent: bool,
}

struct Panel {
    draft: Setup,
    row: usize,
    typing: Option<String>,
    note: String,
}

impl Controls {
    /// Controls saving to `path` (none without an absolute HOME), starting from what launch read
    /// there, with `budget` when this launch drafts with a model.
    pub fn new(
        path: Option<PathBuf>,
        saved: Result<Option<Setup>, String>,
        budget: Option<Arc<CallBudget>>,
    ) -> Self {
        Self {
            path,
            saved,
            budget,
            panel: None,
            agent: false,
        }
    }

    /// For an agent operator: the panel shows the setup but refuses to save or add calls.
    pub fn for_agent(mut self) -> Self {
        self.agent = true;
        self
    }

    /// The header label: calls left when this launch drafts, else `practice`.
    pub fn label(&self) -> String {
        match &self.budget {
            Some(budget) if budget.remaining() == 1 => "1 call left".into(),
            Some(budget) => format!("{} calls left", budget.remaining()),
            None => "practice".into(),
        }
    }

    /// Whether the panel is open and takes every key.
    pub fn is_open(&self) -> bool {
        self.panel.is_some()
    }

    /// Opens the panel on the saved setup, or on practice values when there is none.
    pub fn open(&mut self) {
        let note = match &self.saved {
            Err(why) => why.clone(),
            Ok(_) if self.budget.is_some() => {
                "Harness, model and thinking changes start with the next launch.".into()
            }
            Ok(_) => "Saved changes start with the next launch.".into(),
        };
        self.panel = Some(Panel {
            draft: self
                .saved
                .clone()
                .ok()
                .flatten()
                .unwrap_or_else(Setup::practice),
            row: 0,
            typing: None,
            note,
        });
    }

    /// Handles `key` while the panel is open and reports whether it took it.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if self.panel.is_none() {
            return false;
        }
        if key.kind == KeyEventKind::Release {
            return true;
        }
        let rows = self.rows();
        let panel = self.panel.as_mut().expect("checked open");
        let draft = &mut panel.draft;
        if let Some(text) = &mut panel.typing {
            match key.code {
                KeyCode::Char(c) if !key.modifiers.intersects(!KeyModifiers::SHIFT) => text.push(c),
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Esc => panel.typing = None,
                KeyCode::Enter => {
                    let model = text.trim().to_owned();
                    match draft.harness.check_model(&model) {
                        Err(why) if !model.is_empty() => panel.note = why,
                        _ => {
                            draft.model = model;
                            panel.typing = None;
                        }
                    }
                }
                _ => {}
            }
            return true;
        }
        if key.kind == KeyEventKind::Repeat && key.code != KeyCode::Up && key.code != KeyCode::Down
        {
            return true;
        }
        match key.code {
            KeyCode::Esc => self.panel = None,
            KeyCode::Up => panel.row = (panel.row + rows - 1) % rows,
            KeyCode::Down => panel.row = (panel.row + 1) % rows,
            KeyCode::Enter => match panel.row {
                0 => {
                    // Another harness can't run this one's model or every thinking level.
                    draft.harness = next(&Harness::ALL, &draft.harness);
                    draft.model = draft.harness.models().first().map_or("", |m| m).into();
                    if draft.harness.check_thinking(&draft.thinking).is_err() {
                        draft.thinking = "low".into();
                    }
                }
                1 => match draft.harness.models() {
                    [] => panel.typing = Some(draft.model.clone()),
                    models => draft.model = next(models, &draft.model.as_str()).into(),
                },
                2 => {
                    draft.thinking = next(draft.harness.thinking(), &draft.thinking.as_str()).into()
                }
                3 => draft.calls = next(&CALLS, &draft.calls),
                _ if self.agent => panel.note = AGENT.into(),
                _ => {
                    if let Some(budget) = &self.budget {
                        budget.add(TOP_UP);
                    }
                }
            },
            KeyCode::Char('s') if key.modifiers.is_empty() => self.save(),
            _ => {}
        }
        true
    }

    /// Types a paste into the model while it is being typed; otherwise the open panel ignores it.
    /// Reports whether the panel took it.
    pub fn paste(&mut self, pasted: &str) -> bool {
        let Some(panel) = &mut self.panel else {
            return false;
        };
        if let Some(text) = &mut panel.typing {
            text.extend(pasted.chars().filter(|c| !c.is_control()));
        }
        true
    }

    /// Opens the panel on a left click on the label; while it is open, takes every click.
    pub fn click(&mut self, event: MouseEvent, area: Rect) -> bool {
        if self.is_open() {
            return true;
        }
        let room = crate::label_area(area);
        let width = (self.label().chars().count() as u16).min(room.width);
        let hit = event.kind == crossterm::event::MouseEventKind::Down(MouseButton::Left)
            && event.row == room.y
            && (room.right() - width..room.right()).contains(&event.column);
        if hit {
            self.open();
        }
        hit
    }

    /// Paints the label and, when open, the panel over the screen's upper right.
    pub fn render(&self, frame: &mut Frame, palette: Palette) {
        let area = frame.area();
        frame.render_widget(
            Paragraph::new(self.label())
                .alignment(Alignment::Right)
                .style(palette.muted),
            crate::label_area(area),
        );
        let Some(panel) = &self.panel else {
            return;
        };
        let row = |i: usize, name: &str, value: String| {
            let style = if i == panel.row {
                palette.ink.bold()
            } else {
                palette.ink
            };
            Line::from(vec![
                Span::styled(if i == panel.row { "› " } else { "  " }, palette.jade),
                Span::styled(format!("{name:<10}"), style),
                Span::styled(value, palette.muted),
            ])
        };
        let model = match &panel.typing {
            Some(text) => format!("{text}▏"),
            None if panel.draft.model.is_empty() => "none".into(),
            None => panel.draft.model.clone(),
        };
        let mut lines = vec![
            Line::default(),
            row(0, "Harness", panel.draft.harness.label().into()),
            row(1, "Model", model),
            row(2, "Thinking", panel.draft.thinking.clone()),
            row(3, "Calls", format!("{} each launch", panel.draft.calls)),
        ];
        if let Some(budget) = &self.budget {
            lines.push(row(
                4,
                "Now",
                format!("{} left · enter adds {TOP_UP}", budget.remaining()),
            ));
        }
        lines.extend([
            Line::default(),
            Line::styled("F2 sends your words to this model.", palette.ink),
            Line::styled("The cap is on requests, not dollars.", palette.ink),
            Line::styled(panel.note.clone(), palette.muted),
            Line::default(),
            Line::styled(
                if panel.typing.is_some() {
                    "enter done   esc undo"
                } else {
                    "↑↓ choose   enter change   s save   esc close"
                },
                palette.muted,
            ),
        ]);
        let width = 50.min(area.width.saturating_sub(4));
        let text = Paragraph::new(lines)
            .style(palette.ink)
            .wrap(Wrap { trim: false });
        let height = (text.line_count(width.saturating_sub(4)) as u16 + 2)
            .min(area.height.saturating_sub(4));
        let at =
            Rect::new(area.width.saturating_sub(width + 2), 3, width, height).intersection(area);
        let block = Block::default()
            .title(Span::styled(" Model ", palette.jade))
            .borders(Borders::ALL)
            .border_style(palette.muted)
            .padding(Padding::new(1, 1, 0, 0));
        frame.render_widget(Clear, at);
        frame.render_widget(text.block(block), at);
    }

    fn rows(&self) -> usize {
        if self.budget.is_some() { 5 } else { 4 }
    }

    fn save(&mut self) {
        let panel = self.panel.as_mut().expect("checked open");
        let draft = &panel.draft;
        let checked = Setup::new(
            draft.harness,
            draft.model.clone(),
            draft.thinking.clone(),
            draft.calls,
        );
        panel.note = match (checked, &self.path) {
            _ if self.agent => AGENT.into(),
            (Err(why), _) => why,
            (Ok(_), None) => "Saving needs an absolute HOME; nothing was written.".into(),
            (Ok(setup), Some(path)) => match super::save(path, &setup) {
                Err(why) => why,
                Ok(()) => {
                    let next = if setup.calls > 0 {
                        "A new launch drafts with it."
                    } else {
                        "A new launch is practice."
                    };
                    self.saved = Ok(Some(setup));
                    format!("Saved. {next}")
                }
            },
        };
    }
}

/// The value after `now` in `options`, wrapping; the first when `now` isn't one.
fn next<T: PartialEq + Copy>(options: &[T], now: &T) -> T {
    let at = options.iter().position(|o| o == now);
    options[at.map_or(0, |i| (i + 1) % options.len())]
}
