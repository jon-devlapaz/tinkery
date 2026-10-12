//! The helper's wire format: one JSON object per line each way, versioned. Commands and events
//! that belong to a take carry its id, so the Rust side can drop anything from an older take.

use serde::{Deserialize, Serialize};

/// The protocol version both sides must agree on; a helper reporting another is refused.
pub(super) const VERSION: u32 = 1;
/// Longest event line accepted, in bytes. Longer means the helper is broken, not chatty.
pub(super) const MAX_LINE: usize = 32768;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "command", rename_all = "lowercase")]
/// What Tinkery asks of the helper.
pub enum Command {
    /// Start recording take `take`.
    Start {
        /// The take's id.
        take: u64,
    },
    /// Stop recording and send the take's final text.
    Stop {
        /// The take's id.
        take: u64,
    },
    /// Stop recording and send nothing.
    Cancel {
        /// The take's id.
        take: u64,
    },
    /// Install the engine. Only sent after the operator said yes.
    Install,
    /// Download the speech model. Only sent after the operator said yes.
    Download,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
/// What the helper reports.
pub enum Event {
    /// The engine isn't installed; asking is up to Tinkery.
    NeedsInstall {
        /// Download size, shown in the consent line.
        size_mb: u32,
        /// The consent line to show.
        message: String,
    },
    /// The speech model isn't downloaded; asking is up to Tinkery.
    NeedsModel {
        /// Download size, shown in the consent line.
        size_mb: u32,
        /// The consent line to show.
        message: String,
    },
    /// Starting up; ready or needs-setup follows.
    Loading,
    /// Ready for takes.
    Ready {
        /// The helper's protocol version; see [`VERSION`].
        version: u32,
    },
    /// The microphone is open for this take.
    Listening {
        /// The take's id.
        take: u64,
    },
    /// Text so far for this take; replaces the previous partial.
    Partial {
        /// The take's id.
        take: u64,
        /// Everything heard so far.
        text: String,
    },
    /// The take's finished text; the take is over.
    Final {
        /// The take's id.
        take: u64,
        /// Everything heard.
        text: String,
    },
    /// Something failed: one take when `take` is set, the whole helper when it isn't.
    Error {
        /// The take that failed, if only one did.
        #[serde(default)]
        take: Option<u64>,
        /// A short machine-readable category, such as `permission` or `timeout`.
        kind: String,
        /// One plain line for the operator.
        message: String,
    },
    /// An event this version doesn't know; ignored.
    #[serde(other)]
    Unknown,
}

impl Event {
    /// One line from the helper. Oversized or malformed lines are a protocol failure.
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
/// Why voice couldn't do something, as one line the operator can act on.
pub struct Failure {
    /// A short machine-readable category, as in [`Event::Error`].
    pub kind: String,
    /// One plain line for the operator.
    pub message: String,
}
impl Failure {
    /// A failure of category `kind` saying `message`.
    pub fn new(kind: &str, message: &str) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
    /// The failure as a helper-wide error event, so a host can report its own failures the
    /// same way the helper does.
    pub fn event(self) -> Event {
        Event::Error {
            take: None,
            kind: self.kind,
            message: self.message,
        }
    }
}
