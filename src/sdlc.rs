//! tink-sdlc's API 1 in one explicitly named checkout: its run reports, parsed once, and its
//! commands. It knows no Tinkery session or seed rules; it never stores or discovers anything.

mod run;
pub(crate) mod source;
#[cfg(test)]
mod tests;

pub(crate) use run::{DEADLINE, SCRIPT, read, refusal, run as command};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

/// Read bounded Git metadata without updating the index, on a background worker.
pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = std::process::Command::new("git");
    command.arg("--no-optional-locks").args(args);
    crate::process::text(root, command, "git", &AtomicBool::new(false), DEADLINE)
}

/// A source-owned intake record, not permission to launch or approve anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seed {
    source: PathBuf,
    sha256: String,
}

impl Seed {
    /// Refuse malformed records. Older or absent records cannot establish a join.
    pub(crate) fn parse(run: &Value) -> Result<Option<Self>, String> {
        let record = run
            .get("seed_contract")
            .ok_or("The run doesn't report its original Tink.")?;
        if record.is_null() {
            return Ok(None);
        }
        let source = record["source"]
            .as_str()
            .ok_or("The run has no original Tink Contract location.")?;
        let sha256 = record["sha256"]
            .as_str()
            .filter(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
            .ok_or("The run has no readable Tink Contract fingerprint.")?;
        if !Path::new(source).is_absolute() || record["path"].as_str().is_none() {
            return Err("The run has no readable Tink Contract location.".into());
        }
        Ok(Some(Self {
            source: source.into(),
            sha256: sha256.into(),
        }))
    }

    /// The absolute source path recorded at intake.
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    /// The lowercase SHA-256 of the source bytes recorded at intake.
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Whether the named checkout has tink-sdlc's run reader. Callers keep their own refusal words.
pub(crate) fn installed(root: &Path) -> bool {
    root.join(SCRIPT).is_file()
}

/// One run's API 1 status, parsed once; an unsupported envelope refuses.
pub(crate) fn status(
    root: &Path,
    slug: &str,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<Run, String> {
    status_report(&run::run(
        root,
        &["status".into(), slug.into(), "--json".into()],
        cancel,
        deadline,
    )?)
}

/// The status report's run, parsed once; an unsupported envelope refuses.
pub(crate) fn status_report(output: &str) -> Result<Run, String> {
    Ok(Run::parse(&envelope(output)?["run"]))
}

/// Every run summary in tink-sdlc's order, parsed once. `None` means the report has no runs
/// list, so each caller keeps its own refusal words.
pub(crate) fn list(
    root: &Path,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<Option<Vec<Run>>, String> {
    list_report(&run::run(
        root,
        &["status".into(), "--json".into()],
        cancel,
        deadline,
    )?)
}

/// The list report's summaries, parsed once; `None` when the report has no runs list.
pub(crate) fn list_report(output: &str) -> Result<Option<Vec<Run>>, String> {
    Ok(envelope(output)?["runs"]
        .as_array()
        .map(|runs| runs.iter().map(Run::parse).collect()))
}

/// Read only API 1's envelope; unknown interfaces never supply facts.
fn envelope(output: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(output).map_err(|e| format!("Couldn't read the run report: {e}"))?;
    if value["protocol"].as_str() != Some("tink-sdlc") || value["api_version"].as_u64() != Some(1) {
        return Err(
            "This checkout's run report isn't supported. Update tink-sdlc and read again.".into(),
        );
    }
    Ok(value)
}

macro_rules! word_enum {
    ($name:ident, $fallback:expr, {$($variant:ident => $word:literal),+ $(,)?}) => {
        #[doc = "A recognized API 1 word; unknown values remain unrecognized."]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum $name { $(#[doc = $word] $variant),+ }
        impl $name {
            fn read(value: &Value) -> Option<Self> {
                match value.as_str()? {
                    $($word => Some(Self::$variant),)+
                    _ => $fallback,
                }
            }
        }
    };
}

word_enum!(Verification, None, {
    Blocked => "blocked", Running => "running", NotRun => "not-run", Current => "current",
    Stale => "stale", Failed => "failed", Interrupted => "interrupted", Invalid => "invalid",
    AwaitingAttestation => "awaiting-attestation"
});
word_enum!(GateStatus, None, {
    Approved => "approved", Pending => "pending", Stale => "stale", ChangesRequested => "changes-requested"
});
word_enum!(CheckStatus, None, {
    Passed => "passed", Pending => "pending", Failed => "failed", Unknown => "unknown"
});
word_enum!(Proof, None, { Automated => "automated", Attested => "attested" });
word_enum!(ReceiptResult, None, {
    Passed => "passed", Failed => "failed", Running => "running", AwaitingAttestation => "awaiting-attestation"
});
word_enum!(StepState, None, {
    Prepared => "prepared", Active => "active", Uncertain => "uncertain", Completed => "completed",
    Stopped => "stopped", Unreadable => "unreadable"
});
word_enum!(TurnStatus, None, { Ended => "ended", Running => "not-ended", Unknown => "unknown" });
word_enum!(StopReason, None, { Cap => "cap", Operator => "operator" });

/// A boundary word retains missing, null, malformed and unfamiliar values for contextual refusals.
#[derive(Debug, Clone)]
pub(crate) struct Word<T> {
    value: Option<Value>,
    known: Option<T>,
}

impl<T: Copy + PartialEq> Word<T> {
    /// The recognized value, never a fallback for an unfamiliar word.
    pub(crate) fn known(&self) -> Option<T> {
        self.known
    }
    /// The original spelling for display and exact refusal text, not recognition.
    pub(crate) fn as_str(&self) -> Option<&str> {
        self.value.as_ref()?.as_str()
    }
    /// Whether the source supplied this field, including explicit null.
    pub(crate) fn present(&self) -> bool {
        self.value.is_some()
    }
    /// Compare only a recognized value.
    pub(crate) fn is(&self, value: T) -> bool {
        self.known == Some(value)
    }
}

/// One parsed API record. Metadata excludes word fields so readers cannot reinterpret them.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    /// Non-word metadata, including source-owned evidence and identity.
    pub(crate) data: Value,
    /// Run verification, with unknowns retained until the consumer requires a known state.
    pub(crate) verification: Word<Verification>,
    /// Receipt results are separate from checklist outcomes; pending judgment is never a pass.
    pub(crate) receipt_result: Word<ReceiptResult>,
    /// Validated pending judgment IDs; absent older fields remain empty, malformed lists refuse.
    pub(crate) pending_attestations: Result<Vec<String>, &'static str>,
    /// Opaque receipt identity retains every original field for exact snapshot comparison.
    pub(crate) receipt: Receipt,
    /// A missing or malformed list cannot silently become an empty list.
    pub(crate) gates: Option<Vec<Gate>>,
    /// Typed completion conditions, absent for list summaries.
    pub(crate) checklist: Option<Vec<CheckItem>>,
    /// Older reports can omit steps; explicit null is not omission.
    pub(crate) steps: Option<Vec<Step>>,
    /// Whether the report supplied steps at all.
    pub(crate) steps_present: bool,
    /// Contract state when this record is a worker contract rather than run status.
    pub(crate) state: Word<StepState>,
    /// Contract stop reason, meaningful only for a stopped contract.
    pub(crate) stopped_by: Word<StopReason>,
}

/// Exact receipt equality without exposing its words for a second interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Receipt(Value);

/// A gate's word and source metadata; stage requirements belong to the consuming operation.
#[derive(Debug, Clone)]
pub(crate) struct Gate {
    /// Source-owned stage and decision evidence.
    pub(crate) data: Value,
    /// Recognized gate state or preserved unfamiliar value.
    pub(crate) status: Word<GateStatus>,
}

/// A completion condition's recognized words and uninterpreted evidence metadata.
#[derive(Debug, Clone)]
pub(crate) struct CheckItem {
    /// Exact expectation, ownership flags, check and mark evidence.
    pub(crate) data: Value,
    /// Completion result, not an acceptance claim.
    pub(crate) status: Word<CheckStatus>,
    /// Proof method, not a passing result.
    pub(crate) proof: Word<Proof>,
    /// Missing means active; malformed provisional flags refuse interpretation.
    pub(crate) pending: Result<bool, &'static str>,
    /// Required freshness flag, not a failed sign-off.
    pub(crate) needs_recheck: Result<bool, &'static str>,
}

/// A step's recognized state, turn and stop reason, without orchestration policy.
#[derive(Debug, Clone)]
pub(crate) struct Step {
    /// Name, observed text and obligation metadata.
    pub(crate) data: Value,
    /// Persisted step state.
    pub(crate) state: Word<StepState>,
    /// Observed turn; explicit null differs from omission.
    pub(crate) turn: Word<TurnStatus>,
    /// Stop attribution, never inferred from observed turn text.
    pub(crate) stopped_by: Word<StopReason>,
}

impl Run {
    /// The sole API-word parse entry point. Optional detail fields and unknown words retain their
    /// distinctions so each operation can preserve its existing requirements and refusal wording.
    pub(crate) fn parse(value: &Value) -> Self {
        fn word<T>(value: Option<&Value>, read: fn(&Value) -> Option<T>) -> Word<T> {
            Word {
                known: value.and_then(read),
                value: value.cloned(),
            }
        }
        fn metadata(value: &Value, keys: &[&str]) -> Value {
            let mut value = value.clone();
            if let Some(object) = value.as_object_mut() {
                for key in keys {
                    object.remove(*key);
                }
            }
            value
        }
        let mut data = metadata(
            value,
            &[
                "verification_status",
                "gates",
                "checklist",
                "steps",
                "state",
                "stopped_by",
            ],
        );
        if let Some(receipt) = data.get_mut("verification").and_then(Value::as_object_mut) {
            receipt.remove("result");
            receipt.remove("pending_attestations");
        }
        Self {
            data,
            receipt: Receipt(value["verification"].clone()),
            verification: word(value.get("verification_status"), Verification::read),
            receipt_result: word(value["verification"].get("result"), ReceiptResult::read),
            pending_attestations: match value["verification"].get("pending_attestations") {
                None if Verification::read(&value["verification_status"])
                    != Some(Verification::AwaitingAttestation) =>
                {
                    Ok(Vec::new())
                }
                ids => (|| {
                    let invalid = "The run has no readable pending_attestations list.";
                    let ids = ids.and_then(Value::as_array).ok_or(invalid)?;
                    if ids.is_empty() {
                        return Err(invalid);
                    }
                    let mut pending = Vec::new();
                    for id in ids {
                        let id = id
                            .as_str()
                            .filter(|id| {
                                !id.is_empty()
                                    && id
                                        .bytes()
                                        .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
                            })
                            .ok_or(invalid)?;
                        if pending.iter().any(|other| other == id) {
                            return Err(invalid);
                        }
                        pending.push(id.to_owned());
                    }
                    Ok(pending)
                })(),
            },
            gates: value["gates"].as_array().map(|gates| {
                gates
                    .iter()
                    .map(|gate| Gate {
                        data: metadata(gate, &["status"]),
                        status: word(gate.get("status"), GateStatus::read),
                    })
                    .collect()
            }),
            checklist: value["checklist"].as_array().map(|items| {
                items
                    .iter()
                    .map(|item| CheckItem {
                        data: metadata(item, &["status", "proof", "pending", "needs_recheck"]),
                        status: word(item.get("status"), CheckStatus::read),
                        proof: word(item.get("proof"), Proof::read),
                        pending: match item.get("pending") {
                            None => Ok(false),
                            Some(value) => value
                                .as_bool()
                                .ok_or("A checklist item has an invalid pending flag."),
                        },
                        needs_recheck: item["needs_recheck"]
                            .as_bool()
                            .ok_or("A checklist item has no readable needs_recheck flag."),
                    })
                    .collect()
            }),
            steps: value["steps"].as_array().map(|steps| {
                steps
                    .iter()
                    .map(|step| Step {
                        data: metadata(step, &["state", "turn", "stopped_by"]),
                        state: word(step.get("state"), StepState::read),
                        turn: word(step.get("turn"), TurnStatus::read),
                        stopped_by: word(step.get("stopped_by"), StopReason::read),
                    })
                    .collect()
            }),
            steps_present: value.get("steps").is_some(),
            state: word(value.get("state"), StepState::read),
            stopped_by: word(value.get("stopped_by"), StopReason::read),
        }
    }
}

/// A checklist item's operator obligation, independent of result revision or screen state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Attestation {
    /// Automatic or explicitly worker-owned checks owe the operator nothing.
    Worker,
    /// Missing ownership on an older manual item fails closed; recheck is not a failed sign-off.
    Operator { owed: bool, recheck: bool },
}

impl Attestation {
    /// Read API 1 ownership and obligation; malformed flags or states never mean nothing is owed.
    pub(crate) fn read(record: &CheckItem) -> Result<Self, String> {
        let item = &record.data;
        let flag = |key: &str| {
            item[key]
                .as_bool()
                .ok_or_else(|| format!("A checklist item has no readable {key} flag."))
        };
        let automatic = flag("automatic")?;
        let operator = match item.get("operator_attestation") {
            None => true,
            Some(value) => value
                .as_bool()
                .ok_or("A checklist item has an invalid operator_attestation flag.")?,
        };
        let pending = record.pending?;
        let recheck = record.needs_recheck?;
        let status = record
            .status
            .known()
            .ok_or("A checklist item has no readable status.")?;
        Ok(if automatic || !operator {
            Self::Worker
        } else {
            Self::Operator {
                owed: !pending && status != CheckStatus::Passed,
                recheck,
            }
        })
    }
}

/// AGENTS.md as tink-sdlc compares it: each well-formed generated `tink:rules` block and the
/// blank line before it are dropped, so text the launcher writes is not an authored change.
/// Unbalanced markers leave the bytes unchanged, never guessing where a block ends.
pub(crate) fn without_generated_rules(data: &[u8]) -> Vec<u8> {
    use regex::bytes::Regex;
    use std::sync::LazyLock;
    static BEGIN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<!--\s*tink:rules begin\b.*?-->").unwrap());
    static END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<!--\s*tink:rules end\s*-->").unwrap());
    let lines: Vec<&[u8]> = data.split_inclusive(|&b| b == b'\n').collect();
    let (mut blocks, mut open) = (Vec::new(), None);
    for (number, line) in lines.iter().enumerate() {
        if BEGIN.is_match(line) {
            if open.replace(number).is_some() {
                return data.to_vec();
            }
        } else if END.is_match(line) {
            let Some(begin) = open.take() else {
                return data.to_vec();
            };
            blocks.push((begin, number));
        }
    }
    if open.is_some() || blocks.is_empty() {
        return data.to_vec();
    }
    let (mut out, mut cursor) = (Vec::with_capacity(data.len()), 0);
    for (begin, end) in blocks {
        let separator = begin > cursor && lines[begin - 1].trim_ascii().is_empty();
        out.extend(lines[cursor..begin - usize::from(separator)].concat());
        cursor = end + 1;
    }
    out.extend(lines[cursor..].concat());
    out
}

/// Source run IDs must use the canonical lowercase UUIDv4 spelling.
pub(crate) fn canonical_run_id(id: &str) -> bool {
    static ID: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap()
    });
    ID.is_match(id)
}
