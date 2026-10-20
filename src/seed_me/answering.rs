//! Changing a session as the operator works through Seed Me's interview: their answer, the next
//! question an agent drafted with the defaults it assumed, what a step found, an objection to an
//! assumption, a seed draft, the operator's seed confirmation, and ending the session when the
//! operator says so. A recorded open decision is asked before any new one.
//!
//! Findings become evidence-authority facts, with receipts naming the folder examined. Questions
//! cite those facts; the ledger alone validates claims, support and contradictions. Seed evidence
//! includes those facts and legacy receipts. Only the operator can keep a flagged answer with a
//! reason; model replies never revalidate or settle decisions.
//!
//! Shape's ledger writes are Tinkery's own. A stale displayed version is refused as
//! [`Refusal::Stale`]; nothing here retries, merges or overwrites. Seed decisions, checks,
//! confirmation and ending belong to Tinkery's ledger; the host supplies unconfirmed prose.
//! The confirmed goal never changes. Calls block; run them off the paint path.

use super::ledger;
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

mod seed_reply;

const SEED: &str = "seed-contract.md";

/// Why a change wasn't completed. A later IO failure can leave unconfirmed check records;
/// only the ledger's verified confirmation permits handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Another writer changed the session after the caller read it; the stale version
    /// was refused and nothing was written.
    Stale,
    /// Tinkery, or a check here, refused; the reason in plain words.
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
    /// Typed findings, validated only by the ledger.
    pub found: Vec<Found>,
    /// Fact IDs grounding this decision, passed unchanged to the ledger.
    pub supported_by: Option<Value>,
    /// Earlier decision IDs this question waits for, passed unchanged to the ledger.
    pub prerequisites: Option<Value>,
    /// A suggested answer, recorded as Seed Me's recommendation.
    pub suggestion: Option<String>,
    /// Defaults the step chose instead of asking, added to Seed Me's `assumed` list.
    pub assumed: Vec<Assumption>,
    /// The two ways to settle it, each with its trade-off and undo cost; empty when none were given.
    pub options: Vec<Alternative>,
    /// The strongest case against the suggestion.
    pub against: Option<String>,
    /// Reply confidence: low, medium or high; not recorded in the ledger.
    pub confidence: Option<String>,
}

/// One way to settle a decision, as Seed Me's question format names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternative {
    /// The choice.
    pub choice: String,
    /// Its benefit and cost compared with the other.
    pub tradeoff: String,
    /// How hard it is to undo: cheap, moderate or hard.
    pub undo: String,
}

/// The question's lines that name its options, then the case against the suggestion. Shape
/// reads them back from the question text; Seed Me's ledger has no options field.
pub const OPTION_LABELS: [&str; 2] = ["Option A: ", "Option B: "];
/// The line that carries the strongest case against the suggestion.
pub const AGAINST: &str = "Against my suggestion: ";

impl Ask {
    /// Whether either option is hard to undo, so the operator's instinct comes before any
    /// suggestion and the suggestion isn't recorded yet.
    pub fn hard(&self) -> bool {
        self.options.iter().any(|o| o.undo == "hard")
    }

    /// The question as Seed Me records it: the ask, then each option and the case against.
    fn text(&self) -> String {
        // The model may end its own words with punctuation; the line supplies its own.
        let bare = |s: &str| s.trim_end_matches(['.', ';', ',', ' ']).to_owned();
        let mut text = self.question.clone();
        for (label, o) in OPTION_LABELS.iter().zip(&self.options) {
            text.push_str(&format!(
                "\n{label}{}; {}. Undo cost: {}.",
                bare(&o.choice),
                bare(&o.tradeoff),
                o.undo
            ));
        }
        if let Some(against) = self.against.as_deref().filter(|_| !self.options.is_empty()) {
            text.push_str(&format!("\n{AGAINST}{against}"));
        }
        text
    }

    /// The suggestion unchanged, without reply confidence; blank suggestions are absent.
    pub fn recommendation(&self) -> Option<String> {
        self.suggestion
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
    }
}

/// A default chosen without asking, and why it doesn't need the operator. Never an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    /// What is assumed.
    pub text: String,
    /// Why both reasonable answers leave the work about the same.
    pub why: String,
}

/// A proposed fact with a receipt. Claim rules belong only to the ledger.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Found {
    /// A new fact ID, also usable by this reply's support references.
    pub id: String,
    /// The ledger's claim object, never inferred from prose.
    pub claim: Value,
    /// What was checked, such as a file and line.
    pub checked: String,
    /// The finding's answer: an observation, bounded inference or explicit unknown.
    pub observed: String,
    /// Observations supporting an inference, validated by the ledger.
    #[serde(default, deserialize_with = "present")]
    pub supported_by: Option<Value>,
    /// The existing node contradicted, validated by the ledger.
    #[serde(default, deserialize_with = "present")]
    pub contradicts: Option<Value>,
}

// Absence is optional; explicit null is still input for the ledger to validate, not a fallback.
fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    serde::Deserialize::deserialize(deserializer).map(Some)
}

/// One session's editable state, at one version.
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
    /// Typed facts with IDs and legacy receipts, excluding confirmed-goal provenance and history.
    pub fn findings(&self) -> Vec<Value> {
        let mut found: Vec<_> = nodes(&self.state)
            .into_iter()
            .filter(|n| n["kind"] == "fact" && n["status"] != "superseded")
            .collect();
        found.extend(
            receipts(&self.state, false)
                .into_iter()
                .map(|r| json!({"checked": r.checked, "observed": r.observed})),
        );
        found
    }
    /// Whether the ledger already has a current question; looking must not replace it.
    pub fn has_current_question(&self) -> bool {
        !self.state["current_question"].is_null()
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
fn receipts(state: &Value, include_facts: bool) -> Vec<Receipt> {
    let mut found: Vec<Receipt> = Vec::new();
    let all = nodes(state);
    let counts = |n: &&Value| n["status"] != "superseded" && n["id"] != state["origin"];
    for node in all.iter().filter(counts) {
        for entry in node["evidence"].as_array().into_iter().flatten() {
            // Delegation copies receipts, not new findings. Format each through its fact owner.
            let fact = if node["kind"] == "fact" {
                Some(node)
            } else {
                all.iter().filter(counts).find(|n| {
                    n["kind"] == "fact"
                        && n["evidence"]
                            .as_array()
                            .is_some_and(|entries| entries.contains(entry))
                })
            };
            if !include_facts && fact.is_some() {
                continue;
            }
            let field = |key: &str| entry[key].as_str().unwrap_or("").to_owned();
            let mut receipt = match entry {
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
            if let Some(fact) = fact.filter(|n| !n["claim"].is_null()) {
                receipt
                    .observed
                    .push_str(&claim_suffix(&fact["claim"], fact.get("supported_by")));
            }
            add(&mut found, receipt);
        }
    }
    found
}

/// The displayed claim type, scope and limits, with support IDs when requested.
pub(super) fn claim_suffix(claim: &Value, supported_by: Option<&Value>) -> String {
    let mut text = format!(
        " ({}; scope: {}",
        claim["type"].as_str().unwrap_or(""),
        claim["scope"].as_str().unwrap_or("")
    );
    if let Some(limits) = claim["limits"].as_str() {
        text.push_str(&format!("; limits: {limits}"));
    }
    if let Some(support) = supported_by.and_then(Value::as_array) {
        text.push_str(&format!(
            "; supported by: {}",
            support
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    text.push(')');
    text
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

/// Append proposed facts without guessing types, repairing references or duplicating ledger rules.
fn facts(state: &mut Value, found: &[Found], folder: &Path) {
    let at = now();
    for f in found {
        let mut node = json!({
            "id": f.id, "kind": "fact", "status": "settled", "prerequisites": [],
            "claim": f.claim, "answer": f.observed, "authority": "evidence",
            "authority_source": "read-only investigation in Tinkery Shape",
            "evidence": [receipt(f, &at, folder)],
        });
        if let Some(support) = &f.supported_by {
            node["supported_by"] = support.clone();
        }
        if let Some(target) = &f.contradicts {
            node["contradicts"] = target.clone();
        }
        state["nodes"].as_array_mut().unwrap().push(node);
    }
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

/// Prose-field instructions and installed section guidance and check schema, without conflicting
/// artifact-writing directions. Refuses a missing or incomplete reference. No helper is required.
pub fn contract(skill: &Path) -> Result<String, Refusal> {
    let skill = skill.canonicalize().map_err(|_| {
        Refusal::Refused(
            "Cannot locate Shape instructions. Check the configured skill path.".into(),
        )
    })?;
    let reference = skill
        .parent()
        .ok_or_else(|| Refusal::Refused("Shape instructions have no directory".into()))?
        .join("references/seed-contract.md");
    let reference = std::fs::read_to_string(reference)
        .map_err(|_| Refusal::Refused("Cannot read Shape's seed contract. Check the configured reference file and its read permission.".into()))?;
    seed_reply::instructions(&reference)
}

/// A refusal from Tinkery's own ledger, which saved nothing; it never credits Seed Me. An
/// unknown save is no refusal: it asks the operator to look before trying again.
fn not_saved(reason: String) -> Refusal {
    published(ledger::PublishError::from(reason))
}

fn published(error: ledger::PublishError) -> Refusal {
    let reason = error.reason.lines().next().unwrap_or("");
    match error.save_state {
        _ if reason.contains("stale publication version") => Refusal::Stale,
        ledger::SaveState::NotSaved => Refusal::Refused(format!("Tinkery refused: {reason}")),
        ledger::SaveState::Unknown => Refusal::Refused(format!(
            "This may or may not have saved ({}). Reopen this session to see what it holds before trying again.",
            reason
                .strip_prefix("could not save the ledger: ")
                .unwrap_or(reason)
        )),
    }
}

fn json(text: &str) -> Result<Value, Refusal> {
    serde_json::from_str(text).map_err(|e| Refusal::Refused(format!("Invalid session JSON: {e}")))
}

/// Read `session`'s editable ledger at one version, without writing or calling the helper.
pub fn read(_skill: &Path, session: &Path) -> Result<Ledger, Refusal> {
    let unreadable =
        |reason: String| Refusal::Refused(format!("Cannot read this session: {reason}"));
    let saved = ledger::load(session).map_err(unreadable)?;
    let state = ledger::editable(&saved).map_err(unreadable)?;
    let version = saved["version"]
        .pretty()
        .map_err(unreadable)?
        .trim()
        .parse()
        .map_err(|_| unreadable("the ledger has no usable version".into()))?;
    Ok(Ledger {
        version,
        state: json(&state.pretty().map_err(unreadable)?)?,
    })
}

/// Publish `state` as the change from `expected`; returns the new version.
fn publish(session: &Path, expected: u64, reason: &str, state: &Value) -> Result<u64, Refusal> {
    publish_at(
        session,
        expected,
        reason,
        state,
        &publication_time(SystemTime::now())?,
    )
}

fn ledger_value(value: &Value) -> Result<ledger::Value, Refusal> {
    let unusable = || {
        not_saved(
            "The question or session could not be read. Reopen the session and retry the step."
                .into(),
        )
    };
    let text = serde_json::to_string(value).map_err(|_| unusable())?;
    ledger::Value::parse(&text).map_err(|_| unusable())
}

fn publish_at(
    session: &Path,
    expected: u64,
    reason: &str,
    state: &Value,
    at: &str,
) -> Result<u64, Refusal> {
    // Shape edits sorted serde_json maps, just as the former helper input did.
    let state = ledger_value(state)?;
    let saved = ledger::publish(
        session,
        &state,
        &ledger::Value::Int(expected.to_string()),
        &reason.into(),
        None,
        at,
    )
    .map_err(published)?;
    // The ledger is saved by now, so an unusable version must not read as "not saved".
    let unknown = |reason: String| {
        published(ledger::PublishError {
            reason,
            save_state: ledger::SaveState::Unknown,
        })
    };
    saved["version"]
        .pretty()
        .map_err(unknown)?
        .trim()
        .parse()
        .map_err(|_| unknown("the saved ledger has no usable version".into()))
}

fn publication_time(time: SystemTime) -> Result<String, Refusal> {
    let elapsed = time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| not_saved(e.to_string()))?;
    let secs = elapsed.as_secs();
    let rest = secs % 86_400;
    let micros = elapsed.subsec_micros();
    let fraction = if micros == 0 {
        String::new()
    } else {
        format!(".{micros:06}")
    };
    Ok(format!(
        "{}T{:02}:{:02}:{:02}{fraction}+00:00",
        civil(secs as i64),
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    ))
}

#[cfg(test)]
mod answer_actions_tests;
#[cfg(test)]
mod prerequisite_tests;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod typed_tests;

/// Settle the open question `id` with the operator's `words`, exactly as typed, from the version
/// the operator saw (`expected`). Clears the current question. Returns the new version.
/// Refuses a question that isn't open or a stale version.
pub fn answer(
    skill: &Path,
    session: &Path,
    expected: u64,
    id: &str,
    words: &str,
) -> Result<u64, Refusal> {
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
        session,
        expected,
        &format!("The operator answered {id} in Tinkery Shape."),
        &ledger.state,
    )
}

/// Publish an explicit deferral, delegation or replacement answer. Eligibility, required fields
/// and dependent reopening belong only to the ledger; a refusal preserves the displayed words.
pub fn respond(
    session: &Path,
    expected: u64,
    id: &str,
    action: &ledger::AnswerAction,
) -> Result<u64, Refusal> {
    let saved = ledger::respond(
        session,
        expected,
        id,
        action,
        &publication_time(SystemTime::now())?,
    )
    .map_err(published)?;
    saved["version"]
        .pretty()
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| {
            Refusal::Refused(
                "The answer may have saved. Reopen the session before trying again.".into(),
            )
        })
}

/// Record `ask` and its findings checked in `folder`, from `expected`. The ledger refuses invalid
/// facts or prerequisites atomically and selects the ready question; a held question stays recorded.
/// Returns the new version.
pub fn ask(
    skill: &Path,
    session: &Path,
    expected: u64,
    ask: &Ask,
    folder: &Path,
) -> Result<u64, Refusal> {
    let mut ledger = read(skill, session)?;
    let taken: Vec<String> = nodes(&ledger.state)
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect();
    let id = unique_id(&ask.id, &taken);
    facts(&mut ledger.state, &ask.found, folder);
    let mut node = json!({
        "id": id, "kind": "decision", "status": "unresolved",
        "prerequisites": ask.prerequisites.clone().unwrap_or_else(|| json!([])),
        "evidence": [], "owner": "User", "gate": ask.gate, "label": ask.label,
        "question": ask.text(),
    });
    if let Some(support) = &ask.supported_by {
        node["supported_by"] = support.clone();
    }
    // Seed Me: a hard-to-undo question is published without its recommendation until the
    // operator answers or asks to see it, so the suggestion can't anchor their instinct.
    if let Some(recommendation) = ask.recommendation().filter(|_| !ask.hard()) {
        node["recommendation"] = recommendation.into();
    }
    let mut state = ledger_value(&ledger.state)?;
    ledger::queue_question(&mut state, ledger_value(&node)?).map_err(not_saved)?;
    ledger.state = json(&state.pretty().map_err(not_saved)?)?;
    let mut assumed = ledger.state["assumed"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for a in ask
        .assumed
        .iter()
        .filter(|a| !a.text.trim().is_empty() && !a.why.trim().is_empty())
    {
        if !assumed.iter().any(|known| known["text"] == a.text.as_str()) {
            assumed.push(json!({"text": a.text, "why": a.why}));
        }
    }
    if !assumed.is_empty() {
        ledger.state["assumed"] = Value::Array(assumed);
    }
    publish(
        session,
        expected,
        &format!("Tinkery Shape's agent step drafted the next question, {id}."),
        &ledger.state,
    )
}

/// Keep a flagged node's answer unchanged, publishing only the operator's review reason.
/// The ledger alone validates the target, reason and displayed version; no model runs.
pub fn keep(session: &Path, expected: u64, id: &str, reason: &str) -> Result<(), Refusal> {
    ledger::revalidate(
        session,
        expected,
        id,
        reason,
        &publication_time(SystemTime::now())?,
    )
    .map_err(published)
}

/// Record `recommendation` on open question `id`, from `expected`, once the operator asks to see
/// what a hard-to-undo question withheld. Returns the new version.
pub fn recommend(
    skill: &Path,
    session: &Path,
    expected: u64,
    id: &str,
    recommendation: &str,
) -> Result<u64, Refusal> {
    let mut ledger = read(skill, session)?;
    let node = ledger.state["nodes"]
        .as_array_mut()
        .and_then(|nodes| nodes.iter_mut().find(|n| n["id"] == id))
        .filter(|n| n["status"] == "unresolved")
        .ok_or_else(|| Refusal::Refused(format!("The question {id} is no longer open.")))?;
    node["recommendation"] = recommendation.into();
    publish(
        session,
        expected,
        &format!("The operator asked to see the suggestion for {id} in Tinkery Shape."),
        &ledger.state,
    )
}

/// The operator objects to assuming `text`, from `expected`: it leaves Seed Me's `assumed` list
/// and becomes an open decision, which blocks drafting until it's answered. Returns the new version.
/// Refuses an assumption that isn't recorded.
pub fn object(skill: &Path, session: &Path, expected: u64, text: &str) -> Result<u64, Refusal> {
    let mut ledger = read(skill, session)?;
    let mut assumed = ledger.state["assumed"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let before = assumed.len();
    assumed.retain(|a| a["text"] != text);
    if assumed.len() == before {
        return Err(Refusal::Refused(
            "That assumption is no longer recorded.".into(),
        ));
    }
    ledger.state["assumed"] = Value::Array(assumed);
    let taken: Vec<String> = nodes(&ledger.state)
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect();
    let id = unique_id(&format!("instead-of-{text}"), &taken);
    let node = json!({
        "id": id, "kind": "decision", "status": "unresolved", "prerequisites": [],
        "evidence": [], "owner": "User", "gate": format!("Choose instead of assuming: {text}"),
        "label": OBJECTION, "question": format!("You objected to assuming: {text}. What should it be instead?"),
    });
    ledger.state["nodes"]
        .as_array_mut()
        .ok_or_else(|| Refusal::Refused("The session has no decisions list.".into()))?
        .push(node);
    publish(
        session,
        expected,
        &format!("The operator objected to an assumption in Tinkery Shape; it is now {id}."),
        &ledger.state,
    )
}

/// The label of a decision recorded from an objection to an assumption.
pub const OBJECTION: &str = "Instead of assuming";

/// The probe Seed Me requires the operator to answer before the seed: its ledger id and wording.
pub const PROBE: (&str, &str) = (
    "probe-pre-mortem",
    "Imagine this shipped and turned out wrong. What is the most likely reason?",
);

/// With no current question and the pre-mortem never asked, make it the current question, from
/// `expected`, so the operator answers it in their own words before any seed. Said plainly when
/// findings were already shown, since the answer is then not independent of them. Returns the new
/// version, or `None` when it was asked before or a question is current.
pub fn probe(skill: &Path, session: &Path, expected: u64) -> Result<Option<u64>, Refusal> {
    let mut ledger = read(skill, session)?;
    if ledger.version != expected {
        return Err(Refusal::Stale);
    }
    let (id, question) = PROBE;
    if !ledger.state["current_question"].is_null()
        || nodes(&ledger.state).iter().any(|n| n["id"] == id)
    {
        return Ok(None);
    }
    let question = if ledger.findings().is_empty() {
        question.to_owned()
    } else {
        format!("{question}\nNOT independent: asked after findings were shown.")
    };
    let node = json!({
        "id": id, "kind": "decision", "status": "unresolved", "prerequisites": [],
        "evidence": [], "owner": "User", "gate": "Your own answer to the pre-mortem, or skip it",
        "label": "Probe: pre-mortem", "question": question,
    });
    ledger.state["nodes"]
        .as_array_mut()
        .ok_or_else(|| Refusal::Refused("The session has no decisions list.".into()))?
        .push(node);
    ledger.state["current_question"] = id.into();
    publish(
        session,
        expected,
        "Shape puts the pre-mortem probe to the operator.",
        &ledger.state,
    )
    .map(Some)
}

/// With no current question, make the first open decision whose prerequisites are settled
/// current, from `expected`, so a decision already recorded is asked before any new one.
/// Returns the new version, or `None` when there's a current question or nothing is ready.
pub fn promote(skill: &Path, session: &Path, expected: u64) -> Result<Option<u64>, Refusal> {
    let ledger = read(skill, session)?;
    if ledger.version != expected {
        return Err(Refusal::Stale);
    }
    if !ledger.state["current_question"].is_null() {
        return Ok(None);
    }
    let Some(id) = ledger::next_question(&ledger_value(&ledger.state)?).map_err(not_saved)? else {
        return Ok(None);
    };
    let mut state = ledger.state;
    state["current_question"] = id.clone().into();
    publish(
        session,
        expected,
        &format!("Tinkery Shape asks the open decision {id} before any new one."),
        &state,
    )
    .map(Some)
}

/// Save a Seed Me draft: `confirming` supplies named summary and prose fields, never headings or criteria. `checks`
/// are validated, stored and rendered only by Tinkery's ledger; no check is executed. Evidence
/// comes from saved findings and what this step `found` in `folder`, and accepted decisions from
/// `seed decisions` unchanged, in Seed Me's canonical Full sequence with no wrapper headings.
/// Returns the revision label, from version `expected`.
/// Refuses a stale version, open decisions, a confirmed seed (never replaced), text that would
/// change the seed's structure, invalid records and a finding that looks like a secret.
/// IO failures after storing records can leave unconfirmed records, never a ready seed.
pub fn draft_seed(
    skill: &Path,
    session: &Path,
    expected: u64,
    confirming: &Value,
    checks: &Value,
    found: &[Found],
    folder: &Path,
) -> Result<String, Refusal> {
    let mut ledger = read(skill, session)?;
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
    let prose = seed_reply::prose(confirming)?;
    facts(&mut ledger.state, found, folder);
    let grounding = receipts(&ledger.state, true);
    let path = session.join(SEED);
    let previous =
        match std::fs::read_to_string(&path) {
            Ok(existing) => Some(existing),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(Refusal::Refused(
                "Cannot read seed-contract.md. Check its text and read permission before retrying."
                    .into(),
            )),
        };
    let revision = match previous.as_deref() {
        Some(existing) if existing.lines().next() == Some("status: draft") => {
            let last = existing
                .lines()
                .find_map(|l| l.strip_prefix("revision:"))
                .and_then(|r| r.split_whitespace().next()?.strip_prefix('r'))
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0);
            last + 1
        }
        Some(_) => {
            return Err(Refusal::Refused(
                "A seed that isn't a draft already exists; Shape never replaces it.".into(),
            ));
        }
        None => 1,
    };
    let records = ledger::Value::parse(&checks.to_string()).map_err(not_saved)?;
    let rendered = ledger::checks_block(&records).map_err(not_saved)?;
    let version = if found.is_empty() {
        expected
    } else {
        publish(
            session,
            expected,
            "Shape recorded the seed step's typed findings.",
            &ledger.state,
        )?
    };
    let decisions =
        ledger::decisions_block(&ledger::load(session).map_err(not_saved)?).map_err(not_saved)?;
    let today = today(&grounding);
    let mut body = String::new();
    for heading in ledger::FULL {
        match *heading {
            "## Acceptance criteria" => body.push_str(&rendered),
            "## Decisions (from the ledger)" => {
                body.push_str(&decisions);
                body.push('\n');
            }
            "## Assumed (not confirmed by you)" => {}
            "## Evidence and uncertainty" => {
                body.push_str(&today);
                body.push('\n');
            }
            heading => {
                let name = heading.strip_prefix("## ").unwrap();
                let text = prose.iter().find(|(field, _)| *field == name).unwrap().1;
                body.push_str(heading);
                body.push('\n');
                body.push_str(text);
                if !text.ends_with('\n') {
                    body.push('\n');
                }
                body.push('\n');
            }
        }
    }
    let title = ledger.state["goal"]
        .as_str()
        .and_then(|g| g.lines().next())
        .unwrap_or("");
    let date = local_date();
    let text =
        format!("status: draft\n# Seed: {title}\nrevision: r{revision}\ndate: {date}\n\n{body}");
    ledger::save_draft(session, version, previous.as_deref(), &records, &text)
        .map_err(not_saved)?;
    Ok(format!("r{revision}"))
}

/// Confirm seed `revision` through the ledger, with the operator's `words` as its source.
/// Tinkery refuses a simulated session, a different revision, open decisions and a seed whose
/// decisions don't match the ledger.
pub fn confirm_seed(
    _skill: &Path,
    session: &Path,
    revision: &str,
    words: &str,
) -> Result<(), Refusal> {
    if words.trim().is_empty() || words.contains(['\n', '\r']) {
        return Err(Refusal::Refused(
            "Tinkery needs your words on one line.".into(),
        ));
    }
    ledger::confirm_seed(
        session,
        revision,
        words,
        &publication_time(SystemTime::now())?,
    )
    .map(|_| ())
    .map_err(|reason| Refusal::Refused(format!("Tinkery refused: {reason}")))
}

/// How the operator ends a session: Seed Me's two end statuses. Either is for good.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The seed is confirmed and verified; the interview is finished.
    Completed,
    /// The operator stopped the interview; open questions stay recorded.
    Stopped,
}

/// End `session` through the ledger, after which it is read-only. Refuse completing a seed
/// Tinkery can't verify or a conflicting ended status; completion never needs a viewer.
pub fn end(_skill: &Path, session: &Path, ending: Ending) -> Result<(), Refusal> {
    let (status, reason) = match ending {
        Ending::Completed => (
            "completed",
            "The operator ended the session in Tinkery Shape after confirming the seed.",
        ),
        Ending::Stopped => (
            "stopped",
            "The operator stopped the interview in Tinkery Shape.",
        ),
    };
    ledger::end(
        session,
        status,
        reason,
        &publication_time(SystemTime::now())?,
    )
    .map(|_| ())
    .map_err(published)
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
    let rest = secs % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}+00:00",
        civil(secs as i64),
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Today's date where the operator is, for the seed's `date:` line; receipts keep UTC.
fn local_date() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    civil(secs + utc_offset(secs))
}

/// The local time zone's offset from UTC at `secs`, in seconds; 0 when it can't be read.
#[cfg(unix)]
fn utc_offset(secs: i64) -> i64 {
    let time = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are valid for the call; localtime_r writes only `tm`.
    if unsafe { libc::localtime_r(&time, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i64
}

#[cfg(not(unix))]
fn utc_offset(_secs: i64) -> i64 {
    0
}

/// The `YYYY-MM-DD` date of `secs` after 1970-01-01 (Howard Hinnant's algorithm).
fn civil(secs: i64) -> String {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_reads_the_installed_reference_without_a_helper_script() {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-contract-")
            .tempdir()
            .unwrap();
        let skill = root.path().join("SKILL.md");
        let reference = root.path().join("references/seed-contract.md");
        std::fs::write(&skill, "TEST skill\n").unwrap();
        std::fs::create_dir(reference.parent().unwrap()).unwrap();
        let installed = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".ci-seed-me/skills/seed-me/references/seed-contract.md"),
        )
        .unwrap();
        let expected = installed.replace(
            "Problem statement: current behavior",
            "Problem statement: Keep café words verbatim. Current behavior",
        );
        std::fs::write(&reference, expected).unwrap();

        assert!(!root.path().join("scripts/session.py").exists());
        let instructions = contract(&skill).unwrap();
        assert!(instructions.contains("Keep café words verbatim."));
        assert!(instructions.contains("Seed reply fields"));
        assert!(!instructions.contains("Choose Full or Lean"));
    }

    #[test]
    fn the_seed_is_dated_where_the_operator_is_and_receipts_stay_utc() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(-1), "1969-12-31");
        assert_eq!(civil(951_782_400), "2000-02-29");
        assert!(now().ends_with("+00:00"), "receipts stay UTC");
        // The operator's own clock says what today is; retry once across midnight.
        let today = || {
            let out = std::process::Command::new("date")
                .arg("+%F")
                .output()
                .unwrap();
            String::from_utf8(out.stdout).unwrap().trim().to_owned()
        };
        let (before, ours, after) = (today(), local_date(), today());
        assert!(
            ours == before || ours == after,
            "{ours} vs {before}/{after}"
        );
    }

    #[test]
    fn option_lines_keep_single_punctuation_whatever_the_model_ends_with() {
        let option = |choice: &str, tradeoff: &str| Alternative {
            choice: choice.into(),
            tradeoff: tradeoff.into(),
            undo: "cheap".into(),
        };
        let ask = Ask {
            prerequisites: None,
            supported_by: None,
            id: "view".into(),
            label: "TEST view".into(),
            question: "TEST Where do ended goals go?".into(),
            gate: "TEST view".into(),
            found: vec![],
            suggestion: None,
            assumed: vec![],
            options: vec![
                option("A separate view.", "TEST keeps the garden calm."),
                option("Mixed in;", "TEST one list, "),
            ],
            against: None,
            confidence: None,
        };
        assert_eq!(
            ask.text(),
            "TEST Where do ended goals go?\n\
             Option A: A separate view; TEST keeps the garden calm. Undo cost: cheap.\n\
             Option B: Mixed in; TEST one list. Undo cost: cheap."
        );
    }

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
            today(&receipts(&state, true)),
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
            id: "TEST-fact".into(),
            claim: serde_json::json!({"type":"observation","scope":"TEST file"}),
            supported_by: None,
            contradicts: None,
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
