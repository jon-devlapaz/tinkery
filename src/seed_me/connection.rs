//! Checked creation evidence for a goal discovered in Tend, and the one place it meets Think
//! records for history and origins. No history is invented, and this evidence grants neither
//! seed confirmation nor permission to execute work.

use super::ledger;
use crate::think_record::{self, History, MadeIn, Unavailable};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Evidence tag for the supported Tend creation format.
pub(crate) const ARTIFACT: &str = "tinkery:origin/tend-v1";

/// Exact operator guidance supplied during discovery, never source evidence or authority.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Correction {
    pub(crate) revision: String,
    pub(crate) text: String,
}

impl Correction {
    /// Refuse malformed identities and words without changing the operator's text.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.revision.len() != 48
            || !self.revision.bytes().all(|c| c.is_ascii_hexdigit())
            || self.text.trim().is_empty()
            || self.text.len() > 2048
        {
            return Err("The correction is invalid; use 1–2048 bytes of exact words.".into());
        }
        Ok(())
    }
}

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
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct Origin {
    pub(crate) schema: u8,
    pub(crate) entry: String,
    pub(crate) repo: PathBuf,
    pub(crate) goal: String,
    pub(crate) sources: Vec<Source>,
    pub(crate) support: Vec<Support>,
    pub(crate) interpretation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) correction: Option<Correction>,
}

#[derive(Deserialize)]
#[serde(remote = "Origin", deny_unknown_fields)]
struct OriginFields {
    schema: u8,
    entry: String,
    repo: PathBuf,
    goal: String,
    sources: Vec<Source>,
    support: Vec<Support>,
    interpretation: String,
    #[serde(default)]
    correction: Option<Correction>,
}

impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        let valid = match value["schema"].as_u64() {
            Some(1) => value.get("correction").is_none(),
            Some(2) => value.get("correction").is_some_and(|v| v.is_object()),
            _ => false,
        };
        if !valid {
            return Err(serde::de::Error::custom(
                "Unsupported Tend origin or correction field",
            ));
        }
        let origin = OriginFields::deserialize(value).map_err(serde::de::Error::custom)?;
        origin
            .validate(&origin.goal)
            .map_err(serde::de::Error::custom)?;
        Ok(origin)
    }
}

impl Origin {
    /// Match the evidence tag to its strict origin format.
    pub(crate) fn artifact(&self) -> &'static str {
        if self.schema == 2 {
            "tinkery:origin/tend-v2"
        } else {
            ARTIFACT
        }
    }
    /// Validate the full evidence once, including exact supporting quotations.
    pub(crate) fn validate(&self, goal: &str) -> Result<(), String> {
        let ids: std::collections::HashSet<_> = self.sources.iter().map(|s| &s.id).collect();
        let supporting: std::collections::HashSet<_> = self.support.iter().map(|s| &s.id).collect();
        if !matches!((self.schema, &self.correction), (1, None) | (2, Some(_)))
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
        if let Some(correction) = &self.correction {
            correction.validate()?;
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
        .filter(|origin| {
            entries.next().is_none() && entry["artifact"].string() == Some(origin.artifact())
        })
        .ok_or("The Tend origin is unreadable or duplicated.")?;
    result.validate(goal)?;
    Ok(Some(result))
}

/// Where each saved goal was made, in input order: Tend evidence checked against one Think
/// record scan. Invalid Tend evidence, an unchecked Tend session or a conflicting Think record
/// is `Invalid`, never a fallback. Without Tend evidence, a failed scan refuses. Writes nothing.
pub(crate) fn origins(root: &Path, sessions: &[(&Path, &str)]) -> Result<Vec<MadeIn>, Unavailable> {
    let creation: Vec<_> = sessions
        .iter()
        .map(|(session, goal)| read(session, goal))
        .collect();
    let think = think_record::read_origins(root, sessions);
    if creation.iter().all(|origin| matches!(origin, Ok(None))) {
        return Ok(think?.into_iter().map(|found| found.made_in).collect());
    }
    let mut think = think.ok().map(Vec::into_iter);
    Ok(sessions
        .iter()
        .zip(creation)
        .map(|((session, goal), creation)| {
            let found = think.as_mut().and_then(Iterator::next);
            match creation {
                Ok(None) => found.map_or(MadeIn::Unknown, |found| found.made_in),
                Err(reason) => MadeIn::Invalid(reason),
                // The evidence is read again so a change during the scan cannot pass.
                Ok(Some(origin)) => match found {
                    Some(found)
                        if found.live
                            && read(session, goal).ok().flatten().as_ref() == Some(&origin) =>
                    {
                        if found.claimed && found.made_in != MadeIn::Repo(origin.repo.clone()) {
                            MadeIn::Invalid("Think and Tend origins conflict or are ambiguous.".into())
                        } else {
                            MadeIn::Repo(origin.repo)
                        }
                    }
                    _ => MadeIn::Invalid(
                        "The Tend origin cannot be checked against live sessions and Think records."
                            .into(),
                    ),
                },
            }
        })
        .collect())
}

/// The history of one saved goal: a Tend-created goal shows its checked creation evidence and
/// no turns; any other goal reads its Think record. Invalid Tend evidence refuses. Writes nothing.
pub(crate) fn history(root: &Path, session: &Path, goal: &str) -> Result<History, Unavailable> {
    let unavailable = |details: String| Unavailable {
        message: "Tend origin unavailable.".into(),
        details,
    };
    let Some(origin) = read(session, goal).map_err(unavailable)? else {
        return think_record::read(root, session, goal);
    };
    let made_in = origins(root, &[(session, goal)])?.remove(0);
    if let MadeIn::Invalid(reason) = &made_in {
        return Err(unavailable(reason.clone()));
    }
    Ok(History {
        turns: vec![],
        goal: goal.into(),
        made_in,
        details: serde_json::to_string_pretty(&origin).map_err(|e| unavailable(e.to_string()))?,
        sequence: origin.correction.as_ref().map_or_else(
            || "Created from a Tend connection".into(),
            |c| format!("Created from a Tend connection\n\nOperator guidance supplied during discovery:\n{}", c.text),
        ),
        readings: vec![],
    })
}
