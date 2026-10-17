//! One complete record scan attributes many saved goals without retaining their histories.
//! Malformed records invalidate the scan; duplicates and older matches never establish an origin.

use super::*;
use std::collections::{HashMap, HashSet};

struct Attribution {
    id: Option<String>,
    goal: String,
    made_in: MadeIn,
}

/// Origins in input order, using the history reader's exact identity, goal and uniqueness rules.
/// Enumerates the root once and reads each JSON file at most once; retains only attribution
/// fields. Individual refusals are Unknown; unreadable or unsupported records refuse the whole
/// scan. Sessions must remain unchanged across the scan. Nothing is written or run.
pub(crate) fn read_origins(
    root: &Path,
    sessions: &[(&Path, &str)],
) -> Result<Vec<MadeIn>, Unavailable> {
    let selected: Vec<_> = sessions
        .iter()
        .map(|(path, goal)| current_session(path, goal))
        .collect();
    let mut records: HashMap<PathBuf, Option<Attribution>> = HashMap::new();
    let mut older = HashSet::new();
    for entry in entries(root)? {
        let path = entry
            .map_err(|error| Unavailable::unreadable(root, error))?
            .path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let record = read_file(&path)?;
        if record.schema == 1 {
            older.insert(record.seed_me_session);
            continue;
        }
        let stored = reference(&path, &record)?.clone();
        let attribution = Attribution {
            id: record.seed_me_session_id,
            goal: record.goal,
            made_in: origin(record.schema, record.repo),
        };
        records
            .entry(stored)
            .and_modify(|found| *found = None)
            .or_insert(Some(attribution));
    }
    Ok(sessions
        .iter()
        .zip(selected)
        .map(|((path, goal), selected)| {
            let Ok(selected) = selected else {
                return MadeIn::Unknown;
            };
            let Some(Some(record)) = records.get(&selected.0) else {
                return MadeIn::Unknown;
            };
            if older.contains(&session_id(&selected.0))
                || record.id.as_deref() != Some(selected.1.as_str())
                || record.goal != *goal
                || current_session(path, goal).ok().as_ref() != Some(&selected)
            {
                return MadeIn::Unknown;
            }
            record.made_in.clone()
        })
        .collect())
}
