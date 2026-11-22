//! Session persistence and ledger rules: preserve goals and originals, refuse stale writes.
//! Shared by every phase; no skill, helper process, viewer or model policy.

pub mod answering;
pub(crate) mod connection;
pub(crate) mod connection_save;
pub mod ledger;
pub mod migration;
pub mod ready;
pub(crate) mod retry;
mod save;
mod session;
pub mod shaping;
#[cfg(test)]
mod tests;

pub use save::{Affirmed, Choice, Config, ExitReceipt, Original, Receipt};
pub use session::{MAX_READ_FAILURES, ReadFailure, Record, Session, Stage, read, session_id};

use std::path::{Path, PathBuf};

/// Capture an explicit destination relative to the launch directory, or the checked HOME default.
/// Returns an absolute path without creating it; helpers must receive this same destination.
pub fn resolve_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    match explicit {
        Some(root) if root.as_os_str().is_empty() => {
            Err("--seed-session-root needs a location outside a repository".into())
        }
        Some(root) if root.is_absolute() => {
            migration::refuse_old_root(&root)?;
            Ok(root)
        }
        Some(root) => {
            let root = std::env::current_dir()
                .map_err(|_| "Cannot resolve --seed-session-root PATH. Check the launch folder.")?
                .join(root);
            migration::refuse_old_root(&root)?;
            Ok(root)
        }
        None => crate::data_roots::DataRoots::from_env().sessions,
    }
}

/// Read only an opened regular seed file; symlinks keep their existing target semantics.
/// Nonblocking open prevents a replaced path or FIFO from waiting for a writer.
pub(crate) fn read_seed(path: &Path) -> std::io::Result<Vec<u8>> {
    read_regular(path, "original Tink Contract")
}

fn read_regular(path: &Path, name: &str) -> std::io::Result<Vec<u8>> {
    crate::regular_file::read(path, &format!("The {name} is not a regular file."))
}

/// Read regular seed bytes as UTF-8, refusing invalid text without changing the bytes.
pub(crate) fn read_seed_text(path: &Path) -> std::io::Result<String> {
    read_regular_text(path, "original Tink Contract")
}

fn read_regular_text(path: &Path, name: &str) -> std::io::Result<String> {
    crate::regular_file::read_text(path, &format!("The {name} is not a regular file."))
}
