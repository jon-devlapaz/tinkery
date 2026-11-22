//! The join between tink-sdlc runs and confirmed seeds. A run supplies a goal, or counts as
//! started, only through its recorded canonical source path and current bytes; names and goal
//! text confer no identity. Reads only; it never writes, launches or approves anything.

use super::ready::{self, ReadySeed};
use crate::sdlc::{self, DEADLINE, Seed, Verification};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::atomic::AtomicBool};

#[cfg(test)]
mod tests;

/// A run's seed evidence distinguishes known absence from an unreadable or unmatched original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Goal {
    /// The seed matches and its confirmation verifies, with the exact saved goal.
    Matched(String),
    /// The run explicitly records that no confirmed seed was used.
    NoSeed,
    /// A missing, malformed or unmatched record cannot supply a goal.
    Unknown,
}

/// True only for the canonical source path and the bytes recorded at intake. Retired session
/// storage refuses.
pub(crate) fn matches(seed: &Seed, path: &Path) -> Result<bool, String> {
    super::migration::refuse_old_root(path)?;
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot locate the original Tink Contract: {e}"))?;
    if seed.source().as_os_str() != path.as_os_str() {
        return Ok(false);
    }
    let bytes = super::read_seed(&path)
        .map_err(|e| format!("Cannot read the original Tink Contract: {e}"))?;
    Ok(seed.sha256() == format!("{:x}", Sha256::digest(bytes)))
}

/// Read the exact goal only for a still-matching, verified seed, on a worker. An explicit null
/// record means no seed; missing, changed or unverified original session files stay unknown.
pub(crate) fn goal(run: &Value) -> Goal {
    confirmed(run).map_or_else(|gap| gap, |ready| Goal::Matched(ready.goal))
}

/// A run may supply the goal and retry base only from its own still-matching confirmed seed.
pub(crate) fn confirmed(run: &Value) -> Result<ReadySeed, Goal> {
    let seed = Seed::parse(run)
        .map_err(|_| Goal::Unknown)?
        .ok_or(Goal::NoSeed)?;
    let matched = || {
        if !matches(&seed, seed.source()).ok()? {
            return None;
        }
        let ready = ready::confirmed(seed.source()).ok()?;
        matches(&seed, seed.source()).ok()?.then_some(ready)
    };
    matched().ok_or(Goal::Unknown)
}

/// Read the named checkout once with the shared deadline and cancellation; missing records or
/// unreadable runs refuse the whole start classification, never conceal any seed.
pub(crate) fn seeds(root: &Path, cancel: &AtomicBool) -> Result<Vec<Seed>, String> {
    if !sdlc::installed(root) {
        return Err("The named checkout has no run reader.".into());
    }
    sdlc::list(root, cancel, DEADLINE)?
        .ok_or("The checkout reported no runs list.")?
        .iter()
        .map(|run| {
            if run.verification.is(Verification::Invalid) {
                return Err(
                    "A run couldn't be read. Check it in tink-sdlc, then reopen this view to read again."
                        .into(),
                );
            }
            Seed::parse(&run.data)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|seeds| seeds.into_iter().flatten().collect())
}

/// Whether a session's current seed was recorded by any run. A changed same-path seed is an
/// error (unknown), not a waiting or started claim. A different path never joins.
pub(crate) fn started(seeds: &[Seed], path: &Path) -> Result<bool, String> {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            if seeds
                .iter()
                .any(|seed| seed.source().as_os_str() == path.as_os_str())
            {
                return Err(format!("Cannot locate the Tink Contract: {error}"));
            }
            return Ok(false);
        }
    };
    let mut same_source = false;
    for seed in seeds
        .iter()
        .filter(|seed| seed.source().as_os_str() == path.as_os_str())
    {
        same_source = true;
        if matches(seed, &path)? {
            return Ok(true);
        }
    }
    if same_source {
        Err("The Tink Contract differs from the run. Check it in Shape.".into())
    } else {
        Ok(false)
    }
}
