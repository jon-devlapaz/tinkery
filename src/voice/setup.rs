use super::Command;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
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
    pub fn command(self) -> Command {
        match self {
            Self::Install => Command::Install,
            Self::Model => Command::Download,
        }
    }
    pub fn progress(self) -> &'static str {
        match self {
            Self::Install => "installing voice…",
            Self::Model => "downloading voice model…",
        }
    }
}
#[derive(Default)]
pub struct Consent {
    declined: [bool; 2],
    authorized: [Option<String>; 2],
}
static SESSION: OnceLock<Arc<Mutex<Consent>>> = OnceLock::new();
pub fn session() -> Arc<Mutex<Consent>> {
    SESSION.get_or_init(Default::default).clone()
}
#[derive(Default)]
pub struct Setup {
    pub need: Option<Need>,
    pub prompt: bool,
    pub requested: bool,
    pub message: String,
    pub session: Arc<Mutex<Consent>>,
}
impl Setup {
    pub fn request(&mut self) -> Option<Command> {
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
    pub fn answer(&mut self, yes: bool) -> Option<Command> {
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
    pub fn reset(&mut self) {
        self.need = None;
        self.prompt = false;
        self.requested = false;
        self.message.clear();
    }
}
pub fn key_label(platform: &str) -> &'static str {
    if platform == "macos" { "⌃⌥Z" } else { "6" }
}
