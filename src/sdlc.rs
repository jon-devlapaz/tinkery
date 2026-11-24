//! tink-sdlc's API 1 in one explicitly named checkout: its run reports, parsed once, and its
//! commands. It knows no Tinkery session or seed rules; it never stores or discovers anything.

mod files;
mod run;
pub(crate) mod source;
#[cfg(test)]
mod tests;

pub(crate) use files::{FileGap, Log, MAX_ENTRIES, RunFiles};
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
    sha256: Sha256,
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
            .and_then(Sha256::parse)
            .ok_or("The run has no readable Tink Contract fingerprint.")?;
        if !Path::new(source).is_absolute() || record["path"].as_str().is_none() {
            return Err("The run has no readable Tink Contract location.".into());
        }
        Ok(Some(Self {
            source: source.into(),
            sha256,
        }))
    }

    /// The absolute source path recorded at intake.
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    /// The lowercase SHA-256 of the source bytes recorded at intake.
    pub(crate) fn sha256(&self) -> &str {
        self.sha256.as_str()
    }
}

/// A full Git commit identity: exactly 40 hexadecimal digits, never a branch or abbreviation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Commit(String);

impl Commit {
    /// Refuse anything but 40 hexadecimal digits.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        (value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| Self(value.into()))
    }

    /// The exact spelling the source recorded.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A SHA-256 digest: exactly 64 lowercase hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sha256(String);

impl Sha256 {
    /// Refuse anything but 64 lowercase hexadecimal digits.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        (value.len() == 64
            && value
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .then(|| Self(value.into()))
    }

    /// The digest's hexadecimal spelling.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
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

fn word<T>(value: Option<&Value>, read: fn(&Value) -> Option<T>) -> Word<T> {
    Word {
        known: value.and_then(read),
        value: value.cloned(),
    }
}

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
    /// An open local result: the source records a null PR and no closure.
    pub(crate) local: bool,
    /// The receipt covers its candidate now: a fresh pass, or a fresh result awaiting only
    /// judgments. A current receipt is not acceptance.
    pub(crate) receipt_current: bool,
    /// The receipt's checked candidate commit; absent or malformed means unknown.
    pub(crate) receipt_head: Option<Commit>,
    /// The run's own build checkout. `None` means the named checkout; a present path is
    /// absolute text without NUL, never resolved or trusted to exist.
    pub(crate) checkout: Result<Option<PathBuf>, CheckoutGap>,
    /// A recognized closure outcome; open runs and unfamiliar outcomes have none.
    pub(crate) closure: Option<Closure>,
    /// Gate decision receipts, oldest first. Omission means none; a malformed list is `None`.
    pub(crate) decisions: Option<Vec<Decision>>,
    /// The source's label counts, never the label words.
    pub(crate) labels: Labels,
    /// The intake record, refused when malformed and `None` when older or absent.
    pub(crate) seed: Result<Option<Seed>, String>,
}

/// Why a recorded checkout cannot name a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckoutGap {
    NotText,
    /// Text containing NUL, which no folder name holds.
    Nul,
    /// A relative path, which would depend on the reader's folder.
    Relative,
}

/// A settled run's recognized outcome and its recorded time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Closure {
    /// Merged or abandoned, as recorded.
    pub(crate) outcome: Outcome,
    /// Nanoseconds since the Unix epoch; absent or malformed means unknown.
    pub(crate) time_ns: Option<u64>,
}

/// How a run closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The source names a merge; its commit is `None` when absent or malformed.
    Merged(Option<Commit>),
    /// Closed without a merge.
    Abandoned,
}

/// One gate decision receipt, verbatim; missing words stay unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) stage: Option<u64>,
    /// The recorded decision word.
    pub(crate) decision: Option<String>,
    pub(crate) reviewer: Option<String>,
    /// Who recorded it; only an operator's receipt is a person's gate.
    pub(crate) by: Attribution,
}

/// Who recorded a decision receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attribution {
    /// A person's gate decision.
    Operator,
    /// An agent recorded it; it is not the operator's gate.
    Agent,
    /// tink-sdlc bound the confirmed seed itself.
    Automatic,
}

/// The source's label counts; each is `None` when absent or malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Labels {
    /// Accepted and rejected verdict counts, present only together.
    pub(crate) kinds: Option<(u64, u64)>,
    /// All recorded labels, of every kind.
    pub(crate) count: Option<u64>,
}

/// Exact receipt equality without exposing its words for a second interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Receipt(Value);

/// A gate's recognized words; stage requirements belong to the consuming operation.
#[derive(Debug, Clone)]
pub(crate) struct Gate {
    /// Recognized gate state or preserved unfamiliar value.
    pub(crate) status: Word<GateStatus>,
    /// The gated stage; absent or malformed means unknown.
    pub(crate) stage: Option<Stage>,
}

/// tink-sdlc's gated stages; a number it may add later stays numbered, never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Stage 1.
    Problem,
    /// Stage 2.
    Specification,
    /// Stage 3.
    Plan,
    /// Any other stage number.
    Other(u64),
}

impl Stage {
    /// The source's stage number.
    pub(crate) fn number(self) -> u64 {
        match self {
            Self::Problem => 1,
            Self::Specification => 2,
            Self::Plan => 3,
            Self::Other(n) => n,
        }
    }
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
    /// The proof method, only when the automatic flag and the proof word agree.
    pub(crate) method: Result<Proof, MethodGap>,
    /// Sign-off evidence: `None` for explicit null, an object otherwise.
    pub(crate) mark: Result<Option<Value>, MarkGap>,
}

/// Why a checklist item has no proof method, in the order the rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodGap {
    /// No readable automatic flag.
    NoAutomatic,
    /// No proof word, with the automatic flag the item claims.
    NoProof { automatic: bool },
    /// The proof word is not the one the automatic flag requires.
    Disagrees { automatic: bool },
}

/// Why a checklist item has no readable sign-off evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkGap {
    /// The item omits its mark.
    Missing,
    /// The mark is neither null nor an object.
    Unreadable,
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
    /// Whether the launcher reached the step's response cap; omission is false, and a value
    /// that is not a boolean is refused rather than read as either.
    pub(crate) cap_reached: Result<bool, &'static str>,
}

/// A worker step contract read once for one exact run and checkout. Another schema, run or
/// checkout is refused; every other field keeps its source form for the caller's own checks.
#[derive(Debug, Clone)]
pub(crate) struct StepContract {
    /// The whole record, for fingerprints and caller-specific path checks.
    pub(crate) record: Value,
    /// Recorded step state.
    pub(crate) state: Word<StepState>,
    /// Recorded stop reason, meaningful only for a stopped contract.
    pub(crate) stopped_by: Word<StopReason>,
    /// The recorded cumulative response cap; absent or not a whole number is `None`.
    pub(crate) max_responses: Option<u64>,
    /// The recorded session text; absent or not text is `None`, and empty text is kept.
    pub(crate) session: Option<String>,
}

impl StepContract {
    /// Read a contract bound to `run_id` and `checkout`, or `None` for any other.
    pub(crate) fn read(record: Value, run_id: &str, checkout: &Path) -> Option<Self> {
        if record["schema"].as_u64() != Some(1)
            || record["run_id"].as_str() != Some(run_id)
            || record["checkout"].as_str() != checkout.to_str()
        {
            return None;
        }
        Some(Self {
            state: word(record.get("state"), StepState::read),
            stopped_by: word(record.get("stopped_by"), StopReason::read),
            max_responses: record["max_responses"].as_u64(),
            session: record["session"].as_str().map(Into::into),
            record,
        })
    }
}

impl Run {
    /// The sole API-word parse entry point. Optional detail fields and unknown words retain their
    /// distinctions so each operation can preserve its existing requirements and refusal wording.
    pub(crate) fn parse(value: &Value) -> Self {
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
            &["verification_status", "gates", "checklist", "steps"],
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
                        status: word(gate.get("status"), GateStatus::read),
                        stage: gate["stage"].as_u64().map(|n| match n {
                            1 => Stage::Problem,
                            2 => Stage::Specification,
                            3 => Stage::Plan,
                            n => Stage::Other(n),
                        }),
                    })
                    .collect()
            }),
            checklist: value["checklist"].as_array().map(|items| {
                items
                    .iter()
                    .map(|item| CheckItem {
                        data: metadata(
                            item,
                            &["status", "proof", "pending", "needs_recheck", "mark"],
                        ),
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
                        method: match (item["automatic"].as_bool(), Proof::read(&item["proof"])) {
                            (None, _) => Err(MethodGap::NoAutomatic),
                            (Some(automatic), _) if !item["proof"].is_string() => {
                                Err(MethodGap::NoProof { automatic })
                            }
                            (Some(true), Some(Proof::Automated)) => Ok(Proof::Automated),
                            (Some(false), Some(Proof::Attested)) => Ok(Proof::Attested),
                            (Some(automatic), _) => Err(MethodGap::Disagrees { automatic }),
                        },
                        mark: match item.get("mark") {
                            None => Err(MarkGap::Missing),
                            Some(Value::Null) => Ok(None),
                            Some(mark) if mark.is_object() => Ok(Some(mark.clone())),
                            Some(_) => Err(MarkGap::Unreadable),
                        },
                    })
                    .collect()
            }),
            steps: value["steps"].as_array().map(|steps| {
                steps
                    .iter()
                    .map(|step| Step {
                        data: metadata(
                            step,
                            &["state", "turn", "stopped_by", "response_cap_reached"],
                        ),
                        state: word(step.get("state"), StepState::read),
                        turn: word(step.get("turn"), TurnStatus::read),
                        stopped_by: word(step.get("stopped_by"), StopReason::read),
                        cap_reached: match step.get("response_cap_reached") {
                            None => Ok(false),
                            Some(flag) => flag
                                .as_bool()
                                .ok_or("The worker cap flag is not reported in a supported form."),
                        },
                    })
                    .collect()
            }),
            steps_present: value.get("steps").is_some(),
            local: value.get("pr") == Some(&Value::Null) && value["closure"].is_null(),
            receipt_current: (Verification::read(&value["verification_status"])
                == Some(Verification::Current)
                && value["verification"]["passed"] == true)
                || (Verification::read(&value["verification_status"])
                    == Some(Verification::AwaitingAttestation)
                    && ReceiptResult::read(&value["verification"]["result"])
                        == Some(ReceiptResult::AwaitingAttestation)
                    && value["verification"]["passed"] == false),
            receipt_head: value["verification"]["candidate"]["head"]
                .as_str()
                .and_then(Commit::parse),
            checkout: match value.get("checkout") {
                None => Ok(None),
                Some(path) => match path.as_str() {
                    None => Err(CheckoutGap::NotText),
                    Some(path) if path.contains('\0') => Err(CheckoutGap::Nul),
                    Some(path) if !Path::new(path).is_absolute() => Err(CheckoutGap::Relative),
                    Some(path) => Ok(Some(path.into())),
                },
            },
            closure: {
                let closure = &value["closure"];
                let outcome = match closure["outcome"].as_str() {
                    Some("merged") => Some(Outcome::Merged(
                        closure["merge"].as_str().and_then(Commit::parse),
                    )),
                    Some("abandoned") => Some(Outcome::Abandoned),
                    _ => None,
                };
                outcome.map(|outcome| Closure {
                    outcome,
                    time_ns: closure["time_ns"].as_u64(),
                })
            },
            decisions: match value.get("decisions") {
                None => Some(Vec::new()),
                Some(list) => list.as_array().map(|list| {
                    list.iter()
                        .map(|d| Decision {
                            stage: d["stage"].as_u64(),
                            decision: d["decision"].as_str().map(Into::into),
                            reviewer: d["reviewer"].as_str().map(Into::into),
                            by: if d.get("basis").is_some() {
                                Attribution::Automatic
                            } else if d.get("attribution").is_some() {
                                Attribution::Agent
                            } else {
                                Attribution::Operator
                            },
                        })
                        .collect()
                }),
            },
            labels: Labels {
                kinds: value["labels"]["kinds"]["accepted"]
                    .as_u64()
                    .zip(value["labels"]["kinds"]["rejected"].as_u64()),
                count: value["labels"]["count"].as_u64(),
            },
            seed: Seed::parse(value),
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
