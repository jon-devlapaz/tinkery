use std::{sync::Arc, time::Duration};

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use pinstar::data::CanvasNode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

use super::{
    Focus, Note, Shape,
    canvas::Canvas,
    drafting::{DraftHost, DraftJob, DraftRequest, Thought, WorkingDraft},
    overflow, response,
};
use crate::{MIN_HEIGHT, MIN_WIDTH, Palette, markdown, render_scrolled};

#[derive(Clone, Copy)]
enum Action {
    New,
    Edit,
    Shape,
    Paper,
    Help,
    Leave,
    Focus,
    Quit,
    Pause,
    Revise,
    Cancel,
}

struct Control {
    rect: Rect,
    action: Action,
}
struct HitLayout {
    area: Rect,
    paper: Option<Rect>,
    cue: Rect,
    controls: Vec<Control>,
    feedback: Option<Rect>,
}

type SourceNote = Thought;
const FEEDBACK_HEIGHT: u16 = 4;

struct PendingDraft {
    job: DraftJob,
    sources: Vec<SourceNote>,
    skipped: usize,
    feedback_history: Vec<String>,
}

pub struct Scratchpad {
    canvas: Canvas,
    writer: Shape,
    pub quit: bool,
    paper_open: bool,
    focus: Focus,
    help: bool,
    notice: Option<&'static str>,
    sources: Vec<SourceNote>,
    paper_label: &'static str,
    next_note: usize,
    layout: Option<HitLayout>,
    host: Option<Arc<dyn DraftHost>>,
    pending: Option<PendingDraft>,
    draft: Option<WorkingDraft>,
    feedback_history: Vec<String>,
    feedback: Option<Note>,
    feedback_scroll: u16,
    skipped: usize,
    message: Option<String>,
}

impl Default for Scratchpad {
    fn default() -> Self {
        let mut canvas = Canvas::new("");
        canvas.spatial = true;
        if let CanvasNode::Text(note) = &mut canvas.state.data.nodes[0] {
            note.title = Some("Note 1".into());
        }
        Self {
            canvas,
            writer: Shape::default(),
            quit: false,
            paper_open: false,
            focus: Focus::Card,
            help: false,
            notice: None,
            sources: Vec::new(),
            paper_label: "Working paper",
            next_note: 2,
            layout: None,
            host: None,
            pending: None,
            draft: None,
            feedback_history: Vec::new(),
            feedback: None,
            feedback_scroll: 0,
            skipped: 0,
            message: None,
        }
    }
}

impl Scratchpad {
    pub fn with_host(host: Arc<dyn DraftHost>) -> Self {
        Self {
            host: Some(host),
            ..Self::default()
        }
    }

    pub fn request_running(&self) -> bool {
        self.pending.is_some()
    }

    pub fn notes(&self) -> &[CanvasNode] {
        &self.canvas.state.data.nodes
    }
    pub fn selected_note(&self) -> Option<&str> {
        self.canvas.selected_id()
    }
    pub fn sent_note(&self) -> Option<&str> {
        self.writer.sent_note()
    }
    pub fn paper(&self) -> String {
        if self.sent_note().is_none() {
            "# A place to begin\n\nWhat brought you here? A problem, hunch, or plan.\n\nSelect one or more notes and press F2 to shape an investigation goal.\n\nDraft suggestions are not decisions.".to_owned()
        } else {
            self.writer.paper()
        }
    }
    pub fn inspector_open(&self) -> bool {
        self.paper_open
    }
    pub fn editing(&self) -> bool {
        self.canvas.state.floating_editor.is_some()
    }

    pub fn tick(&mut self, elapsed: Duration, area: Rect) {
        self.writer.help = self.help;
        self.writer.tick(elapsed, area);
        if let Some(result) = self.pending.as_ref().and_then(|pending| pending.job.poll()) {
            let pending = self.pending.take().unwrap();
            self.notice = None;
            match result {
                Ok(draft) => {
                    let scroll = self.writer.paper_scroll;
                    let text = pending
                        .sources
                        .iter()
                        .map(|note| note.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    self.writer.begin(text, draft.sections(&pending.sources));
                    self.writer.shown = self.writer.sections.len();
                    self.writer.paper_scroll = scroll;
                    self.sources = pending.sources;
                    self.skipped = pending.skipped;
                    self.paper_label = "Working paper / model";
                    self.draft = Some(draft);
                    self.feedback_history = pending.feedback_history;
                    self.feedback = None;
                    self.message = None;
                }
                Err(error) => self.message = Some(format!("Draft failed: {error}")),
            }
        }
    }

    fn one_selected(&mut self) -> bool {
        if self.canvas.state.selection.all().len() != 1 || self.canvas.selected_text().is_none() {
            self.notice = Some("Select one sticky note first.");
            return false;
        }
        true
    }

    fn send(&mut self) {
        if self.pending.is_some() {
            self.notice = Some("A request is running; Esc cancels it outside editing.");
            return;
        }
        if let Some(feedback) = &self.feedback {
            if feedback.text.trim().is_empty() {
                self.notice = Some("Write what to keep, cut, or reshape first.");
                return;
            }
            let request = DraftRequest {
                thoughts: self.sources.clone(),
                previous_draft: self.draft.clone(),
                feedback: Some(feedback.text.clone()),
                earlier_feedback: self.feedback_history.clone(),
            };
            self.start_request(request, self.skipped);
            return;
        }
        let selected = self.canvas.state.selection.all();
        if selected.is_empty() {
            self.notice = Some("Select one or more sticky notes first.");
            return;
        }
        let mut sources: Vec<_> = self
            .notes()
            .iter()
            .filter(|node| selected.contains(node.id()))
            .filter_map(|node| {
                let CanvasNode::Text(note) = node else {
                    return None;
                };
                Some(SourceNote {
                    id: note.id.clone(),
                    title: note.title.clone().unwrap_or_else(|| note.id.clone()),
                    text: if self.selected_note() == Some(note.id.as_str()) {
                        self.canvas.selected_text().unwrap()
                    } else {
                        note.text.clone()
                    },
                })
            })
            .collect();
        if sources.len() != selected.len() {
            self.notice = Some("Selected notes are unavailable. Select them again.");
            return;
        }
        let skipped = sources
            .iter()
            .filter(|note| note.text.trim().is_empty())
            .count();
        sources.retain(|note| !note.text.trim().is_empty());
        if sources.is_empty() {
            self.notice = Some("No written thoughts selected; previous paper retained.");
            return;
        }
        if self.host.is_some() {
            self.canvas.finish_edit();
            self.start_request(
                DraftRequest {
                    thoughts: sources,
                    previous_draft: None,
                    feedback: None,
                    earlier_feedback: Vec::new(),
                },
                skipped,
            );
            return;
        }
        let thoughts = sources
            .iter()
            .map(|note| (note.title.as_str(), note.text.as_str()))
            .collect::<Vec<_>>();
        let draft = response::investigation(&thoughts);
        let text = sources
            .iter()
            .map(|note| note.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        self.canvas.finish_edit();
        self.writer.begin(text, draft.sections);
        self.sources = sources;
        self.skipped = skipped;
        self.paper_label = draft.label;
        self.paper_open = true;
        self.notice = None;
    }

    fn start_request(&mut self, request: DraftRequest, skipped: usize) {
        let host = self.host.as_ref().unwrap().clone();
        let mut feedback_history = request.earlier_feedback.clone();
        if let Some(feedback) = &request.feedback {
            feedback_history.push(feedback.clone());
        }
        self.pending = Some(PendingDraft {
            sources: request.thoughts.clone(),
            skipped,
            feedback_history,
            job: DraftJob::start(host, request),
        });
        self.paper_open = true;
        self.notice = None;
        self.message = None;
    }

    fn revise(&mut self) {
        if self.pending.is_some() {
            self.notice = Some("Wait for the draft before giving feedback.");
        } else if self.draft.is_some() {
            self.canvas.finish_edit();
            self.canvas.state.context_menu = None;
            self.feedback = Some(Note::new(""));
            self.feedback_scroll = 0;
            self.paper_open = true;
            self.notice = None;
            self.message = None;
        } else {
            self.notice = Some("Shape thoughts with the model before revising a draft.");
        }
    }

    fn cancel(&mut self) {
        self.pending = None;
        self.feedback = None;
        self.notice = Some("Cancelled; previous paper retained.");
        self.message = None;
    }

    fn edit(&mut self) {
        if self.one_selected() {
            self.canvas.start_edit();
            self.focus = Focus::Card;
            self.notice = None;
        }
    }

    fn describe_new_notes(&mut self) {
        let mut added = false;
        for node in &mut self.canvas.state.data.nodes {
            if let CanvasNode::Text(note) = node
                && note.title.is_none()
            {
                note.title = Some(format!("Note {}", self.next_note));
                self.next_note += 1;
                note.width = 270.0;
                note.height = 120.0;
                added = true;
            }
        }
        if added {
            self.canvas.start_edit();
            self.focus = Focus::Card;
        }
    }

    fn new_note(&mut self) {
        self.canvas.finish_edit();
        self.canvas.state.context_menu = None;
        let area = self.canvas.area;
        let width = if self.paper_open {
            area.width.saturating_sub(inspector(area).width)
        } else {
            area.width
        };
        let columns = (width / 30).max(1) as usize;
        let index = self.next_note - 1;
        let x = 3 + (index % columns) as u16 * 30;
        let y = (area.height.saturating_sub(12) / 2 + ((index / columns) % 2) as u16 * 3)
            .min(area.height.saturating_sub(12));
        let (x, y) =
            self.canvas
                .state
                .screen_to_canvas(x, y, Rect::new(0, 0, area.width, area.height));
        self.canvas.state.add_text_node(x, y);
        self.describe_new_notes();
        self.notice = None;
    }

    fn toggle_paper(&mut self) {
        self.canvas.cancel_gesture();
        self.paper_open = !self.paper_open;
        if !self.paper_open {
            self.focus = Focus::Card;
        }
    }

    fn action(&mut self, action: Action) {
        match action {
            Action::New => self.new_note(),
            Action::Edit => self.edit(),
            Action::Shape => self.send(),
            Action::Paper => self.toggle_paper(),
            Action::Help => {
                self.canvas.cancel_gesture();
                self.help = !self.help;
            }
            Action::Leave => self.canvas.finish_edit(),
            Action::Focus => {
                self.canvas.finish_edit();
                self.paper_open = true;
                self.focus = if self.focus == Focus::Card {
                    Focus::Paper
                } else {
                    Focus::Card
                };
            }
            Action::Quit => self.quit = true,
            Action::Pause => self.writer.paused = !self.writer.paused,
            Action::Revise => self.revise(),
            Action::Cancel => self.cancel(),
        }
    }

    pub fn paste(&mut self, text: &str, area: Rect) {
        if !self.help && area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT {
            if let Some(feedback) = &mut self.feedback {
                if self.pending.is_some() {
                    self.notice = Some("Wait or cancel before editing feedback.");
                } else {
                    self.notice = feedback.insert(text).err();
                }
            } else if self.editing() {
                self.notice = self.canvas.paste(text).err();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, area: Rect) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Char('c') && control {
            self.quit = true;
            return;
        }
        if !self.editing()
            && self.feedback.is_none()
            && self.canvas.state.context_menu.is_none()
            && key.code == KeyCode::Char('q')
        {
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
        if control && key.code == KeyCode::Char('s') {
            self.notice = Some("Unsaved prototype: saving is disabled.");
            return;
        }
        if self.feedback.is_some() && self.pending.is_some() {
            if key.code == KeyCode::Esc {
                self.cancel();
            } else if key.code == KeyCode::PageUp {
                self.writer.paper_scroll = self.writer.paper_scroll.saturating_sub(8);
            } else if key.code == KeyCode::PageDown {
                self.scroll_paper(true, 8);
            } else {
                self.notice = Some("Wait or cancel before editing feedback.");
            }
            return;
        }
        if let Some(feedback) = &mut self.feedback {
            self.notice = None;
            match key.code {
                KeyCode::Esc => self.cancel(),
                KeyCode::Char('u') if control => {
                    feedback.text.clear();
                    feedback.cursor = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.notice = feedback.insert(&c.to_string()).err()
                }
                KeyCode::Enter => self.notice = feedback.insert("\n").err(),
                KeyCode::Backspace => feedback.backspace(),
                KeyCode::Delete => feedback.delete(),
                KeyCode::Left => feedback.left(),
                KeyCode::Right => feedback.right(),
                KeyCode::Up => feedback.move_row(false, area.width.saturating_sub(8)),
                KeyCode::Down => feedback.move_row(true, area.width.saturating_sub(8)),
                KeyCode::Home => feedback.home(),
                KeyCode::End => feedback.end(),
                KeyCode::PageUp => {
                    self.writer.paper_scroll = self.writer.paper_scroll.saturating_sub(8)
                }
                KeyCode::PageDown => self.scroll_paper(true, 8),
                _ => {}
            }
            return;
        }
        if control && key.code == KeyCode::Char('n') {
            self.new_note();
            return;
        }
        if self.editing() {
            match key.code {
                KeyCode::Esc => self.canvas.finish_edit(),
                KeyCode::Tab => self.action(Action::Focus),
                _ => self.notice = self.canvas.edit_key(key).err(),
            }
            return;
        }
        if key.code == KeyCode::Esc && self.pending.is_some() {
            self.cancel();
            return;
        }
        if self.canvas.state.context_menu.is_some() {
            self.canvas.action_key(key);
            self.describe_new_notes();
            return;
        }
        if self.focus == Focus::Card && self.canvas.action_key(key) {
            self.notice = None;
            return;
        }
        match key.code {
            KeyCode::Char('r') if self.host.is_some() => self.revise(),
            KeyCode::Char('n') => self.new_note(),
            KeyCode::Char('e') | KeyCode::Enter => self.edit(),
            KeyCode::Char('p') => self.toggle_paper(),
            KeyCode::Char('?') => self.action(Action::Help),
            KeyCode::Tab | KeyCode::BackTab => self.action(Action::Focus),
            KeyCode::Esc if self.paper_open => {
                self.paper_open = false;
                self.focus = Focus::Card;
            }
            KeyCode::Char('x') if self.writer.drafting() => self.action(Action::Pause),
            KeyCode::Up | KeyCode::Char('k') if self.focus == Focus::Paper => {
                self.writer.paper_scroll = self.writer.paper_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') if self.focus == Focus::Paper => {
                self.scroll_paper(true, 1)
            }
            KeyCode::PageUp if self.focus == Focus::Paper => {
                self.writer.paper_scroll = self.writer.paper_scroll.saturating_sub(8)
            }
            KeyCode::PageDown if self.focus == Focus::Paper => self.scroll_paper(true, 8),
            KeyCode::Home if self.focus == Focus::Paper => self.writer.paper_scroll = 0,
            KeyCode::End if self.focus == Focus::Paper => {
                self.writer.paper_scroll = self.writer.max_paper_scroll
            }
            _ => {}
        }
    }

    fn scroll_paper(&mut self, down: bool, amount: u16) {
        self.writer.paper_scroll = if down {
            self.writer
                .paper_scroll
                .saturating_add(amount)
                .min(self.writer.max_paper_scroll)
        } else {
            self.writer.paper_scroll.saturating_sub(amount)
        };
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        let Some(layout) = &self.layout else {
            return;
        };
        if layout.area != area || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            return;
        }
        if self.feedback.is_some() {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(control) = layout
                    .controls
                    .iter()
                    .find(|control| control.rect.contains((event.column, event.row).into()))
                {
                    let action = control.action;
                    self.action(action);
                } else if let Some(inner) = layout.feedback
                    && self.pending.is_none()
                    && inner.contains((event.column, event.row).into())
                {
                    let feedback = self.feedback.as_mut().unwrap();
                    feedback.cursor = feedback.wrap(inner.width).index_at(
                        event.row - inner.y + self.feedback_scroll,
                        event.column - inner.x,
                    );
                }
            }
            return;
        }
        if !self.help
            && event.kind == MouseEventKind::Down(MouseButton::Left)
            && self.canvas.menu_hit(event.column, event.row)
        {
            self.canvas.mouse(event);
            self.focus = Focus::Card;
            self.notice = None;
            self.describe_new_notes();
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some(control) = layout
                .controls
                .iter()
                .find(|control| control.rect.contains((event.column, event.row).into()))
        {
            let action = control.action;
            self.canvas.cancel_gesture();
            self.action(action);
            return;
        }
        if self.help {
            return;
        }
        if !self.canvas.captured()
            && let Some(paper) = layout.paper
            && paper.contains((event.column, event.row).into())
        {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                let page = layout.cue.contains((event.column, event.row).into());
                self.canvas.finish_edit();
                self.focus = Focus::Paper;
                if page {
                    self.scroll_paper(true, 8);
                }
            } else if matches!(
                event.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            ) {
                self.scroll_paper(event.kind == MouseEventKind::ScrollDown, 3);
            }
            return;
        }
        if self.canvas.mouse(event) {
            if matches!(event.kind, MouseEventKind::Down(_)) {
                self.focus = Focus::Card;
                self.notice = None;
            }
            self.describe_new_notes();
        }
    }

    fn source_status(&self) -> &'static str {
        if self.sources.is_empty() {
            return "No thoughts sent";
        }
        if self
            .sources
            .iter()
            .any(|source| !self.notes().iter().any(|note| note.id() == source.id))
        {
            return "Source removed / draft retained";
        }
        for source in &self.sources {
            let note = self
                .notes()
                .iter()
                .find(|note| note.id() == source.id)
                .unwrap();
            let text = if self.selected_note() == Some(source.id.as_str()) {
                self.canvas.text()
            } else {
                note.text().to_owned()
            };
            if text != source.text {
                return "Unsent changes";
            }
        }
        "Draft / unconfirmed"
    }
}

fn inspector(canvas: Rect) -> Rect {
    let width = canvas.width.saturating_sub(32).min(55);
    Rect::new(canvas.right() - width, canvas.y, width, canvas.height)
}

pub fn render(frame: &mut Frame, app: &mut Scratchpad, palette: Palette) {
    let area = frame.area();
    app.layout = None;
    frame.render_widget(Block::default().style(palette.ink), area);
    let mode = if app.host.is_some() {
        "real model / unsaved"
    } else {
        "simulated / unsaved"
    };
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        app.canvas.cancel_gesture();
        frame.render_widget(Paragraph::new(format!("tinkery / scratchpad / {mode}\n\nResize to at least {MIN_WIDTH} x {MIN_HEIGHT}.\nCurrent: {} x {}\n\nCtrl-C to quit", area.width, area.height)).style(palette.ink).wrap(Wrap { trim: false }), area);
        return;
    }
    let page = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(page);
    let header = Layout::horizontal([Constraint::Min(1), Constraint::Length(20)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new("tinkery / scratchpad").style(palette.ink.bold()),
        header[0],
    );
    frame.render_widget(
        Paragraph::new(mode)
            .style(palette.muted)
            .alignment(Alignment::Right),
        header[1],
    );
    frame.render_widget(
        Paragraph::new("What brought you here? A problem, hunch, or plan.").style(palette.muted),
        rows[1],
    );
    let count = app.notes().len();
    let selected = app.canvas.state.selection.all().len();
    let label = if app.editing() {
        "Scratchpad / editing"
    } else {
        "Scratchpad / Pinstar"
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{label} / {count} {} / {selected} selected",
            if count == 1 { "note" } else { "notes" }
        ))
        .style(if app.focus == Focus::Card {
            palette.jade
        } else {
            palette.muted
        }),
        rows[2],
    );
    if app.canvas.area.width == 0 {
        app.canvas.state.viewport_x = (rows[3].width as f64 / 2.0 - 3.0) / app.canvas.state.zoom;
    }
    app.canvas.draw(frame, rows[3], palette);
    let overlay = inspector(rows[3]);
    let mut controls = Vec::new();
    let mut cue = Rect::default();
    if app.help || app.paper_open {
        let block = Block::default()
            .style(palette.ink)
            .borders(Borders::LEFT)
            .border_style(palette.muted)
            .padding(Padding::new(1, 1, 0, 0));
        let inner = block.inner(overlay);
        frame.render_widget(Clear, overlay);
        frame.render_widget(block, overlay);
        if app.help {
            frame.render_widget(Paragraph::new("Keys / Pinstar scratchpad\nClick select; double-click edit\nDrag move note / pan empty space\nMiddle-drag pan; right-drag select\nWheel / +/- zoom; Ctrl-F fit all\nn / Ctrl-N new sticky note\ne / Enter edit selected note\ns resize; Esc finishes\nDel / right-click > Delete note\nCtrl-U clear; Ctrl-Z/Y undo / redo\nF2 shape; r feedback (model mode)\np show / hide paper inspector\nTab focus paper / canvas\nj/k / PgUp/PgDn scroll paper\nx pause demo; q / Ctrl-C quit\n? / Esc close help").style(palette.ink), inner);
        } else {
            let paper_rows = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);
            let title = Layout::horizontal([Constraint::Min(1), Constraint::Length(7)])
                .split(paper_rows[0]);
            frame.render_widget(
                Paragraph::new(app.paper_label).style(if app.focus == Focus::Paper {
                    palette.jade
                } else {
                    palette.muted
                }),
                title[0],
            );
            frame.render_widget(
                Paragraph::new(if app.editing() { "close" } else { "p close" })
                    .style(palette.muted),
                title[1],
            );
            controls.push(Control {
                rect: title[1],
                action: Action::Paper,
            });
            let source = match app.sources.as_slice() {
                [] => "Select notes and press F2.".into(),
                [note] => format!("From {} / {}", note.title, app.source_status()),
                notes => format!("From {} notes / {}", notes.len(), app.source_status()),
            };
            frame.render_widget(Paragraph::new(source).style(palette.muted), paper_rows[1]);
            if !app.sources.is_empty() {
                frame.render_widget(
                    Paragraph::new(format!(
                        "Shaped {} {}; skipped {} {}.",
                        app.sources.len(),
                        if app.sources.len() == 1 {
                            "note"
                        } else {
                            "notes"
                        },
                        app.skipped,
                        if app.skipped == 1 { "blank" } else { "blanks" },
                    ))
                    .style(palette.muted),
                    paper_rows[2],
                );
            }
            let paper = app.paper();
            let mut paper_area = paper_rows[3];
            if app.feedback.is_some() {
                paper_area.height = paper_area.height.saturating_sub(FEEDBACK_HEIGHT);
            }
            render_scrolled(
                frame,
                Paragraph::new(markdown(&paper, palette)).wrap(Wrap { trim: false }),
                paper_area,
                &mut app.writer.paper_scroll,
                &mut app.writer.max_paper_scroll,
            );
            let continuation = overflow(app.writer.paper_scroll, app.writer.max_paper_scroll);
            let hint = if continuation.is_empty() {
                String::new()
            } else if app.focus == Focus::Paper {
                format!("{continuation} / j/k scroll")
            } else {
                format!("{continuation} / Tab to read")
            };
            frame.render_widget(Paragraph::new(hint).style(palette.muted), paper_rows[4]);
            cue = paper_rows[4];
        }
    }
    if !app.help {
        app.canvas.draw_menu(frame, palette);
    }
    let mut feedback_area = None;
    if let Some(feedback) = &app.feedback {
        let box_area = Rect::new(
            rows[3].x,
            rows[3].bottom() - FEEDBACK_HEIGHT,
            rows[3].width,
            FEEDBACK_HEIGHT,
        );
        let block = Block::default()
            .title("Keep / cut / reshape")
            .borders(Borders::ALL)
            .style(palette.ink)
            .border_style(palette.jade)
            .padding(Padding::horizontal(1));
        let inner = block.inner(box_area);
        frame.render_widget(Clear, box_area);
        frame.render_widget(block, box_area);
        let text_area = inner;
        let wrapped = feedback.wrap(text_area.width);
        app.feedback_scroll = app.feedback_scroll.min(wrapped.cursor.0);
        if wrapped.cursor.0 >= app.feedback_scroll + text_area.height {
            app.feedback_scroll = wrapped.cursor.0 + 1 - text_area.height;
        }
        frame.render_widget(
            Paragraph::new(wrapped.lines.join("\n"))
                .style(palette.ink)
                .scroll((app.feedback_scroll, 0)),
            text_area,
        );
        if app.pending.is_none() {
            frame.set_cursor_position((
                text_area.x + wrapped.cursor.1.min(text_area.width - 1),
                text_area.y + wrapped.cursor.0 - app.feedback_scroll,
            ));
        }
        feedback_area = Some(text_area);
    }
    let status = if app.pending.is_some() {
        "Shaping draft... / Esc cancels outside editing"
    } else if app.writer.drafting() {
        if app.writer.paused {
            "Paused"
        } else {
            "Drafting..."
        }
    } else if app.sent_note().is_some() {
        app.source_status()
    } else {
        "Ready / shape selected thoughts into an investigation goal"
    };
    frame.render_widget(
        Paragraph::new(app.notice.or(app.message.as_deref()).unwrap_or(status))
            .style(palette.muted),
        rows[5],
    );
    if app.writer.drafting() && !app.help && app.notice.is_none() {
        controls.push(Control {
            rect: Rect::new(rows[5].x, rows[5].y, status.len() as u16, 1),
            action: Action::Pause,
        });
    }
    let paper_label = if app.paper_open {
        "p close paper"
    } else {
        "p paper"
    };
    let commands = if app.feedback.is_some() {
        vec![
            ("F2 revise", Action::Shape),
            ("Esc cancel", Action::Cancel),
            ("Ctrl-C quit", Action::Quit),
        ]
    } else if app.help {
        vec![
            ("? close help", Action::Help),
            ("Ctrl-C quit", Action::Quit),
        ]
    } else if app.editing() {
        vec![
            ("Ctrl-N new", Action::New),
            ("F2 shape", Action::Shape),
            ("Esc leave", Action::Leave),
            (
                if app.paper_open {
                    "close paper"
                } else {
                    "paper"
                },
                Action::Paper,
            ),
            ("Ctrl-C quit", Action::Quit),
        ]
    } else {
        vec![
            ("n new", Action::New),
            ("e edit", Action::Edit),
            ("F2 shape", Action::Shape),
            (paper_label, Action::Paper),
            ("? help", Action::Help),
        ]
    };
    let mut commands = commands;
    if app.host.is_some() && !app.help && !app.editing() && app.feedback.is_none() {
        commands.insert(3, ("r revise", Action::Revise));
    }
    let text = commands
        .iter()
        .map(|(label, _)| *label)
        .collect::<Vec<_>>()
        .join("   ");
    frame.render_widget(Paragraph::new(text).style(palette.muted), rows[6]);
    let mut x = rows[6].x;
    for (label, action) in commands {
        controls.push(Control {
            rect: Rect::new(x, rows[6].y, label.len() as u16, 1),
            action,
        });
        x += label.len() as u16 + 3;
    }
    app.layout = Some(HitLayout {
        area,
        paper: app.paper_open.then_some(overlay),
        cue,
        controls,
        feedback: feedback_area,
    });
}

pub fn snapshot(
    width: u16,
    height: u16,
    app: &mut Scratchpad,
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

#[cfg(test)]
#[path = "scratchpad_tests.rs"]
mod tests;
