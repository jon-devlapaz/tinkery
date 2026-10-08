mod canvas;
#[cfg(test)]
mod canvas_tests;
mod note;
mod response;
pub mod scratchpad;

use std::time::Duration;

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::Text,
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

use crate::{MIN_HEIGHT, MIN_WIDTH, Palette, markdown, render_scrolled};
pub use note::Note;

const EXAMPLE: &str =
    "I keep losing track of why we made certain choices.\n\nI want those reasons near the work.";
const STEP: Duration = Duration::from_millis(650);
const BORDER: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Card,
    Paper,
}

#[derive(Clone, Copy)]
enum ClickAction {
    Key(KeyCode, KeyModifiers),
    Pause,
}

struct ClickTarget {
    rect: Rect,
    action: ClickAction,
}

struct MouseLayout {
    area: Rect,
    card_title: Rect,
    card: Rect,
    inner: Rect,
    paper: Rect,
    paper_cue: Rect,
    targets: Vec<ClickTarget>,
}

pub struct Shape {
    pub note: Note,
    pub focus: Focus,
    pub editing: bool,
    pub help: bool,
    pub quit: bool,
    sent: Option<String>,
    sections: Vec<String>,
    shown: usize,
    paused: bool,
    elapsed: Duration,
    paper_scroll: u16,
    max_paper_scroll: u16,
    card_scroll: u16,
    card_width: u16,
    notice: Option<&'static str>,
    mouse_layout: Option<MouseLayout>,
    follow_cursor: bool,
    canvas: Option<canvas::Canvas>,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            note: Note::new(EXAMPLE),
            focus: Focus::Card,
            editing: false,
            help: false,
            quit: false,
            sent: None,
            sections: Vec::new(),
            shown: 0,
            paused: false,
            elapsed: Duration::ZERO,
            paper_scroll: 0,
            max_paper_scroll: 0,
            card_scroll: 0,
            card_width: 25,
            notice: None,
            mouse_layout: None,
            follow_cursor: true,
            canvas: None,
        }
    }
}

impl Shape {
    pub fn canvas() -> Self {
        let mut app = Self::default();
        app.canvas = Some(canvas::Canvas::new(&app.note.text));
        app
    }

    fn sync_canvas_note(&mut self) {
        if let Some(canvas) = &self.canvas {
            self.note = Note::new(&canvas.text());
            self.editing = canvas.state.floating_editor.is_some();
        }
    }

    pub fn drafting(&self) -> bool {
        self.sent.is_some() && self.shown < self.sections.len()
    }

    pub fn sent_note(&self) -> Option<&str> {
        self.sent.as_deref()
    }

    pub fn paper(&self) -> String {
        if self.sent.is_none() {
            return "# A place to begin\n\nPut an unfinished thought on the card.\n\nF2 gives it a first shape.".to_owned();
        }
        self.sections[..self.shown].join("\n\n")
    }

    pub fn send(&mut self) {
        self.sync_canvas_note();
        if self.note.text.trim().is_empty() {
            self.notice = Some("Write a thought on the card first.");
            return;
        }
        if let Some(canvas) = &mut self.canvas {
            canvas.finish_edit();
        }
        self.sent = Some(self.note.text.clone());
        self.sections = response::sections(&self.note.text);
        self.shown = 1;
        self.elapsed = Duration::ZERO;
        self.paused = false;
        self.paper_scroll = 0;
        self.editing = false;
        self.notice = None;
    }

    pub fn tick(&mut self, elapsed: Duration, area: Rect) {
        if self.paused
            || self.help
            || area.width < MIN_WIDTH
            || area.height < MIN_HEIGHT
            || !self.drafting()
        {
            return;
        }
        self.elapsed += elapsed;
        if self.elapsed >= STEP {
            // Add one complete section, never jump or scroll the reader after a delayed frame.
            self.shown += 1;
            self.elapsed = Duration::ZERO;
        }
    }

    pub fn paste(&mut self, text: &str, area: Rect) {
        if self.editing && !self.help && area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT {
            self.notice = if let Some(canvas) = &mut self.canvas {
                canvas.paste(text).err()
            } else {
                self.note.insert(text).err()
            };
            self.sync_canvas_note();
            self.follow_cursor = true;
        }
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        let Some(layout) = &self.mouse_layout else {
            return;
        };
        if layout.area != area || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            return;
        }
        let contains = |rect: Rect| rect.contains((event.column, event.row).into());
        if !self.help
            && let Some(canvas) = &mut self.canvas
        {
            if canvas.mouse(event) {
                if matches!(
                    event.kind,
                    MouseEventKind::Down(MouseButton::Left | MouseButton::Middle)
                ) {
                    self.focus = Focus::Card;
                }
                self.sync_canvas_note();
                return;
            }
            if event.kind == MouseEventKind::Down(MouseButton::Left) && contains(layout.card_title)
            {
                canvas.start_edit();
                self.focus = Focus::Card;
                self.sync_canvas_note();
                return;
            }
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(target) = layout.targets.iter().find(|target| contains(target.rect)) {
                let action = target.action;
                match action {
                    ClickAction::Key(code, modifiers) => {
                        self.handle_key(KeyEvent::new(code, modifiers), area)
                    }
                    ClickAction::Pause => self.paused = !self.paused,
                }
                return;
            }
            if self.help {
                return;
            }
            if contains(layout.card) || contains(layout.card_title) {
                if contains(layout.inner) {
                    let wrapped = self.note.wrap(layout.inner.width);
                    self.note.cursor = wrapped.index_at(
                        event.row - layout.inner.y + self.card_scroll,
                        event.column - layout.inner.x,
                    );
                }
                self.focus = Focus::Card;
                self.editing = true;
                self.follow_cursor = true;
            } else if contains(layout.paper) {
                if let Some(canvas) = &mut self.canvas {
                    canvas.finish_edit();
                }
                self.focus = Focus::Paper;
                self.editing = false;
                if contains(layout.paper_cue) && self.paper_scroll < self.max_paper_scroll {
                    self.paper_scroll = self
                        .paper_scroll
                        .saturating_add(8)
                        .min(self.max_paper_scroll);
                }
            }
        } else if !self.help
            && matches!(
                event.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            )
        {
            let down = event.kind == MouseEventKind::ScrollDown;
            if contains(layout.card) {
                let max = (self.note.wrap(layout.inner.width).lines.len() as u16)
                    .saturating_sub(layout.inner.height);
                self.card_scroll = if down {
                    self.card_scroll.saturating_add(3).min(max)
                } else {
                    self.card_scroll.saturating_sub(3)
                };
                self.follow_cursor = false;
            } else if contains(layout.paper) {
                self.paper_scroll = if down {
                    self.paper_scroll
                        .saturating_add(3)
                        .min(self.max_paper_scroll)
                } else {
                    self.paper_scroll.saturating_sub(3)
                };
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, area: Rect) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        if !self.editing && matches!(key.code, KeyCode::Char('q' | 'Q')) {
            self.quit = true;
            return;
        }
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            return;
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                self.help = false;
            }
            return;
        }
        if key.code == KeyCode::F(2) {
            self.send();
            return;
        }
        if self.editing
            && let Some(canvas) = &mut self.canvas
        {
            self.notice = None;
            if matches!(key.code, KeyCode::Esc | KeyCode::Tab) {
                canvas.finish_edit();
                if key.code == KeyCode::Tab {
                    self.focus = Focus::Paper;
                }
            } else if key.code == KeyCode::Char('s')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                self.notice = Some("Unsaved prototype: saving is disabled.");
            } else {
                self.notice = canvas.edit_key(key).err();
            }
            self.sync_canvas_note();
            return;
        }
        if !self.editing
            && !self.help
            && self.focus == Focus::Card
            && let Some(canvas) = &mut self.canvas
        {
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.notice = Some("Unsaved prototype: saving is disabled.");
                return;
            }
            if canvas.action_key(key) {
                self.sync_canvas_note();
                self.notice = None;
                return;
            }
        }
        if self.editing {
            self.follow_cursor = true;
            self.notice = None;
            match key.code {
                KeyCode::Esc | KeyCode::Tab => self.editing = false,
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.note.text.clear();
                    self.note.cursor = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.notice = self.note.insert(&c.to_string()).err();
                }
                KeyCode::Enter => self.notice = self.note.insert("\n").err(),
                KeyCode::Backspace => self.note.backspace(),
                KeyCode::Delete => self.note.delete(),
                KeyCode::Left => self.note.left(),
                KeyCode::Right => self.note.right(),
                KeyCode::Up => self.note.move_row(false, self.card_width),
                KeyCode::Down => self.note.move_row(true, self.card_width),
                KeyCode::Home => self.note.home(),
                KeyCode::End => self.note.end(),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('?') => {
                if let Some(canvas) = &mut self.canvas {
                    canvas.cancel_gesture();
                }
                self.help = true;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if self.focus == Focus::Card {
                    Focus::Paper
                } else {
                    Focus::Card
                };
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                self.focus = Focus::Card;
                self.editing = true;
                self.follow_cursor = true;
                if let Some(canvas) = &mut self.canvas {
                    canvas.start_edit();
                }
            }
            KeyCode::Esc => self.focus = Focus::Card,
            KeyCode::Char('x') if self.drafting() => self.paused = !self.paused,
            KeyCode::Up | KeyCode::Char('k') if self.focus == Focus::Paper => {
                self.paper_scroll = self.paper_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') if self.focus == Focus::Paper => {
                self.paper_scroll = self
                    .paper_scroll
                    .saturating_add(1)
                    .min(self.max_paper_scroll)
            }
            KeyCode::PageUp if self.focus == Focus::Paper => {
                self.paper_scroll = self.paper_scroll.saturating_sub(8)
            }
            KeyCode::PageDown if self.focus == Focus::Paper => {
                self.paper_scroll = self
                    .paper_scroll
                    .saturating_add(8)
                    .min(self.max_paper_scroll)
            }
            KeyCode::Home if self.focus == Focus::Paper => self.paper_scroll = 0,
            KeyCode::End if self.focus == Focus::Paper => self.paper_scroll = self.max_paper_scroll,
            _ => {}
        }
    }
}

pub fn render(frame: &mut Frame, app: &mut Shape, palette: Palette) {
    let area = frame.area();
    app.mouse_layout = None;
    frame.render_widget(Block::default().style(palette.ink), area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        if let Some(canvas) = &mut app.canvas {
            canvas.cancel_gesture();
        }
        frame.render_widget(Paragraph::new(format!(
            "tinkery / seed / simulated / unsaved\n\nResize to at least {MIN_WIDTH} x {MIN_HEIGHT}.\nCurrent: {} x {}\n\nCtrl-C to quit", area.width, area.height,
        )).style(palette.ink).wrap(Wrap { trim: false }), area);
        return;
    }
    let page = Rect::new(
        area.x + area.width.saturating_sub(110) / 2,
        area.y + area.height.saturating_sub(36) / 2,
        area.width.min(110),
        area.height.min(36),
    )
    .inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(page);
    let header = Layout::horizontal([Constraint::Min(1), Constraint::Length(20)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new("tinkery / seed").style(palette.ink.bold()),
        header[0],
    );
    frame.render_widget(
        Paragraph::new("simulated / unsaved")
            .style(palette.muted)
            .alignment(Alignment::Right),
        header[1],
    );
    let columns = Layout::horizontal([Constraint::Length(29), Constraint::Min(1)])
        .spacing(5)
        .split(rows[2]);
    let card_rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(12),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(columns[0]);
    frame.render_widget(
        Paragraph::new(if app.canvas.is_some() {
            if app.editing {
                "Note canvas / editing"
            } else {
                "Note canvas / Pinstar"
            }
        } else if app.editing {
            "Your note / editing"
        } else {
            "Your note"
        })
        .style(if app.focus == Focus::Card {
            palette.jade
        } else {
            palette.muted
        }),
        card_rows[0],
    );
    let (card_area, inner) = if let Some(canvas) = &mut app.canvas {
        let area = Rect::new(
            columns[0].x,
            card_rows[1].y,
            columns[0].width,
            columns[0].height.saturating_sub(5),
        );
        canvas.draw(frame, area, palette);
        let status = if app.sent.as_ref().is_some_and(|sent| *sent != app.note.text) {
            "Unsent changes"
        } else {
            ""
        };
        for (offset, text) in [status, "Drag card / wheel zoom", "Ctrl-F fit / s resize"]
            .into_iter()
            .enumerate()
        {
            frame.render_widget(
                Paragraph::new(text).style(palette.muted),
                Rect::new(area.x, area.bottom() + offset as u16, area.width, 1),
            );
        }
        (area, area)
    } else {
        let card = Block::default()
            .borders(Borders::ALL)
            .border_set(BORDER)
            .border_style(palette.muted)
            .padding(Padding::new(1, 1, 1, 1));
        let inner = card.inner(card_rows[1]);
        frame.render_widget(card, card_rows[1]);
        app.card_width = inner.width;
        let note = app.note.wrap(inner.width);
        if app.follow_cursor {
            if note.cursor.0 < app.card_scroll {
                app.card_scroll = note.cursor.0;
            }
            if note.cursor.0 >= app.card_scroll + inner.height {
                app.card_scroll = note.cursor.0 + 1 - inner.height;
            }
        }
        app.card_scroll = app
            .card_scroll
            .min((note.lines.len() as u16).saturating_sub(inner.height));
        frame.render_widget(
            Paragraph::new(Text::from(
                note.lines
                    .iter()
                    .map(|line| ratatui::text::Line::raw(line.as_str()))
                    .collect::<Vec<_>>(),
            ))
            .style(palette.ink)
            .scroll((app.card_scroll, 0)),
            inner,
        );
        if app.editing
            && !app.help
            && note.cursor.0 >= app.card_scroll
            && note.cursor.0 < app.card_scroll + inner.height
        {
            frame.set_cursor_position((
                inner.x + note.cursor.1.min(inner.width - 1),
                inner.y + note.cursor.0 - app.card_scroll,
            ));
        }
        let card_status = match &app.sent {
            None if app.note.text == EXAMPLE => "",
            None => "Not sent",
            Some(sent) if *sent == app.note.text => "",
            Some(_) => "Unsent changes",
        };
        frame.render_widget(
            Paragraph::new(card_status)
                .style(palette.muted)
                .wrap(Wrap { trim: false }),
            card_rows[2],
        );
        let card_max_scroll = (note.lines.len() as u16).saturating_sub(inner.height);
        frame.render_widget(
            Paragraph::new(overflow(app.card_scroll, card_max_scroll)).style(palette.muted),
            card_rows[3],
        );

        (card_rows[1], inner)
    };
    let paper_rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(columns[1]);
    app.mouse_layout = Some(MouseLayout {
        area,
        card_title: card_rows[0],
        card: card_area,
        inner,
        paper: columns[1],
        paper_cue: paper_rows[2],
        targets: Vec::new(),
    });
    if app.help {
        frame.render_widget(Paragraph::new(if app.canvas.is_some() {
            "Keys / Pinstar canvas\n\nClick        Select note\nDouble-click Edit note\nDrag         Move note / pan empty space\nMiddle-drag  Pan canvas\nWheel / +/-  Zoom; Ctrl-G grid\nCtrl-F       Fit note in view\ns            Resize; Esc finishes\ne / Enter    Edit note\nCtrl-U       Clear note while editing\nF2           Send to sample writer\nTab / Esc    Leave editing\nTab          Focus paper\nj/k / Pg     Scroll paper\nx            Pause / resume writer\n? / Esc      Close help"
        } else {
            "Keys\n\ne / Enter   Edit card\nF2          Send to sample writer\nEnter       New line while editing\nArrows/click Place editing cursor\nCtrl-U      Clear card while editing\nEsc         Leave editing / return to card\nTab         Switch card and paper\nj/k         Scroll paper\nPgUp/PgDn   Page; Home/End jump\nx           Pause / resume writer\nq / Ctrl-C  Quit (Ctrl-C while editing)\nWheel       Scroll card or paper\n? / Esc     Close help\n\nTemplate writer. No sessions or saves."
        })
            .style(palette.ink).wrap(Wrap { trim: false }), columns[1]);
    } else {
        frame.render_widget(
            Paragraph::new(response::source_label(app.sent_note())).style(
                if app.focus == Focus::Paper {
                    palette.jade
                } else {
                    palette.muted
                },
            ),
            paper_rows[0],
        );
        let paper = app.paper();
        render_scrolled(
            frame,
            Paragraph::new(markdown(&paper, palette)).wrap(Wrap { trim: false }),
            paper_rows[1],
            &mut app.paper_scroll,
            &mut app.max_paper_scroll,
        );
        let continuation = overflow(app.paper_scroll, app.max_paper_scroll);
        let cue = if continuation.is_empty() {
            String::new()
        } else if app.focus == Focus::Paper {
            format!("{continuation} / j/k scroll")
        } else {
            format!("{continuation} / Tab to read")
        };
        frame.render_widget(Paragraph::new(cue).style(palette.muted), paper_rows[2]);
    }
    if let Some(notice) = app.notice {
        frame.render_widget(Paragraph::new(notice).style(palette.muted), rows[4]);
        return;
    }
    let footer = Layout::horizontal([Constraint::Length(30), Constraint::Min(1)]).split(rows[4]);
    let status = if app.drafting() {
        if app.paused { "Paused" } else { "Drafting..." }
    } else if app.sent.is_some() {
        "Draft / unconfirmed"
    } else {
        "Ready"
    };
    frame.render_widget(Paragraph::new(status).style(palette.muted), footer[0]);
    let key_action = |key| ClickAction::Key(key, KeyModifiers::NONE);
    let controls = if app.help {
        vec![
            ("? close help", key_action(KeyCode::Char('?'))),
            (
                "Ctrl-C quit",
                ClickAction::Key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            ),
        ]
    } else if app.editing {
        vec![
            ("F2 send", key_action(KeyCode::F(2))),
            ("Esc leave", key_action(KeyCode::Esc)),
            (
                "Ctrl-C quit",
                ClickAction::Key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            ),
        ]
    } else {
        vec![
            ("e edit", key_action(KeyCode::Char('e'))),
            ("F2 send", key_action(KeyCode::F(2))),
            ("Tab focus", key_action(KeyCode::Tab)),
            ("? help", key_action(KeyCode::Char('?'))),
        ]
    };
    let text = controls
        .iter()
        .map(|(label, _)| *label)
        .collect::<Vec<_>>()
        .join("   ");
    frame.render_widget(
        Paragraph::new(text.as_str())
            .style(palette.muted)
            .alignment(Alignment::Right),
        footer[1],
    );
    let mut x = footer[1].right().saturating_sub(text.len() as u16);
    let can_pause = app.drafting() && !app.help;
    let layout = app.mouse_layout.as_mut().unwrap();
    for (label, action) in controls {
        layout.targets.push(ClickTarget {
            rect: Rect::new(x, footer[1].y, label.len() as u16, 1),
            action,
        });
        x += label.len() as u16 + 3;
    }
    if can_pause {
        layout.targets.push(ClickTarget {
            rect: Rect::new(footer[0].x, footer[0].y, status.len() as u16, 1),
            action: ClickAction::Pause,
        });
    }
}

fn overflow(scroll: u16, max_scroll: u16) -> &'static str {
    match (scroll > 0, scroll < max_scroll) {
        (true, true) => "more above / below",
        (true, false) => "more above",
        (false, true) => "more below",
        (false, false) => "",
    }
}

pub fn snapshot(
    width: u16,
    height: u16,
    app: &mut Shape,
    no_color: bool,
) -> Result<String, std::convert::Infallible> {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))?;
    terminal.draw(|frame| render(frame, app, Palette::new(no_color)))?;
    let buffer = terminal.backend().buffer();
    Ok((0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
