//! Each place's side of the menu's [`Visit`] contract: how its keys, help and reads map to an
//! [`Outcome`]. Keys that a place's help blocks stay blocked; no outcome here starts an agent,
//! a Shape model call or a Taste save that the place itself did not already guard.

use super::*;
use crate::{
    taste::Taste,
    think::brain_dump::Exit,
    watch::{self, Watch},
};

pub(super) fn titled(frame: &mut Frame, title: &str, palette: Palette) -> Rect {
    chrome(
        frame,
        Line::from(Span::styled(title.to_string(), palette.ink)),
        palette,
    )
}

fn ctrl_c(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
}

/// Think alone ends Tinkery the moment it is left with nothing saved; otherwise a model ask.
fn after_think(dump: &mut BrainDump, kept: &Kept) -> Outcome {
    if kept.alone && *dump.exit() == Exit::Menu && dump.saved().is_none() {
        Outcome::Quit
    } else if dump.take_model_request() {
        Outcome::Model
    } else {
        Outcome::Stay
    }
}

impl Visit for BrainDump {
    fn key(&mut self, key: KeyEvent, kept: &mut Kept) -> Outcome {
        self.handle_key(key);
        after_think(self, kept)
    }

    fn paste(&mut self, text: &str) {
        BrainDump::paste(self, text);
    }

    fn mouse(&mut self, event: MouseEvent, area: Rect, _now: Instant, kept: &Kept) -> Outcome {
        self.handle_mouse(event, area);
        after_think(self, kept)
    }

    /// A finished visit returns to the menu, which reads again since a goal may have been
    /// saved, or to Tend on the goal it names.
    fn poll(&mut self, kept: &mut Kept, _now: Instant) -> Outcome {
        self.tick();
        if let Some(saved) = self.take_thought_saved() {
            kept.thought_saved = Some(saved);
        }
        let outcome = match self.exit() {
            Exit::Stay => return Outcome::Stay,
            Exit::Menu if kept.alone && self.saved().is_none() => return Outcome::Quit,
            Exit::Menu => Outcome::Back,
            Exit::Tend(path) => to(Open::TendSession(path.clone())),
        };
        kept.request_sessions();
        outcome
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        brain_dump::render(frame, self, palette);
    }

    fn close(mut self: Box<Self>, kept: &mut Kept) {
        kept.key_bar = self.key_bar_on();
        if let Some(receipt) = self.finish_handoff() {
            kept.receipts.push(receipt);
        }
        kept.alone = false;
    }

    fn take_copy(&mut self) -> Option<String> {
        self.take_copy_request()
    }

    fn copy_result(&mut self, sent: bool) {
        BrainDump::copy_result(self, sent);
    }

    fn presentation(&self) -> Option<brain_dump::Presentation> {
        BrainDump::presentation(self)
    }

    fn presented(&mut self, presentation: brain_dump::Presentation) {
        BrainDump::presented(self, presentation);
    }
}

/// A fresh Tend for this launch, after asking for a fresh scan; the last result shows meanwhile.
pub(super) fn tend(kept: &mut Kept) -> Tend {
    kept.request_sessions();
    let (sessions, records, store) = (
        kept.sessions_root.clone(),
        kept.record_root.clone(),
        kept.link_root.clone(),
    );
    let repo = kept
        .launch
        .repo
        .as_ref()
        .map(|repo| repo.root().to_path_buf());
    let shape_agent = kept.launch.shape_agent.clone();
    let actor = kept.launch.confirmation.clone().tend_save();
    let connect: tend::Connect = Box::new(move || {
        let sessions = sessions
            .clone()
            .ok_or("The sessions folder is unavailable.")?;
        let records = records.clone()?;
        let store = store.clone()?;
        let repo = repo
            .clone()
            .ok_or("Open Tinkery inside a repository first.")?;
        Ok(tend::connection::Context {
            sessions,
            records,
            store,
            repo,
            call: shape_agent.as_ref().map(|agent| agent.drafting()),
            actor: actor.clone(),
        })
    });
    let mut tend = Tend::new(
        kept.link_root.clone(),
        kept.launch.repo.as_ref().map(|repo| repo.root()),
        kept.record_root.clone(),
        connect,
    );
    if let Some((words, duplicate)) = kept.thought_saved.take() {
        tend.thought_saved(&words, duplicate);
    }
    tend.offer(&kept.discovery);
    tend
}

impl Visit for Tend {
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        match Tend::key(self, key) {
            tend::Next::Stay => Outcome::Stay,
            tend::Next::Menu => Outcome::Home,
            tend::Next::Think(None) => to(Open::Place(Place::Think)),
            tend::Next::Think(Some(words)) => to(Open::Words(words)),
            tend::Next::Shape(path) => to(Open::Shape(path)),
        }
    }

    fn paste(&mut self, text: &str) {
        Tend::paste(self, text);
    }

    fn mouse(&mut self, event: MouseEvent, area: Rect, now: Instant, _kept: &Kept) -> Outcome {
        self.pointer(event, body(area), now);
        Outcome::Stay
    }

    /// A goal saved from Find connections opens Shape on it, over Tend.
    fn poll(&mut self, kept: &mut Kept, now: Instant) -> Outcome {
        let Some(receipt) = Tend::poll(self, &kept.discovery, now) else {
            return Outcome::Stay;
        };
        let session = receipt.session.clone();
        kept.receipts.push(seed_me::ExitReceipt::Saved {
            receipt,
            record: None,
        });
        to(Open::Shape(session))
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, "tinkery › tend", palette);
        Tend::render(self, frame, body, palette);
    }

    /// Back from Shape: the same goal, from a fresh read.
    fn resumed(&mut self, kept: &mut Kept) {
        self.returned();
        kept.request_sessions();
        self.offer(&kept.discovery);
    }
}

/// Shape and its help overlay.
pub(super) struct Shaping {
    shape: Box<Shape>,
    help: bool,
}

impl Shaping {
    pub(super) fn new(shape: Shape) -> Self {
        Self {
            shape: Box::new(shape),
            help: false,
        }
    }
}

impl Visit for Shaping {
    /// F1 (or ? when Shape isn't taking words) for help, where a digit runs its numbered
    /// action, and Esc steps back. Held keys don't repeat an open, a save or a confirmation.
    /// Shift types capitals. Only F2 and the numbered actions write or run.
    fn key(&mut self, key: KeyEvent, kept: &mut Kept) -> Outcome {
        let shape = &mut self.shape;
        let typing = shape.takes_text();
        let editor_key = typing
            && !self.help
            && matches!(
                key.code,
                KeyCode::Char(_)
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::Backspace
                    | KeyCode::Delete
                    | KeyCode::Enter
                    | KeyCode::Tab
            );
        let shifted_char =
            key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_));
        if key.modifiers != KeyModifiers::NONE && !editor_key && !shifted_char
            || key.kind == KeyEventKind::Repeat
                && (matches!(key.code, KeyCode::F(2..=5)) || key.code == KeyCode::Enter && !typing)
        {
            return Outcome::Stay;
        }
        match key.code {
            KeyCode::Char('?') if typing && !self.help => {
                shape.key(key);
            }
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.help = !self.help;
                shape.reset_help();
            }
            KeyCode::Esc if self.help => self.help = false,
            _ if self.help => {
                if shape.pick(key) {
                    self.help = false;
                } else {
                    shape.scroll_help(key.code);
                }
            }
            _ => {
                let leave = shape.key(key);
                if shape.wants_sessions() {
                    kept.request_sessions();
                }
                if leave == crate::shape::Leave::Back {
                    return Outcome::Back;
                }
            }
        }
        if let Some(target) = shape.take_approve_request() {
            to(Open::Seed(target))
        } else if shape.take_model_request() {
            Outcome::Model
        } else {
            Outcome::Stay
        }
    }

    fn paste(&mut self, text: &str) {
        if !self.help {
            self.shape.paste(text);
        }
    }

    /// Hand Shape the readable sessions and unreadable count from its fresh discovery result,
    /// and let an open session's read finish.
    fn poll(&mut self, kept: &mut Kept, _now: Instant) -> Outcome {
        let shape = &mut self.shape;
        shape.tick();
        if shape.wants_sessions()
            && kept.discovery.fresh()
            && let Some(landed) = kept.discovery.landed()
        {
            let unreadable = landed.outcome.as_ref().map_or(0, |found| found.unreadable);
            shape.offer(
                match &landed.outcome {
                    Ok(found) => Ok(&found.sessions),
                    Err(error) => Err(error.clone()),
                },
                unreadable,
            );
            if let Ok(found) = &landed.outcome {
                shape.offer_failures(&found.failures);
            }
        }
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, "tinkery › shape", palette);
        if self.help {
            self.shape.render_help(frame, body, palette);
        } else {
            self.shape.render(frame, body, palette);
        }
    }

    /// Back from Approve: the same session through its normal read, help closed.
    fn resumed(&mut self, _kept: &mut Kept) {
        self.help = false;
        self.shape.reread();
    }
}

/// Approve and its help flag; the verifier it starts is the menu's one [`approve::Reader`].
pub(super) struct Approving {
    view: Box<approve::Approve>,
    help: bool,
}

impl Approving {
    pub(super) fn new(view: approve::Approve, kept: &mut Kept) -> Self {
        kept.request_sessions();
        let mut approving = Self {
            view: Box::new(view),
            help: false,
        };
        approving.offer(kept);
        approving
    }

    fn offer(&mut self, kept: &mut Kept) {
        if self.view.wants_sessions()
            && kept.discovery.fresh()
            && let Some(landed) = kept.discovery.landed()
        {
            let (sessions, unavailable, reasons) = match &landed.outcome {
                Ok(found) => (
                    Ok(found.sessions.clone()),
                    found.unreadable,
                    found
                        .failures
                        .iter()
                        .map(|f| format!("{}: {}", f.path.display(), f.reason))
                        .collect(),
                ),
                Err(reason) => (Err(reason.clone()), 0, Vec::new()),
            };
            self.view.offer(
                sessions,
                unavailable,
                reasons,
                &mut kept.approve_reader,
                kept.record_root.clone(),
            );
        }
    }
}

impl Visit for Approving {
    /// Esc returns to whatever Approve was opened over; F10 returns to the menu.
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        if ctrl_c(key) {
            return Outcome::Quit;
        }
        if key.modifiers == KeyModifiers::NONE && self.view.key(key, &mut self.help) {
            return if key.code == KeyCode::F(10) {
                Outcome::Home
            } else {
                Outcome::Back
            };
        }
        match self.view.take_worker_request() {
            Some(review) => to(Open::Review(review)),
            None => Outcome::Stay,
        }
    }

    fn poll(&mut self, kept: &mut Kept, _now: Instant) -> Outcome {
        self.offer(kept);
        kept.approve_reader.poll(Some(&mut self.view));
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, "tinkery › approve", palette);
        self.view.render(frame, body, palette, self.help);
    }

    fn close(self: Box<Self>, kept: &mut Kept) {
        kept.approve_reader.cancel();
    }

    fn take_copy(&mut self) -> Option<String> {
        self.view.take_copy_request()
    }

    fn copy_result(&mut self, sent: bool) {
        self.view.copy_result(sent);
    }
}

impl Visit for worker::Panel {
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        if ctrl_c(key) {
            return Outcome::Quit;
        }
        match worker::Panel::key(self, key) {
            None => Outcome::Stay,
            Some(worker::Destination::Home) => Outcome::Home,
            Some(worker::Destination::Watch(root)) => to(Open::Watch(root)),
            Some(worker::Destination::Taste(root)) => to(Open::Taste(root)),
        }
    }

    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        worker::Panel::poll(self);
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, kept: &Kept) {
        let body = titled(frame, "tinkery › worker", palette);
        worker::Panel::render(self, frame, body, palette);
        if kept.quit_after_worker {
            frame.render_widget(
                Paragraph::new("Finishing the current request before exit…").style(palette.muted),
                Rect::new(body.x, body.bottom().saturating_sub(2), body.width, 1),
            );
        }
    }

    /// The panel outlives the visit: `b` on the menu shows it again.
    fn close(self: Box<Self>, kept: &mut Kept) {
        kept.worker = Some(*self);
    }

    fn busy(&self) -> bool {
        worker::Panel::busy(self)
    }
}

/// Watch, Taste or Measure under the menu's shared help rule: `?` or F1 toggles help, Esc
/// closes it, and in help only `q` acts, returning to the menu.
pub(super) struct Helped<P> {
    place: Box<P>,
    help: bool,
}

impl<P> Helped<P> {
    pub(super) fn new(place: P) -> Self {
        Self {
            place: Box::new(place),
            help: false,
        }
    }

    /// The help rule's answer, or `None` when the key belongs to the place.
    fn help_key(&mut self, key: KeyEvent) -> Option<Outcome> {
        if matches!(key.code, KeyCode::Char('?') | KeyCode::F(1)) {
            self.help = !self.help;
            return Some(Outcome::Stay);
        }
        if self.help && key.code == KeyCode::Esc {
            self.help = false;
            return Some(Outcome::Stay);
        }
        self.help.then(|| {
            if key.code == KeyCode::Char('q') {
                Outcome::Home
            } else {
                Outcome::Stay
            }
        })
    }

    fn render_help(&self, frame: &mut Frame, body: Rect, text: String, palette: Palette) {
        frame.render_widget(
            Paragraph::new(text)
                .style(palette.ink)
                .wrap(Wrap { trim: false }),
            body,
        );
    }
}

/// F10 returns Watch or Taste to the menu on a plain press.
fn plain_press(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && key.kind == KeyEventKind::Press
}

impl Visit for Helped<Watch> {
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        if key.code == KeyCode::F(10) {
            return if plain_press(key) {
                Outcome::Home
            } else {
                Outcome::Stay
            };
        }
        if ctrl_c(key) {
            return Outcome::Quit;
        }
        if let Some(outcome) = self.help_key(key) {
            return outcome;
        }
        match self.place.key(key) {
            Some(watch::Exit::Leave) => Outcome::Home,
            Some(watch::Exit::Worker { root, slug, id }) => to(Open::Run { root, slug, id }),
            None => Outcome::Stay,
        }
    }

    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        self.place.poll();
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, "tinkery › watch", palette);
        if self.help {
            self.render_help(frame, body, self.place.help_text().into(), palette);
        } else {
            self.place.render(frame, body, palette);
        }
    }
}

impl Visit for Helped<Taste> {
    /// A label being saved finishes before F10 leaves, so its outcome is never left unknown.
    /// The label form takes words: `?` and q type; F1 still opens help.
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        let taste = &mut self.place;
        if key.code == KeyCode::F(10) {
            return if !taste.saving() && plain_press(key) {
                Outcome::Home
            } else {
                Outcome::Stay
            };
        }
        if ctrl_c(key) {
            return Outcome::Quit;
        }
        if taste.takes_text() && !self.help && key.code != KeyCode::F(1) {
            taste.key(key);
            return Outcome::Stay;
        }
        if let Some(outcome) = self.help_key(key) {
            return outcome;
        }
        let taste = &mut self.place;
        if taste.key(key) {
            Outcome::Home
        } else if let Some(rejected) = taste.take_retry() {
            to(Open::Retry(rejected))
        } else if let Some(draft) = taste.take_draft() {
            to(Open::Draft(draft))
        } else {
            Outcome::Stay
        }
    }

    fn paste(&mut self, text: &str) {
        if !self.help {
            self.place.paste(text);
        }
    }

    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        self.place.poll();
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, "tinkery › taste", palette);
        if self.help {
            self.render_help(frame, body, self.place.help_text(), palette);
        } else {
            self.place.render(frame, body, palette);
        }
    }

    fn take_copy(&mut self) -> Option<String> {
        self.place.take_copy_request()
    }

    fn copy_result(&mut self, sent: bool) {
        self.place.copy_result(sent);
    }
}

impl Visit for Helped<measure::Measure> {
    /// F10 and Ctrl-C quit Tinkery from Measure.
    fn key(&mut self, key: KeyEvent, _kept: &mut Kept) -> Outcome {
        if ctrl_c(key) || key.code == KeyCode::F(10) {
            return Outcome::Quit;
        }
        if let Some(outcome) = self.help_key(key) {
            return outcome;
        }
        if self.place.key(key) {
            Outcome::Home
        } else {
            Outcome::Stay
        }
    }

    fn poll(&mut self, _kept: &mut Kept, _now: Instant) -> Outcome {
        self.place.poll();
        Outcome::Stay
    }

    fn render(&mut self, frame: &mut Frame, palette: Palette, _kept: &Kept) {
        let body = titled(frame, &self.place.title(), palette);
        self.place.render(frame, body, palette, self.help);
    }
}
