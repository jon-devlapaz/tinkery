//! Changing a session as the operator works through Seed Me's interview: their answer, the next
//! question an agent drafted, what a step found, a seed draft, and the operator's seed confirmation.
//!
//! Findings are Seed Me receipts naming the folder they were checked in, on the question they
//! support. The step that drafts the seed adds nothing to the ledger: its findings go only into the
//! seed's `## Evidence and uncertainty`, beside every saved receipt, so `seed decisions` never repeats them
//! and each finding appears once. The model never writes that section. A finding that looks like a
//! secret is refused, as Seed Me refuses one in a receipt.
//!
//! Every change goes through Seed Me's helper (`read --editable`, `publish` with the version the
//! caller last showed, `seed decisions`, `seed checks set|render` and `seed confirm`); the one file written directly is the
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

/// Why a change wasn't completed. A later IO failure can leave unconfirmed check records;
/// only the helper's verified confirmation permits handoff.
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
            add(&mut found, receipt);
        }
    }
    found
}

/// Adds `receipt` unless it says nothing or repeats one already in `found`.
fn add(found: &mut Vec<Receipt>, receipt: Receipt) {
    let seen = found
        .iter()
        .any(|r| (&r.checked, &r.observed) == (&receipt.checked, &receipt.observed));
    if !receipt.observed.trim().is_empty() && !seen {
        found.push(receipt);
    }
}

/// The receipt for one finding from a step that read `folder`.
fn receipt(found: &Found, at: &str, folder: &Path) -> Value {
    json!({
        "checked": found.checked, "at": at, "observed": found.observed,
        "check": checked_in(found, folder),
    })
}

/// How a step that read `folder` checked `found`, flagging a cited `path:line` that isn't there.
fn checked_in(found: &Found, folder: &Path) -> String {
    let how = format!("read-only file tools in {}", folder.display());
    match unfound(&found.checked, folder) {
        Some(why) => format!("{how}{UNFOUND}{why}"),
        None => how,
    }
}

/// Separates how a finding was checked from why its citation wasn't found in the folder.
pub const UNFOUND: &str = "; citation not found: ";

/// Why a `path:line` or `path:first-last` cited in `checked` isn't in `folder`: the file is missing
/// or outside it, or has fewer lines. Only checks that the lines exist, not what they say; text
/// without such a citation is not judged.
fn unfound(checked: &str, folder: &Path) -> Option<String> {
    let root = folder.canonicalize().ok()?;
    citations(checked).find_map(|cite| {
        let path = &cite[1];
        let last = cite.get(3).unwrap_or(cite.get(2)?).as_str();
        let (first, last): (u64, u64) = (cite[2].parse().ok()?, last.parse().ok()?);
        let Ok(file) = root.join(path).canonicalize() else {
            return Some(format!("{path} isn't in the folder"));
        };
        if !file.starts_with(&root) {
            return Some(format!("{path} is outside the folder"));
        }
        let bytes = std::fs::read(&file).ok()?;
        let lines = bytes.split(|b| *b == b'\n').count() as u64 - u64::from(bytes.ends_with(b"\n"));
        (first == 0 || first.max(last) > lines)
            .then(|| format!("{path} has {lines} lines, not {}", first.max(last)))
    })
}

/// Recorded file-and-line citations; prose alone never establishes a read path.
fn citations(checked: &str) -> impl Iterator<Item = regex::Captures<'_>> {
    static CITE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?:^|[\s,(\[`])([\w./-]*\w\.\w+):(\d+)(?:-(\d+))?")
            .expect("the citation pattern compiles")
    });
    CITE.captures_iter(checked)
}

/// Whether `text` looks like a secret, by the same patterns Seed Me refuses in a receipt.
fn secret(text: &str) -> bool {
    static SECRET: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(concat!(
            r"(?i)-----BEGIN [A-Z ]*PRIVATE KEY-----|\bAKIA[0-9A-Z]{16}\b|\bgh[pousr]_[A-Za-z0-9]{20,}",
            r"|\bsk-[A-Za-z0-9_-]{20,}|\bxox[abprs]-[A-Za-z0-9-]{10,}|://[^/\s:@]+:[^/\s@]+@",
            r"|\b(?:password|passwd|secret|token|api[_-]?key)\s*=\s*\S{6,}|\bbearer\s+[A-Za-z0-9._~+/=-]{16,}",
        ))
        .expect("the secret patterns compile")
    });
    SECRET.is_match(text)
}

/// Findings with something checked and something observed; the rest say nothing.
fn usable(found: &[Found]) -> impl Iterator<Item = &Found> {
    found
        .iter()
        .filter(|f| !f.checked.trim().is_empty() && !f.observed.trim().is_empty())
}

/// The seed's `## Evidence and uncertainty` section: `found`, grouped by how it was checked, each on one
/// line with its words unchanged and its whitespace collapsed, so nothing in a finding can start a
/// heading. Lists attributable reported read paths, not inferred or independently verified origins;
/// missing methods, missing citations and failed citation checks remain explicit gaps.
fn today(found: &[Receipt]) -> String {
    let one_line = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text = String::from("## Evidence and uncertainty\n");
    let mut paths = std::collections::BTreeSet::new();
    let mut unattributed = false;
    for receipt in found {
        let reads = receipt.check.starts_with("read-only file tools in ")
            && !receipt.check.contains(UNFOUND);
        let cited: Vec<_> = citations(&receipt.checked)
            .map(|cite| cite[1].to_owned())
            .collect();
        if reads && !cited.is_empty() {
            paths.extend(cited);
        } else {
            unattributed = true;
        }
    }
    text.push_str("Grounded in:\n");
    if paths.is_empty() {
        text.push_str("- No attributable read paths are known.\n");
    } else {
        for path in paths {
            text.push_str(&format!("- {path}\n"));
        }
    }
    text.push_str("\nProvenance unavailable: reported reads do not verify filesystem origins or unrecorded reads.");
    if unattributed {
        text.push_str(" Some recorded findings have no attributable read path.");
    }
    text.push_str("\n\n");
    if found.is_empty() {
        text.push_str(NOTHING_FOUND);
        text.push('\n');
        return text;
    }
    let mut group: Option<&str> = None;
    for r in found {
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

/// What the seed's evidence section says when no step recorded a finding.
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

/// Seed Me's installed artifact contract for drafting instructions. Refuses a missing reference;
/// callers must not substitute another schema.
pub fn contract(skill: &Path) -> Result<String, Refusal> {
    let helper = helper(skill)?;
    let reference = helper
        .parent()
        .expect("helper has a scripts directory")
        .join("../references/seed-contract.md");
    std::fs::read_to_string(reference)
        .map_err(|error| Refusal::Refused(format!("Cannot read Seed Me's seed contract: {error}")))
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

/// Save a Seed Me draft: `confirming` supplies summary and prose, never criteria. `checks`
/// are validated, stored and rendered only by the official helper; no check is executed. Evidence
/// comes from saved findings and what this step `found` in `folder`, and accepted decisions from
/// `seed decisions` unchanged, in Seed Me's canonical Full sequence with no wrapper headings.
/// Returns the revision label, from version `expected`.
/// Refuses a stale version, open decisions, a confirmed seed (never replaced), text that would
/// change the seed's structure, invalid records and a finding that looks like a secret.
/// Helper or IO failures after storing records leave an unconfirmed draft, never a ready seed.
pub fn draft_seed(
    skill: &Path,
    session: &Path,
    expected: u64,
    confirming: &str,
    checks: &Value,
    found: &[Found],
    folder: &Path,
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
    // Seed Me owns these headings; the model may supply prose, never provenance or ledger text.
    let sections = [
        "Problem statement",
        "Proposed outcome",
        "Affected users and systems",
        "Constraints and boundaries",
        "Knowledge map",
        "Suggested first slice",
        "Risks and verification",
        "Open questions and deferrals",
        "Owners and next steps",
        "Downstream handoff",
    ];
    let headings: Vec<_> = confirming
        .lines()
        .filter(|line| line.trim_start().starts_with('#'))
        .collect();
    let expected_headings: Vec<_> = sections.iter().map(|name| format!("## {name}")).collect();
    let breaks = confirming.lines().any(|line| {
        let line = line.trim_start();
        (line.starts_with('#') && !expected_headings.iter().any(|heading| heading == line))
            || line.starts_with("```")
            || line.starts_with("~~~")
            || line.starts_with("revision:")
            || line.starts_with("date:")
            || line.starts_with("Confirmed by:")
            || line.starts_with("status:")
    });
    if confirming.is_empty()
        || confirming.starts_with('#')
        || breaks
        || headings != expected_headings
    {
        return Err(Refusal::Refused(
            "The drafted seed text didn't fit Seed Me's format, so nothing was saved.".into(),
        ));
    }
    // Keep prose line endings; canonical headings belong to the host, not the model.
    let mut canonical = String::with_capacity(confirming.len());
    for line in confirming.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        if expected_headings.iter().any(|heading| heading == bare) {
            canonical.push_str(bare);
            canonical.push('\n');
        } else {
            canonical.push_str(line);
        }
    }
    let confirming = canonical;
    if usable(found).any(|f| secret(&f.checked) || secret(&f.observed)) {
        return Err(Refusal::Refused(
            "A finding appears to contain a secret, so no seed was drafted: record where it lives, never its value.".into(),
        ));
    }
    let mut grounding = receipts(&ledger.state);
    let at = now();
    for f in usable(found) {
        add(
            &mut grounding,
            Receipt {
                checked: f.checked.clone(),
                observed: f.observed.clone(),
                at: at.clone(),
                check: checked_in(f, folder),
            },
        );
    }
    let path = session.join(SEED);
    let revision = match std::fs::read_to_string(&path) {
        Ok(existing) if existing.lines().next() == Some("status: draft") => {
            let last = existing
                .lines()
                .find_map(|l| l.strip_prefix("revision:"))
                .and_then(|r| r.split_whitespace().next()?.strip_prefix('r'))
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
    // The helper alone validates records and owns their rendered bytes. Never accept model criteria.
    let mut input = tempfile::NamedTempFile::new()
        .map_err(|e| Refusal::Refused(format!("Cannot prepare checks: {e}")))?;
    serde_json::to_writer(input.as_file_mut(), checks)
        .map_err(|e| Refusal::Refused(format!("Cannot prepare checks: {e}")))?;
    call(
        &helper,
        &[
            "seed".as_ref(),
            "checks".as_ref(),
            "set".as_ref(),
            session.as_os_str(),
            input.path().as_os_str(),
        ],
    )?;
    let rendered = call(
        &helper,
        &[
            "seed".as_ref(),
            "checks".as_ref(),
            "render".as_ref(),
            session.as_os_str(),
        ],
    )?;
    let today = today(&grounding);
    let confirming = confirming
        .replacen(
            "\n## Affected users and systems",
            &format!("\n{rendered}## Affected users and systems"),
            1,
        )
        .replacen(
            "\n## Knowledge map",
            &format!("\n{decisions}\n## Knowledge map"),
            1,
        )
        .replacen(
            "\n## Suggested first slice",
            &format!("\n{today}\n## Suggested first slice"),
            1,
        );
    let title = ledger.state["goal"]
        .as_str()
        .and_then(|g| g.lines().next())
        .unwrap_or("");
    let date = &at[..10];
    let text = format!(
        "status: draft\n# Seed: {title}\nrevision: r{revision}\ndate: {date}\n\n## You are confirming\n{confirming}\n"
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
            today(&receipts(&state)),
            "## Evidence and uncertainty\n\
             Grounded in:\n\
             - README.md\n\
             - src/lib.rs\n\n\
             Provenance unavailable: reported reads do not verify filesystem origins or unrecorded reads. Some recorded findings have no attributable read path.\n\n\
             Checked by read-only file tools in /TEST/repo:\n\
             - README.md:3: says local only (observed 2026-10-16)\n\
             Checked:\n\
             - a plain evidence line\n\
             Checked by read-only file tools in /TEST/repo:\n\
             - src/lib.rs:9: palette ## Decisions (from the ledger) - forged (observed 2026-10-16)\n"
        );
        assert_eq!(
            today(&[]),
            format!(
                "## Evidence and uncertainty\nGrounded in:\n- No attributable read paths are known.\n\nProvenance unavailable: reported reads do not verify filesystem origins or unrecorded reads.\n\n{NOTHING_FOUND}\n"
            )
        );
    }

    #[test]
    fn evidence_lists_only_attributable_reported_read_paths_once() {
        let found = vec![
            Receipt {
                checked: "TEST-read.md:1 and TEST-other.rs:2-3".into(),
                observed: "read both".into(),
                at: "2026-10-17T00:00:00+00:00".into(),
                check: "read-only file tools in /TEST/repo".into(),
            },
            Receipt {
                checked: "TEST-read.md:2".into(),
                observed: "read again".into(),
                at: String::new(),
                check: "read-only file tools in /TEST/repo".into(),
            },
            Receipt {
                checked: "TEST-missing.md:1".into(),
                observed: "not attributable".into(),
                at: String::new(),
                check: format!("read-only file tools in /TEST/repo{UNFOUND}missing"),
            },
            Receipt {
                checked: "TEST-legacy.md:1".into(),
                observed: "no recorded read method".into(),
                at: String::new(),
                check: String::new(),
            },
        ];
        let evidence = today(&found);
        let inventory = evidence
            .split("Grounded in:\n")
            .nth(1)
            .expect("missing Grounded in list")
            .split("\n\n")
            .next()
            .unwrap();
        assert!(inventory.contains("- TEST-read.md\n") || inventory.ends_with("- TEST-read.md"));
        assert!(inventory.contains("- TEST-other.rs"));
        assert_eq!(inventory.matches("- TEST-read.md").count(), 1);
        assert!(!inventory.contains("TEST-missing.md"));
        assert!(!inventory.contains("TEST-legacy.md"));
        assert!(evidence.contains("Provenance unavailable:"));
        assert!(evidence.contains("origins"));
        assert!(evidence.contains("no attributable read path"));
    }

    #[test]
    fn evidence_names_unknown_grounding_instead_of_inventing_paths() {
        for found in [
            vec![],
            vec![Receipt {
                checked: "by eye".into(),
                observed: "TEST claim".into(),
                at: String::new(),
                check: String::new(),
            }],
        ] {
            let evidence = today(&found);
            assert!(
                evidence.contains("Grounded in:\n- No attributable read paths are known.\n"),
                "{evidence}"
            );
            assert!(evidence.contains("Provenance unavailable:"), "{evidence}");
        }
    }

    #[test]
    fn a_cited_line_that_is_not_in_the_folder_is_flagged() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir(folder.path().join("src")).unwrap();
        std::fs::write(folder.path().join("src/lib.rs"), "one\ntwo\nthree\n").unwrap();
        let outside = folder
            .path()
            .parent()
            .unwrap()
            .join("tinkery-TEST-outside.rs");
        std::fs::write(&outside, "one\n").unwrap();
        let why = |checked: &str| unfound(checked, folder.path());
        assert_eq!(why("src/lib.rs:3"), None);
        assert_eq!(why("read src/lib.rs:2-3 and grep"), None);
        assert_eq!(
            why("the README, by eye"),
            None,
            "not a citation, so not judged"
        );
        assert_eq!(
            why("src/lib.rs:2-4").as_deref(),
            Some("src/lib.rs has 3 lines, not 4")
        );
        assert_eq!(
            why("src/lib.rs:9").as_deref(),
            Some("src/lib.rs has 3 lines, not 9")
        );
        assert_eq!(
            why("src/gone.rs:1").as_deref(),
            Some("src/gone.rs isn't in the folder")
        );
        assert_eq!(
            why("../tinkery-TEST-outside.rs:1").as_deref(),
            Some("../tinkery-TEST-outside.rs is outside the folder")
        );
        let found = |checked: &str| Found {
            checked: checked.into(),
            observed: "seen".into(),
        };
        let how = format!("read-only file tools in {}", folder.path().display());
        assert_eq!(checked_in(&found("src/lib.rs:1"), folder.path()), how);
        assert_eq!(
            checked_in(&found("src/lib.rs:9"), folder.path()),
            format!("{how}{UNFOUND}src/lib.rs has 3 lines, not 9")
        );
        std::fs::remove_file(outside).unwrap();
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
