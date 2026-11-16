//! Exact repository thoughts in Tend's private store. Thoughts never become sessions or model
//! sources. Only explicit saving publishes words; opening Think returns an unsent copy.

use super::{Entry, Tend};
use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use std::path::PathBuf;
impl Tend {
    /// Visible repository originals, absent from the all-goals view.
    pub(super) fn thought_entries(&self) -> &[String] {
        if self.connection_scope() {
            self.thoughts.as_ref().map_or(&[], Garden::entries)
        } else {
            &[]
        }
    }

    /// The number of selectable goals and thoughts in this scope.
    pub(super) fn entry_count(&self) -> usize {
        self.sessions.len() + self.thought_entries().len()
    }

    /// The selected identity, independent of its displayed title.
    pub(super) fn selected_entry(&self) -> Option<Entry> {
        if let Some(session) = self.current() {
            Some(Entry::Goal(session.path.clone()))
        } else {
            self.selected
                .checked_sub(self.sessions.len())
                .and_then(|i| self.thought_entries().get(i))
                .cloned()
                .map(Entry::Thought)
        }
    }

    /// Restore exact identity or select the first available entry.
    pub(super) fn select_entry(&mut self, entry: Option<Entry>) {
        self.selected = match entry {
            Some(Entry::Goal(path)) => self.sessions.iter().position(|s| s.path == path),
            Some(Entry::Thought(words)) => self
                .thought_entries()
                .iter()
                .position(|w| *w == words)
                .map(|i| self.sessions.len() + i),
            None => None,
        }
        .unwrap_or(0);
    }

    /// Open the selected original, never infer a goal from a thought.
    pub(crate) fn open_entry(&mut self) {
        if let Some(Entry::Thought(words)) = self.selected_entry() {
            let thoughts = self.thoughts.as_mut().unwrap();
            thoughts.view();
            if let Some(index) = thoughts.entries().iter().position(|entry| *entry == words) {
                thoughts.selected = index;
            } else {
                thoughts.viewing = false;
                self.notice = "Thought unavailable. Select another entry.".into();
            }
            self.select_entry(Some(Entry::Thought(words)));
            self.prepare_graph();
        } else if self.current().is_some() {
            self.detail = true;
            self.scroll = 0;
            self.notice.clear();
            if let Some(thoughts) = &mut self.thoughts {
                thoughts.clear_save_notice();
            }
        }
    }

    /// Keep the last viewed thought selected when the reader closes.
    pub(crate) fn thought_reader_return(&mut self) {
        let entry = self
            .thoughts
            .as_ref()
            .and_then(Garden::words)
            .map(|words| Entry::Thought(words.into()));
        if let Some(thoughts) = &mut self.thoughts {
            thoughts.refresh();
        }
        self.select_entry(entry.clone());
        if entry.is_some() && self.selected_entry() != entry {
            self.notice = "Thought unavailable. Select another entry.".into();
        }
        self.prepare_graph();
    }

    /// Select the exact durable thought and show its save result.
    pub(crate) fn thought_saved(&mut self, words: &str, duplicate: bool) {
        if let Some(thoughts) = &mut self.thoughts {
            thoughts.saved(words, duplicate);
        }
        let entry = Some(Entry::Thought(words.into()));
        self.select_entry(entry.clone());
        self.hidden_thought = (self.selected_entry() != entry).then_some(entry).flatten();
        self.prepare_graph();
    }

    /// Incomplete thought reads remain visible without claiming absence.
    pub(super) fn thought_status(&self) -> String {
        if self.connection_scope() {
            self.thoughts.as_ref().map_or(String::new(), Garden::status)
        } else {
            String::new()
        }
    }

    /// Thought status carries no goal readiness or authority.
    pub(super) fn thought_line(&self) -> Option<(String, String)> {
        let Entry::Thought(words) = self.selected_entry()? else {
            return None;
        };
        let notice = self.thoughts.as_ref().map_or("", |t| t.notice.as_str());
        Some((
            format!("Thought: {}", words.lines().next().unwrap_or("")),
            if notice.is_empty() {
                String::new()
            } else {
                format!(" · {notice}")
            },
        ))
    }
}

/// Repository-bound words and their reader. No goal or seed authority lives here.
pub(crate) struct Garden {
    store: crate::thought::Store,
    repo: PathBuf,
    entries: Result<Vec<String>, String>,
    unreadable: usize,
    selected: usize,
    viewing: bool,
    scroll: u16,
    notice: String,
}

impl Garden {
    /// Read only thoughts belonging to the captured launch repository; never infer attribution.
    pub(crate) fn open(root: Result<PathBuf, String>, repo: PathBuf) -> Self {
        let mut garden = Self {
            store: crate::thought::Store::new(root, repo.clone()),
            repo,
            entries: Ok(vec![]),
            unreadable: 0,
            selected: 0,
            viewing: false,
            scroll: 0,
            notice: String::new(),
        };
        garden.refresh();
        garden
    }

    /// Refresh originals while retaining exact reader identity.
    pub(super) fn refresh(&mut self) {
        let keep = self.words().map(str::to_owned);
        self.unreadable = 0;
        self.entries = self.store.read().map(|(entries, unreadable)| {
            self.unreadable = unreadable;
            entries
        });
        self.selected = keep
            .and_then(|words| self.entries().iter().position(|entry| *entry == words))
            .unwrap_or(0);
    }

    fn entries(&self) -> &[String] {
        self.entries.as_deref().unwrap_or_default()
    }

    fn words(&self) -> Option<&str> {
        self.entries().get(self.selected).map(String::as_str)
    }

    /// Show the completed save and select its exact words without granting goal authority.
    pub(crate) fn saved(&mut self, words: &str, duplicate: bool) {
        self.refresh();
        if let Ok(entries) = &self.entries
            && let Some(index) = entries.iter().position(|entry| entry == words)
        {
            self.selected = index;
        }
        self.notice = if duplicate {
            "Thought already saved."
        } else {
            "Thought saved."
        }
        .into();
    }

    /// End save feedback when its Tend visit ends, retaining the saved words and selection.
    pub(crate) fn clear_save_notice(&mut self) {
        self.notice.clear();
    }

    /// Open the saved thought list without granting goal authority.
    pub(crate) fn view(&mut self) {
        self.refresh();
        self.selected = self.selected.min(
            self.entries
                .as_ref()
                .map_or(0, |e| e.len().saturating_sub(1)),
        );
        self.viewing = true;
        self.scroll = 0;
        self.notice.clear();
    }

    /// Whether this local view owns keyboard and paste input.
    pub(crate) fn active(&self) -> bool {
        self.viewing
    }

    /// Return an explicit unsent Think draft only on `g`; Shape is never a destination.
    pub(crate) fn key(&mut self, key: KeyEvent) -> Option<String> {
        if key.kind != KeyEventKind::Press {
            return None;
        }
        if matches!(key.code, KeyCode::Esc | KeyCode::F(10)) {
            self.viewing = false;
            self.notice.clear();
            return None;
        }
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        let count = self.entries.as_ref().map_or(0, Vec::len);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(count.saturating_sub(1));
                self.scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.scroll = 0;
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(8),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(8),
            KeyCode::Char('g') => return self.entries.as_ref().ok()?.get(self.selected).cloned(),
            _ => {}
        }
        None
    }

    fn status(&self) -> String {
        if self.entries.is_err() {
            "Thoughts could not be read. Press v for details.".into()
        } else if self.unreadable > 0 {
            format!("{} thoughts could not be read.", self.unreadable)
        } else {
            String::new()
        }
    }

    /// Render the labelled local record. Rendering reads no files.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let heading = format!(
            "Saved thoughts\nRepository: {}\ng opens these words in Think, unsent. Confirm a goal there. No Shape.{}",
            self.repo.display(),
            if self.unreadable > 0 {
                format!("\n{} thoughts could not be read.", self.unreadable)
            } else {
                String::new()
            }
        );
        let heading = Paragraph::new(heading)
            .style(palette.ink)
            .wrap(Wrap { trim: false });
        let rows = heading.line_count(area.width).min(area.height as usize) as u16;
        frame.render_widget(
            heading,
            Rect {
                height: rows,
                ..area
            },
        );
        let body = Rect::new(
            area.x,
            area.y.saturating_add(rows + 1),
            area.width,
            area.height.saturating_sub(rows + 2),
        );
        let text = match &self.entries {
            Ok(entries) if entries.is_empty() => "No thoughts saved.".into(),
            Ok(entries) => format!(
                "Thought {} of {}\n{}",
                self.selected + 1,
                entries.len(),
                entries[self.selected]
                    .chars()
                    .map(|c| if c.is_control() && c != '\n' {
                        '�'
                    } else {
                        c
                    })
                    .collect::<String>()
            ),
            Err(reason) => format!("Thoughts could not be read: {reason}"),
        };
        let p = Paragraph::new(text)
            .style(palette.ink)
            .wrap(Wrap { trim: false });
        self.scroll = self.scroll.min(
            p.line_count(body.width)
                .saturating_sub(body.height as usize) as u16,
        );
        frame.render_widget(p.scroll((self.scroll, 0)), body);
        frame.render_widget(
            Paragraph::new("↑↓ thoughts   PgUp/PgDn scroll   g think   esc garden")
                .style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}
