mod process;
mod protocol;

pub use process::ProcessHost;
pub use protocol::{Command, Event, Failure};
use std::time::{Duration, Instant};

pub trait VoiceHost {
    fn send(&mut self, command: Command) -> Result<(), Failure>;
    fn poll(&mut self) -> Option<Event>;
    fn shutdown(&mut self);
    fn restart(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Loading,
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
}

impl Voice {
    pub fn new(host: Box<dyn VoiceHost>) -> Self {
        Self {
            host,
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
        }
    }
    pub fn toggle(&mut self, now: Instant) {
        if self.active() {
            self.stop(false, now);
            return;
        }
        if self.phase == Phase::Closed {
            self.host.restart();
            self.phase = Phase::Loading;
            self.deadline = Some(now + Duration::from_secs(30));
        }
        if !self.available {
            if self.failure.is_none() {
                self.notice = "Voice is loading. Press F6 when ready.".into();
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
                "Voice timed out. Your words are safe; F6 reloads voice.",
            ));
            self.phase = Phase::Closed;
            return None;
        }
        for _ in 0..64 {
            let Some(event) = self.host.poll() else {
                break;
            };
            match event {
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
mod tests;
