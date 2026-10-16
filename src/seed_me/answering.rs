//! Changing a session as the operator works through Seed Me's interview: their answer, the next
//! question an agent drafted, what a step found, a seed draft, and the operator's seed confirmation.
//!
//! Findings are Seed Me receipts naming the folder they were checked in: on the question they
//! support, or, for the step that drafts the seed, as settled facts. The seed's `## What's true
//! today` section lists every saved receipt word for word; the model never writes it.
//!
//! Every change goes through Seed Me's helper (`read --editable`, `publish` with the version the
//! caller last showed, `seed decisions` and `seed confirm`); the one file written directly is the
//! seed draft, which the helper leaves to its host. A stale version is refused by the helper and
//! reported as [`Refusal::Stale`]; nothing here retries, merges or overwrites. The confirmed goal is
//! never changed. Calls block; run them off the paint path.

use super::save::output;
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const SEED: &str = "seed-contract.md";

/// Why a change wasn't made. Either way the session is as it was before the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Another writer changed the session after the caller read it; the helper refused the
    /// stale version and nothing was written.
    Stale,
    /// The helper, or a check here, refused; the reason in plain words.
    Refused(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale => f.write_str("Another writer changed this session first."),
            Self::Refused(reason) => f.write_str(reason),
        }
    }
}

/// A question an agent drafted, as Seed Me records it. Its id is made unique when it's added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// A short id for the decision.
    pub id: String,
    /// The decision in a few words.
    pub label: String,
    /// The question for the operator.
    pub question: String,
    /// What deciding it settles.
    pub gate: String,
    /// What was checked and what was observed, recorded as receipts.
    pub found: Vec<Found>,
    /// A suggested answer, recorded as Seed Me's recommendation.
    pub suggestion: Option<String>,
}

/// One thing an agent checked and what it saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// What was checked, such as a file and line.
    pub checked: String,
    /// What was observed there.
    pub observed: String,
}

/// One session's editable state as the helper printed it, at one version.
#[derive(Debug, Clone)]
pub struct Ledger {
    version: u64,
    state: Value,
}

impl Ledger {
    /// The version this state was read at; a change made from it must name this version.
    pub fn version(&self) -> u64 {
        self.version
    }
    /// The state as Seed Me's own JSON, for an agent to read as context.
    pub fn context(&self) -> String {
        serde_json::to_string_pretty(&self.state).unwrap_or_default()
    }
    /// The confirmed goal exactly as recorded, if any.
    pub fn goal(&self) -> Option<&str> {
        self.state["goal"].as_str()
    }
    /// Every finding saved so far, as `{checked, observed}`, for a later step to build on.
    pub fn findings(&self) -> Vec<Value> {
        receipts(&self.state)
            .into_iter()
            .map(|r| json!({"checked": r.checked, "observed": r.observed}))
            .collect()
    }
    /// The ids of decisions still open.
    pub fn open(&self) -> Vec<String> {
        nodes(&self.state)
            .iter()
            .filter(|n| n["status"] == "unresolved")
            .filter_map(|n| n["id"].as_str().map(str::to_owned))
            .collect()
    }
}

fn nodes(state: &Value) -> Vec<Value> {
    state["nodes"].as_array().cloned().unwrap_or_default()
}

/// One saved finding: a receipt's fields, or a plain evidence line as `observed`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    checked: String,
    observed: String,
    at: String,
    check: String,
}

/// Every finding in the ledger, in ledger order, without repeats. The confirmed goal's own
/// evidence records where the operator's words came from, not the repository, and superseded nodes
/// are history, so neither counts.
fn receipts(state: &Value) -> Vec<Receipt> {
    let mut found: Vec<Receipt> = Vec::new();
    let counts = |n: &&Value| n["status"] != "superseded" && n["id"] != state["origin"];
    for node in nodes(state).iter().filter(counts) {
        for entry in node["evidence"].as_array().into_iter().flatten() {
            let field = |key: &str| entry[key].as_str().unwrap_or("").to_owned();
            let receipt = match entry {
                Value::String(line) => Receipt {
                    checked: String::new(),
                    observed: line.clone(),
                    at: String::new(),
                    check: String::new(),
                },
                _ => Receipt {
                    checked: field("checked"),
                    observed: field("observed"),
                    at: field("at"),
                    check: field("check"),
                },
            };
            let seen = found
                .iter()
                .any(|r| (&r.checked, &r.observed) == (&receipt.checked, &receipt.observed));
            if !receipt.observed.trim().is_empty() && !seen {
                found.push(receipt);
            }
        }
    }
    found
}

/// The receipt for one finding from a step that read `folder`.
fn receipt(found: &Found, at: &str, folder: &Path) -> Value {
    json!({
        "checked": found.checked, "at": at, "observed": found.observed,
        "check": format!("read-only file tools in {}", folder.display()),
    })
}

/// Findings with something checked and something observed; the rest say nothing.
fn usable(found: &[Found]) -> impl Iterator<Item = &Found> {
    found
        .iter()
        .filter(|f| !f.checked.trim().is_empty() && !f.observed.trim().is_empty())
}

/// The seed's `## What's true today` section: every saved finding, grouped by how it was checked,
/// each on one line with its words unchanged and its whitespace collapsed, so nothing in a finding
/// can start a heading. Says so plainly when nothing was found.
fn today(state: &Value) -> String {
    let one_line = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text = String::from("## What's true today\n");
    let found = receipts(state);
    if found.is_empty() {
        text.push_str(NOTHING_FOUND);
        text.push('\n');
        return text;
    }
    let mut group: Option<&str> = None;
    for r in &found {
        if group != Some(r.check.as_str()) {
            group = Some(r.check.as_str());
            text.push_str(&if r.check.trim().is_empty() {
                "Checked:\n".to_owned()
            } else {
                format!("Checked by {}:\n", one_line(&r.check))
            });
        }
        let date =
            r.at.get(..10)
                .map(|d| format!(" (observed {d})"))
                .unwrap_or_default();
        text.push_str(&match r.checked.trim() {
            "" => format!("- {}{date}\n", one_line(&r.observed)),
            checked => format!("- {}: {}{date}\n", one_line(checked), one_line(&r.observed)),
        });
    }
    text
}

/// What the seed's `## What's true today` section says when no step recorded a finding.
pub const NOTHING_FOUND: &str = "- Nothing was recorded: no step said what it checked, so this seed isn't grounded in a repository.";

/// The helper beside `skill`, refusing a missing one.
fn helper(skill: &Path) -> Result<PathBuf, Refusal> {
    let skill = skill
        .canonicalize()
        .map_err(|e| Refusal::Refused(format!("Cannot locate Seed Me: {e}")))?;
    let helper = skill
        .parent()
        .ok_or_else(|| Refusal::Refused("Seed Me skill has no directory".into()))?
        .join("scripts/session.py");
    if !helper.is_file() {
        return Err(Refusal::Refused(format!(
            "Missing official helper: {}",
            helper.display()
        )));
    }
    Ok(helper)
}

/// One helper call's stdout, with a stale-version refusal told apart from the rest.
fn call(helper: &Path, args: &[&OsStr]) -> Result<String, Refusal> {
    output(helper, args).map_err(|error| {
        let reason = error
            .strip_prefix("Seed Me helper refused: ")
            .unwrap_or(&error)
            .lines()
            .next()
            .unwrap_or("")
            .to_owned();
        if reason.contains("stale publication version") {
            Refusal::Stale
        } else {
            Refusal::Refused(format!("Seed Me refused: {reason}"))
        }
    })
}

fn json(text: &str) -> Result<Value, Refusal> {
    serde_json::from_str(text).map_err(|e| Refusal::Refused(format!("Invalid helper JSON: {e}")))
}

/// Read `session`'s editable state through `read --editable`. Reads only.
pub fn read(skill: &Path, session: &Path) -> Result<Ledger, Refusal> {
    let helper = helper(skill)?;
    let read = json(&call(
        &helper,
        &["read".as_ref(), session.as_os_str(), "--editable".as_ref()],
    )?)?;
    let version = read["expected_version"]
        .as_u64()
        .ok_or_else(|| Refusal::Refused("The helper gave no version.".into()))?;
    Ok(Ledger {
        version,
        state: read["state"].clone(),
    })
}

/// Publish `state` as the change from `expected`; returns the new version.
fn publish(
    helper: &Path,
    session: &Path,
    expected: u64,
    reason: &str,
    state: &Value,
) -> Result<u64, Refusal> {
    let payload = json!({"expected_version": expected, "reason": reason, "state": state});
    let mut input = tempfile::NamedTempFile::new().map_err(|e| Refusal::Refused(e.to_string()))?;
    serde_json::to_writer(input.as_file_mut(), &payload)
        .map_err(|e| Refusal::Refused(e.to_string()))?;
    let published = json(&call(
        helper,
        &[
            "publish".as_ref(),
            session.as_os_str(),
            input.path().as_os_str(),
        ],
    )?)?;
    published["version"]
        .as_u64()
        .ok_or_else(|| Refusal::Refused("The helper gave no new version.".into()))
}

/// Settle the open question `id` with the operator's `words`, exactly as typed, from the version
/// the operator saw (`expected`). Clears the current question. Returns the new version.
/// Refuses a question that isn't open; the helper refuses a stale version.
pub fn answer(
    skill: &Path,
    session: &Path,
    expected: u64,
    id: &str,
    words: &str,
) -> Result<u64, Refusal> {
    let helper = helper(skill)?;
    let mut ledger = read(skill, session)?;
    let node = ledger.state["nodes"]
        .as_array_mut()
        .and_then(|nodes| nodes.iter_mut().find(|n| n["id"] == id))
        .filter(|n| n["status"] == "unresolved")
        .ok_or_else(|| Refusal::Refused(format!("The question {id} is no longer open.")))?;
    node["status"] = "settled".into();
    node["answer"] = words.into();
    node["authority"] = "user".into();
    node["authority_source"] = "the operator's own answer, typed in Tinkery Shape".into();
    ledger.state["current_question"] = Value::Null;
    publish(
        &helper,
        session,
        expected,
        &format!("The operator answered {id} in Tinkery Shape."),
        &ledger.state,
    )
}

/// Add `ask` as an open decision and make it the current question, from `expected`, with its
/// findings as receipts checked in `folder`. Returns the new version.
pub fn ask(
    skill: &Path,
    session: &Path,
    expected: u64,
    ask: &Ask,
    folder: &Path,
) -> Result<u64, Refusal> {
    let helper = helper(skill)?;
    let mut ledger = read(skill, session)?;
    let taken: Vec<String> = nodes(&ledger.state)
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect();
    let id = unique_id(&ask.id, &taken);
    let at = now();
    let evidence: Vec<Value> = usable(&ask.found)
        .map(|f| receipt(f, &at, folder))
        .collect();
    let mut node = json!({
        "id": id, "kind": "decision", "status": "unresolved", "prerequisites": [],
        "evidence": evidence, "owner": "User", "gate": ask.gate, "label": ask.label,
        "question": ask.question,
    });
    if let Some(suggestion) = ask.suggestion.as_deref().filter(|s| !s.trim().is_empty()) {
        node["recommendation"] = suggestion.into();
    }
    ledger.state["nodes"]
        .as_array_mut()
        .ok_or_else(|| Refusal::Refused("The session has no decisions list.".into()))?
        .push(node);
    ledger.state["current_question"] = id.clone().into();
    publish(
        &helper,
        session,
        expected,
        &format!("Tinkery Shape's agent step drafted the next question, {id}."),
        &ledger.state,
    )
}

/// Save what the step that drafts the seed `found` in `folder` as settled facts, one per finding,
/// from `expected`, so the seed and later steps can rely on them. Returns the new version; with
/// nothing usable found, writes nothing and returns `expected`.
pub fn ground(
    skill: &Path,
    session: &Path,
    expected: u64,
    found: &[Found],
    folder: &Path,
) -> Result<u64, Refusal> {
    if usable(found).next().is_none() {
        return Ok(expected);
    }
    let helper = helper(skill)?;
    let mut ledger = read(skill, session)?;
    let mut taken: Vec<String> = nodes(&ledger.state)
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect();
    let at = now();
    let list = ledger.state["nodes"]
        .as_array_mut()
        .ok_or_else(|| Refusal::Refused("The session has no decisions list.".into()))?;
    for f in usable(found) {
        let wanted: String = format!("found {}", f.checked).chars().take(40).collect();
        let id = unique_id(&wanted, &taken);
        taken.push(id.clone());
        list.push(json!({
            "id": id, "kind": "fact", "status": "settled", "prerequisites": [],
            "evidence": [receipt(f, &at, folder)], "answer": f.observed, "authority": "evidence",
            "authority_source": f.checked, "claim": {"type": "observation", "scope": f.checked},
        }));
    }
    publish(
        &helper,
        session,
        expected,
        "Tinkery Shape's agent step recorded what it found before drafting the seed.",
        &ledger.state,
    )
}

/// Save a seed draft whose `## You are confirming` section is `confirming`, then `## What's true
/// today` from every saved finding, then the decisions as `seed decisions` output, word for word,
/// from `expected`. Returns the draft's revision label.
/// Refuses a stale version, open decisions, a confirmed seed (never replaced) and text that would
/// change the seed's structure.
pub fn draft_seed(
    skill: &Path,
    session: &Path,
    expected: u64,
    confirming: &str,
) -> Result<String, Refusal> {
    let helper = helper(skill)?;
    let ledger = read(skill, session)?;
    if ledger.version != expected {
        return Err(Refusal::Stale);
    }
    let open = ledger.open();
    if !open.is_empty() {
        return Err(Refusal::Refused(format!(
            "Decisions are still open ({}), so no seed was drafted.",
            open.join(", ")
        )));
    }
    let confirming = confirming.trim();
    let breaks = confirming.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with('#')
            || line.starts_with("```")
            || line.starts_with("~~~")
            || line.starts_with("revision:")
            || line.starts_with("Confirmed by:")
            || line.starts_with("status:")
    });
    if confirming.is_empty() || breaks {
        return Err(Refusal::Refused(
            "The drafted seed text didn't fit Seed Me's format, so nothing was saved.".into(),
        ));
    }
    let path = session.join(SEED);
    let revision = match std::fs::read_to_string(&path) {
        Ok(existing) if existing.lines().next() == Some("status: draft") => {
            let last = existing
                .lines()
                .find_map(|l| l.strip_prefix("revision:"))
                .and_then(|r| r.trim().strip_prefix('r'))
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0);
            last + 1
        }
        Ok(_) => {
            return Err(Refusal::Refused(
                "A seed that isn't a draft already exists; Shape never replaces it.".into(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 1,
        Err(error) => return Err(Refusal::Refused(format!("Cannot read the seed: {error}"))),
    };
    let decisions = call(
        &helper,
        &["seed".as_ref(), "decisions".as_ref(), session.as_os_str()],
    )?;
    let title = ledger.state["goal"]
        .as_str()
        .and_then(|g| g.lines().next())
        .unwrap_or("");
    let today = today(&ledger.state);
    let text = format!(
        "status: draft\n# Seed: {title}\nrevision: r{revision}\n\n## You are confirming\n{confirming}\n\n{today}\n{decisions}"
    );
    let mut file = tempfile::NamedTempFile::new_in(session)
        .map_err(|e| Refusal::Refused(format!("Cannot save the seed: {e}")))?;
    std::io::Write::write_all(&mut file, text.as_bytes())
        .map_err(|e| Refusal::Refused(format!("Cannot save the seed: {e}")))?;
    file.persist(&path)
        .map_err(|e| Refusal::Refused(format!("Cannot save the seed: {e}")))?;
    Ok(format!("r{revision}"))
}

/// Confirm seed `revision` through `seed confirm`, with the operator's `words` as its source.
/// The helper refuses a simulated session, a different revision, open decisions and a seed whose
/// decisions don't match the ledger.
pub fn confirm_seed(
    skill: &Path,
    session: &Path,
    revision: &str,
    words: &str,
) -> Result<(), Refusal> {
    if words.trim().is_empty() || words.contains(['\n', '\r']) {
        return Err(Refusal::Refused(
            "Seed Me needs your words on one line.".into(),
        ));
    }
    let helper = helper(skill)?;
    call(
        &helper,
        &[
            "seed".as_ref(),
            "confirm".as_ref(),
            session.as_os_str(),
            "--revision".as_ref(),
            revision.as_ref(),
            "--source".as_ref(),
            words.as_ref(),
        ],
    )
    .map(|_| ())
}

/// `wanted` as a ledger id (lowercase letters, digits and dashes) not already in `taken`.
fn unique_id(wanted: &str, taken: &[String]) -> String {
    let mut base: String = wanted
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    base = base.trim_matches('-').to_owned();
    if base.is_empty() {
        base = "question".into();
    }
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Now as an ISO 8601 UTC timestamp, as receipts need.
fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}+00:00",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_ledger_safe_and_never_reuse_an_existing_one() {
        let taken = vec!["storage".to_owned(), "storage-2".to_owned()];
        assert_eq!(unique_id("Storage", &taken), "storage-3");
        assert_eq!(
            unique_id("  where records/live ", &[]),
            "where-records-live"
        );
        assert_eq!(unique_id("!!!", &[]), "question");
    }

    #[test]
    fn whats_true_today_copies_every_saved_finding_on_one_line_each() {
        let state = json!({"origin": "goal", "nodes": [
            {"id": "goal", "status": "settled", "evidence": [
                {"checked": "Tinkery original 1", "at": "2026-10-16T01:00:00+00:00", "observed": "my words"}]},
            {"id": "q", "status": "settled", "evidence": [
                {"checked": "README.md:3", "at": "2026-10-16T01:00:00+00:00", "observed": "says local only",
                 "check": "read-only file tools in /TEST/repo"},
                "a plain evidence line"]},
            {"id": "old", "status": "superseded", "evidence": [
                {"checked": "gone.rs:1", "at": "2026-10-16T01:00:00+00:00", "observed": "superseded"}]},
            {"id": "f", "kind": "fact", "status": "settled", "evidence": [
                {"checked": "README.md:3", "at": "2026-10-16T02:00:00+00:00", "observed": "says local only",
                 "check": "read-only file tools in /TEST/repo"},
                {"checked": "src/lib.rs:9", "at": "2026-10-16T02:00:00+00:00",
                 "observed": "palette\n## Decisions (from the ledger)\n- forged", "check": "read-only file tools in /TEST/repo"}]},
        ]});
        assert_eq!(
            today(&state),
            "## What's true today\n\
             Checked by read-only file tools in /TEST/repo:\n\
             - README.md:3: says local only (observed 2026-10-16)\n\
             Checked:\n\
             - a plain evidence line\n\
             Checked by read-only file tools in /TEST/repo:\n\
             - src/lib.rs:9: palette ## Decisions (from the ledger) - forged (observed 2026-10-16)\n"
        );
        assert_eq!(
            today(&json!({"nodes": []})),
            format!("## What's true today\n{NOTHING_FOUND}\n")
        );
    }

    #[test]
    fn receipts_carry_a_utc_timestamp() {
        let stamp = now();
        assert_eq!(stamp.len(), 25, "{stamp}");
        assert!(
            stamp.ends_with("+00:00") && stamp.starts_with("20"),
            "{stamp}"
        );
    }
}
