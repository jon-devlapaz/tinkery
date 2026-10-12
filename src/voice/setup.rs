//! Consent for voice setup: whether the engine or model is missing, whether the operator has
//! been asked, and what they answered. A download or install command exists only after a yes.

use super::Command;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Need {
    Install,
    Model,
}
impl Need {
    fn index(self) -> usize {
        match self {
            Self::Install => 0,
            Self::Model => 1,
        }
    }
    pub(super) fn command(self) -> Command {
        match self {
            Self::Install => Command::Install,
            Self::Model => Command::Download,
        }
    }
    pub(super) fn progress(self) -> &'static str {
        match self {
            Self::Install => "installing voice…",
            Self::Model => "downloading voice model…",
        }
    }
}
/// The operator's answers to install and download prompts. A no is kept for the whole run; a
/// yes covers only the exact prompt it answered, so a different download asks again.
#[derive(Default)]
pub struct Consent {
    declined: [bool; 2],
    authorized: [Option<String>; 2],
}
static SESSION: OnceLock<Arc<Mutex<Consent>>> = OnceLock::new();
pub(super) fn session() -> Arc<Mutex<Consent>> {
    SESSION.get_or_init(Default::default).clone()
}
#[derive(Default)]
pub(super) struct Setup {
    pub need: Option<Need>,
    pub prompt: bool,
    pub requested: bool,
    pub message: String,
    pub session: Arc<Mutex<Consent>>,
}
impl Setup {
    pub(super) fn request(&mut self) -> Option<Command> {
        self.requested = true;
        let need = self.need?;
        let session = self.session.lock().unwrap();
        if session.declined[need.index()] {
            self.prompt = false;
        } else if session.authorized[need.index()].as_ref() == Some(&self.message) {
            return Some(need.command());
        } else {
            self.prompt = true;
        }
        None
    }
    pub(super) fn answer(&mut self, yes: bool) -> Option<Command> {
        if !self.prompt {
            return None;
        }
        self.prompt = false;
        let need = self.need?;
        let mut session = self.session.lock().unwrap();
        session.declined[need.index()] = !yes;
        session.authorized[need.index()] = yes.then(|| self.message.clone());
        yes.then(|| need.command())
    }
    pub(super) fn reset(&mut self) {
        self.need = None;
        self.prompt = false;
        self.requested = false;
        self.message.clear();
    }
}
/// The voice key as the key bar names it on `platform`: the chord on macOS, `6` (for F6)
/// elsewhere. F6 and Ctrl+Alt+Z work everywhere either way.
pub fn key_label(platform: &str) -> &'static str {
    if platform == "macos" { "⌃⌥Z" } else { "6" }
}
