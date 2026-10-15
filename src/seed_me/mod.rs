//! Seed Me: the one module that knows its files, its helper and its prompt.
//!
//! Think saves a confirmed goal through [`Config::confirm`]; tend reads sessions through
//! [`read`] and copies [`continuation_prompt`]. Neither knows a Seed Me file name, helper
//! command or ledger field, so a change to Seed Me's format is a change to this module only.

mod save;
mod session;
#[cfg(test)]
mod tests;

pub use save::{Affirmed, Choice, Config, Original, Receipt};
pub use session::{Record, Session, Stage, read, session_id};

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// The default session root under an absolute HOME; creates nothing and refuses missing,
/// empty or relative HOME with the explicit-root remedy. Explicit roots remain caller choices.
pub fn default_root(home: Option<&OsStr>) -> Result<PathBuf, String> {
    match home.map(Path::new) {
        Some(home) if home.is_absolute() => Ok(home.join(".local/share/seed-me/sessions")),
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
        Some(root) if root.is_absolute() => Ok(root),
        Some(root) => std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .map_err(|error| format!("Cannot resolve --seed-session-root PATH: {error}")),
        None => default_root(std::env::var_os("HOME").as_deref()),
    }
}

/// The prompt an operator pastes into an agent to continue `session` with the skill at `skill`.
/// It asks to read before changing anything and to stop short of seed confirmation and
/// implementation; it never asks to replace the confirmed goal. The same bytes for think's
/// receipt and tend's detail.
pub fn continuation_prompt(skill: &Path, session: &Path) -> String {
    format!(
        "Use the seed-me skill at \"{}\" to continue the existing Seed Me session at \"{}\". I want to continue this session and shape its confirmed goal into a seed contract.\n\nRead the session with the skill's scripts/session.py read and status commands before changing anything. Do not initialize a new session or replace the confirmed goal. Preserve recorded decisions, constraints, exclusions and unresolved context. Tinkery's answered and skipped questions are context, not additional settled Seed Me decisions. If the session is not active, stop and report its status.\n\nFollow the skill's interview and viewer lifecycle. Goal confirmation is not seed confirmation or permission to implement. Save the draft as this session's seed-contract.md; ask me to confirm the displayed revision before using session.py seed confirm. Do not start implementation.",
        skill.display(),
        session.display()
    )
}
