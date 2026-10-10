mod process;
mod protocol;
mod setup;
pub use setup::{Consent, key_label};

pub use process::ProcessHost;
pub use protocol::{Command, Event, Failure};
use std::time::{Duration, Instant};

pub trait VoiceHost {
    fn setup_session(&self) -> std::sync::Arc<std::sync::Mutex<Consent>> {
        Default::default()
    }
    fn send(&mut self, command: Command) -> Result<(), Failure>;
    fn poll(&mut self) -> Option<Event>;
    fn shutdown(&mut self);
    fn restart(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Setup,
    Installing,
    Idle,
    Starting,
    Listening,
    Finalizing,
    Closed,
}

pub struct Final {
    pub text: String,
    pub send: bool,
}

pub struct Voice {
    host: Box<dyn VoiceHost>,
    phase: Phase,
    available: bool,
    next_take: u64,
    take: Option<u64>,
    partial: String,
    pending_send: bool,
    deadline: Option<Instant>,
    notice: String,
    failure: Option<Failure>,
    setup: setup::Setup,
}

impl Voice {
    pub fn new(host: Box<dyn VoiceHost>) -> Self {
        let setup = setup::Setup {
            session: host.setup_session(),
            ..Default::default()
        };
        Self {
            host,
            setup,
            phase: Phase::Loading,
            available: false,
            next_take: 0,
            take: None,
            partial: String::new(),
            pending_send: false,
            deadline: Some(Instant::now() + Duration::from_secs(30)),
            notice: String::new(),
            failure: None,
        }
    }
    pub fn available(&self) -> bool {
        self.available
    }
    pub fn setup_prompt(&self) -> bool {
        self.setup.prompt
    }
    pub fn answer_setup(&mut self, yes: bool, now: Instant) {
        if let Some(command) = self.setup.answer(yes) {
            self.begin_setup(command, now);
        } else {
            self.notice.clear();
        }
    }
    fn begin_setup(&mut self, command: Command, now: Instant) {
        self.phase = Phase::Installing;
        self.setup.prompt = false;
        self.notice = self.setup.need.unwrap().progress().into();
        self.failure = None;
        self.deadline = Some(now + Duration::from_secs(600));
        self.command(command);
    }
    fn request_setup(&mut self, now: Instant) {
        if let Some(command) = self.setup.request() {
            self.begin_setup(command, now);
        } else if self.setup.prompt {
            self.notice = self.setup.message.clone();
        } else {
            self.notice.clear();
        }
    }
    pub fn active(&self) -> bool {
        self.take.is_some()
    }
    pub fn partial(&self) -> &str {
        &self.partial
    }
    pub fn notice(&self) -> &str {
        &self.notice
    }
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }
    fn clear_take(&mut self) {
        self.take = None;
        self.partial.clear();
        self.pending_send = false;
        self.deadline = None;
        self.phase = Phase::Idle;
    }
    pub fn reject(&mut self, failure: Failure) {
        self.cancel();
        self.notice = failure
            .message
            .replace(['\r', '\n'], " ")
            .chars()
            .filter(|c| !c.is_control())
            .take(1024)
            .collect();
        self.failure = Some(failure);
    }
    fn command(&mut self, command: Command) {
        if let Err(failure) = self.host.send(command) {
            self.clear_take();
            self.available = false;
            self.reject(failure);
            self.setup.prompt = false;
            self.host.shutdown();
            self.phase = Phase::Closed;
        }
    }
    pub fn toggle(&mut self, now: Instant) {
        if self.active() {
            self.stop(false, now);
            return;
        }
        if self.phase == Phase::Installing {
            return;
        }
        if self.phase == Phase::Closed {
            self.setup.requested = true;
            self.host.restart();
            self.phase = Phase::Loading;
            self.deadline = Some(now + Duration::from_secs(30));
        }
        if self.setup.need.is_some() && self.phase == Phase::Setup {
            self.request_setup(now);
            return;
        }
        if !self.available {
            self.setup.requested = true;
            if self.failure.is_none() {
                self.notice = "Voice is loading. Press the voice key when ready.".into();
            }
            return;
        }
        self.next_take += 1;
        self.take = Some(self.next_take);
        self.phase = Phase::Starting;
        self.notice.clear();
        self.failure = None;
        self.deadline = Some(now + Duration::from_secs(30));
        self.command(Command::Start {
            take: self.next_take,
        });
    }
    pub fn stop(&mut self, send: bool, now: Instant) {
        let Some(take) = self.take else {
            return;
        };
        self.pending_send |= send;
        if self.phase != Phase::Finalizing {
            self.phase = Phase::Finalizing;
            self.deadline = Some(now + Duration::from_secs(30));
            self.command(Command::Stop { take });
        }
    }
    pub fn cancel(&mut self) {
        let take = self.take;
        self.clear_take();
        if let Some(take) = take {
            self.command(Command::Cancel { take });
        }
    }
    pub fn close(&mut self) {
        self.cancel();
        self.host.shutdown();
        self.setup.reset();
        self.available = false;
        self.phase = Phase::Closed;
        self.notice.clear();
        self.failure = None;
    }
    pub fn tick(&mut self, now: Instant) -> Option<Final> {
        if self.deadline.is_some_and(|end| now >= end) {
            self.close();
            self.reject(Failure::new(
                "timeout",
                "Voice timed out. Press the voice key to retry.",
            ));
            self.phase = Phase::Closed;
            return None;
        }
        for _ in 0..64 {
            let Some(event) = self.host.poll() else {
                break;
            };
            let needed = match &event {
                Event::NeedsInstall { .. } => Some(setup::Need::Install),
                Event::NeedsModel { .. } => Some(setup::Need::Model),
                _ => None,
            };
            match event {
                Event::NeedsInstall { size_mb, message }
                | Event::NeedsModel { size_mb, message }
                    if !self.active()
                        && matches!(
                            self.phase,
                            Phase::Loading | Phase::Setup | Phase::Installing
                        ) =>
                {
                    if size_mb == 0 || message.len() > 1024 {
                        self.reject(Failure::new("protocol", "Invalid voice setup response."));
                        continue;
                    }
                    self.setup.need = needed;
                    self.setup.message = message;
                    self.phase = Phase::Setup;
                    self.available = false;
                    self.deadline = None;
                    self.failure = None;
                    self.notice.clear();
                    if self.setup.requested {
                        self.request_setup(now);
                    }
                }
                Event::Loading
                    if !self.active()
                        && matches!(self.phase, Phase::Loading | Phase::Installing) =>
                {
                    self.phase = Phase::Loading;
                    self.deadline = Some(now + Duration::from_secs(30));
                }
                Event::Ready { version } if self.phase == Phase::Loading => {
                    if version != protocol::VERSION {
                        self.close();
                        self.reject(Failure::new(
                            "protocol",
                            "Incompatible voice protocol. Restart think.",
                        ));
                        self.phase = Phase::Closed;
                    } else {
                        self.available = true;
                        self.setup.reset();
                        self.phase = Phase::Idle;
                        self.deadline = None;
                        self.notice.clear();
                        self.failure = None;
                    }
                }
                Event::Listening { take } if self.take == Some(take) => {
                    if self.phase != Phase::Finalizing {
                        self.phase = Phase::Listening;
                        self.deadline = None;
                    }
                }
                Event::Partial { take, text } if self.take == Some(take) => {
                    if text.len() > 4096 {
                        self.reject(Failure::new(
                            "capture",
                            "Voice text exceeds 4096 bytes. Try a shorter take.",
                        ));
                    } else {
                        self.partial = text;
                    }
                }
                Event::Final { take, text } if self.take == Some(take) => {
                    let send = self.pending_send;
                    self.clear_take();
                    return Some(Final { text, send });
                }
                Event::Error {
                    take,
                    kind,
                    message,
                } if take.is_none() || take == self.take => {
                    let global = take.is_none();
                    self.reject(Failure::new(&kind, &message));
                    if global {
                        self.setup.prompt = false;
                        self.host.shutdown();
                        self.available = false;
                        self.phase = Phase::Closed;
                    }
                }
                _ => {}
            }
        }
        None
    }
    pub fn missed(&mut self) {
        self.notice = "Didn't catch that.".into();
    }
}
impl Drop for Voice {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn silence_filler(text: &str) -> bool {
    matches!(
        text.trim()
            .to_lowercase()
            .trim_end_matches(['.', '!', '?'])
            .trim(),
        "" | "thank you" | "you"
    )
}

#[cfg(test)]
mod setup_tests;
#[cfg(test)]
mod tests;
