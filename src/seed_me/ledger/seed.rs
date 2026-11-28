//! Seed bytes, decisions and confirmation receipts share one verifier. Reads never upgrade old receipts.

use super::{
    Result, Value, checks, editable, load, require, review_flags,
    text::{text, whitespace},
    value::{int, object},
};
use crate::confirmation::{Actor, GoalConfirmation};
use crate::seed_me::files::{
    CONFIRMATION, CONFIRMED, DRAFT, SEED as FILE, SIMULATED, Section, revision_line as revision,
};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
pub(super) fn split_terminator(line: &str) -> (&str, &str) {
    let body = if let Some(body) = line.strip_suffix('\n') {
        body.strip_suffix('\r').unwrap_or(body)
    } else {
        line
    };
    (body, &line[body.len()..])
}
pub(super) fn seed_lines(directory: &Path) -> Result<Vec<String>> {
    let path = directory.join(FILE);
    require(
        path.is_file(),
        format!(
            "No Tink Contract found. Save it at {} first.",
            path.display()
        ),
    )?;
    Ok(crate::seed_me::files::read_seed_text(&path)
        .map_err(|e| e.to_string())?
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect())
}
pub(super) fn repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = quote.to_string();
    for c in value.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if !super::text::printable(c) => {
                let code = c as u32;
                if code <= 0xff {
                    out.push_str(&format!("\\x{code:02x}"));
                } else if code <= 0xffff {
                    out.push_str(&format!("\\u{code:04x}"));
                } else {
                    out.push_str(&format!("\\U{code:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn require_seed_body(directory: &Path, refusal: &str) -> Result<()> {
    let lines = seed_lines(directory)?;
    let lines: Vec<_> = lines.iter().map(|s| split_terminator(s).0).collect();
    let fence = Regex::new(r"^ {0,3}(```|~~~)").unwrap();
    let mut inside = false;
    let fenced: Vec<_> = lines
        .iter()
        .map(|s| {
            if fence.is_match(s) {
                inside = !inside;
                false
            } else {
                inside
            }
        })
        .collect();
    let index = lines.iter().enumerate().position(|(i, s)| {
        i > 0 && !fenced[i] && s.trim_end_matches(whitespace) == Section::Confirming.heading()
    });

    let valid = index.is_some_and(|index| {
        (1..index).any(|i| !fenced[i] && revision(lines[i]).is_some())
            && (index + 1..lines.len())
                .take_while(|i| !lines[*i].starts_with('#') || fenced[*i])
                .any(|i| {
                    !lines[i].trim_matches(whitespace).is_empty() && revision(lines[i]).is_none()
                })
    });
    require(
        valid,
        format!(
            "{refusal}: {} is not a Tink Contract; after its status line it needs a 'revision:' line and a '## You are confirming' section with content",
            directory.join(FILE).display()
        ),
    )
}
pub(super) fn require_seed_line(directory: &Path, expected: &str) -> Result<()> {
    let path = directory.join(FILE);
    require(
        path.is_file(),
        format!(
            "cannot complete: {} is missing; save the Tink Contract there with line 1 {}",
            path.display(),
            repr(expected)
        ),
    )?;
    let lines = seed_lines(directory)?;
    let first = lines.first().map_or("", |s| split_terminator(s).0);
    require(
        first == expected,
        format!(
            "cannot complete: line 1 of {} must be exactly {} for this session, found {}",
            path.display(),
            repr(expected),
            repr(first)
        ),
    )?;
    require_seed_body(directory, "cannot complete")
}

fn decisions_digest(ledger: &Value) -> Result<String> {
    let state = editable(ledger)?;
    let covered = object(
        &["draft", "goal", "origin", "nodes", "assumed", "operator"].map(|k| (k, state[k].clone())),
    );
    Ok(sha256(covered.canonical(false)?.as_bytes()))
}
fn one_line(s: &str) -> String {
    s.split(whitespace)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn decisions_lines(ledger: &Value) -> Result<Vec<(String, String)>> {
    let mut rows = vec![("heading".into(), "## Decisions (from the ledger)".into())];
    for node in ledger["nodes"].items() {
        if super::resolved_unknown(node) {
            continue;
        }
        let after = if node["prerequisites"].truth() {
            format!(
                " (after: {})",
                node["prerequisites"]
                    .items()
                    .iter()
                    .map(|s| s.string().unwrap())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        };
        let id = node["id"].string().unwrap();
        if node["status"].is("settled") {
            rows.push((
                id.into(),
                format!(
                    "- {id} [{}] {} (source: {}){after}",
                    node["authority"].string().unwrap(),
                    one_line(node["answer"].string().unwrap()),
                    one_line(node["authority_source"].string().unwrap())
                ),
            ));
            if let Some(d) = node.get("delegation") {
                rows.push((
                    id.into(),
                    format!(
                        "  Delegation (caller-declared, not authenticated): {}",
                        d.canonical(true)?
                    ),
                ));
            }
        } else if node["status"].is("deferred") {
            rows.push((
                id.into(),
                format!(
                    "- {id} deferred: {} (revisit if: {}){after}",
                    one_line(node["defer_reason"].string().unwrap()),
                    one_line(node["revisit_condition"].string().unwrap())
                ),
            ));
        } else {
            continue;
        }
        if id == super::UNOBSERVED_NEED {
            rows.push((
                id.into(),
                "  Unknown: nobody has observed this need.".into(),
            ));
        }
    }
    if let Some(base) = crate::seed_me::retry::base(ledger)? {
        rows.push((
            "starting commit".into(),
            format!("- Starting commit from Taste: {}", base.head),
        ));
    }
    rows.extend([
        ("heading".into(), "".into()),
        ("heading".into(), "## Assumed (not confirmed by you)".into()),
    ]);
    if !ledger["assumed"].truth() {
        rows.push(("assumed".into(), "- none".into()));
    }
    for a in ledger["assumed"].items() {
        let line = one_line(a["text"].string().unwrap());
        rows.push((
            format!(
                "assumed {}",
                repr(&line.chars().take(40).collect::<String>())
            ),
            format!("- {line} (why: {})", one_line(a["why"].string().unwrap())),
        ));
    }
    Ok(rows)
}

/// Only complete goal projections are literal: their originals remain unchanged.
pub(super) fn goal_lines(ledger: &Value) -> Result<Vec<String>> {
    let title = ledger["goal"]
        .string()
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("");
    let mut lines = vec![format!("# Seed: {title}")];
    lines.extend(
        decisions_lines(ledger)?
            .into_iter()
            .filter(|(id, _)| Some(id.as_str()) == ledger["origin"].string())
            .map(|(_, line)| line),
    );
    Ok(lines)
}

/// The ledger-owned Decisions and Assumed sections, verbatim and read-only.
pub fn decisions_block(ledger: &Value) -> Result<String> {
    Ok(decisions_lines(ledger)?
        .into_iter()
        .map(|(_, s)| s + "\n")
        .collect())
}
pub(super) fn require_seed_decisions(
    directory: &Path,
    ledger: &Value,
    refusal: &str,
) -> Result<()> {
    let lines = seed_lines(directory)?;
    let lines: Vec<_> = lines.iter().map(|s| split_terminator(s).0).collect();
    let normalized = format!("\n{}\n", lines.join("\n"));
    if normalized.contains(&format!("\n{}", decisions_block(ledger)?)) {
        return Ok(());
    }
    let stale: Vec<_> = decisions_lines(ledger)?
        .into_iter()
        .filter(|(label, line)| label != "heading" && !lines.contains(&line.as_str()))
        .map(|(label, _)| label)
        .collect();
    let detail = if stale.is_empty() {
        "the block is incomplete or out of order".into()
    } else {
        format!("not current: {}", stale.join(", "))
    };
    Err(format!(
        "{refusal}: the Tink Contract's decisions do not match the ledger ({detail}); reopen Shape and draft again from the current ledger"
    ))
}

/// Read a saved regular contract verbatim. A missing contract has no bytes.
pub(crate) fn read_seed_bytes(directory: &Path) -> Result<Option<Vec<u8>>> {
    let path = directory.join(FILE);
    if path.is_file() {
        crate::seed_me::files::read_seed(&path)
            .map(Some)
            .map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

fn summary(bytes: Option<&[u8]>) -> Value {
    let Some(bytes) = bytes else {
        return Value::Null;
    };
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.split('\n');
    let first = lines.next().unwrap_or_default().trim_end_matches('\r');
    let label = lines.find_map(revision);
    object(&[
        ("line_1", first.into()),
        ("revision", label.map_or(Value::Null, Value::from)),
        ("sha256", sha256(bytes).as_str().into()),
    ])
}

/// Summarize the saved bytes without claiming confirmation or repairing invalid text.
pub fn seed_summary(directory: &Path) -> Result<Value> {
    Ok(summary(read_seed_bytes(directory)?.as_deref()))
}

/// Acceptance records and their exact section in the captured contract, including its heading.
pub(crate) struct Acceptance {
    /// The saved check records that produced this section.
    pub(crate) records: Value,
    /// UTF-8 byte range in the contract that verified these records.
    pub(crate) section: std::ops::Range<usize>,
}

fn matching_checks(
    directory: &Path,
    ledger: &Value,
    body: &str,
    allow_legacy: bool,
) -> Result<Acceptance> {
    let records = checks::read_checks(directory)?;
    let section = checks::require_match(body, ledger, &records, allow_legacy)?;
    Ok(Acceptance { records, section })
}

fn confirmation_record(directory: &Path) -> Option<(Value, Option<GoalConfirmation>)> {
    let bytes =
        super::super::read_regular(&directory.join(CONFIRMATION), "confirmation record").ok()?;
    let record = Value::parse(std::str::from_utf8(&bytes).ok()?).ok()?;
    let fields = [
        "schema_version",
        "checks_sha256",
        "session_id",
        "operator",
        "revision",
        "source",
        "confirmed_at",
        "seed_sha256",
        "decisions_sha256",
    ];
    let mut allowed = fields.to_vec();
    let actor = if record["schema_version"] == int(3) && record["operator"].is("agent") {
        let actor: GoalConfirmation =
            serde_json::from_value(record["actor"].to_json().ok()?).ok()?;
        actor.actor().ok()?.context()?;
        allowed.push("actor");
        Some(actor)
    } else {
        None
    };
    (super::exact(&record, &allowed)
        && (actor.is_some() || record["schema_version"] == int(2))
        && fields
            .iter()
            .filter(|k| **k != "schema_version")
            .all(|k| text(&record[k])))
    .then_some((record, actor))
}

/// Whether a session's confirmed Tink still matches its receipt, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// True only when the receipt, seed bytes, decisions and checks all match.
    pub verified: bool,
    /// The confirming operator, present only on a verified handoff.
    pub operator: Option<String>,
    /// Why the handoff is or is not verified, in the words to show.
    pub reason: String,
}

impl Handoff {
    fn value(&self) -> Value {
        let mut value = object(&[("verified", Value::Bool(self.verified))]);
        if let Some(operator) = &self.operator {
            value.set("operator", operator.as_str().into());
        }
        value.set("reason", self.reason.as_str().into());
        value
    }
}

/// Verify matching operator or named-agent receipts, seed bytes, decisions and acceptance records.
/// Legacy or interrupted confirmation stays unverified; this proves content, not identity or truth.
pub fn verify_handoff(directory: &Path, ledger: &Value) -> Result<Handoff> {
    if let Some(practice) = practice_handoff(ledger) {
        return Ok(practice);
    }
    let bytes = read_seed_bytes(directory)?;
    verify_saved(
        directory,
        ledger,
        &summary(bytes.as_deref()),
        bytes.as_deref(),
    )
    .map(|(handoff, _)| handoff)
}

fn practice_handoff(ledger: &Value) -> Option<Handoff> {
    (!super::one_of(&super::operator(ledger), &["human", "agent"])).then(|| Handoff {
        verified: false,
        operator: None,
        reason: "a simulated session is a practice run that no person confirmed".into(),
    })
}

fn verify_saved(
    directory: &Path,
    ledger: &Value,
    seed: &Value,
    bytes: Option<&[u8]>,
) -> Result<(Handoff, Option<Acceptance>)> {
    let refused = |reason: String| {
        Ok((
            Handoff {
                verified: false,
                operator: None,
                reason,
            },
            None,
        ))
    };
    let Some(bytes) = bytes else {
        return refused("no Tink Contract is saved".into());
    };
    if !seed["line_1"].is(CONFIRMED) {
        return refused(format!(
            "the Tink is not confirmed (line 1 is {})",
            repr(seed["line_1"].string().ok_or("Invalid seed status line.")?)
        ));
    }
    let Some((record, actor)) = confirmation_record(directory) else {
        return refused(
            "the Tink has no confirmation record from `seed confirm` in this session".into(),
        );
    };
    if record["session_id"] != ledger["session_id"]
        || !(record["operator"].is("human")
            || (record["schema_version"] == int(3) && record["operator"].is("agent")))
    {
        return refused("the confirmation record belongs to another session".into());
    }
    let Some(label) = record["revision"].string() else {
        return refused("the confirmation revision is missing".into());
    };
    let body = std::str::from_utf8(bytes);
    if let Some(actor) = actor {
        let (Some(source), Some(timestamp)) =
            (record["source"].string(), record["confirmed_at"].string())
        else {
            return refused("the confirmation source or date is missing".into());
        };
        let expected = confirmation_note(source, label, timestamp, actor.actor()?)?;
        if !body
            .map_err(|_| "stream did not contain valid UTF-8")?
            .split_inclusive('\n')
            .any(|line| split_terminator(line).0 == expected)
        {
            return refused("the agent attribution does not match the confirmed Tink".into());
        }
    }
    if record["seed_sha256"] != seed["sha256"] {
        return refused(format!(
            "the Tink Contract changed after revision {label} was confirmed"
        ));
    }
    if record["decisions_sha256"].string() != Some(&decisions_digest(ledger)?) {
        return refused(format!(
            "the decisions changed after revision {label} was confirmed"
        ));
    }
    let acceptance = body
        .ok()
        .and_then(|body| matching_checks(directory, ledger, body, true).ok());
    let Some(acceptance) = acceptance else {
        return refused(
            "checks are missing, invalid or differ from the rendered Tink Contract".into(),
        );
    };
    if record["checks_sha256"].string() != Some(&checks::digest(&acceptance.records)?) {
        return refused(format!(
            "the checks changed after revision {label} was confirmed"
        ));
    }
    Ok((
        Handoff {
            verified: true,
            operator: record["operator"].string().map(str::to_owned),
            reason: format!("revision {label} as confirmed"),
        },
        Some(acceptance),
    ))
}

/// Read today's intake JSON, deliberately omitting viewer state and never opening a network connection.
pub fn status(directory: &Path) -> Result<Value> {
    super::read_consistent(directory, || {
        let ledger = load(directory)?;
        Ok(status_from_ledger(directory, &ledger)?.value())
    })
}

/// A session's intake status: counts, the saved seed and the handoff check.
pub(crate) struct Status<'a> {
    ledger: &'a Value,
    open: u64,
    settled: u64,
    /// The saved seed's summary, or null when none is saved.
    pub(crate) seed: Value,
    /// The exact bytes used to derive the summary and verify any confirmation.
    pub(crate) seed_bytes: Option<Vec<u8>>,
    /// The confirmed Acceptance section and records bound to its verified receipt.
    pub(crate) checks: Option<Acceptance>,
    /// Whether the confirmed Tink still matches its receipt.
    pub(crate) handoff: Handoff,
}

impl Status<'_> {
    /// Read the displayed draft's editable checks. Only the current acceptance format permits editing.
    pub(crate) fn read_draft_checks(&self, directory: &Path) -> Option<Acceptance> {
        let body = std::str::from_utf8(self.seed_bytes.as_deref()?).ok()?;
        matching_checks(directory, self.ledger, body, false).ok()
    }

    /// The intake JSON, in its published key order.
    pub(crate) fn value(&self) -> Value {
        object(&[
            ("status", self.ledger["status"].clone()),
            ("revision", self.ledger["revision"].clone()),
            ("version", self.ledger["version"].clone()),
            ("open", int(self.open)),
            ("settled", int(self.settled)),
            ("current_question", self.ledger["current_question"].clone()),
            ("seed", self.seed.clone()),
            ("handoff", self.handoff.value()),
        ])
    }
}

/// Derive status from one validated ledger while rechecking seed and handoff artifacts.
fn status_from_ledger<'a>(directory: &Path, ledger: &'a Value) -> Result<Status<'a>> {
    status_from_bytes(directory, ledger, read_seed_bytes(directory)?)
}

/// Verify the given contract bytes without reopening the seed path.
pub(crate) fn status_from_bytes<'a>(
    directory: &Path,
    ledger: &'a Value,
    seed_bytes: Option<Vec<u8>>,
) -> Result<Status<'a>> {
    let count = |status| {
        ledger["nodes"]
            .items()
            .iter()
            .filter(|n| n["status"].is(status))
            .count() as u64
    };
    let seed = summary(seed_bytes.as_deref());
    let (handoff, checks) = match practice_handoff(ledger) {
        Some(practice) => (practice, None),
        None => verify_saved(directory, ledger, &seed, seed_bytes.as_deref())?,
    };
    Ok(Status {
        ledger,
        open: count("unresolved"),
        settled: count("settled"),
        seed,
        seed_bytes,
        checks,
        handoff,
    })
}

pub(super) fn confirmed_as(
    directory: &Path,
    ledger: &Value,
    revision_label: &str,
    source: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<(String, Value, Value)> {
    require(
        super::one_of(&super::operator(ledger), &["human", "agent"]),
        format!(
            "a simulated session cannot be confirmed: its Tink Contract stays {}",
            repr(SIMULATED)
        ),
    )?;
    require(
        ledger["status"].is("active"),
        format!(
            "session is not active ({}): confirm before ending it",
            ledger["status"].string().ok_or("Invalid session status.")?
        ),
    )?;
    require(
        ledger["origin"] != Value::Null,
        "the goal is not confirmed yet",
    )?;
    let unresolved: Vec<_> = ledger["nodes"]
        .items()
        .iter()
        .filter(|n| n["status"].is("unresolved"))
        .map(|n| {
            n["id"]
                .string()
                .ok_or_else(|| "Invalid decision identity.".to_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    require(
        unresolved.is_empty(),
        format!("unresolved nodes remain: {}", unresolved.join(", ")),
    )?;
    let flags = review_flags(&ledger["nodes"]);
    let mut flagged: Vec<_> = flags.fields().iter().map(|(k, _)| k.as_str()).collect();
    flagged.sort();
    require(
        flagged.is_empty(),
        format!("nodes flagged for review remain: {}", flagged.join(", ")),
    )?;
    let mut lines = seed_lines(directory)?;
    let first = lines.first().map_or("", |s| split_terminator(s).0);
    require(
        first != CONFIRMED,
        format!(
            "the Tink is already confirmed (line 1 is {}); to confirm a change, save a new revision with line 1 {} and no 'Confirmed by:' line, show it, and confirm that",
            repr(first),
            repr(DRAFT)
        ),
    )?;
    require(
        first == DRAFT,
        format!(
            "line 1 of {FILE} must be exactly {} to confirm it, found {}",
            repr(DRAFT),
            repr(first)
        ),
    )?;
    let stale: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(i, s)| *i > 0 && s.starts_with("Confirmed by:"))
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    require(
        stale.is_empty(),
        format!(
            "{FILE} still has an earlier 'Confirmed by:' line (line {}); remove it from the new draft",
            stale.join(", ")
        ),
    )?;
    let found = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, s)| revision(s).map(|r| (i, r.to_owned())));
    let (index, label) = found.ok_or_else(|| {
        format!("{FILE} has no 'revision:' line, so the revision cannot be matched")
    })?;
    require(
        label == revision_label.trim_matches(whitespace),
        format!(
            "revision {} does not match the file's revision line ({})",
            repr(revision_label.trim_matches(whitespace)),
            repr(&label)
        ),
    )?;
    require_seed_body(directory, "cannot confirm")?;
    require_seed_decisions(directory, ledger, "cannot confirm")?;
    let records = checks::require_seed_checks(directory, ledger, false)?;
    if let Some((check, gap)) =
        checks::draft_checks(&records).find_map(|check| check.gap().map(|gap| (check, gap)))
    {
        return Err(format!(
            "Cannot confirm: check {}. {}. Nothing was confirmed.",
            check.index + 1,
            gap.missing()
        ));
    }

    let first_end = split_terminator(&lines[0]).1;
    let first_end = if first_end.is_empty() {
        "\n"
    } else {
        first_end
    }
    .to_owned();
    let (body, terminator) = split_terminator(&lines[index]);
    let note = confirmation_note(source, &label, timestamp, actor)?;
    let next = if !terminator.is_empty() {
        note + terminator
    } else {
        let body = body.to_owned();
        lines[index] = body + &first_end;
        note
    };
    lines[0] = CONFIRMED.to_owned() + &first_end;
    lines.insert(index + 1, next);
    let confirmed = lines.concat();
    let digest = sha256(confirmed.as_bytes());
    let mut record = object(&[
        ("schema_version", int(2)),
        ("checks_sha256", checks::digest(&records)?.as_str().into()),
        ("session_id", ledger["session_id"].clone()),
        ("operator", actor.session_operator().into()),
        ("revision", label.as_str().into()),
        ("source", source.into()),
        ("confirmed_at", timestamp.into()),
        ("seed_sha256", digest.as_str().into()),
        (
            "decisions_sha256",
            decisions_digest(ledger)?.as_str().into(),
        ),
    ]);
    if let Some(context) = actor.context() {
        record.set("schema_version", int(3));
        record.set(
            "actor",
            Value::from(
                serde_json::to_value(context).map_err(|_| "Cannot encode seed attribution.")?,
            ),
        );
    }
    let result = object(&[
        (
            "seed",
            directory.join(FILE).to_string_lossy().as_ref().into(),
        ),
        ("line_1", CONFIRMED.into()),
        ("revision", label.as_str().into()),
        ("sha256", digest.as_str().into()),
    ]);
    Ok((confirmed, record, result))
}

fn confirmation_note(
    source: &str,
    label: &str,
    timestamp: &str,
    actor: Actor<'_>,
) -> Result<String> {
    let date = timestamp.get(..10).ok_or("Invalid confirmation date.")?;
    let attribution = actor.context().map_or(String::new(), |context| {
        format!("; caller-declared {}", one_line(&context.details()))
    });
    Ok(format!(
        "Confirmed by: \"{}{attribution}\" (revision {label}, {date}). Not approved for implementation.",
        source
    ))
}

#[cfg(test)]
mod tests;
