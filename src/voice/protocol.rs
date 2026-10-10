use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const MAX_LINE: usize = 32768;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum Command {
    Start { take: u64 },
    Stop { take: u64 },
    Cancel { take: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    Ready {
        version: u32,
    },
    Listening {
        take: u64,
    },
    Partial {
        take: u64,
        text: String,
    },
    Final {
        take: u64,
        text: String,
    },
    Error {
        #[serde(default)]
        take: Option<u64>,
        kind: String,
        message: String,
    },
    #[serde(other)]
    Unknown,
}

impl Event {
    pub fn parse(line: &[u8]) -> Result<Self, Failure> {
        if line.len() > MAX_LINE {
            return Err(Failure::new(
                "protocol",
                "Voice response exceeds its limit. Restart think.",
            ));
        }
        serde_json::from_slice(line)
            .map_err(|_| Failure::new("protocol", "Malformed voice response. Restart think."))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: String,
    pub message: String,
}
impl Failure {
    pub fn new(kind: &str, message: &str) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
    pub fn event(self) -> Event {
        Event::Error {
            take: None,
            kind: self.kind,
            message: self.message,
        }
    }
}
