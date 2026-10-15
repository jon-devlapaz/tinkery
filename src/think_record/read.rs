//! Read one saved conversation without changing its files. Attribution requires a complete scan,
//! one current-format record with the exact session path and Seed Me identity, and an unchanged goal.
//! Older records cannot prove that connection; malformed files cannot hide a possible duplicate.

use super::{Event, SCHEMA, session_id};
use crate::seed_me;
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

/// One turn as recorded, with the person's words unchanged.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RecordedTurn {
    /// The recorded UTC time, without estimating time spent working.
    pub at: String,
    /// The recorded words or question action.
    #[serde(flatten)]
    pub event: Event,
}

/// A verified connection to one saved goal; reading supplies no model context and writes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// Every recorded turn, in order and with original words intact.
    pub turns: Vec<RecordedTurn>,
    /// The exact confirmed goal, checked against the current saved session.
    pub goal: String,
    /// Source, model and check information for technical help, kept apart from the conversation.
    pub details: String,
}

/// Why no history can be attributed safely; this does not invalidate the saved goal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unavailable {
    /// A short explanation for the history view.
    pub message: String,
    /// Source paths and the precise reason, for technical help.
    pub details: String,
}
impl Unavailable {
    fn new(message: &str, details: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: details.into(),
        }
    }

    fn unreadable(path: &Path, reason: impl std::fmt::Display) -> Self {
        Self::new(
            "Think history could not be read.",
            format!("{}: {reason}", path.display()),
        )
    }
}

// Limit each file before parsing or formatting its checks. A larger record is unavailable,
// never partially shown. Scanning holds at most one selected record plus one file in memory.
const MAX_RECORD_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct SavedRecord {
    schema: u8,
    tinkery: String,
    origin: String,
    model: Option<String>,
    thinking: Option<String>,
    prompt_hash: Option<String>,
    started: String,
    confirmed: String,
    seed_me_session: String,
    seed_me_session_path: Option<PathBuf>,
    seed_me_session_id: Option<String>,
    turns: Vec<RecordedTurn>,
    checks: Vec<serde_json::Value>,
    goal: String,
}

/// Reads the unique history for `session` and its exact `goal`, without writing or running
/// commands. Scans JSON files under `root`; missing, unreadable, unsupported, conflicting or
/// older matching records refuse attribution. A record over 16 MiB is unreadable. The session
/// must retain the same goal and identity before and after the scan; moved sessions are not guessed.
pub fn read(root: &Path, session: &Path, goal: &str) -> Result<History, Unavailable> {
    let selected = current_session(session, goal)?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(missing(root));
        }
        Err(error) => return Err(Unavailable::unreadable(root, error)),
    };
    let mut found = None;
    for entry in entries {
        let path = entry
            .map_err(|error| Unavailable::unreadable(root, error))?
            .path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let record = read_file(&path)?;
        if record.schema == 1 {
            if record.seed_me_session == session_id(&selected.0) {
                return Err(Unavailable::new(
                    "This older Think history cannot be matched safely.",
                    format!(
                        "{}: schema 1 stores only the session folder name; its root is unknown. No file was changed.",
                        path.display()
                    ),
                ));
            }
            continue;
        }
        let stored = record
            .seed_me_session_path
            .as_ref()
            .ok_or_else(|| Unavailable::unreadable(&path, "the saved session path is missing"))?;
        if !stored.is_absolute()
            || stored
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
            || record.seed_me_session != session_id(stored)
        {
            return Err(Unavailable::unreadable(
                &path,
                "the saved session reference is invalid",
            ));
        }
        // A changed symlink cannot turn a noncanonical reference into a match. Missing unrelated
        // sessions are allowed: their old absolute locator still does not identify this session.
        if let Ok(canonical) = stored.canonicalize()
            && canonical != *stored
        {
            return Err(Unavailable::unreadable(
                &path,
                "the saved session path is no longer canonical",
            ));
        }
        if *stored != selected.0 {
            continue;
        }
        if found.is_some() {
            return Err(Unavailable::new(
                "More than one Think record matches this saved session.",
                format!(
                    "{}: another matching record is {}. No record was chosen.",
                    root.display(),
                    path.display()
                ),
            ));
        }
        found = Some((path, record));
    }
    let (path, record) = found.ok_or_else(|| missing(root))?;
    if record.seed_me_session_id.as_deref() != Some(selected.1.as_str()) {
        return Err(Unavailable::new(
            "Think history belongs to a different saved session.",
            format!(
                "{}: the recorded Seed Me identity differs from the current session at this path.",
                path.display()
            ),
        ));
    }
    if record.goal != goal {
        return Err(Unavailable::new(
            "Think history disagrees with the saved goal.",
            format!(
                "{}: the recorded goal does not exactly match the saved goal.",
                path.display()
            ),
        ));
    }
    if current_session(session, goal)? != selected {
        return Err(Unavailable::new(
            "The saved session changed while history was read.",
            "Open the saved goal again before reading its history.",
        ));
    }
    let details = format!(
        concat!(
            "Think record: {}\nSession: {}\nSession ID: {}\nFormat: {}\nTinkery: {}\n",
            "Origin: {}\nModel: {}\nThinking: {}\nPrompt: {}\nStarted: {}\nConfirmed: {}\n\nChecks:\n{}"
        ),
        path.display(),
        selected.0.display(),
        selected.1,
        record.schema,
        record.tinkery,
        record.origin,
        record.model.as_deref().unwrap_or("not recorded"),
        record.thinking.as_deref().unwrap_or("not recorded"),
        record.prompt_hash.as_deref().unwrap_or("not recorded"),
        record.started,
        record.confirmed,
        serde_json::to_string_pretty(&record.checks).expect("JSON values always serialize"),
    );
    Ok(History {
        turns: record.turns,
        goal: record.goal,
        details,
    })
}

fn missing(root: &Path) -> Unavailable {
    Unavailable::new(
        "No Think history was found for this saved goal.",
        format!(
            "{}: no record identifies this saved session. A saved goal may have no Think record.",
            root.display()
        ),
    )
}

fn current_session(session: &Path, goal: &str) -> Result<(PathBuf, String), Unavailable> {
    match seed_me::read(session) {
        seed_me::Record::Session(current) if current.goal == goal => {
            let id = seed_me::session_id(&current.path, goal).map_err(|reason| {
                Unavailable::new("The saved session identity could not be checked.", reason)
            })?;
            Ok((current.path, id))
        }
        seed_me::Record::Session(_) => Err(Unavailable::new(
            "The saved goal changed. Open it again.",
            format!(
                "{}: the current saved goal differs from the selected goal.",
                session.display()
            ),
        )),
        _ => Err(Unavailable::new(
            "The saved session could not be read.",
            format!(
                "{}: Seed Me could not read the current session.",
                session.display()
            ),
        )),
    }
}

fn read_file(path: &Path) -> Result<SavedRecord, Unavailable> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| Unavailable::unreadable(path, error))?;
    if !metadata.is_file() {
        return Err(Unavailable::unreadable(
            path,
            "expected a regular JSON file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| Unavailable::unreadable(path, error))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(Unavailable::unreadable(
            path,
            "the record exceeds the 16 MiB reading limit",
        ));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| Unavailable::unreadable(path, error))?;
    match value.get("schema").and_then(serde_json::Value::as_u64) {
        Some(1) => {}
        Some(schema) if schema == u64::from(SCHEMA) => {}
        Some(schema) => {
            return Err(Unavailable::new(
                "Think history uses an unsupported format.",
                format!(
                    "{}: schema {schema} is unsupported; it could contain a matching record.",
                    path.display()
                ),
            ));
        }
        None => {
            return Err(Unavailable::unreadable(
                path,
                "the record schema is missing or invalid",
            ));
        }
    }
    let record: SavedRecord =
        serde_json::from_value(value).map_err(|error| Unavailable::unreadable(path, error))?;
    if record.schema == SCHEMA
        && record
            .seed_me_session_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
    {
        return Err(Unavailable::unreadable(
            path,
            "the recorded Seed Me identity is missing",
        ));
    }
    if record.started.is_empty()
        || record.confirmed.is_empty()
        || record.turns.iter().any(|turn| turn.at.is_empty())
    {
        return Err(Unavailable::unreadable(path, "a recorded time is empty"));
    }
    Ok(record)
}

#[cfg(test)]
mod tests;
