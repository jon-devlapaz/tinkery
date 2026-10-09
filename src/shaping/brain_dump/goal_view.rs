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
        self.details = false;
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
                    self.paper_scroll = 0;
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
                KeyCode::PageDown | KeyCode::Down => {
                    self.paper_scroll = self.paper_scroll.saturating_add(5)
                }
                KeyCode::PageUp | KeyCode::Up => {
                    self.paper_scroll = self.paper_scroll.saturating_sub(5)
                }
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
    let inner = app.agent_area;
    if let Some(receipt) = &app.receipt {
        let path_lines = Paragraph::new(receipt.session.display().to_string())
            .wrap(Wrap { trim: false })
            .line_count(inner.width);
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(path_lines.min(usize::from(inner.height.saturating_sub(4))) as u16),
            Constraint::Length(2),
        ])
        .flex(ratatui::layout::Flex::Start)
        .split(inner);
        frame.render_widget(
            Paragraph::new("Goal confirmed — seed not written yet").style(palette.ink),
            rows[0],
        );
        let p = Paragraph::new(receipt.session.display().to_string())
            .wrap(Wrap { trim: false })
            .style(palette.ink);
        let max = p
            .line_count(rows[1].width)
            .saturating_sub(rows[1].height as usize) as u16;
        app.paper_scroll = app.paper_scroll.min(max);
        frame.render_widget(p.scroll((app.paper_scroll, 0)), rows[1]);
        frame.render_widget(
            Paragraph::new("Continue with Seed Me in any harness").style(palette.muted),
            Rect::new(rows[2].x, rows[2].y + 1, rows[2].width, 1),
        );
        return;
    }
    let Some(review) = app.goal_review.as_mut() else {
        return;
    };
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(8)]).split(inner);
    let questions = review
        .affirmation
        .unresolved
        .iter()
        .map(|q| format!("• {}", q.text))
        .collect::<Vec<_>>()
        .join("\n");
    let body = if questions.is_empty() {
        review.affirmation.goal.clone()
    } else {
        format!("{}\n\nStill open\n{}", review.affirmation.goal, questions)
    };
    let p = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .style(palette.ink);
    review.max = p
        .line_count(rows[0].width)
        .saturating_sub(rows[0].height as usize) as u16;
    review.scroll = review.scroll.min(review.max);
    frame.render_widget(p.scroll((review.scroll, 0)), rows[0]);
    let saving = if app.handoff_job.is_some() {
        "\nSaving…"
    } else if app.notice.starts_with("Handoff not complete") {
        "\nHandoff not complete."
    } else {
        ""
    };
    let prompt = format!(
        "Running out of questions doesn't mean I understood you.\n\nCreates a Seed Me session; this goal can't be edited after.\n\ntype confirm   {}{}",
        review.input, saving
    );
    frame.render_widget(
        Paragraph::new(prompt.clone())
            .wrap(Wrap { trim: false })
            .style(palette.jade),
        rows[1],
    );
    if app.handoff_job.is_none() {
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
