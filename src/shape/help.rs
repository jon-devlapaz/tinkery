//! Shape's F1 help: what the keys on screen do now, then how Shape works and what it never does,
//! with the discovery read's bounded failures. Paths are display-escaped, never changed. It
//! scrolls and paints through [`crate::help`].

use super::*;
use crate::pi::RESPONSE;

impl Shape {
    /// Help for what is on screen now: what to do, then how it works with the paths it reads.
    pub fn help_text(&self) -> String {
        let agent = self.agent.as_deref();
        let back = match self.open.as_ref().map(|open| (&open.mode, open.editor())) {
            Some((Mode::Running(..), _)) => "Esc stop",
            Some((_, Editor::Aside(Aside::TeachBack, _))) => "Esc cancel teach-back",
            Some((_, Editor::Aside(Aside::Change, _))) => "Esc cancel change",
            Some((_, Editor::Form(_))) => "Esc cancel form",
            _ => "Esc back",
        };
        let (now, how) = match &self.open {
            Some(open) => {
                let failure = match &open.reading {
                    Reading::Done(Err(error)) => format!("What went wrong:\n{error}\n\n"),
                    _ => String::new(),
                };
                let model = agent.map_or_else(
                    || {
                        if self.held {
                            HELD
                        } else {
                            "No model is set up, so Shape only reads."
                        }
                        .into()
                    },
                    |a| {
                        let step = if matches!(
                            &open.reading,
                            Reading::Done(Ok(Shaping {
                                stage: Shaped::Drafted {
                                    revision: Some(_),
                                    text: Some(_)
                                },
                                version: Some(_),
                                ..
                            }))
                        ) {
                            format!("Each teach-back comparison uses one {RESPONSE}, without repository access.")
                        } else {
                            format!("Each {RESPONSE} counts; a step that reads files may use several. Manual actions use none.")
                        };
                        format!(
                            "{} {step}\n\
                             Tinkery saves every change; the model never writes the session.",
                            a.left()
                        )
                    },
                );
                let reads = match (agent, &open.folder) {
                    (Some(_), Folder::Known(reads)) => {
                        format!("\nThe next step reads files in: {}", reads.folder.display())
                    }
                    _ => String::new(),
                };
                let teach_back = format!(
                    "Say what will and won't be built in your own words. Enter saves and compares (one {RESPONSE}); Esc cancels. Mismatches are guesses and never block confirming."
                );
                let change = format!(
                    "Say what should change in the draft. Enter saves your words as your decision and redrafts (one or more {RESPONSE}s); Esc cancels. Nothing is confirmed."
                );
                let now = match (&open.mode, agent, open.typing()) {
                    (Mode::Running(..), ..) => "A step is running. Esc stops it.",
                    (Mode::Idle(Editor::Form(_)) | Mode::Writing(_, _, Editor::Form(_)), ..) => {
                        "Edit the form. ^J adds a line; Esc cancels without saving."
                    }
                    (Mode::Writing(..), ..) => "Tinkery is saving a change.",
                    (Mode::Objecting, ..) => {
                        "Press the number of the assumption you object to; it becomes an open question. Esc keeps them all."
                    }
                    (Mode::Asking(Ending::Completed), ..) => {
                        "enter ends this session as completed, for good; Esc keeps it open."
                    }
                    (Mode::Asking(Ending::Stopped), ..) => {
                        "enter stops this session for good; Esc keeps it open."
                    }
                    (Mode::Choosing(_), ..) => {
                        "Type the folder the next step should read. ^J adds a line; Enter uses it; Esc keeps the one shown."
                    }
                    (
                        _,
                        Some(_),
                        Some(Typing {
                            act: Act::TeachBack { .. },
                            ..
                        }),
                    ) => &teach_back,
                    (
                        _,
                        Some(_),
                        Some(Typing {
                            act: Act::Change { .. },
                            ..
                        }),
                    ) => &change,
                    (
                        _,
                        Some(_),
                        Some(Typing {
                            act: Act::Keep { .. },
                            ..
                        }),
                    ) => {
                        "Type why you're keeping this answer. Enter keeps it unchanged with your reason; no model runs."
                    }
                    (
                        _,
                        Some(_),
                        Some(Typing {
                            act: Act::Answer { .. },
                            ..
                        }),
                    ) => {
                        concat!(
                            "Type your answer. ^J adds a line. Enter saves it and looks into the next question. A step may use several responses. ",
                            "PgUp/PgDn scroll the reading."
                        )
                    }
                    (
                        _,
                        Some(_),
                        Some(Typing {
                            act: Act::Confirm { .. },
                            ..
                        }),
                    ) => {
                        concat!(
                            "Read the Tink Contract, type confirm, then Enter confirms it. To change the draft instead, ",
                            "F1 Revise the draft. It is not approval to build. ",
                            "Without a completed teach-back, confirming records: Skipped teach-back."
                        )
                    }
                    (_, Some(_), Some(_)) => {
                        "Edit the form. ^J adds a line; Esc cancels without saving."
                    }
                    _ => "Check the status of this Tink. ↑↓ or PgUp/PgDn scroll.",
                };
                let form = match open.editor() {
                    Editor::Form(form) => format!("\n{}", form.description()),
                    _ => String::new(),
                };
                (
                    now.to_owned(),
                    format!(
                        "{failure}{model}{reads}{form}\nSession: {}",
                        open.session.display()
                    ),
                )
            }
            None => (
                "↑↓ / j k choose a goal waiting for a Tink · Enter opens it.".to_owned(),
                match &self.list {
                    Some(Some(Err(error))) => format!("What went wrong:\n{error}"),
                    Some(Some(Ok(_))) if self.unreadable > 0 => self.discovery_help(),
                    _ => "Goals whose Tink is ready, ended or unknown are in tend.".into(),
                },
            ),
        };
        let promise = if self.held {
            "Shape reads and calls no model. An agent answers Shape with --shape-pi; Esc goes back."
                .to_owned()
        } else if agent.is_some() {
            "Opening, scrolling and typing write nothing. Enter runs the one action the key bar names; F1 lists the rest by number. \
                 Saving an answer, a deferral or a delegation then looks into the next; only F2 on a drafted Tink confirms it, with your words. \
                 Revision saves and looks, asking recorded dependents before a model call. \
                 Ending a session, from F1, asks once more, then ends it for good."
                .to_owned()
        } else {
            "Shape reads and calls no model; its only write is ending a session, from F1, which asks once more first. \
                 To set up Shape, press Esc until the menu, then m: choose a model and responses, and s saves them. \
                 Then q quits; start Tinkery again, keeping the same --seed-session-root if you supplied one. \
                 Open Tend (2), select this saved goal, press Enter, then s to reopen Shape."
                .to_owned()
        };
        let editing = if self.takes_text() {
            "\nArrows, Home/End and Ctrl+Left/Right move; Shift selects; Ctrl+W or Alt+Backspace deletes a word.\n\
             Ctrl+U undoes, Ctrl+R redoes; PgUp/PgDn scroll the reading."
        } else {
            ""
        };
        // The numbered actions come first, so even a short panel shows them without scrolling.
        let rows = self.actions.text(&self.rows());
        let rows = if rows.is_empty() {
            rows
        } else {
            format!("{rows}\n\n")
        };
        format!(
            "Right now\n{rows}{now}{editing}\n{back}\n\nF1 close\n\nHow it works\n{how}\n\n{promise}"
        )
    }

    /// Show the retained failures and disclose omitted ones without inventing a reason.
    fn discovery_help(&self) -> String {
        let mut text = format!(
            "Unreadable session records: {}.\nSome session ledgers could not be read. \
             The list may be missing goals.\nInspect the files below, then Esc to close help and Esc to return to the menu; reopen Shape to read again.\n",
            self.unreadable
        );
        for failure in &self.failures {
            text.push_str(&format!(
                "\n{}\n{}\n",
                crate::shown_line(&failure.path.to_string_lossy()),
                crate::shown_line(&failure.reason)
            ));
        }
        let more = self.unreadable.saturating_sub(self.failures.len());
        if more > 0 {
            let records = if more == 1 { "record" } else { "records" };
            text.push_str(&format!("\n{more} more unreadable {records} not listed.\n"));
        }
        text
    }

    /// A newly opened help view starts at its first row, without changing the discovery read.
    pub(crate) fn reset_help(&mut self) {
        self.actions.reset();
        self.help_scroll.reset();
    }

    /// Scroll only help; navigation and scrolling never launch a read or a step.
    pub(crate) fn scroll_help(&mut self, code: KeyCode) {
        self.help_scroll.key(code);
    }

    /// Wrapped, scrollable F1 with a fixed close instruction, including capped failure lists.
    pub(crate) fn render_help(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let text = self.help_text();
        let pages = self
            .actions
            .hint(self.rows().len())
            .map_or(String::new(), |hint| format!("←/→ {hint} · "));
        let footer = format!("{pages}PgUp/PgDn scroll · F1 close · Esc back");
        self.help_scroll
            .render(frame, area, (text, footer), palette);
    }
}

#[cfg(test)]
mod tests;
