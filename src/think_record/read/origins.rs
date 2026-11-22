//! One complete record scan attributes saved goals by unique live ledger identity, never by root.
//! Malformed records invalidate the scan; duplicate records and live IDs refuse attribution.
//! Reads Think records only; Tend creation evidence is combined elsewhere.

use super::*;
use std::collections::{HashMap, HashSet};

/// What the Think records say about one saved session, from one complete scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Attribution {
    /// The recorded origin, or `Unknown` when no single exact record attributes the goal.
    pub(crate) made_in: MadeIn,
    /// The session stayed readable, unchanged and uniquely identified through the scan.
    pub(crate) live: bool,
    /// Some record of any schema names this live session, whatever its goal or record uniqueness.
    pub(crate) claimed: bool,
}

/// Attributions in input order, sharing history's identity, exact-goal and uniqueness rules.
/// Scans each current session root and the record root once before rechecking identities.
/// Retains attribution only, never turns; malformed files refuse the scan. Writes nothing.
pub(crate) fn read_origins(
    root: &Path,
    sessions: &[(&Path, &str)],
) -> Result<Vec<Attribution>, Unavailable> {
    let selected: Vec<_> = sessions
        .iter()
        .map(|(path, goal)| current_session(path, goal))
        .collect();
    let identities = identity_index(sessions.iter().map(|(path, _)| *path))?;
    let mut records: HashMap<String, Option<(String, MadeIn)>> = HashMap::new();
    let mut older = HashSet::new();
    for entry in record_files(root)? {
        let (path, record) = entry?;
        if record.schema == 1 {
            older.insert(record.seed_me_session);
            continue;
        }
        let identity = record_identity(&path, &record)?;
        records
            .entry(identity)
            .and_modify(|found| *found = None)
            .or_insert(Some((record.goal, origin(record.schema, record.repo))));
    }
    let after = identity_index(sessions.iter().map(|(path, _)| *path))?;
    Ok(sessions
        .iter()
        .zip(selected)
        .map(|((path, goal), selected)| {
            let Ok(selected) = selected else {
                return Attribution {
                    made_in: MadeIn::Unknown,
                    live: false,
                    claimed: false,
                };
            };
            let older = older.contains(&session_id(&selected.0));
            let live = require_unique(&identities, &selected).is_ok()
                && require_unique(&after, &selected).is_ok()
                && current_session(path, goal).ok().as_ref() == Some(&selected);
            let found = records.get(&selected.1);
            let made_in = match found {
                Some(Some((recorded, made_in))) if live && !older && recorded == goal => {
                    made_in.clone()
                }
                _ => MadeIn::Unknown,
            };
            Attribution {
                made_in,
                live,
                claimed: live && (found.is_some() || older),
            }
        })
        .collect())
}
