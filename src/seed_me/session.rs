//! Reading one saved session straight from Seed Me's files. Read-only: nothing here writes.
//! Files are read directly rather than through the helper, so a scan of thousands of
//! sessions stays fast (see menu.md's measured scope).

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

/// The stage declared by Seed Me's files, not verification of a seed's confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Active, with a confirmed goal and no seed contract yet.
    GoalSaved,
    /// Active, with a seed contract still in draft.
    SeedDrafted,
    /// Active, with a seed declaring confirmation; only the helper can verify it.
    SeedUnchecked,
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
            ("active", Some("status: confirmed for intake")) => Self::SeedUnchecked,
            ("active", Some("status: simulated")) => Self::Simulated,
            _ => Self::Unknown,
        }
    }
    /// The stage in plain words, for a list row.
    pub fn label(self) -> &'static str {
        match self {
            Self::GoalSaved => "goal saved",
            Self::SeedDrafted => "seed drafted",
            Self::SeedUnchecked => "seed needs checking",
            Self::Ended => "ended",
            Self::Unknown => "status unknown",
            Self::Simulated => "practice seed",
        }
    }
    /// Whether the continuation prompt applies: only a live session with a real goal.
    pub fn can_copy(self) -> bool {
        matches!(
            self,
            Self::GoalSaved | Self::SeedDrafted | Self::SeedUnchecked
        )
    }
    /// What Seed Me does next at this stage; none when there is nothing to continue.
    pub fn next(self) -> Option<&'static str> {
        match self {
            Self::GoalSaved => Some("Next: shape it into a seed with Seed Me."),
            Self::SeedDrafted => Some("Next: confirm the seed draft with Seed Me."),
            Self::SeedUnchecked => Some("Next: check the seed with Seed Me."),
            _ => None,
        }
    }
}

/// One readable session.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// The ledger's nonempty, control-free session_id; never inferred from a path or title.
    pub id: Option<String>,
    /// The session folder, canonicalised.
    pub path: PathBuf,
    /// The confirmed goal as recorded, or a plain placeholder when none is.
    pub goal: String,
    /// Its declared stage; seed confirmation has not been verified by this file read.
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
    let id = ledger
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && !id.chars().any(char::is_control))
        .map(str::to_owned);
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
            id,
            path,
            goal,
            stage,
            updated,
        }),
        Err(_) => Record::Unreadable,
    }
}

/// Read Seed Me's existing session identifier only while its saved goal still equals `goal`.
/// Refuses missing identity, an unsupported ledger format or a changed goal; never invents an
/// identifier from the directory name and never changes the ledger.
pub fn session_id(path: &Path, goal: &str) -> Result<String, String> {
    let ledger_path = path.join("ledger.json");
    let text = fs::read_to_string(&ledger_path)
        .map_err(|error| format!("{}: {error}", ledger_path.display()))?;
    let ledger: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{}: {error}", ledger_path.display()))?;
    let refuse = |reason: &str| format!("{}: {reason}", ledger_path.display());
    if ledger.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(refuse("unsupported Seed Me ledger schema"));
    }
    if ledger.get("goal").and_then(Value::as_str) != Some(goal) {
        return Err(refuse("the saved goal changed"));
    }
    ledger
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| refuse("the saved session has no identity"))
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn file_only_seed_status_never_claims_verified_readiness() {
        let root = tempfile::tempdir().unwrap();
        let ledger = root.path().join("ledger.json");
        fs::write(
            &ledger,
            r#"{"schema_version":1,"session_id":"TEST-id","status":"active","goal":"TEST goal"}"#,
        )
        .unwrap();
        let seed = root.path().join("seed-contract.md");
        fs::write(&seed, "status: confirmed for intake\n# TEST changed seed\n").unwrap();
        for confirmation in [None, Some("not JSON"), Some(r#"{"revision":"r1"}"#)] {
            if let Some(text) = confirmation {
                fs::write(root.path().join("seed-confirmation.json"), text).unwrap();
            }
            let before = fs::read(&seed).unwrap();
            let Record::Session(session) = read(root.path()) else {
                panic!("the declared stage must remain readable");
            };
            assert_eq!(session.stage.label(), "seed needs checking");
            assert_eq!(
                session.stage.next(),
                Some("Next: check the seed with Seed Me.")
            );
            assert!(
                session.stage.can_copy(),
                "manual continuation stays available"
            );
            assert_eq!(fs::read(&seed).unwrap(), before);
        }
    }

    #[test]
    fn identity_comes_from_the_same_saved_goal_and_never_from_the_folder_name() {
        let root = tempfile::tempdir().unwrap();
        let session = root.path().join("TEST-folder-name");
        fs::create_dir(&session).unwrap();
        let ledger = session.join("ledger.json");
        let original = serde_json::json!({
            "schema_version": 1,
            "session_id": "TEST-existing-id",
            "goal": "TEST goal",
        });
        let original_bytes = serde_json::to_vec(&original).unwrap();
        fs::write(&ledger, &original_bytes).unwrap();
        assert_eq!(
            session_id(&session, "TEST goal").unwrap(),
            "TEST-existing-id"
        );
        assert_eq!(fs::read(&ledger).unwrap(), original_bytes);
        assert!(
            session_id(&session, "TEST different")
                .unwrap_err()
                .contains("goal changed")
        );
        for replacement in [Value::Null, "".into(), "  ".into(), 5.into()] {
            let mut value = original.clone();
            value["session_id"] = replacement;
            fs::write(&ledger, value.to_string()).unwrap();
            assert!(
                session_id(&session, "TEST goal")
                    .unwrap_err()
                    .contains("no identity")
            );
        }
        let mut value = original;
        value["schema_version"] = 2.into();
        fs::write(&ledger, value.to_string()).unwrap();
        assert!(
            session_id(&session, "TEST goal")
                .unwrap_err()
                .contains("unsupported")
        );
    }
}
