//! Measure: the ledger's read-only report and failure disclosure. Nothing here writes or retries.

use std::{
    io,
    path::Path,
    process::{Command, Output},
};

use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

mod viewport;
use viewport::Viewport;

const COMMAND: &str = "python3 -B -m tink_substrate ledger report";
const UNKNOWN: &str = "Recorded work and attention are unknown.";

pub(crate) struct Reading {
    report: Report,
    scroll: usize,
    help_scroll: usize,
    how: bool,
    viewport: Viewport,
}

enum Report {
    Complete(String),
    Failed(Failure),
}

struct Failure {
    cause: Cause,
    details: String,
}

#[derive(Debug, PartialEq)]
enum Cause {
    PythonMissing,
    ModuleUnavailable,
    DirectoryUnavailable,
    Launch,
    Exit,
}

impl Cause {
    fn explanation(&self) -> &'static str {
        match self {
            Self::PythonMissing => "Tinkery couldn’t find what it needs to read the ledger.",
            Self::ModuleUnavailable => "Tinkery couldn’t load what it needs to read the ledger.",
            Self::DirectoryUnavailable => "The configured ledger directory couldn’t be opened.",
            Self::Launch => "The ledger reader couldn’t be started.",
            Self::Exit => "The ledger reader exited without completing the report.",
        }
    }
}

impl Reading {
    pub(crate) fn new(substrate_dir: &Path) -> Self {
        let result = Command::new("python3")
            .args(["-B", "-m", "tink_substrate", "ledger", "report"])
            .current_dir(substrate_dir)
            .output();
        Self::from_result(substrate_dir, result)
    }

    fn from_result(directory: &Path, result: io::Result<Output>) -> Self {
        let report = match result {
            Ok(out) if out.status.success() => {
                Report::Complete(String::from_utf8_lossy(&out.stdout).into_owned())
            }
            result => {
                let (cause, diagnostic) = match result {
                    Ok(out) => {
                        let stderr = display_bytes(&out.stderr);
                        let cause = if missing_module(&stderr) {
                            Cause::ModuleUnavailable
                        } else {
                            Cause::Exit
                        };
                        let stderr = if stderr.is_empty() {
                            "No error details were returned.".into()
                        } else {
                            stderr
                        };
                        let status = out
                            .status
                            .code()
                            .map_or_else(|| out.status.to_string(), |code| code.to_string());
                        (
                            cause,
                            format!("Exit status: {status}\n\nError output:\n{stderr}"),
                        )
                    }
                    Err(error) => {
                        let metadata = directory.metadata();
                        let unavailable = match &metadata {
                            Ok(m) => !m.is_dir(),
                            Err(e) => matches!(
                                e.kind(),
                                io::ErrorKind::NotFound
                                    | io::ErrorKind::NotADirectory
                                    | io::ErrorKind::PermissionDenied
                            ),
                        };
                        let cause = if unavailable {
                            Cause::DirectoryUnavailable
                        } else if metadata.is_ok() && error.kind() == io::ErrorKind::NotFound {
                            Cause::PythonMissing
                        } else {
                            Cause::Launch
                        };
                        (
                            cause,
                            format!(
                                "Launch error: {}",
                                display_bytes(error.to_string().as_bytes())
                            ),
                        )
                    }
                };
                let path = display_bytes(directory.as_os_str().as_encoded_bytes());
                Report::Failed(Failure {
                    cause,
                    details: format!("Command: {COMMAND}\nWorking directory: {path}\n{diagnostic}"),
                })
            }
        };
        Self {
            report,
            scroll: 0,
            help_scroll: 0,
            how: false,
            viewport: Viewport::default(),
        }
    }

    pub(crate) fn failed(&self) -> bool {
        matches!(self.report, Report::Failed(_))
    }

    /// Returns to the menu only on an explicit back action; help never starts another read.
    pub(crate) fn key(&mut self, key: KeyEvent, help: &mut bool) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::F(1) | KeyCode::Char('?') if self.failed() => {
                *help = !*help;
                self.how = false;
                self.help_scroll = 0;
            }
            KeyCode::Esc if *help && self.how => {
                self.how = false;
                self.help_scroll = 0;
            }
            KeyCode::Esc if *help => *help = false,
            KeyCode::Esc => return true,
            KeyCode::Char('h') if *help && self.failed() => {
                self.how = !self.how;
                self.help_scroll = 0;
            }
            code => scroll(
                if *help && self.failed() {
                    &mut self.help_scroll
                } else {
                    &mut self.scroll
                },
                code,
            ),
        }
        if !self.failed() {
            self.scroll = self.scroll.min(usize::from(u16::MAX));
        }
        false
    }

    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette, help: bool) {
        let Report::Failed(failure) = &self.report else {
            if let Report::Complete(body) = &self.report {
                frame.render_widget(
                    Paragraph::new(body.as_str())
                        .style(palette.ink)
                        .scroll((self.scroll as u16, 0)),
                    area,
                );
            }
            return;
        };
        let (text, footer) = if help && self.how {
            (
                format!(
                    "How it works\n\nTinkery requested a report from the installed ledger reader.\n\
                This read failed; it does not establish that no work was recorded.\n\n\
                Details may contain private paths or output. Review before sharing.\n\
                PgUp/PgDn or ↑↓ / j k scroll. F10 or Ctrl-C quits Tinkery.\n\n{}",
                    failure.details
                ),
                "esc back   h help   ?/F1 close   q menu",
            )
        } else if help {
            (
                format!(
                    "Right now\n\nThe ledger report couldn’t be read.\n{UNKNOWN}\n\n\
                ↑↓ / j k or PgUp/PgDn scroll.\nF10 or Ctrl-C quits Tinkery."
                ),
                "esc/?/F1 close   h how it works   q menu",
            )
        } else {
            (
                format!(
                    "Ledger report unavailable\n\n{}\n{UNKNOWN}",
                    failure.cause.explanation()
                ),
                "esc back   ? help",
            )
        };
        let content = Rect {
            height: area.height.saturating_sub(2),
            ..area
        };
        let offset = if help {
            &mut self.help_scroll
        } else {
            &mut self.scroll
        };
        self.viewport.render(frame, content, palette, text, offset);
        frame.render_widget(
            Paragraph::new(footer).style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}

fn scroll(offset: &mut usize, code: KeyCode) {
    *offset = match code {
        KeyCode::Down | KeyCode::Char('j') => offset.saturating_add(1),
        KeyCode::Up | KeyCode::Char('k') => offset.saturating_sub(1),
        KeyCode::PageDown => offset.saturating_add(10),
        KeyCode::PageUp => offset.saturating_sub(10),
        _ => *offset,
    };
}

// Recognize only Python's single-line missing-target-module diagnostic, not dependency errors.
fn missing_module(stderr: &str) -> bool {
    let line = stderr.trim_end_matches('\n');
    let Some(interpreter) = line.strip_suffix(": No module named tink_substrate") else {
        return false;
    };
    !line.contains('\n')
        && Path::new(interpreter)
            .file_name()
            .and_then(|p| p.to_str())
            .is_some_and(|p| {
                p == "python3"
                    || p.strip_prefix("python3.").is_some_and(|version| {
                        !version.is_empty() && version.chars().all(|c| c.is_ascii_digit())
                    })
            })
}

// Keep diagnostics legible without letting subprocess bytes become terminal instructions.
fn display_bytes(bytes: &[u8]) -> String {
    let mut display = String::new();
    for chunk in bytes.utf8_chunks() {
        for ch in chunk.valid().chars() {
            if ch != '\n'
                && (ch.is_control()
                    || matches!(
                        ch,
                        '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
                    ))
            {
                display.extend(ch.escape_default());
            } else {
                display.push(ch);
            }
        }
        for byte in chunk.invalid() {
            display.push_str(&format!("\\x{byte:02x}"));
        }
    }
    display
}

#[cfg(test)]
mod tests;
