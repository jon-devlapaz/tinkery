use super::*;
use handoff::{Affirmation, Config};
use std::{path::PathBuf, sync::mpsc};

pub(super) struct Review {
    pub affirmation: Affirmation,
    pub(super) input: String,
    pub(super) scroll: u16,
    pub(super) max: u16,
}
impl BrainDump {
    pub fn with_seed_me(mut self, skill: PathBuf, root: Option<PathBuf>) -> Self {
        self.goal_config = Some(Config::new(skill, root));
        self
    }
    pub fn review_goal(&mut self) {
        if self.receipt.is_some() || self.handoff_job.is_some() {
            return;
        }
        if !self.real || self.goal_config.is_none() {
            self.notice = "Goal confirmation needs real mode and --seed-me; practice does not create a session.".into();
            return;
        }
        if self.running()
            || self.ready.is_some()
            || !self.input.text.is_empty()
            || self.applied != self.sources.len()
        {
            self.notice =
                "Submit or clear local text and wait for reshaping before reviewing a goal.".into();
            return;
        }
        let Some(g) = &self.guess else {
            self.notice = "No displayed reading to confirm.".into();
            return;
        };
        let quoted_sources = g.framings[self.reading]
            .supports
            .iter()
            .map(|a| Source {
                id: a.source,
                text: a.quote.clone(),
                in_reply_to: None,
            })
            .collect::<Vec<_>>();
        for term in meaning_check::acronyms(&quoted_sources) {
            if !meaning_check::retains(&g.framings[self.reading].text, &term) {
                self.notice = format!(
                    "Selected reading lost authored term {term}; reshape before confirming."
                );
                return;
            }
        }
        let mut unresolved = g
            .questions
            .iter()
            .filter(|q| self.available(q))
            .cloned()
            .collect::<Vec<_>>();
        for q in &self.skipped {
            if !self.settled.iter().any(|s| same_question(&s.question, q))
                && !unresolved.iter().any(|u| same_question(u, q))
            {
                unresolved.push(q.clone());
            }
        }
        self.original = false;
        self.help = false;
        self.notice =
            "Nothing saved. Review the goal and remaining questions before affirming.".into();
        self.goal_review=Some(Review{affirmation:Affirmation{goal:agent_text(&g.framings[self.reading].text),outcome:g.outcome.clone(),options:g.alternatives.clone(),sources:self.sources.clone(),answered:self.settled.clone(),unresolved,source:"Tinkery operator typed confirm and pressed Enter after reviewing the displayed goal; goal only, not seed or implementation approval.".into()},input:String::new(),scroll:0,max:u16::MAX});
    }
    pub(super) fn goal_tick(&mut self) {
        let result = self
            .handoff_job
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
            self.handoff_job = None;
            if self.exit_after_handoff {
                self.quit = true;
            }
            match result {
                Ok(receipt) => {
                    self.notice = "Goal confirmed. The seed isn't written yet.".into();
                    self.goal_review = None;
                    self.receipt = Some(receipt);
                }
                Err(error) => {
                    self.notice = format!(
                        "Handoff not complete: {error}{}",
                        self.goal_config
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
    pub fn finish_handoff(&mut self) -> Option<String> {
        if let Some(receipt) = self.receipt.as_mut() {
            let stopped = receipt.stop_viewer();
            let mut text = receipt.text();
            text.push_str(&format!(
                "\nSession remains active. Saved ledger: {}",
                receipt.session.join("ledger-view.html").display()
            ));
            if let Err(e) = stopped {
                text.push_str(&format!("\nViewer shutdown failed: {e}"));
            }
            return Some(text);
        }
        self.goal_config
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
        if self.handoff_job.is_some() {
            self.notice="Saving the affirmed goal; wait for read-back. Durable writes cannot be undone by Esc.".into();
            return true;
        }
        if self.receipt.is_some() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.original = true;
                    self.original_scroll = 0;
                }
                _ => {}
            }
            return true;
        }
        let Some(review) = self.goal_review.as_mut() else {
            return false;
        };
        match key.code {
            KeyCode::Esc=>self.goal_review=None,
            KeyCode::PageDown|KeyCode::Down=>review.scroll=review.scroll.saturating_add(5).min(review.max),
            KeyCode::PageUp|KeyCode::Up=>review.scroll=review.scroll.saturating_sub(5),
            KeyCode::Backspace=>{review.input.pop();},
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) && review.input.len()<32=>review.input.push(c),
            KeyCode::Enter if review.input=="confirm" && review.scroll>=review.max=>{
                let config=self.goal_config.clone().unwrap();let affirmation=review.affirmation.clone();
                let (tx,rx)=mpsc::channel();
                std::thread::spawn(move||{let _=tx.send(config.confirm(affirmation));});
                self.handoff_job=Some(rx);self.notice="Saving the explicitly affirmed goal through Seed Me…".into();
            }
            KeyCode::Enter=>self.notice="Read the complete review (PgDn if needed), then type confirm and press Enter. Nothing saved.".into(),
            _=>{}
        }
        true
    }
    pub(super) fn goal_paste(&mut self, text: &str) -> bool {
        if self.handoff_job.is_some() || self.receipt.is_some() {
            return true;
        }
        if let Some(review) = self.goal_review.as_mut() {
            if text.chars().all(|c| !c.is_control()) && review.input.len() + text.len() <= 32 {
                review.input.push_str(text);
            }
            return true;
        }
        false
    }
}
pub(super) fn render_goal(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    if app.original || app.help {
        return;
    }
    if app.goal_review.is_none() && app.receipt.is_none() {
        return;
    }
    let area = frame.area();
    let rect = Rect::new(2, 3, area.width - 4, area.height - 6);
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(if app.receipt.is_some() {
            " Goal confirmed / q exits "
        } else {
            " Confirm this goal / Esc cancels "
        })
        .style(palette.ink);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if let Some(receipt) = &app.receipt {
        frame.render_widget(Paragraph::new(format!("{}\n\nTinkery's handoff is read-only. q exits; Ctrl-O originals.\nNo seed confirmation, intake readiness, or implementation approval.",receipt.text())).wrap(Wrap{trim:false}).style(palette.ink),inner);
        return;
    }
    let review = app.goal_review.as_mut().unwrap();
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(7)]).split(inner);
    let questions = if review.affirmation.unresolved.is_empty() {
        "None recorded. This is not proof of understanding.".into()
    } else {
        review
            .affirmation
            .unresolved
            .iter()
            .map(|q| format!("• {}", q.text))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let body = format!(
        "Goal you are confirming:\n{}\n\nRemaining unresolved questions (do not block goal confirmation):\n{}",
        review.affirmation.goal, questions
    );
    let p = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .style(palette.ink);
    review.max = p
        .line_count(rows[0].width)
        .saturating_sub(rows[0].height as usize) as u16;
    review.scroll = review.scroll.min(review.max);
    frame.render_widget(p.scroll((review.scroll, 0)), rows[0]);
    frame.render_widget(Paragraph::new(format!("Running out of questions doesn't mean I understood you.\nGoal only; outcome/options remain proposals. No seed or implementation approval.\nCreates a real Seed Me session outside the repository.\n{}\nConfirm this goal: type confirm, then Enter: {}\n{}",if review.max>0{"PgUp/PgDn reviews the complete goal and questions."}else{"Esc cancels without creating a session."},review.input,app.notice)).wrap(Wrap{trim:false}).style(palette.jade),rows[1]);
}
