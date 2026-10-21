//! Session persistence and ledger rules: preserve goals and originals, refuse stale writes.
//! Shared by every phase; no skill, helper process, viewer or model policy.

pub mod answering;
pub mod ledger;
pub mod migration;
pub mod ready;
mod save;
mod session;
pub mod shaping;
#[cfg(test)]
mod tests;

pub use save::{Affirmed, Choice, Config, Original, Receipt};
pub use session::{MAX_READ_FAILURES, ReadFailure, Record, Session, Stage, read, session_id};

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// The default session root under an absolute HOME; creates nothing and refuses missing,
/// empty or relative HOME with the explicit-root remedy. Explicit roots remain caller choices.
pub fn default_root(home: Option<&OsStr>) -> Result<PathBuf, String> {
    match home.map(Path::new) {
        Some(home) if home.is_absolute() => Ok(home.join(".local/share/tinkery/sessions")),
        _ => {
            Err("HOME is unset or not absolute; pass --seed-session-root PATH for sessions.".into())
        }
    }
}

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
        None => default_root(std::env::var_os("HOME").as_deref()),
    }
}
