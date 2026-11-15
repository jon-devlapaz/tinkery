//! Exact repository thoughts in Tend's private store. Thoughts never become sessions or model
//! sources. Only explicit saving publishes words; opening Think returns an unsent copy.

use crate::{Palette, words::Words};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
};

const LIMIT: usize = 128 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Thought {
    schema: u8,
    repo: PathBuf,
    words: String,
}

/// Repository-bound words and their local editor. No goal or seed authority lives here.
pub(crate) struct Garden {
    root: Result<PathBuf, String>,
    repo: PathBuf,
    entries: Result<Vec<String>, String>,
    selected: usize,
    editor: Option<Words>,
    viewing: bool,
    scroll: u16,
    notice: String,
}

impl Garden {
    /// Read only thoughts belonging to the captured launch repository; never infer attribution.
    pub(crate) fn open(root: Result<PathBuf, String>, repo: PathBuf) -> Self {
        let mut garden = Self {
            root,
            repo,
            entries: Ok(vec![]),
            selected: 0,
            editor: None,
            viewing: false,
            scroll: 0,
            notice: String::new(),
        };
        garden.entries = garden.read();
        garden
    }

    fn prefix(&self) -> String {
        format!(
            "thought-{:x}-",
            Sha256::digest(self.repo.as_os_str().as_encoded_bytes())
        )
    }

    fn file(&self, words: &str) -> Result<PathBuf, String> {
        Ok(self.root.as_ref().map_err(Clone::clone)?.join(format!(
            "{}{:x}.json",
            self.prefix(),
            Sha256::digest(words.as_bytes())
        )))
    }

    fn read_one(&self, path: &PathBuf) -> Result<String, String> {
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("A thought is not a regular local file.".into());
        }
        let mut bytes = vec![];
        file.take((LIMIT * 6 + 4096) as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > LIMIT * 6 + 4096 {
            return Err("A saved thought is oversized.".into());
        }
        let thought: Thought = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if thought.schema != 1
            || thought.repo != self.repo
            || thought.words.is_empty()
            || thought.words.len() > LIMIT
            || self.file(&thought.words)? != *path
        {
            return Err("A thought has invalid words, repository or format.".into());
        }
        Ok(thought.words)
    }

    fn read(&self) -> Result<Vec<String>, String> {
        let root = self.root.as_ref().map_err(Clone::clone)?;
        let files = match fs::read_dir(root) {
            Ok(files) => files,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.to_string()),
        };
        let mut entries = vec![];
        for file in files {
            let path = file.map_err(|e| e.to_string())?.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&self.prefix()))
                && path.extension().is_some_and(|e| e == "json")
            {
                entries.push(self.read_one(&path)?);
            }
        }
        entries.sort();
        Ok(entries)
    }

    fn save(&self, words: &str) -> Result<bool, String> {
        if words.is_empty() || words.len() > LIMIT {
            return Err("Write a thought of 1 to 131072 bytes. Nothing was saved.".into());
        }
        let root = self.root.as_ref().map_err(Clone::clone)?;
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let path = self.file(words)?;
        let thought = Thought {
            schema: 1,
            repo: self.repo.clone(),
            words: words.into(),
        };
        let bytes = serde_json::to_vec(&thought).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        match temp.persist_noclobber(&path) {
            Ok(_) => {
                fs::File::open(root)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                if self.read_one(&path)? != words {
                    return Err("The existing thought differs. Nothing was replaced.".into());
                }
                Ok(false)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Start a local exact-word editor. No save or provider call occurs.
    pub(crate) fn edit(&mut self) {
        self.editor = Some(Words::new(""));
        self.notice.clear();
    }

    /// Open the saved thought list without granting goal authority.
    pub(crate) fn view(&mut self) {
        self.entries = self.read();
        self.selected = self.selected.min(
            self.entries
                .as_ref()
                .map_or(0, |e| e.len().saturating_sub(1)),
        );
        self.viewing = true;
        self.scroll = 0;
    }

    /// Whether this local view owns keyboard and paste input.
    pub(crate) fn active(&self) -> bool {
        self.editor.is_some() || self.viewing
    }

    /// Paste exact received words without saving them.
    pub(crate) fn paste(&mut self, text: &str) {
        if let Some(editor) = &mut self.editor {
            editor.insert(text);
        }
    }

    /// Return an explicit unsent Think draft only on `g`; Shape is never a destination.
    pub(crate) fn key(&mut self, key: KeyEvent) -> Option<String> {
        if key.kind != KeyEventKind::Press {
            return None;
        }
        if matches!(key.code, KeyCode::Esc | KeyCode::F(10)) {
            self.editor = None;
            self.viewing = false;
            self.notice.clear();
            return None;
        }
        if let Some(editor) = &mut self.editor {
            if key.code == KeyCode::F(2) && key.modifiers == KeyModifiers::NONE {
                let words = editor.text().to_owned();
                match self.save(&words) {
                    Ok(created) => {
                        self.entries = self.read();
                        self.selected = self
                            .entries
                            .as_ref()
                            .ok()
                            .and_then(|entries| entries.iter().position(|w| w == &words))
                            .unwrap_or(0);
                        self.editor = None;
                        self.viewing = true;
                        self.notice = if created {
                            "Thought saved. No goal confirmed."
                        } else {
                            "Thought already saved. One entry. No goal confirmed."
                        }
                        .into();
                    }
                    Err(reason) => self.notice = format!("Thought not saved: {reason}"),
                }
            } else {
                editor.key(key);
            }
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
        match &self.entries {
            Ok(entries) if entries.is_empty() => String::new(),
            Ok(entries) => format!(
                "Thoughts: {} · v read · t new thought\nThought: {}",
                entries.len(),
                crate::clipped(
                    entries[self.selected.min(entries.len() - 1)]
                        .lines()
                        .next()
                        .unwrap_or(""),
                    60
                )
            ),
            Err(_) => "Thoughts could not be read. Press v for details. t new thought".into(),
        }
    }

    /// Render the exact-word editor or labelled local record. Rendering reads no files.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let editing = self.editor.is_some();
        let heading = format!(
            "{}\nRepository: {}\n{}",
            if editing {
                "New thought"
            } else {
                "Saved thoughts"
            },
            self.repo.display(),
            if editing {
                "F2 saves exact words here. No model call. No goal confirmation."
            } else {
                "g opens these words in Think, unsent. Confirm a goal there. No Shape."
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
            area.height.saturating_sub(rows + 4),
        );
        if let Some(editor) = &mut self.editor {
            editor.render(frame, body, palette, false);
        } else {
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
        }
        frame.render_widget(
            Paragraph::new(self.notice.as_str())
                .style(palette.ink)
                .wrap(Wrap { trim: false }),
            Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 2),
        );
        frame.render_widget(
            Paragraph::new(if editing {
                "F2 save thought only   esc cancel"
            } else {
                "↑↓ thoughts   PgUp/PgDn scroll   g think   esc garden"
            })
            .style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}
