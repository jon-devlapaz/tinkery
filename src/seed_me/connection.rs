//! Checked creation evidence for a goal discovered in Tend. No history is invented,
//! and this evidence grants neither seed confirmation nor permission to execute work.

use super::ledger;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Evidence tag for the supported Tend creation format.
pub(crate) const ARTIFACT: &str = "tinkery:origin/tend-v1";

/// The exact goal input used for discovery; unrelated ledger revisions are excluded.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Source {
    pub(crate) id: String,
    pub(crate) goal: String,
    pub(crate) repo: PathBuf,
    pub(crate) status: String,
}

/// Reviewed interpretation and verbatim supporting words, distinct from the new goal.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Support {
    pub(crate) id: String,
    pub(crate) quote: String,
}

/// Versioned creation evidence survives later attempts; it contains no save ownership.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Origin {
    pub(crate) schema: u8,
    pub(crate) entry: String,
    pub(crate) repo: PathBuf,
    pub(crate) goal: String,
    pub(crate) sources: Vec<Source>,
    pub(crate) support: Vec<Support>,
    pub(crate) interpretation: String,
}

impl Origin {
    /// Validate the full evidence once, including exact supporting quotations.
    pub(crate) fn validate(&self, goal: &str) -> Result<(), String> {
        let ids: std::collections::HashSet<_> = self.sources.iter().map(|s| &s.id).collect();
        let supporting: std::collections::HashSet<_> = self.support.iter().map(|s| &s.id).collect();
        if self.schema != 1
            || self.entry != "tend"
            || self.goal != goal
            || goal.trim().is_empty()
            || !self.repo.is_absolute()
            || self.sources.len() < 2
            || self.sources.len() > 20
            || ids.len() != self.sources.len()
            || supporting.len() < 2
            || self.interpretation.trim().is_empty()
            || self.sources.iter().any(|s| {
                s.id.is_empty()
                    || s.goal.trim().is_empty()
                    || s.repo != self.repo
                    || s.status != "active"
            })
            || self.support.iter().any(|s| {
                s.quote.trim().is_empty()
                    || !self
                        .sources
                        .iter()
                        .any(|input| input.id == s.id && input.goal.contains(&s.quote))
            })
        {
            return Err("The Tend origin is invalid or conflicts with the saved goal.".into());
        }
        Ok(())
    }
}

/// Missing legacy evidence is distinct from an invalid Tend record. Reads no Think history.
pub(crate) fn read(session: &Path, goal: &str) -> Result<Option<Origin>, String> {
    // Legacy discovery accepts older ledgers that do not have the current full write schema.
    // Only a marked Tend origin enters the stricter new validation boundary.
    let bytes = match super::read_regular(&session.join("ledger.json"), "goal ledger") {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None), // The existing session reader owns missing/unreadable ledger errors.
    };
    let raw: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(raw) => raw,
        Err(_)
            if !bytes
                .windows(b"tinkery:origin/tend".len())
                .any(|w| w == b"tinkery:origin/tend") =>
        {
            return Ok(None);
        }
        Err(_) => return Err("The Tend ledger is malformed.".into()),
    };
    // A removed artifact must not turn a known Tend confirmation into legacy unknown.
    let known_tend = std::fs::symlink_metadata(session.join("tend-handoff.json")).is_ok()
        || raw["nodes"].as_array().into_iter().flatten().any(|node| {
            node["authority_source"] == "Tend: explicit save of displayed connection goal"
        });
    let marked = raw["nodes"].as_array().into_iter().flatten().any(|node| {
        node["evidence"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|entry| {
                entry["artifact"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("tinkery:origin/tend"))
            })
    });
    if !marked && !known_tend {
        return Ok(None);
    }
    let value = ledger::load(session)?;
    let Some(origin) = value["nodes"]
        .array()
        .unwrap_or_default()
        .iter()
        .find(|n| n["id"] == value["origin"])
    else {
        return Err("The Tend origin node is missing.".into());
    };
    let mut entries = origin["evidence"]
        .array()
        .unwrap_or_default()
        .iter()
        .filter(|entry| {
            entry["artifact"]
                .string()
                .is_some_and(|s| s.starts_with("tinkery:origin/tend"))
        });
    let Some(entry) = entries.next() else {
        return Err("The Tend evidence is not on the confirmed goal.".into());
    };
    let parsed = entry["observed"]
        .string()
        .and_then(|s| serde_json::from_str::<Origin>(s).ok());
    let result = parsed
        .filter(|_| entries.next().is_none() && entry["artifact"].string() == Some(ARTIFACT))
        .ok_or("The Tend origin is unreadable or duplicated.")?;
    result.validate(goal)?;
    Ok(Some(result))
}
