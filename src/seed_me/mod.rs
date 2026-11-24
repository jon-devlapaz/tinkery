//! Session persistence and ledger rules: preserve goals and originals, refuse stale writes.
//! Shared by every phase; no skill, helper process, viewer or model policy.

pub(crate) mod connection;
pub(crate) mod connection_save;
pub(crate) mod files;
pub mod ledger;
pub mod migration;
pub mod ready;
pub(crate) mod retry;
pub(crate) mod runs;
mod save;
mod session;
#[cfg(test)]
mod tests;

pub use save::{Affirmed, Choice, Config, ExitReceipt, Original, Receipt};
pub(crate) use session::valid_id;
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

fn read_regular(path: &Path, name: &str) -> std::io::Result<Vec<u8>> {
    crate::regular_file::read(path, &format!("The {name} is not a regular file."))
}

fn read_regular_text(path: &Path, name: &str) -> std::io::Result<String> {
    crate::regular_file::read_text(path, &format!("The {name} is not a regular file."))
}
