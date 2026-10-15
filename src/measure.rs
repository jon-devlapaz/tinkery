//! Measure: explicitly requested ledger reports, owned cleanup and safe failure disclosure; no writes or retries.

use std::{io, path::Path, process::Output, sync::Arc};

use crate::Palette;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect, widgets::Paragraph};
use unicode_segmentation::UnicodeSegmentation;

mod plain;
mod reader;
mod viewport;
pub(crate) use reader::Reader;
use viewport::Viewport;

const COMMAND: &str = "python3 -B -m tink_substrate ledger report";
const UNKNOWN: &str = "Recorded work and attention are unknown.";
const CLUSTER_LIMIT: usize = 4096;

pub(crate) struct Reading {
    report: Report,
    scroll: usize,
    help_scroll: usize,
    how: bool,
    viewport: Viewport,
    request: Option<u64>,
    cleanup_error: Option<String>,
    text: [Arc<str>; 3],
    plain: Option<plain::Plain>,
}

enum Report {
    Waiting(&'static str),
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
    Timeout,
    OutputLimit,
    Cancelled,
    Collection,
    DisplayLimit,
}

impl Cause {
    fn explanation(&self) -> &'static str {
        match self {
            Self::PythonMissing => "Tinkery couldn’t find what it needs to read the ledger.",
            Self::ModuleUnavailable => "Tinkery couldn’t load what it needs to read the ledger.",
            Self::DirectoryUnavailable => "The configured ledger directory couldn’t be opened.",
            Self::Launch => "The ledger reader couldn’t be started.",
            Self::Exit => "The ledger reader exited without completing the report.",
            Self::Timeout => "The ledger report did not finish in time.",
            Self::OutputLimit => {
                "The ledger report exceeded the output limit; details are incomplete."
            }
            Self::Cancelled => "The ledger read was stopped; no report was accepted.",
            Self::Collection => "The ledger report could not be collected safely.",
            Self::DisplayLimit => {
                "The ledger report contains text too large to display safely; details are incomplete."
            }
        }
    }
}

impl Reading {
    #[cfg(test)]
    fn new(substrate_dir: &Path) -> Self {
        let result = std::process::Command::new("python3")
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
        Self::with_report(report)
    }

    fn with_report(mut report: Report) -> Self {
        let body = match &report {
            Report::Complete(body) => body.as_str(),
            Report::Failed(failure) => failure.details.as_str(),
            Report::Waiting(message) => message,
        };
        if body
            .graphemes(true)
            .any(|cluster| cluster.len() > CLUSTER_LIMIT)
        {
            report = Report::Failed(Failure {
                cause: Cause::DisplayLimit,
                details: format!(
                    "Command: {COMMAND}\nA Unicode character cluster exceeded the {CLUSTER_LIMIT}-byte display limit.\nReport and diagnostic text were withheld; no complete report was accepted."
                ),
            });
        }
        let plain = if let Report::Complete(body) = &report {
            Some(plain::Plain::new(body))
        } else {
            None
        };
        let text = match &report {
            Report::Complete(_) => [Arc::from(""), Arc::from(""), Arc::from("")],
            Report::Waiting(message) => [
                Arc::from(format!("{message}\n\n{UNKNOWN}")),
                Arc::from(format!(
                    "Right now\n\n{message}\n{UNKNOWN}\n\nLeaving stops this read. Nothing is retried automatically.\nF10 or Ctrl-C quits Tinkery."
                )),
                Arc::from(""),
            ],
            Report::Failed(failure) => [
                Arc::from(format!(
                    "Ledger report unavailable\n\n{}\n{UNKNOWN}",
                    failure.cause.explanation()
                )),
                Arc::from(format!(
                    "Right now\n\nThe ledger report couldn’t be read.\n{UNKNOWN}\n\n↑↓ / j k or PgUp/PgDn scroll.\nF10 or Ctrl-C quits Tinkery."
                )),
                Arc::from(format!(
                    concat!(
                        "How it works\n\nTinkery requested a report from the installed ledger reader.\n",
                        "This read failed; it does not establish that no work was recorded.\n\n",
                        "Details may contain private paths or output. Review before sharing.\n",
                        "PgUp/PgDn or ↑↓ / j k scroll. F10 or Ctrl-C quits Tinkery.\n\n{}"
                    ),
                    failure.details
                )),
            ],
        };
        Self {
            report,
            scroll: 0,
            help_scroll: 0,
            how: false,
            viewport: Viewport::default(),
            request: None,
            cleanup_error: None,
            text,
            plain,
        }
    }

    fn waiting(request: Option<u64>, message: &'static str) -> Self {
        let mut reading = Self::with_report(Report::Waiting(message));
        reading.request = request;
        reading
    }

    fn unavailable(cause: Cause, details: String) -> Self {
        Self::with_report(Report::Failed(Failure { cause, details }))
    }

    fn report_cause(&self) -> Option<&Cause> {
        if let Report::Failed(f) = &self.report {
            Some(&f.cause)
        } else {
            None
        }
    }

    fn add_cleanup_error(&mut self, error: &str) {
        self.cleanup_error = Some(error.into());
        if let Report::Failed(f) = &mut self.report {
            f.details.push_str("\n\nCleanup errors:\n");
            f.details.push_str(&display_bytes(error.as_bytes()));
            let mut text = self.text[2].to_string();
            text.push_str("\n\nCleanup errors:\n");
            text.push_str(&display_bytes(error.as_bytes()));
            self.text[2] = Arc::from(text);
        }
    }

    fn waiting_state(&self) -> bool {
        matches!(self.report, Report::Waiting(_))
    }

    pub(crate) fn local_help(&self) -> bool {
        !matches!(self.report, Report::Complete(_))
    }

    pub(crate) fn failed(&self) -> bool {
        matches!(self.report, Report::Failed(_))
    }

    /// Returns to the menu only on an explicit back action; help never starts another read.
    pub(crate) fn key(&mut self, key: KeyEvent, help: &mut bool) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::F(1) | KeyCode::Char('?') if self.local_help() => {
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
                if *help && self.local_help() {
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
        if let Report::Waiting(_) = &self.report {
            let text = self.text[usize::from(help)].clone();
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
                Paragraph::new(if help {
                    "esc/?/F1 close   q menu"
                } else {
                    "esc back   ? help"
                })
                .style(palette.muted),
                Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
            );
            return;
        }
        if let Report::Complete(_) = &self.report {
            self.plain
                .as_ref()
                .unwrap()
                .render(frame, area, palette, self.scroll);
            return;
        }
        let (text, footer) = if help && self.how {
            (
                self.text[2].clone(),
                "esc back   h help   ?/F1 close   q menu",
            )
        } else if help {
            (
                self.text[1].clone(),
                "esc/?/F1 close   h how it works   q menu",
            )
        } else {
            (self.text[0].clone(), "esc back   ? help")
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
