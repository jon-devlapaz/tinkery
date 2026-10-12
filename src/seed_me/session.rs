//! Reading one saved session straight from Seed Me's files. Read-only: nothing here writes.
//! Files are read directly rather than through the helper, so a scan of thousands of
//! sessions stays fast (see menu.md's measured scope).

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

/// How far a session has come, from Seed Me's own files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Active, with a confirmed goal and no seed contract yet.
    GoalSaved,
    /// Active, with a seed contract still in draft.
    SeedDrafted,
    /// Active, with a seed contract confirmed for intake.
    SeedReady,
    /// Completed or stopped; nothing to continue.
    Ended,
    /// The files say something this module doesn't recognise. Never guessed.
    Unknown,
    /// A practice session's seed, not a real one.
    Simulated,
}
impl Stage {
    /// The stage a ledger `status` and the seed contract's text (if one exists) mean. Anything
    /// it doesn't recognise is `Unknown`, never a guess.
    pub fn from_files(status: &str, seed: Option<&str>) -> Self {
        match (status, seed.and_then(|s| s.lines().next())) {
            ("completed" | "stopped", _) => Self::Ended,
            ("active", None) => Self::GoalSaved,
            ("active", Some("status: draft")) => Self::SeedDrafted,
            ("active", Some("status: confirmed for intake")) => Self::SeedReady,
            ("active", Some("status: simulated")) => Self::Simulated,
            _ => Self::Unknown,
        }
    }
    /// The stage in plain words, for a list row.
    pub fn label(self) -> &'static str {
        match self {
            Self::GoalSaved => "goal saved",
            Self::SeedDrafted => "seed drafted",
            Self::SeedReady => "seed ready",
            Self::Ended => "ended",
            Self::Unknown => "status unknown",
            Self::Simulated => "practice seed",
        }
    }
    /// Whether the continuation prompt applies: only a live session with a real goal.
    pub fn can_copy(self) -> bool {
        matches!(self, Self::GoalSaved | Self::SeedDrafted | Self::SeedReady)
    }
    /// What Seed Me does next at this stage; none when there is nothing to continue.
    pub fn next(self) -> Option<&'static str> {
        match self {
            Self::GoalSaved => Some("Next: shape it into a seed with Seed Me."),
            Self::SeedDrafted => Some("Next: confirm the seed draft with Seed Me."),
            Self::SeedReady => Some("Next: the seed is ready for intake."),
            _ => None,
        }
    }
}

/// One readable session.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// The session folder, canonicalised.
    pub path: PathBuf,
    /// The confirmed goal as recorded, or a plain placeholder when none is.
    pub goal: String,
    /// How far it has come.
    pub stage: Stage,
    /// The ledger's last update time as Seed Me wrote it; empty when absent.
    pub updated: String,
}

/// What one folder holds.
pub enum Record {
    /// A session that could be read.
    Session(Session),
    /// No ledger: some other folder, not counted.
    NotASession,
    /// A ledger exists but can't be read or has no status. Never guessed or counted.
    Unreadable,
}

/// Read the session in folder `path`. Promises a `Session` only when the ledger parses and has
/// a status; refuses to guess a stage for an active session with no goal (`Stage::Unknown`).
pub fn read(path: &Path) -> Record {
    use std::io::ErrorKind::{NotADirectory, NotFound};
    let text = match fs::read_to_string(path.join("ledger.json")) {
        Ok(text) => text,
        Err(error) if matches!(error.kind(), NotFound | NotADirectory) => {
            return Record::NotASession;
        }
        Err(_) => return Record::Unreadable,
    };
    let Ok(ledger) = serde_json::from_str::<Value>(&text) else {
        return Record::Unreadable;
    };
    let Some(status) = ledger.get("status").and_then(Value::as_str) else {
        return Record::Unreadable;
    };
    let goal = ledger
        .get("goal")
        .and_then(Value::as_str)
        .unwrap_or("(no goal recorded)")
        .to_string();
    let updated = ledger
        .get("updated_at")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let stage = if ledger
        .get("goal")
        .and_then(Value::as_str)
        .is_none_or(|goal| goal.trim().is_empty())
        && status == "active"
    {
        Stage::Unknown
    } else {
        match fs::read_to_string(path.join("seed-contract.md")) {
            Ok(seed) => Stage::from_files(status, Some(&seed)),
            Err(error) if error.kind() == NotFound => Stage::from_files(status, None),
            Err(_) => Stage::Unknown,
        }
    };
    match path.canonicalize() {
        Ok(path) => Record::Session(Session {
            path,
            goal,
            stage,
            updated,
        }),
        Err(_) => Record::Unreadable,
    }
}
