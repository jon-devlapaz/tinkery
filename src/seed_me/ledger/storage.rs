//! Session publication: one cooperating writer, atomic private ledgers and read-back save outcomes.
//! The browser snapshot is not part of a ledger publication.

use super::{Value, load, transition};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::fd::AsRawFd,
    path::Path,
};

/// What a refused publication established about the durable ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    /// Checks refused or read-back found the unchanged ledger.
    NotSaved,
    /// Read-back could not establish either the old or proposed ledger.
    Unknown,
}

/// A publication refusal, including whether retrying requires reconciliation first.
#[derive(Debug)]
pub struct PublishError {
    /// The helper-compatible first reason line, except platform IO diagnostics.
    pub reason: String,
    /// The established durable outcome; an unreadable ledger never counts as saved.
    pub save_state: SaveState,
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}
impl std::error::Error for PublishError {}
impl From<String> for PublishError {
    fn from(reason: String) -> Self {
        Self {
            reason,
            save_state: SaveState::NotSaved,
        }
    }
}

struct Writer(File);
impl Writer {
    fn acquire(directory: &Path) -> Result<Self, PublishError> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(directory.join(".writer.lock"))
            .map_err(|e| e.to_string())?;
        // SAFETY: the descriptor remains owned by `file` through this call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return Err(if error.kind() == io::ErrorKind::WouldBlock {
                "session already has a writer".to_owned()
            } else {
                error.to_string()
            }
            .into());
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
    let result = transition(
        &ledger,
        state,
        expected_version,
        reason,
        revalidated,
        timestamp,
    )?;
    if result != ledger && result["status"].string() == Some("completed") {
        return Err("publish cannot complete a session: use `end --status completed`, which checks the confirmed seed".to_owned().into());
    }
    // JSON refusals precede the IO attempt, as in the helper's atomic_write.
    let bytes = if result != ledger {
        result.pretty()?
    } else {
        String::new()
    };
    commit(
        directory,
        &ledger,
        result,
        |directory, _| atomic_write(directory, &bytes),
        load,
    )
}

fn atomic_write(directory: &Path, bytes: &str) -> io::Result<()> {
    let mut temporary = tempfile::Builder::new()
        .prefix(".ledger-")
        .suffix(".tmp")
        .tempfile_in(directory)?;
    temporary.write_all(bytes.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(directory.join("ledger.json"))
        .map_err(|e| e.error)?;
    Ok(())
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
        reason: format!("could not save the ledger: {cause}"),
        save_state: if unchanged {
            SaveState::NotSaved
        } else {
            SaveState::Unknown
        },
    })
}

#[cfg(test)]
mod tests;
