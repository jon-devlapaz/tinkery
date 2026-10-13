//! Review and receipt: freezing the displayed goal, the typed `confirm`, saving through Seed Me,
//! and the receipt that follows a save that read back. Saving records the goal and the
//! person's originals, then the think record when a record folder is set; it never approves a
//! seed or any implementation. A record that fails to write leaves the saved goal standing.

use super::*;
use crate::goal::confirmed::{SavedGoal, goal_block, render_saved_goal};
use crate::seed_me::{self, Config, Receipt};
use crate::think::record;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

/// How the receipt's notice begins when the goal saved but its think record didn't.
const RECORD_FAILED: &str = "Saved, but the think record wasn't written:";

/// The goal under review and everything think records with it.
#[derive(Clone)]
pub(super) struct Affirmation {
    pub goal: String,
    pub parts: GoalParts,
    pub other_goals: Vec<String>,
    pub deferred: Vec<String>,
    pub outcome: String,
    pub options: Vec<Approach>,
    pub sources: Vec<Source>,
    pub answered: Vec<Settled>,
    pub unresolved: Vec<Question>,
    pub unresolved_notes: Vec<String>,
    pub decisions: Vec<OpenDecision>,
    pub source: String,
}
impl Affirmation {
    /// What Seed Me records: the originals verbatim, and think's questions as provisional context.
    pub(super) fn affirmed(&self) -> seed_me::Affirmed {
        let context = json!({
            "answered": self.answered,
            "unresolved": self.unresolved,
            "unresolved_notes": self.unresolved_notes,
            "decisions": self.decisions,
            "goal_parts": self.parts,
            "success": self.parts.done_when,
            "constraints": {"must": self.parts.must, "must_not": self.parts.must_not},
            "other_goals": self.other_goals,
            "deferred": self.deferred,
            "problem": self.parts.why,
        });
        seed_me::Affirmed {
            goal: self.goal.clone(),
            outcome: self.outcome.clone(),
            options: self
                .options
                .iter()
                .map(|o| seed_me::Choice {
                    label: o.label.clone(),
                    benefit: o.benefit.clone(),
                    cost: o.cost.clone(),
                    undo_cost: o.undo_cost.clone(),
                })
                .collect(),
            originals: self
                .sources
                .iter()
                .map(|s| seed_me::Original {
                    id: s.id,
                    in_reply_to: s.in_reply_to.clone(),
                    text: s.text.clone(),
                })
                .collect(),
            context: context.to_string(),
            source: self.source.clone(),
        }
    }
}

pub(super) struct Review {
    pub affirmation: Affirmation,
    pub(super) input: String,
    pub(super) scroll: u16,
    pub(super) max: u16,
}
impl BrainDump {
    /// Saves confirmed goals with the Seed Me skill at `skill`, under `root` or Seed Me's own
    /// default. Without this, review explains that saving needs it.
    pub fn with_seed_me(mut self, skill: PathBuf, root: Option<PathBuf>) -> Self {
        self.handoff.config = Some(Config::new(skill, root));
        self
    }
    /// Writes a think record into `root` with every confirmed save, and says so on the review
    /// screen. Without this, confirming saves the goal only.
    pub fn with_record_root(mut self, root: PathBuf) -> Self {
        self.handoff.record_root = Some(root);
        self
    }
    /// True when confirming also writes a think record.
    pub(super) fn records(&self) -> bool {
        self.handoff.record_root.is_some()
    }
    /// Where this visit's think record was written, or why it wasn't; `None` before a save or
    /// when no record folder is set.
    pub(super) fn record_line(&self) -> Option<String> {
        self.handoff.record.as_ref().map(|r| match r {
            Ok(path) => format!("Think record: {}", path.display()),
            Err(e) => format!("Think record not written: {e}"),
        })
    }
    /// Writes the think record for the session just saved in `receipt`: every turn, every
    /// check, and the goal exactly as it was confirmed on the review screen.
    fn write_record(&self, root: &Path, receipt: &Receipt) -> Result<PathBuf, String> {
        let goal = self
            .handoff
            .review
            .as_ref()
            .map(|r| r.affirmation.goal.as_str())
            .ok_or("No confirmed goal to record.")?;
        let origin = self.host.origin();
        record::write(
            root,
            &record::Session {
                origin: &origin,
                turns: self.conversation.turns(),
                checks: self
                    .diagnostics()
                    .iter()
                    .filter_map(|d| serde_json::from_str(d).ok())
                    .collect(),
                goal,
                seed_me_session: &receipt.session,
                confirmed: std::time::SystemTime::now(),
            },
        )
    }
    /// F3: opens review on the displayed goal, frozen as shown. Refused, with a visible reason, in
    /// practice, without Seed Me, during a take, or while words are unsent or a reading is
    /// still coming, so what is confirmed is exactly what was read.
    pub fn review_goal(&mut self) {
        if self.voice_active() {
            self.refuse("Stop or cancel voice before reviewing a goal.");
            return;
        }
        if self.handoff.receipt.is_some() || self.handoff.job.is_some() {
            return;
        }
        if !self.real || self.handoff.config.is_none() {
            self.refuse("Saving a goal needs real mode and Seed Me.");
            return;
        }
        // Each blocker names itself, so the person knows which one to clear.
        if self.running() {
            self.refuse("Still thinking; review once the reading is back.");
            return;
        }
        if !self.input.text.is_empty() {
            self.refuse("Send (F2) or clear your unsent words, then review.");
            return;
        }
        if self.shaping.ready.is_some() {
            self.refuse("A reply is waiting; F2 applies it, then review.");
            return;
        }
        if !self.conversation.caught_up() {
            self.refuse("The reading is behind your words; F2 retries, then review.");
            return;
        }
        let Some(g) = self.conversation.board() else {
            self.refuse("No reading to review yet; F2 sends your words.");
            return;
        };
        let Some(goal) = g.parts.compose() else {
            self.refuse("No goal to review yet; parts remain open.");
            return;
        };
        let open_parts = self.conversation.open_parts();
        let unresolved = self.conversation.unresolved();
        self.screen.original = false;
        self.screen.help = false;
        self.screen.details = false;
        self.screen.source_scroll = 0;
        self.screen.source_cursor = 0;
        self.screen.selection = None;
        self.screen.drag_anchor = None;
        self.notice =
            "Nothing saved. Review the goal and remaining questions before affirming.".into();
        let missing = open_parts
            .into_iter()
            .filter(|p| {
                !unresolved.iter().any(|q| q.target == Some(*p))
                    && !g.decisions.iter().any(|d| d.part == *p)
            })
            .map(|p| format!("{} — still open", p.label()))
            .collect::<Vec<_>>();
        let notes = g
            .unresolved_notes
            .iter()
            .filter(|note| {
                !g.decisions
                    .iter()
                    .any(|d| d.covered_by(&unresolved) && d.note() == **note)
            })
            .cloned()
            .collect::<Vec<_>>();
        self.handoff.review = Some(Review {
            affirmation: Affirmation {
                goal,
                parts: g.parts.clone(),
                other_goals: g.other_goals.clone(),
                deferred: g.deferred.clone(),
                outcome: g.parts.done_when.clone().unwrap_or_default(),
                options: g.alternatives.iter().map(board::Candidate::wire).collect(),
                sources: self.conversation.sources().to_vec(),
                answered: self.conversation.settled().to_vec(),
                decisions: g.decisions.clone(),
                unresolved,
                unresolved_notes: missing.into_iter()
                    .chain(notes)
                    .chain(g.misfits.iter().map(|s| format!("{} (original {})", s.quote, s.source)))
                    .collect(),
                source: "Tinkery operator typed confirm and pressed Enter after reviewing the displayed goal; goal only, not seed or implementation approval.".into(),
            },
            input: String::new(),
            scroll: 0,
            max: u16::MAX,
        });
    }
    pub(super) fn goal_tick(&mut self) {
        let result = self
            .handoff
            .job
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                    "Handoff worker disconnected; inspect the recovery path before retrying."
                        .into(),
                )),
            });
        if let Some(result) = result {
            self.handoff.job = None;
            if self.handoff.exit_after {
                self.leave();
            }
            match result {
                Ok(receipt) => {
                    self.handoff.record = self
                        .handoff
                        .record_root
                        .clone()
                        .map(|root| self.write_record(&root, &receipt));
                    self.notice = match &self.handoff.record {
                        Some(Err(e)) => format!("{RECORD_FAILED} {e}"),
                        _ => "Saved.".into(),
                    };
                    self.handoff.review = None;
                    self.screen.paper_scroll = 0;
                    self.handoff.receipt = Some(receipt);
                }
                Err(error) => {
                    self.notice = format!(
                        "Handoff not complete: {error}{}",
                        self.handoff
                            .config
                            .as_ref()
                            .and_then(Config::recovery_path)
                            .map_or(String::new(), |p| format!(
                                " Recovery session: {}",
                                p.display()
                            ))
                    );
                }
            }
        }
    }
    /// The text to print after the terminal is restored: the receipt (stopping the viewer this
    /// visit started; the session stays active), or where to look after a save that didn't
    /// finish. `None` when nothing was saved or attempted.
    pub fn finish_handoff(&mut self) -> Option<String> {
        if let Some(receipt) = self.handoff.receipt.as_mut() {
            let stopped = receipt.stop_viewer();
            let mut text = receipt.text();
            text.push_str(&format!(
                "\nSession remains active. Saved ledger: {}",
                receipt.saved_view().display()
            ));
            if let Some(line) = self.record_line() {
                text.push_str(&format!("\n{line}"));
            }
            if let Err(e) = stopped {
                text.push_str(&format!("\nViewer shutdown failed: {e}"));
            }
            return Some(text);
        }
        self.handoff
            .config
            .as_ref()
            .and_then(Config::recovery_path)
            .map(|p| {
                format!(
                    "Incomplete handoff; inspect Seed Me session {}. No seed confirmed.",
                    p.display()
                )
            })
    }
    pub(super) fn goal_key(&mut self, key: KeyEvent) -> bool {
        if self.handoff.job.is_some() {
            self.notice="Saving the affirmed goal; wait for read-back. Durable writes cannot be undone by Esc.".into();
            return true;
        }
        if self.handoff.receipt.is_some() {
            match key.code {
                KeyCode::Char('c') if key.modifiers.is_empty() => {
                    self.copy = self
                        .handoff
                        .receipt
                        .as_ref()
                        .map(Receipt::continuation_prompt);
                }
                KeyCode::Char('s') if key.modifiers.is_empty() => {
                    if let Some(receipt) = &self.handoff.receipt {
                        self.exit = Exit::Shape(receipt.session.clone());
                    }
                }
                KeyCode::PageDown | KeyCode::Down => {
                    self.screen.paper_scroll = self.screen.paper_scroll.saturating_add(5)
                }
                KeyCode::PageUp | KeyCode::Up => {
                    self.screen.paper_scroll = self.screen.paper_scroll.saturating_sub(5)
                }
                _ => {}
            }
            return true;
        }
        let Some(review) = self.handoff.review.as_mut() else {
            return false;
        };
        match key.code {
            KeyCode::Esc => self.handoff.review = None,
            KeyCode::PageDown | KeyCode::Down => {
                review.scroll = review.scroll.saturating_add(5).min(review.max)
            }
            KeyCode::PageUp | KeyCode::Up => review.scroll = review.scroll.saturating_sub(5),
            KeyCode::Backspace => {
                review.input.pop();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL) && review.input.len() < 32 =>
            {
                review.input.push(c)
            }
            KeyCode::Enter if review.input == "confirm" && review.scroll >= review.max => {
                let config = self.handoff.config.clone().unwrap();
                let affirmation = review.affirmation.affirmed();
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(config.confirm(affirmation));
                });
                self.handoff.job = Some(rx);
                self.notice = "Saving the explicitly affirmed goal through Seed Me…".into();
            }
            KeyCode::Enter if review.scroll < review.max => {
                self.refuse("Read to the end first: PgDn scrolls. Then type confirm and Enter.")
            }
            KeyCode::Enter => self.refuse("Type confirm, then Enter. Nothing saved."),
            _ => {}
        }
        true
    }
    pub(super) fn goal_paste(&mut self, text: &str) -> bool {
        if self.handoff.job.is_some() || self.handoff.receipt.is_some() {
            return true;
        }
        if let Some(review) = self.handoff.review.as_mut() {
            if text.chars().all(|c| !c.is_control()) && review.input.len() + text.len() <= 32 {
                review.input.push_str(text);
            }
            return true;
        }
        false
    }
}
pub(super) fn render_goal(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    let inner = app.screen.agent_area;
    if let Some(receipt) = &app.handoff.receipt {
        let notice = if app.notice == "Copied."
            || app.notice.starts_with("Clipboard")
            || app.notice.starts_with(RECORD_FAILED)
        {
            app.notice.as_str()
        } else {
            ""
        };
        render_saved_goal(
            frame,
            &mut app.screen.paper_scroll,
            inner,
            palette,
            SavedGoal {
                goal: &receipt.goal,
                next: Some("Next: shape it into a seed with Seed Me."),
                notice,
                saved: true,
            },
        );
        return;
    }
    let Some(review) = app.handoff.review.as_mut() else {
        return;
    };
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(inner);
    let mut body = format!("You are confirming\n{}", review.affirmation.goal);
    if !review.affirmation.other_goals.is_empty() {
        body.push_str(&format!(
            "\n\nalso in your dump: {}",
            review.affirmation.other_goals.join("; ")
        ));
    }
    if !review.affirmation.deferred.is_empty() {
        body.push_str(&format!(
            "\n\nfor the seed: {}",
            review.affirmation.deferred.join("; ")
        ));
    }
    let mut open = review
        .affirmation
        .unresolved
        .iter()
        .map(|q| {
            q.target
                .map_or_else(|| q.text.clone(), |p| format!("{} — {}", p.label(), q.text))
        })
        .collect::<Vec<_>>();
    open.extend(review.affirmation.unresolved_notes.iter().cloned());
    if !open.is_empty() {
        body.push_str(&format!("\n\nStill open\n{}", open.join("\n\n")));
    }
    let p = goal_block(body, palette);
    review.max = p
        .line_count(rows[0].width)
        .saturating_sub(rows[0].height as usize) as u16;
    review.scroll = review.scroll.min(review.max);
    frame.render_widget(p.scroll((review.scroll, 0)), rows[0]);
    let saving = if app.handoff.job.is_some() {
        "\nSaving…"
    } else if app.notice.starts_with("Handoff not complete") {
        "\nHandoff not complete."
    } else {
        ""
    };
    // A refused Enter says why in the instruction's place, until the next key.
    let instruction = if app.screen.refused {
        app.notice.clone()
    } else {
        format!(
            "type confirm to save{}. You can't edit it after.",
            if app.handoff.record_root.is_some() {
                " the goal and its record"
            } else {
                ""
            }
        )
    };
    let prompt = format!("{instruction}\n{}{}", review.input, saving);
    frame.render_widget(
        Paragraph::new(prompt.clone())
            .wrap(Wrap { trim: false })
            .style(palette.jade),
        rows[1],
    );
    if app.handoff.job.is_none() {
        app.screen.back_button = Rect::new(rows[1].x, rows[1].bottom() - 1, 4, 1);
        frame.render_widget(
            Paragraph::new("Back").style(palette.muted),
            app.screen.back_button,
        );
        let mut note = Note::new(&prompt);
        note.cursor = prompt.len();
        let wrapped = note.wrap(rows[1].width);
        if wrapped.cursor.0 < rows[1].height {
            frame.set_cursor_position((
                rows[1].x + wrapped.cursor.1.min(rows[1].width - 1),
                rows[1].y + wrapped.cursor.0,
            ));
        }
    }
}
