//! Draft reviews and teach-back use confirmed-goal receipts so the pinned reader keeps working.
//! They are interview records, not new evidence for the goal, authority, or permission to build.

use super::{CompletionReview, Value, editable, find, require, value::object};

const PREFIX: &str = "tinkery:teach-back/";
/// The skip recorded exactly when confirmation has no completed teach-back for these draft bytes.
pub const TEACH_BACK_SKIPPED: &str = "Skipped teach-back.";

/// A comparison lists model guesses verbatim; even a mismatch never holds confirmation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeachBackComparison {
    /// Mismatches with the draft, in the model's words; an empty list reports none.
    pub mismatches: Vec<String>,
}

/// A drafting review or explicit teach-back action; comparison follows capture.
pub enum TeachBackRecord<'a> {
    /// The operator's unchanged words, including original line endings.
    Words(&'a str),
    /// A completed comparison, never a confirmation gate.
    Compared(&'a TeachBackComparison),
    /// The drafting step's validated review, not evidence that implementation ran.
    Review(&'a CompletionReview),
}

pub(super) fn is_record(receipt: &Value) -> bool {
    receipt["artifact"]
        .string()
        .is_some_and(|s| s.starts_with(PREFIX) || s.starts_with("tinkery:review/"))
}

pub(super) fn evidence(node: &Value) -> Value {
    Value::Array(
        node["evidence"]
            .items()
            .iter()
            .filter(|r| !is_record(r))
            .cloned()
            .collect(),
    )
}

fn identity(revision: &str, seed: &str, label: &str) -> String {
    format!(
        "{label} {revision} · {}",
        super::seed::sha256(seed.as_bytes())
    )
}

pub(super) fn require_draft(seed: &str, revision: &str) -> Result<(), String> {
    require(
        seed.lines().next() == Some("status: draft"),
        "A review needs an unconfirmed draft. Reopen Shape.",
    )?;
    require(
        seed.lines()
            .find_map(|line| line.strip_prefix("revision:"))
            .is_some_and(|r| r.trim() == revision),
        "The draft revision changed. Reopen Shape before trying again.",
    )
}

fn receipts(ledger: &Value) -> &[Value] {
    find(&ledger["nodes"], ledger["origin"].string().unwrap_or(""))["evidence"].items()
}

fn receipt(revision: &str, seed: &str, kind: &str, words: &str, timestamp: &str) -> Value {
    let label = if kind.starts_with("review/") {
        "Review"
    } else {
        "Teach-back"
    };
    object(&[
        ("checked", identity(revision, seed, label).as_str().into()),
        ("at", timestamp.into()),
        ("observed", words.into()),
        (
            "check",
            "explicit Shape draft review or teach-back; not evidence or implementation approval"
                .into(),
        ),
        ("artifact", format!("tinkery:{kind}").as_str().into()),
    ])
}

pub(super) fn append(
    ledger: &Value,
    revision: &str,
    seed: &str,
    record: &TeachBackRecord<'_>,
    timestamp: &str,
) -> Result<Value, String> {
    require_draft(seed, revision)?;
    let mut additions = Vec::new();
    match record {
        TeachBackRecord::Words(words) => additions.push(receipt(
            revision,
            seed,
            "teach-back/words",
            words,
            timestamp,
        )),
        TeachBackRecord::Compared(comparison) => {
            require(
                receipts(ledger)
                    .iter()
                    .rev()
                    .find(|r| r["checked"].is(&identity(revision, seed, "Teach-back")))
                    .is_some_and(|r| r["artifact"].is(&format!("{PREFIX}words"))),
                "No teach-back words were saved for this draft. Capture them before comparing.",
            )?;
            for mismatch in &comparison.mismatches {
                additions.push(receipt(
                    revision,
                    seed,
                    "teach-back/mismatch",
                    mismatch,
                    timestamp,
                ));
            }
            additions.push(receipt(
                revision,
                seed,
                "teach-back/reviewed",
                "Teach-back compared.",
                timestamp,
            ));
        }
        TeachBackRecord::Review(review) => {
            let words = serde_json::to_string(review).map_err(|_| {
                "The completion review could not be recorded. Retry the step.".to_owned()
            })?;
            additions.push(receipt(
                revision,
                seed,
                "review/completed",
                &words,
                timestamp,
            ));
        }
    }
    add(ledger, additions)
}

fn add(ledger: &Value, additions: Vec<Value>) -> Result<Value, String> {
    for receipt in &additions {
        super::validate_receipt(receipt)?;
    }
    let mut state = editable(ledger)?;
    let origin = ledger["origin"].clone();
    let node = state["nodes"]
        .list_mut()
        .iter_mut()
        .find(|n| n["id"] == origin)
        .ok_or_else(|| "No confirmed goal is recorded. Reopen Shape.".to_owned())?;
    node["evidence"].list_mut().extend(additions);
    Ok(state)
}

pub(super) fn skip(
    ledger: &Value,
    revision: &str,
    seed: &str,
    timestamp: &str,
) -> Result<Value, String> {
    let checked = identity(revision, seed, "Teach-back");
    if receipts(ledger).iter().any(|r| {
        r["checked"].is(&checked)
            && ["reviewed", "skipped"]
                .iter()
                .any(|kind| r["artifact"].is(&format!("{PREFIX}{kind}")))
    }) {
        return editable(ledger);
    }
    add(
        ledger,
        vec![receipt(
            revision,
            seed,
            "teach-back/skipped",
            TEACH_BACK_SKIPPED,
            timestamp,
        )],
    )
}

/// Display the drafting step's last review for these exact bytes, always labelled as guesses.
/// Stored words stay unchanged; no line claims that a proposed check ran.
pub fn completion_report(ledger: &Value, revision: &str, seed: &str) -> Vec<String> {
    let checked = identity(revision, seed, "Review");
    let Some(receipt) = receipts(ledger)
        .iter()
        .rev()
        .find(|r| r["checked"].is(&checked) && r["artifact"].is("tinkery:review/completed"))
    else {
        return Vec::new();
    };
    let Ok(review) =
        serde_json::from_str::<CompletionReview>(receipt["observed"].string().unwrap_or(""))
    else {
        return vec!["Drafting step guess: saved review could not be read. Retry the step.".into()];
    };
    let mut lines = Vec::new();
    for decided in review.decided {
        lines.push(format!("Drafting step guess — decided: {decided}"));
    }
    if review.done.is_empty() {
        lines.push("Drafting step guess: Nothing observed done.".into());
    } else {
        for done in review.done {
            lines.push(format!("Drafting step guess — observed done: {done}"));
        }
    }
    for failure in review.failure_cases {
        lines.push(format!(
            "Drafting step guess: If {}, {} must fail.",
            failure.case, failure.statement
        ));
    }
    lines
}

/// Display the last completed comparison for these exact draft bytes, without judging its words.
/// Receipts remain unchanged; display labels distinguish the operator's words from model guesses.
pub fn teach_back_report(ledger: &Value, revision: &str, seed: &str) -> Vec<String> {
    let checked = identity(revision, seed, "Teach-back");
    let all: Vec<_> = receipts(ledger)
        .iter()
        .filter(|r| r["checked"].is(&checked))
        .collect();
    let Some(end) = all
        .iter()
        .rposition(|r| r["artifact"].is(&format!("{PREFIX}reviewed")))
    else {
        return Vec::new();
    };
    let Some(start) = all[..end]
        .iter()
        .rposition(|r| r["artifact"].is(&format!("{PREFIX}words")))
    else {
        return Vec::new();
    };
    let mut lines = vec![format!(
        "Your teach-back: {}",
        all[start]["observed"].string().unwrap_or("")
    )];
    let mismatches = &all[start + 1..end];
    if mismatches.is_empty() {
        lines.push("Teach-back guess: no mismatches reported.".into());
    } else {
        lines.extend(mismatches.iter().map(|r| {
            format!(
                "Teach-back mismatch (guess): {}",
                r["observed"].string().unwrap_or("")
            )
        }));
    }
    lines
}
