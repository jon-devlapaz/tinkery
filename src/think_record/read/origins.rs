//! One complete record scan attributes saved goals by unique live ledger identity, never by root.
//! Malformed records invalidate the scan; duplicate records and live IDs refuse attribution.

use super::*;
use std::collections::{HashMap, HashSet};

struct Attribution {
    goal: String,
    made_in: MadeIn,
}

/// Origins in input order, sharing history's identity, exact-goal and uniqueness rules.
/// Scans each current session root and the record root once before rechecking identities.
/// Retains attribution only, never turns; malformed files refuse the scan. Writes nothing.
fn legacy_origins(root: &Path, sessions: &[(&Path, &str)]) -> Result<Vec<MadeIn>, Unavailable> {
    let selected: Vec<_> = sessions
        .iter()
        .map(|(path, goal)| current_session(path, goal))
        .collect();
    let identities = identity_index(sessions.iter().map(|(path, _)| *path))?;
    let mut records: HashMap<String, Option<Attribution>> = HashMap::new();
    let mut older = HashSet::new();
    for entry in record_files(root)? {
        let (path, record) = entry?;
        if record.schema == 1 {
            older.insert(record.seed_me_session);
            continue;
        }
        let identity = record_identity(&path, &record)?;
        let attribution = Attribution {
            goal: record.goal,
            made_in: origin(record.schema, record.repo),
        };
        records
            .entry(identity)
            .and_modify(|found| *found = None)
            .or_insert(Some(attribution));
    }
    let after = identity_index(sessions.iter().map(|(path, _)| *path))?;
    Ok(sessions
        .iter()
        .zip(selected)
        .map(|((path, goal), selected)| {
            let Ok(selected) = selected else {
                return MadeIn::Unknown;
            };
            let Some(Some(record)) = records.get(&selected.1) else {
                return MadeIn::Unknown;
            };
            if older.contains(&session_id(&selected.0))
                || require_unique(&identities, &selected).is_err()
                || require_unique(&after, &selected).is_err()
                || record.goal != *goal
                || current_session(path, goal).ok().as_ref() != Some(&selected)
            {
                return MadeIn::Unknown;
            }
            record.made_in.clone()
        })
        .collect())
}

/// Read creation origins through one boundary. Invalid Tend evidence remains a blocking result.
pub(crate) fn read_origins(
    root: &Path,
    sessions: &[(&Path, &str)],
) -> Result<Vec<MadeIn>, Unavailable> {
    let creation: Vec<_> = sessions
        .iter()
        .map(|(session, goal)| seed_me::connection::read(session, goal))
        .collect();
    if creation.iter().all(|origin| matches!(origin, Ok(None))) {
        return legacy_origins(root, sessions);
    }
    let legacy = legacy_origins(root, sessions);
    let identities = identity_index(sessions.iter().map(|(path, _)| *path));
    let claims: Result<(HashSet<String>, HashSet<String>), Unavailable> = (|| {
        let mut claims = HashSet::new();
        let mut older = HashSet::new();
        for entry in record_files(root)? {
            let (path, record) = entry?;
            if record.schema == 1 {
                older.insert(record.seed_me_session);
            } else {
                claims.insert(record_identity(&path, &record)?);
            }
        }
        Ok((claims, older))
    })();
    let after = identity_index(sessions.iter().map(|(path, _)| *path));
    Ok(sessions.iter().zip(creation).enumerate().map(|(index, ((session, goal), creation))| {
        let old = legacy.as_ref().ok().and_then(|list| list.get(index)).cloned().unwrap_or(MadeIn::Unknown);
        match creation {
            Ok(None) => old,
            Err(reason) => MadeIn::Invalid(reason),
            Ok(Some(origin)) => {
                let selected = current_session(session, goal).ok();
                let valid_identity = selected.as_ref().is_some_and(|selected|
                    identities.as_ref().is_ok_and(|ids| require_unique(ids, selected).is_ok())
                    && after.as_ref().is_ok_and(|ids| require_unique(ids, selected).is_ok()));
                if !valid_identity || legacy.is_err() || claims.is_err()
                    || seed_me::connection::read(session, goal).ok().flatten().as_ref() != Some(&origin) {
                    return MadeIn::Invalid("The Tend origin cannot be checked against live sessions and Think records.".into());
                }
                let claimed = selected.is_some_and(|(path, id)| claims.as_ref().is_ok_and(|(claims, older)| claims.contains(&id) || older.contains(&session_id(&path))));
                if claimed && old != MadeIn::Repo(origin.repo.clone()) {
                    MadeIn::Invalid("Think and Tend origins conflict or are ambiguous.".into())
                } else { MadeIn::Repo(origin.repo) }
            }
        }
    }).collect())
}
