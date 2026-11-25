//! A retained worker-action screen. Reviewing is inert; named keys admit bounded SDLC actions.
//! The menu retains an in-flight request across navigation, and no action retries itself.

use super::{
    Action, Config, Gate, Job, Recovery, Review, Run, Snapshot, Start, State, Turn, cap_advice,
};
use crate::{Palette, sdlc::Verification};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

/// Navigation only: neither destination starts a worker or preview.
pub(crate) enum Destination {
    /// The menu.
    Home,
    /// Watch over this run's checkout.
    Watch(PathBuf),
    /// Taste over this run's checkout.
    Taste(PathBuf),
}

enum Request {
    Start,
    Run(Action),
}

/// Cap guidance read from recent launch logs off the terminal thread; F2 and n wait for it.
struct Advice {
    landed: mpsc::Receiver<String>,
    stop: Arc<AtomicBool>,
}

impl Advice {
    fn read(checkout: PathBuf, cap: usize) -> Self {
        let (send, landed) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let (cancel, owner) = (stop.clone(), thread::current());
        thread::spawn(move || {
            let _ = send.send(cap_advice(&checkout, cap, &cancel));
            owner.unpark();
        });
        Self { landed, stop }
    }
}

impl Drop for Advice {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// One selected start or pinned run, including its finite request across visits.
pub(crate) struct Panel {
    goal: String,
    disclosure: String,
    advice: Option<Advice>,
    /// Cap guidance for `n`, read again after each fresh read that offers a new step.
    step_advice: Option<String>,
    gate: Gate,
    /// This launch's separate worker allowance; a new step uses it, never a saved default.
    allowance: Option<Config>,
    start: Option<Start>,
    /// The start in flight; it comes back for an explicit retry only if nothing was started.
    starting: Option<Start>,
    run: Option<Run>,
    snapshot: Option<Snapshot>,
    fresh: bool,
    /// SDLC refused the last `l`; only then may `n` replace a prepared step.
    launch_refused: bool,
    launching: bool,
    job: Option<Job>,
    notice: String,
    help: bool,
    scroll: u16,
    page: u16,
    read_to_end: bool,
}

impl Panel {
    /// Review the exact seed and separate worker allowance without running anything. Cap
    /// guidance is read in the background; F2 is offered only once it is shown.
    pub(crate) fn review(
        Review { seed, root }: Review,
        record_root: Result<PathBuf, String>,
        config: Option<Config>,
        gate: Gate,
    ) -> Self {
        let allowance = config.clone();
        let (start, disclosure, advice) = match (seed.target, config) {
            (Some(target), Some(config)) => {
                let advice = Advice::read(root.clone(), config.responses);
                let start = Start {
                    target,
                    root,
                    record_root,
                    config,
                    authorization: gate.clone(),
                };
                let disclosure = start.disclosure("Reading recent Pi launch logs…");
                (Some(start), disclosure, Some(advice))
            }
            _ => (
                None,
                "No worker allowance. Relaunch with --worker-model provider/model:thinking \
                         and --worker-responses N. Drafting calls do not authorize a worker."
                    .into(),
                None,
            ),
        };
        Self {
            goal: seed.goal,
            disclosure,
            advice,
            step_advice: None,
            gate,
            allowance,
            start,
            starting: None,
            run: None,
            snapshot: None,
            fresh: false,
            launch_refused: false,
            launching: false,
            job: None,
            notice: String::new(),
            help: false,
            scroll: 0,
            page: 1,
            read_to_end: false,
        }
    }

    /// Open an existing run that Watch selected, pinned to the identity Watch read; this reads
    /// only, and no new allowance is inferred.
    pub(crate) fn existing(
        root: PathBuf,
        slug: String,
        id: String,
        gate: Gate,
        allowance: Option<Config>,
    ) -> Self {
        let run = Run { root, slug, id };
        let job = Job::act(gate.clone(), run.clone(), Action::Refresh);
        Self {
            goal: String::new(),
            disclosure: gate.disclosure(None),
            advice: None,
            step_advice: None,
            gate,
            allowance,
            start: None,
            starting: None,
            run: Some(run),
            snapshot: None,
            fresh: false,
            launch_refused: false,
            launching: false,
            job: Some(job),
            notice: "Reading worker…".into(),
            help: false,
            scroll: 0,
            page: 1,
            read_to_end: false,
        }
    }

    /// Whether admission or another finite request must finish before the application exits.
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Land one source-owned outcome, never an inferred success or automatic next action.
    pub(crate) fn poll(&mut self) {
        if let Some(advice) = &self.advice
            && let Some(words) = match advice.landed.try_recv() {
                Ok(words) => Some(words),
                Err(mpsc::TryRecvError::Empty) => None,
                // The reading thread ended without guidance; F2 is offered without it.
                Err(mpsc::TryRecvError::Disconnected) => Some("Cap guidance unavailable.".into()),
            }
        {
            self.advice = None;
            match self.start.as_ref().or(self.starting.as_ref()) {
                Some(start) => self.disclosure = start.disclosure(&words),
                None => self.step_advice = Some(words),
            }
            self.read_to_end = false;
        }
        let Some(result) = self.job.as_mut().and_then(Job::poll) else {
            return;
        };
        self.job = None;
        self.scroll = 0;
        let starting = self.starting.take();
        let launching = std::mem::take(&mut self.launching);
        match result {
            Ok(snapshot) => {
                if snapshot.state != State::Prepared {
                    self.launch_refused = false;
                }
                self.run = Some(snapshot.run.clone());
                let checkout = snapshot.run.root.clone();
                self.snapshot = Some(snapshot);
                self.fresh = true;
                self.step_advice = None;
                self.advice = self
                    .allowance
                    .as_ref()
                    .filter(|_| self.can_start_new_step())
                    .map(|config| Advice::read(checkout, config.responses));
                self.notice =
                    "Read from tink-sdlc. r reads again; nothing resumes automatically.".into();
            }
            Err(failure) => {
                self.launch_refused |= launching;
                self.fresh = false;
                self.notice = format!("Request refused or uncertain: {}", failure.message);
                match failure.recovery {
                    Recovery::Run(run) => self.run = Some(run),
                    Recovery::Retry => {
                        self.start = starting;
                        self.scroll = u16::MAX;
                    }
                    Recovery::Inspect => self.scroll = u16::MAX,
                }
            }
        }
        self.read_to_end = false;
    }

    /// Handle one deliberate key. Help, repeats, modifiers and navigation never execute work.
    pub(crate) fn key(&mut self, key: KeyEvent) -> Option<Destination> {
        if key.kind != KeyEventKind::Press || key.modifiers != KeyModifiers::NONE {
            return None;
        }
        if key.code == KeyCode::F(10) {
            return Some(Destination::Home);
        }
        if matches!(key.code, KeyCode::F(1) | KeyCode::Char('?')) {
            self.help = !self.help;
            self.scroll = 0;
            return None;
        }
        if self.help {
            if key.code == KeyCode::Esc {
                self.help = false;
                self.scroll = 0;
            } else {
                self.scroll(key.code);
            }
            return None;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Some(Destination::Home),
            KeyCode::Char('w') => {
                return self
                    .run
                    .as_ref()
                    .map(|r| Destination::Watch(r.root.clone()));
            }
            KeyCode::Char('t') => {
                return self
                    .run
                    .as_ref()
                    .map(|r| Destination::Taste(r.root.clone()));
            }
            _ => {}
        }
        let request = self
            .requests()
            .find(|(code, _, _)| *code == key.code)
            .and_then(|(_, _, request)| request);
        match request {
            Some(Request::Start) => {
                let start = self.start.take().expect("offered start");
                self.starting = Some(start.clone());
                self.job = Some(Job::start(start));
                self.notice =
                    "Starting worker… Leaving this screen does not cancel admission.".into();
            }
            Some(Request::Run(action)) => {
                self.launching = matches!(action, Action::Launch(_));
                let run = self.run.clone().expect("offered run action");
                self.fresh = false;
                self.job = Some(Job::act(self.gate.clone(), run, action));
                self.notice = "Requesting the named action from tink-sdlc…".into();
            }
            None => self.scroll(key.code),
        }
        None
    }

    /// Each row owns its key and label. A disclosure hint stands in for a request
    /// until the end of that disclosure is on screen.
    fn requests(&self) -> impl Iterator<Item = (KeyCode, &'static str, Option<Request>)> {
        let idle = !self.busy() && !self.help;
        let has_run = idle && self.run.is_some();
        let current = has_run && self.fresh;
        let pin = self.snapshot.as_ref().and_then(|s| s.pin.as_ref());
        let can_stop = has_run
            && (!current
                || self
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| matches!(snapshot.state, State::Active { .. })));
        let run = Request::Run;
        [
            (
                KeyCode::F(2),
                "F2 start worker",
                Some("End to start"),
                (idle && self.start.is_some() && self.advice.is_none()).then_some(Request::Start),
            ),
            (
                KeyCode::Char('l'),
                "l launch prepared",
                Some("End to launch"),
                pin.filter(|_| current && self.can_launch())
                    .map(|pin| run(Action::Launch(pin.clone()))),
            ),
            (
                KeyCode::Char('f'),
                "f finish",
                None,
                (current && self.can_finish()).then(|| run(Action::Finish)),
            ),
            (
                KeyCode::Char('u'),
                "u resume",
                Some("End to resume"),
                pin.filter(|_| current && self.can_resume())
                    .map(|pin| run(Action::Resume(pin.clone()))),
            ),
            (
                KeyCode::Char('n'),
                "n new worker step",
                Some("End to start a new step"),
                self.allowance
                    .clone()
                    .filter(|_| current && self.can_start_new_step() && self.advice.is_none())
                    .map(|config| {
                        run(Action::Next {
                            pin: pin.cloned(),
                            config,
                        })
                    }),
            ),
            (
                KeyCode::Char('r'),
                "r read",
                None,
                has_run.then(|| run(Action::Refresh)),
            ),
            (
                KeyCode::Char('v'),
                "v verify",
                None,
                current.then(|| run(Action::Verify)),
            ),
            (
                KeyCode::Char('s'),
                "s stop",
                None,
                can_stop.then(|| run(Action::Stop)),
            ),
        ]
        .into_iter()
        .filter_map(|(key, label, hint, request)| {
            let request = request?;
            Some(match hint {
                Some(hint) if !self.read_to_end => (key, hint, None),
                _ => (key, label, Some(request)),
            })
        })
    }

    fn keys(&self) -> String {
        if self.busy() {
            return "Request in progress; navigation keeps it running.".into();
        }
        if self.help {
            return "↑↓ read   esc close help".into();
        }
        let mut keys: Vec<_> = self.requests().map(|(_, label, _)| label).collect();
        if self.run.is_none() {
            keys.push("↑↓ read");
        } else {
            if !self.fresh {
                keys.push("w watch");
            }
            keys.push("t taste");
        }
        keys.extend(["? help", "esc back"]);
        keys.join("  ")
    }

    fn can_resume(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|s| matches!(s.state, State::Active { turn: Turn::Ended }))
    }

    fn can_finish(&self) -> bool {
        self.can_resume()
            && self
                .snapshot
                .as_ref()
                .is_some_and(|s| s.verification.is(Verification::Current))
    }

    fn can_start_new_step(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| {
            s.state.needs_new_step()
                && !(s.state == State::Uncertain && self.gate.agent())
                && (s.state != State::Prepared || self.launch_refused)
        })
    }

    fn can_launch(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|s| s.state == State::Prepared && s.pin.is_some())
    }

    fn scroll(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(crate::page(self.page)),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(crate::page(self.page)),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = u16::MAX,
            _ => {}
        }
    }

    /// Draw the source state and consequences; painting makes no source reads or requests.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, palette: Palette) {
        let mut body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
        let mut lines = Vec::new();
        let mut add = |text: &str| {
            for line in text.lines() {
                lines.push(Line::raw(crate::shown_line(line)));
            }
        };
        if self.help {
            add(
                "Worker actions\n\nF2 starts only the reviewed Tink with its separate worker allowance.\n\
                 r reads status. l launches a prepared step; u resumes an ended turn on its existing model/session and cumulative cap.\n\
                 n starts a new step (build-2, build-3…) when the current one can do nothing more:\n\
                 no step yet, launch uncertain, completed or stopped. Its cap is this launch's worker allowance.\n\
                 v verifies first. Only current passing checks allow f to record the normally ended worker.\n\
                 After a refused request, only r and s work until a read succeeds.\n\
                 Failed checks keep the ended worker resumable after that read. s stops it. The recorded reason is the response cap only if the worker's log shows the cap marker.\n\
                 w opens Watch; t opens Taste, where r reads and v runs a declared preview.\n\
                 Esc/F10 return Home. b there reopens this worker.\n\
                 Admission finishes before Tinkery exits. The detached worker stays owned by SDLC.\n\
                 No refresh or navigation automatically retries, resumes, verifies or opens a preview.",
            );
        } else if let Some(snapshot) = &self.snapshot {
            if !self.fresh {
                add(
                    "Current worker and verification state unknown. Last successful read follows:\n",
                );
            }
            add(&format!(
                "Worker: {}\n\nRun: {}  step: {}\nCheckout: {}",
                snapshot.state.words(),
                snapshot.run.slug,
                snapshot.step,
                snapshot.run.root.display()
            ));
            if !snapshot.model.is_empty() {
                add(&format!(
                    "Pi model: {}\nResponse cap: {} (cumulative; one response may be in flight).",
                    snapshot.model, snapshot.responses
                ));
            }
            add(&format!(
                "Verification: {}\n\n{}",
                snapshot.verification.as_str().unwrap_or("unknown"),
                snapshot.next
            ));
            if snapshot.verification.is(Verification::AwaitingAttestation) {
                add(&format!(
                    "waiting on your judgment: {}\nOnly a person can record these judgments in tink-sdlc. Then v verifies again and f records the ended turn.",
                    snapshot.pending_attestations.join(", ")
                ));
            }
            if self.fresh
                && matches!(
                    snapshot.state,
                    State::Active {
                        turn: Turn::AtCap | Turn::Gone
                    }
                )
            {
                add("\nNext: s records the stop. Then n can start a new step.");
            }
            if self.fresh && self.can_launch() {
                add(
                    "\nl sends this prepared task and source context to the displayed provider within its existing cap. It grants no extra responses.",
                );
            }
            if self.fresh && self.can_start_new_step() {
                let state = snapshot.state;
                let finished =
                    state == State::Completed && snapshot.verification.is(Verification::Current);
                // Say first what the current step still needs, so n is never the default.
                add(match state {
                    State::Prepared => {
                        "\nNext: SDLC refused to launch this step. If l keeps being refused, n replaces it with a new step."
                    }
                    State::Uncertain => {
                        "\nNext: a worker may still be running. Check it with \
                         python3 _system/scripts/sdlc.py step <run> <step> show. \
                         Only when none runs does n start a new step. An agent cannot press n here."
                    }
                    _ if finished => "\nNext: t opens Taste. Press n only for a Taste finding.",
                    _ => "",
                });
                add(&match &self.allowance {
                    Some(config) => format!(
                        "\nn prepares step {} and launches it with {} and a new cap of {} responses, separate from earlier steps. \
                         If no build checkout exists yet, it opens one (stage 3) first. \
                         It sends the task and source context to that provider. The new step reads the brief in runs/{}/brief.md; add the finding there first.\n{}",
                        snapshot.next_step, config.model, config.responses, snapshot.run.slug,
                        self.step_advice.as_deref().unwrap_or("Reading recent Pi launch logs…")
                    ),
                    None => "\nA new worker step needs this launch's allowance. Relaunch with --worker-model provider/model:thinking and --worker-responses N, then press b.".into(),
                });
            }
            if self.fresh && self.can_resume() {
                add(
                    "\nu sends the existing task and source context to the same provider again within the remaining cap.\n\
                     v verifies before f records the ended turn; failed checks leave u available. Neither action accepts the result.",
                );
            }
        } else {
            add(if self.run.is_some() && self.busy() {
                "Current worker state unknown."
            } else if self.run.is_some() {
                "Worker result unavailable; current state unknown. r reads again"
            } else {
                "Review worker start"
            });
            add(&format!("\n{}\n\n{}", self.goal, self.disclosure));
            if let Some(run) = &self.run {
                add(&format!(
                    "Run: {}\nCheckout: {}",
                    run.slug,
                    run.root.display()
                ));
            }
        }
        if !self.notice.is_empty() {
            add(&format!("\n{}", self.notice));
        }
        if self.snapshot.is_some() {
            add(&self.gate.disclosure(None));
        }
        let text = Paragraph::new(Text::from(lines))
            .style(palette.ink)
            .wrap(Wrap { trim: false });
        let rows = text.line_count(body.width);
        if rows > body.height as usize && body.height > 1 {
            body.height -= 1;
        }
        self.page = body.height;
        let hidden = rows
            .saturating_sub(body.height as usize)
            .min(u16::MAX as usize) as u16;
        self.scroll = self.scroll.min(hidden);
        if !self.help && !self.busy() && body.height > 0 && self.scroll == hidden {
            self.read_to_end = true;
        }
        let keys = self.keys();
        if let Some(cue) = crate::more_cue(self.scroll as usize, hidden as usize) {
            frame.render_widget(
                Paragraph::new(cue).style(palette.muted),
                Rect::new(body.x, body.bottom(), body.width, 1),
            );
        }
        frame.render_widget(text.scroll((self.scroll, 0)), body);
        frame.render_widget(
            Paragraph::new(keys).style(palette.muted),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}
