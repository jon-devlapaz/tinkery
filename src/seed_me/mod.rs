//! Seed Me: the one module that knows its files, its helper and its prompt.
//!
//! Think saves a confirmed goal through [`Config::confirm`]; shape reads sessions through
//! [`read`] and copies [`continuation_prompt`]. Neither knows a Seed Me file name, helper
//! command or ledger field, so a change to Seed Me's format is a change to this module only.

mod save;
mod session;
#[cfg(test)]
mod tests;

pub use save::{Affirmed, Choice, Config, Original, Receipt};
pub use session::{Record, Session, Stage, read};

use std::path::{Path, PathBuf};

/// Where Seed Me keeps sessions for a user whose home is `home`. Only a default: the
/// operator may point Tinkery elsewhere, and nothing here creates the folder.
pub fn default_root(home: &Path) -> PathBuf {
    home.join(".local/share/seed-me/sessions")
}

/// The prompt an operator pastes into an agent to continue `session` with the skill at `skill`.
/// It asks to read before changing anything and to stop short of seed confirmation and
/// implementation; it never asks to replace the confirmed goal. The same bytes for think's
/// receipt and shape's detail.
pub fn continuation_prompt(skill: &Path, session: &Path) -> String {
    format!(
        "Use the seed-me skill at \"{}\" to continue the existing Seed Me session at \"{}\". I want to continue this session and shape its confirmed goal into a seed contract.\n\nRead the session with the skill's scripts/session.py read and status commands before changing anything. Do not initialize a new session or replace the confirmed goal. Preserve recorded decisions, constraints, exclusions and unresolved context. Tinkery's answered and skipped questions are context, not additional settled Seed Me decisions. If the session is not active, stop and report its status.\n\nFollow the skill's interview and viewer lifecycle. Goal confirmation is not seed confirmation or permission to implement. Save the draft as this session's seed-contract.md; ask me to confirm the displayed revision before using session.py seed confirm. Do not start implementation.",
        skill.display(),
        session.display()
    )
}
