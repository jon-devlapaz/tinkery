//! Exact repository thoughts in Tend's private store. Thoughts never become sessions or model
//! sources. Only explicit saving publishes words; opening Think returns an unsent copy.

use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use std::path::PathBuf;
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

    fn refresh(&mut self) {
        self.entries = self.store.read().map(|(entries, unreadable)| {
            self.unreadable = unreadable;
            entries
        });
    }

    /// Show the completed save and select its exact words without granting goal authority.
    pub(crate) fn saved(&mut self, words: &str, duplicate: bool) {
        if let Ok(entries) = &self.entries
            && let Some(index) = entries.iter().position(|entry| entry == words)
        {
            self.selected = index;
        }
        self.notice = if duplicate {
            "Thought already saved. No goal confirmed."
        } else {
            "Thought saved. No goal confirmed."
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

    /// A labelled garden summary. Full words remain in the local read view.
    pub(crate) fn summary(&self) -> String {
        let mut summary = match &self.entries {
            Ok(entries) if entries.is_empty() => "t think".into(),
            Ok(entries) => format!("Thoughts: {} · v read · t think", entries.len()),
            Err(_) => "Thoughts could not be read. Press v for details. t think".into(),
        };
        if !self.notice.is_empty() {
            summary.push_str(&format!("\n{}", self.notice));
        }
        if let Ok(entries) = &self.entries
            && let Some(words) = entries.get(self.selected)
        {
            summary.push_str(&format!(
                "\nThought: {}",
                crate::clipped(words.lines().next().unwrap_or(""), 60)
            ));
        }
        if self.unreadable > 0 {
            summary.push_str(&format!(
                "\n{} thoughts could not be read.",
                self.unreadable
            ));
        }
        summary
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
