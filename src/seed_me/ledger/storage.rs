//! Session publication: one cooperating writer, atomic private ledgers and read-back save outcomes.
//! The browser snapshot is not part of a ledger publication.

use super::{Value, load, transition};
use crate::confirmation::Actor;
use crate::seed_me::files;
use std::{
    fs::{File, OpenOptions},
    io,
    os::fd::AsRawFd,
    path::Path,
};

/// Create a private session with a unique folder identity and an empty validated ledger.
/// Never replaces a session; no browser, provider or external helper is involved.
pub fn initialize(root: &Path, timestamp: &str) -> Result<std::path::PathBuf, String> {
    initialize_for(root, timestamp, Actor::OPERATOR)
}

/// Create a session for the same authorized actor that will publish its goal and answers.
/// Refuse retired storage before creating any folder or ledger.
pub(crate) fn initialize_for(
    root: &Path,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<std::path::PathBuf, String> {
    crate::seed_me::migration::refuse_old_root(root)?;
    std::fs::create_dir_all(root)
        .map_err(|_| "Cannot create the sessions folder. Check its permissions.")?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(root)
        .map_err(|_| "Cannot create a private session. Check the sessions folder.")?;
    let path = directory
        .path()
        .canonicalize()
        .map_err(|_| "Cannot locate the new session.")?;
    let id = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("The session has no usable identity.")?;
    initialize_at(&path, id, timestamp, actor)?;
    let _ = directory.keep();
    Ok(path)
}

/// Initialize an owned private staging directory with its fixed future session identity.
/// The caller creates and owns the directory; this never selects another destination.
pub(crate) fn initialize_at(
    path: &Path,
    id: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<(), String> {
    if path.join(files::LEDGER).exists() {
        return Err("The staging ledger already exists.".into());
    }
    let value = Value::from(serde_json::json!({
        "schema_version":1,"session_id":id,"version":0,"revision":0,
        "created_at":timestamp,"updated_at":timestamp,"status":"active",
        "draft":{"goal":"","outcome":"","options":[]},"goal":null,"origin":null,
        "current_question":null,"frontier":[],"nodes":[],"assumed":[],"operator":actor.session_operator()
    }));
    atomic_write(path, &value.pretty()?)
        .map_err(|_| "Cannot save the new session. Check its folder permissions.".into())
}

/// What a refused publication established about the durable ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    /// Checks refused or read-back found the unchanged ledger.
    NotSaved,
    /// Read-back could not establish either the old or proposed ledger.
    Unknown,
}

/// Why a publication was refused. Callers choose their words from the kind, never from text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishKind {
    /// The displayed version is no longer the saved one.
    Stale,
    /// Another writer holds the session.
    Busy,
    /// A write was attempted, but read-back did not find the proposed ledger; holds the cause.
    Unverified(String),
    /// Any other refusal, in the words to show.
    Refused(String),
}

/// A publication refusal, including whether retrying requires reconciliation first.
#[derive(Debug)]
pub struct PublishError {
    /// Why the publication was refused.
    pub kind: PublishKind,
    /// The established durable outcome; an unreadable ledger never counts as saved.
    pub save_state: SaveState,
}

impl PublishError {
    /// A refusal in the caller's own words, with the durable outcome it established.
    pub(crate) fn said(reason: impl Into<String>, save_state: SaveState) -> Self {
        Self {
            kind: PublishKind::Refused(reason.into()),
            save_state,
        }
    }
    /// Only an explicit version refusal is a conflict. Diagnostic details may quote user text.
    pub(crate) fn stale(&self) -> bool {
        self.save_state == SaveState::NotSaved && self.kind == PublishKind::Stale
    }
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            PublishKind::Stale => f.write_str(super::STALE_VERSION),
            PublishKind::Busy => f.write_str("session already has a writer"),
            PublishKind::Unverified(cause) => write!(f, "could not save the ledger: {cause}"),
            PublishKind::Refused(reason) => f.write_str(reason),
        }
    }
}
impl std::error::Error for PublishError {}
/// Ledger rules refuse in words; the stale-version refusal is the one rule that has a kind.
impl From<String> for PublishError {
    fn from(reason: String) -> Self {
        let kind = if reason == super::STALE_VERSION {
            PublishKind::Stale
        } else {
            PublishKind::Refused(reason)
        };
        Self {
            kind,
            save_state: SaveState::NotSaved,
        }
    }
}

/// Read a complete publication without creating a lock in an old or newly initialized session.
pub(in crate::seed_me) fn read_consistent<T>(
    directory: &Path,
    read: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let path = directory.join(".writer.lock");
    let busy = "The session is being updated. Reopen it to read the completed change.";
    let lock = match crate::regular_file::open(
        OpenOptions::new().read(true),
        0,
        &path,
        "the session's writer lock is not a regular file",
    ) {
        Ok(file) => {
            // The open descriptor holds the shared lock until the captured read finishes.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } != 0 {
                let error = io::Error::last_os_error();
                return Err(if error.kind() == io::ErrorKind::WouldBlock {
                    busy.into()
                } else {
                    error.to_string()
                });
            }
            Some(file)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    let result = read()?;
    // The first writer creates its lock before it can publish anything and keeps the lock file.
    if lock.is_none() && path.try_exists().map_err(|e| e.to_string())? {
        return Err(busy.into());
    }
    Ok(result)
}

struct Writer(File);
impl Writer {
    fn acquire(directory: &Path) -> Result<Self, PublishError> {
        let file = crate::regular_file::open(
            OpenOptions::new().append(true).create(true),
            0,
            &directory.join(".writer.lock"),
            "the session's writer lock is not a regular file",
        )
        .map_err(|e| e.to_string())?;
        // SAFETY: the descriptor remains owned by `file` through this call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return Err(if error.kind() == io::ErrorKind::WouldBlock {
                PublishError {
                    kind: PublishKind::Busy,
                    save_state: SaveState::NotSaved,
                }
            } else {
                error.to_string().into()
            });
        }
        Ok(Self(file))
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        // SAFETY: `self` still owns this descriptor. Closing it also releases the lock.
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

/// Publish from a displayed version with the supplied UTC timestamp, refusing completion here.
/// Hold the shared writer lock through validation and read-back; never refresh the browser view.
pub fn publish(
    directory: &Path,
    state: &Value,
    expected_version: &Value,
    reason: &Value,
    revalidated: Option<&Value>,
    timestamp: &str,
) -> Result<Value, PublishError> {
    let _writer = Writer::acquire(directory)?;
    let ledger = load(directory)?;
    publication(
        directory,
        &ledger,
        state,
        expected_version,
        reason,
        revalidated,
        timestamp,
    )
}

/// Revalidate one node with the operator's reason, leaving its answer unchanged. Unlike an
/// ordinary matching publication, a stale review is refused even when the editable state matches.
/// Holds the writer lock while checking the displayed version and publishing the review.
pub fn revalidate(
    directory: &Path,
    expected: u64,
    id: &str,
    reason: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<(), PublishError> {
    let _writer = Writer::acquire(directory)?;
    let ledger = load(directory)?;
    let expected = super::value::int(expected);
    super::require(ledger["version"] == expected, super::STALE_VERSION)?;
    let state = super::editable(&ledger)?;
    super::validate_revalidation(&super::value::object(&[(id, reason.into())]))?;
    let review_reason = actor.context().map_or_else(
        || reason.to_owned(),
        |_| actor.reason(&format!("kept this answer with reason:\n{reason}")),
    );
    let revalidated = super::value::object(&[(id, review_reason.as_str().into())]);
    publication(
        directory,
        &ledger,
        &state,
        &expected,
        &actor
            .reason("kept a flagged answer with a reason.")
            .as_str()
            .into(),
        Some(&revalidated),
        timestamp,
    )
    .map(|_| ())
}

/// Publish an explicit operator answer action from the displayed version. The ledger alone
/// owns eligibility, required fields and reopening; stale versions never mutate anything.
pub fn respond(
    directory: &Path,
    expected: u64,
    id: &str,
    action: &super::AnswerAction,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<Value, PublishError> {
    let _writer = Writer::acquire(directory).map_err(|error| {
        if error.kind == PublishKind::Busy {
            error
        } else {
            "The session cannot be locked for saving. Check its folder permissions, then try again."
                .to_owned()
                .into()
        }
    })?;
    let ledger = load(directory).map_err(|_| {
        "The session could not be read. Reopen it and check its saved ledger before trying again.".to_owned()
    })?;
    let expected = super::value::int(expected);
    super::require(ledger["version"] == expected, super::STALE_VERSION)?;
    if matches!(action, super::AnswerAction::Revisit(_)) {
        super::require(
            !directory.join(files::SEED).exists(),
            "A Tink Contract already exists. Reopen Shape before answering a deferred prerequisite.",
        )?;
    }
    let (mut state, mut reviewed) = super::answers::change(&ledger, id, action)?;
    if let Some(context) = actor.context()
        && let Some(node) = state["nodes"]
            .list_mut()
            .iter_mut()
            .find(|n| n["id"].is(id))
        && node["status"].is("settled")
    {
        node.set("authority", "delegated".into());
        node.set(
            "authority_source",
            actor
                .reason("recorded this answer action in Tinkery Shape.")
                .as_str()
                .into(),
        );
        node["evidence"].list_mut().push(super::value::object(&[
            (
                "checked",
                "Caller-declared authorization, not authenticated identity".into(),
            ),
            ("at", timestamp.into()),
            ("observed", context.details().as_str().into()),
        ]));
    }
    let verb = match action {
        super::AnswerAction::Defer { .. } => "deferred",
        super::AnswerAction::Delegate => "delegated",
        super::AnswerAction::Revise(_) => "revised",
        super::AnswerAction::Revisit(_) => "answered deferred prerequisite",
    };
    let reason: Value = actor
        .reason(&format!("{verb} {id} in Tinkery Shape."))
        .as_str()
        .into();
    if reviewed.get(id).is_some() {
        // A changed answer can itself retire a direct contradiction flag. Only revalidate
        // flags the ordinary transition still holds, using that same rule owner.
        let proposed = transition(&ledger, &state, &expected, &reason, None, timestamp)?;
        if super::review_flags(&proposed["nodes"]).get(id).is_none() {
            reviewed = super::value::object(&[]);
        }
    }
    publication(
        directory,
        &ledger,
        &state,
        &expected,
        &reason,
        Some(&reviewed),
        timestamp,
    )
    .map_err(|error| match error.kind {
        PublishKind::Unverified(_) => PublishError::said(
            "The ledger save could not be verified. Check the session folder before trying again.",
            error.save_state,
        ),
        _ => error,
    })
}

fn publication(
    directory: &Path,
    ledger: &Value,
    state: &Value,
    expected_version: &Value,
    reason: &Value,
    revalidated: Option<&Value>,
    timestamp: &str,
) -> Result<Value, PublishError> {
    let result = transition(
        ledger,
        state,
        expected_version,
        reason,
        revalidated,
        timestamp,
    )?;
    if result != *ledger && result["status"].string() == Some("completed") {
        return Err("publish cannot complete a session: use `end --status completed`, which checks the confirmed Tink".to_owned().into());
    }
    // JSON refusals precede the IO attempt, as in the former helper's atomic_write.
    let bytes = if result != *ledger {
        result.pretty()?
    } else {
        String::new()
    };
    commit(
        directory,
        ledger,
        result,
        |directory, _| atomic_write(directory, &bytes),
        load,
    )
}

fn atomic_write(directory: &Path, bytes: &str) -> io::Result<()> {
    atomic_text(&directory.join(files::LEDGER), bytes, None)
}

fn atomic_text(path: &Path, bytes: &str, mode: Option<std::fs::Permissions>) -> io::Result<()> {
    use crate::regular_file::Existing;
    let existing = mode.map_or(Existing::Replace, Existing::ReplaceWith);
    // A file in place is saved; session files never reported directory durability.
    crate::regular_file::publish(path, bytes.as_bytes(), existing).map(|_| ())
}

fn commit(
    directory: &Path,
    ledger: &Value,
    result: Value,
    write: impl FnOnce(&Path, &Value) -> io::Result<()>,
    read: impl FnOnce(&Path) -> Result<Value, String>,
) -> Result<Value, PublishError> {
    if result == *ledger {
        return Ok(result);
    }
    let written = write(directory, &result);
    let current = read(directory);
    if current.as_ref().is_ok_and(|v| *v == result) {
        return Ok(result);
    }
    let unchanged = current.as_ref().is_ok_and(|v| v == ledger);
    let cause = match written {
        Err(error) => error.to_string(),
        Ok(()) => current
            .err()
            .unwrap_or_else(|| "ledger read-back did not match the publication".into()),
    };
    Err(PublishError {
        kind: PublishKind::Unverified(cause),
        save_state: if unchanged {
            SaveState::NotSaved
        } else {
            SaveState::Unknown
        },
    })
}

/// Save an unconfirmed host draft and its records under one writer lock, rechecking the displayed
/// ledger and prior seed. IO failures can leave records saved, but never grant confirmation.
pub(crate) fn save_draft(
    directory: &Path,
    expected: u64,
    previous: Option<&str>,
    records: &Value,
    seed: &str,
) -> Result<(), String> {
    super::checks::validate_checks(records)?;
    super::require(
        files::status(seed) == Some(files::Status::Draft),
        "a Tink Contract draft must start with status: draft",
    )?;
    let _writer = Writer::acquire(directory).map_err(|e| e.to_string())?;
    let ledger = load(directory)?;
    super::require(
        ledger["version"] == super::value::int(expected),
        super::STALE_VERSION,
    )?;
    super::require(ledger["status"].is("active"), "ended session is read-only")?;
    let path = directory.join(files::SEED);
    let current =
        match crate::regular_file::read_text(&path, "the Tink Contract is not a regular file") {
            Ok(text) => Some(text),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
    super::require(
        current
            .as_ref()
            .is_none_or(|s| files::status(s) == Some(files::Status::Draft)),
        "a Tink Contract that is not a draft already exists; never replace it",
    )?;
    super::require(
        current.as_deref() == previous,
        "the Tink Contract changed while the draft was prepared; reopen this session before trying again",
    )?;
    atomic_text(&directory.join(files::CHECKS), &records.pretty()?, None)
        .map_err(|e| format!("could not save check records: {e}"))?;
    atomic_text(&path, seed, None)
        .map_err(|e| format!("check records saved, but the Tink Contract draft was not saved: {e}"))
}

/// Record a drafting review or teach-back against the displayed ledger and exact draft bytes.
/// A stale draft refuses without saving; model text never becomes a confirmation gate.
pub fn record_teach_back(
    directory: &Path,
    expected: u64,
    revision: &str,
    seed: &str,
    record: &super::TeachBackRecord<'_>,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<u64, PublishError> {
    let _writer = Writer::acquire(directory).map_err(|_| {
        "The session cannot be locked. Reopen it before recording this review again.".to_owned()
    })?;
    let ledger = load(directory).map_err(|_| {
        "The session could not be read. Reopen it before recording this review again.".to_owned()
    })?;
    super::require(
        ledger["version"] == super::value::int(expected),
        super::STALE_VERSION,
    )?;
    let current =
        crate::regular_file::read_text(&directory.join(files::SEED), "not a regular file")
            .map_err(|_| {
                "The draft could not be read. Reopen Shape before recording this review again."
                    .to_owned()
            })?;
    super::require(
        current == seed,
        "The draft changed. Reopen Shape before recording this review again.",
    )?;
    let state = super::teach_back::append(&ledger, revision, seed, record, timestamp)?;
    let saved = publication(
        directory,
        &ledger,
        &state,
        &ledger["version"],
        &actor
            .reason("requested a draft review or teach-back in Tinkery Shape.")
            .as_str()
            .into(),
        None,
        timestamp,
    )
    .map_err(|error| match error.kind {
        PublishKind::Unverified(_) => PublishError::said(
            "The review save could not be verified. Reopen the session before trying again.",
            error.save_state,
        ),
        _ => error,
    })?;
    saved["version"].counter().ok_or_else(|| {
        PublishError::said(
            "The review may have saved. Reopen the session before trying again.",
            SaveState::Unknown,
        )
    })
}

/// Store strict records while holding the session writer lock; never execute a check.
pub fn set_checks(directory: &Path, records: &Value) -> Result<Value, String> {
    super::checks::validate_checks(records)?;
    let _writer = Writer::acquire(directory).map_err(|e| e.to_string())?;
    super::require(
        load(directory)?["status"].is("active"),
        "ended session is read-only",
    )?;
    let path = directory.join(files::CHECKS);
    atomic_text(&path, &records.pretty()?, None).map_err(|e| e.to_string())?;
    Ok(super::value::object(&[
        ("checks", path.to_string_lossy().as_ref().into()),
        ("count", super::value::int(records.items().len() as u64)),
        ("sha256", super::checks::digest(records)?.as_str().into()),
    ]))
}

/// Repair the selected check in the exact displayed draft, without a model, for a resolved
/// `_actor`; the repair records no attribution. Preserve the statement and typed words; stale
/// inputs and non-drafts refuse before writes.
pub(crate) fn repair_check(
    directory: &Path,
    expected: u64,
    seed: &str,
    edit: (&super::checks::DraftCheck, super::checks::CheckMethod, &str),
    _actor: Actor<'_>,
) -> Result<(), String> {
    let (target, method, words) = edit;
    super::require(
        !words.trim().is_empty() && !words.contains('\0'),
        "Type the check command or how-to-judge words first.",
    )?;
    let _writer = Writer::acquire(directory).map_err(|e| e.to_string())?;
    let ledger = load(directory)?;
    super::require(
        ledger["version"] == super::value::int(expected),
        super::STALE_VERSION,
    )?;
    super::require(ledger["status"].is("active"), "ended session is read-only")?;
    let path = directory.join(files::SEED);
    super::require(
        files::read_seed_text(&path).map_err(|e| e.to_string())? == seed,
        "The draft changed. Read it again before changing a check.",
    )?;
    super::require(
        files::status(seed) == Some(files::Status::Draft),
        "Only a draft check can change.",
    )?;
    let records = super::checks::require_seed_checks(directory, &ledger, false)?;
    let mut items = records.items().to_vec();
    let record = items
        .get_mut(target.index)
        .ok_or("The selected check is no longer in the draft.")?;
    target.edit(record, method, words)?;
    let repaired = Value::Array(items);
    super::checks::validate_goal_checks(ledger["goal"].string().unwrap_or(""), &repaired)?;
    let old = super::checks::checks_block(&records)?;
    let new = super::checks::checks_block(&repaired)?;
    super::require(
        seed.matches(&old).count() == 1,
        "The draft acceptance section cannot be replaced safely.",
    )?;
    let draft = seed.replacen(&old, &new, 1);
    atomic_text(&directory.join(files::CHECKS), &repaired.pretty()?, None)
        .map_err(|e| e.to_string())?;
    atomic_text(&path, &draft, None)
        .map_err(|e| format!("Check records saved, but the draft was not saved: {e}"))
}

/// Confirm only a displayed draft revision, after all refusals, under the shared writer lock.
/// The receipt is written first, so an interrupted seed replacement cannot grant handoff.
pub fn confirm_seed(
    directory: &Path,
    revision: &str,
    source: &str,
    timestamp: &str,
) -> Result<Value, String> {
    confirm(
        directory,
        revision,
        source,
        timestamp,
        false,
        Actor::OPERATOR,
        None,
    )
}

/// Confirm for `actor` after recording a missing teach-back as skipped; permission is checked
/// before any write. A completed comparison, including mismatches, never blocks confirmation, and
/// human confirmations keep their existing receipt format.
pub fn confirm_as(
    directory: &Path,
    revision: &str,
    source: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<Value, String> {
    confirm(directory, revision, source, timestamp, true, actor, None)
}

/// Confirm the exact displayed draft and ledger version under the session writer lock.
/// Stale inputs refuse before recording a teach-back skip or confirmation.
pub fn confirm_displayed_as(
    directory: &Path,
    expected: u64,
    revision: &str,
    seed: &str,
    source: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<Value, String> {
    confirm(
        directory,
        revision,
        source,
        timestamp,
        true,
        actor,
        Some((expected, seed)),
    )
}

fn confirm(
    directory: &Path,
    revision: &str,
    source: &str,
    timestamp: &str,
    record_skip: bool,
    actor: Actor<'_>,
    displayed: Option<(u64, &str)>,
) -> Result<Value, String> {
    super::require(
        super::text::text(&revision.into()),
        "--revision is required: the displayed label",
    )?;
    super::require(
        super::text::text(&source.into()),
        "--source is required: quote the words that confirmed this revision",
    )?;
    super::require(!source.contains(['\n', '\r']), "--source must be one line")?;
    let _writer = Writer::acquire(directory).map_err(|e| e.to_string())?;
    let ledger = load(directory)?;
    if let Some((expected, seed)) = displayed {
        super::require(
            ledger["version"] == super::value::int(expected),
            super::STALE_VERSION,
        )?;
        let current =
            crate::regular_file::read_text(&directory.join(files::SEED), "not a regular file")
                .map_err(|_| {
                    "The draft could not be read. Reopen Shape before confirming.".to_owned()
                })?;
        super::require(
            current == seed,
            "The draft changed. Read it again before confirming.",
        )?;
    }
    let ledger = if record_skip {
        // Refusals precede the skip record: an invalid confirmation must write nothing.
        super::seed::confirmed_as(directory, &ledger, revision, source, timestamp, actor)?;
        let draft =
            crate::regular_file::read_text(&directory.join(files::SEED), "not a regular file")
                .map_err(|_| {
                    "The draft could not be read. Reopen Shape before confirming.".to_owned()
                })?;
        let state = super::teach_back::skip(&ledger, revision, &draft, timestamp)?;
        publication(
        directory,
        &ledger,
        &state,
        &ledger["version"],
        &actor.reason("confirmed without a completed teach-back.").as_str().into(),
        None,
        timestamp,
    )
    .map_err(|_| {
        "The teach-back skip could not be verified. Reopen this session before confirming again."
            .to_owned()
    })?
    } else {
        ledger
    };
    let (confirmed, record, result) =
        super::seed::confirmed_as(directory, &ledger, revision, source, timestamp, actor)?;
    let path = directory.join(files::SEED);
    atomic_text(
        &directory.join(files::CONFIRMATION),
        &record.pretty()?,
        None,
    )
    .map_err(|e| e.to_string())?;
    let permissions = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .permissions();
    atomic_text(&path, &confirmed, Some(permissions)).map_err(|e| e.to_string())?;
    Ok(result)
}

/// End a session permanently, checking completion against the exact confirmed seed, never a viewer.
/// Uses publication's read-back outcome and lock; a matching repeat is a no-op.
pub fn end(
    directory: &Path,
    status: &str,
    reason: &str,
    timestamp: &str,
) -> Result<Value, PublishError> {
    super::require(
        ["stopped", "completed"].contains(&status),
        "invalid end status",
    )?;
    let _writer = Writer::acquire(directory)?;
    let ledger = load(directory)?;
    super::require(
        ledger["status"].is("active") || ledger["status"].is(status),
        "ended session is read-only",
    )?;
    let mut state = super::editable(&ledger)?;
    state.set("status", status.into());
    state.set("current_question", Value::Null);
    let result = transition(
        &ledger,
        &state,
        &ledger["version"],
        &reason.into(),
        None,
        timestamp,
    )?;
    if status == "completed" {
        let simulated = super::operator(&ledger).is("simulated");
        super::seed::require_seed_line(
            directory,
            if simulated {
                files::SIMULATED
            } else {
                files::CONFIRMED
            },
        )?;
        if !simulated {
            let handoff = super::seed::verify_handoff(directory, &ledger)?;
            super::require(
                handoff.verified,
                format!(
                    "cannot complete: {}; save the current revision as a draft, show it, and confirm it again with `seed confirm`",
                    handoff.reason
                ),
            )?;
        }
        super::seed::require_seed_decisions(directory, &ledger, "cannot complete")?;
        super::checks::require_seed_checks(directory, &ledger, true)?;
    }
    let bytes = result.pretty()?;
    commit(
        directory,
        &ledger,
        result,
        |directory, _| atomic_write(directory, &bytes),
        load,
    )
}

#[cfg(test)]
mod golden_tests;
#[cfg(test)]
mod tests;
