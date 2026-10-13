//! Explicit-data evaluation with scripted replies only. No provider, session save or confirmation.
#[path = "support/capture.rs"]
mod capture;

use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
use tinkery::think::{
    brain_dump::{
        BoardHost, BoardRequest, BrainDump, Guess, decision_review_input, merge_decision_review,
    },
    record::hash,
};

const USAGE: &str = "Supply --dataset FILE --output NEW_TEMP_DIRECTORY --max-requests N. Replies are fake; no default dataset or live mode exists.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    provenance: Provenance,
    steps: Vec<Step>,
    replies: VecDeque<Reply>,
    #[serde(default)]
    expected: std::collections::BTreeMap<String, Value>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    kind: InputKind,
    description: String,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum InputKind {
    SyntheticTestData,
    OperatorSupplied,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Step {
    Send { text: String },
    Retry,
}
#[derive(Clone, Copy, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Shaping,
    DecisionReview,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    kind: Kind,
    #[serde(flatten)]
    result: ReplyResult,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplyResult {
    Text(String),
    Failure(String),
}

struct Fake {
    cap: usize,
    state: Mutex<FakeState>,
}
#[derive(Default)]
struct FakeState {
    replies: VecDeque<Reply>,
    sends: usize,
    requests: Vec<Value>,
    refused: Vec<Value>,
}
impl Fake {
    fn request(&self, kind: Kind, send: usize) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        // Charge before touching the fake provider, including scripted failures and retries.
        if state.requests.len() >= self.cap {
            state.refused.push(json!({"kind": kind, "send": send}));
            return Err(
                "Fake request cap exhausted; next request refused before invocation.".into(),
            );
        }
        let result = match state.replies.pop_front() {
            Some(reply) if reply.kind == kind => match reply.result {
                ReplyResult::Text(text) => Ok(text),
                ReplyResult::Failure(error) => Err(error),
            },
            Some(_) => Err("Fake reply kind does not match the next request.".into()),
            None => Err("No scripted fake reply remains.".into()),
        };
        state.requests.push(json!({"kind": kind, "send": send,
            "outcome": if result.is_ok() { "reply" } else { "failure" },
            "error": result.as_ref().err()}));
        result
    }
}
impl BoardHost for Fake {
    fn reshape(&self, request: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        let send = {
            let mut state = self.state.lock().unwrap();
            state.sends += 1;
            state.sends
        };
        let raw = self.request(Kind::Shaping, send)?;
        let mut changes = vec![];
        // Like PiHost, readable invalid boards still get a review before decoding.
        let merged = decision_review_input(&request, &raw).and_then(|_| {
            self.request(Kind::DecisionReview, send)
                .ok()
                .and_then(|review| merge_decision_review(&raw, &review, &mut changes))
        });
        Guess::decode(merged.as_deref().unwrap_or(&raw), &request, &mut changes)
    }
}

fn options(args: impl Iterator<Item = String>) -> Result<(PathBuf, PathBuf, usize), String> {
    let mut args = args;
    let (mut dataset, mut output, mut cap) = (None, None, None);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or(USAGE)?;
        match arg.as_str() {
            "--dataset" if dataset.is_none() => dataset = Some(PathBuf::from(value)),
            "--output" if output.is_none() => output = Some(PathBuf::from(value)),
            "--max-requests" if cap.is_none() => cap = Some(value.parse().map_err(|_| USAGE)?),
            _ => return Err(USAGE.into()),
        }
    }
    Ok((
        dataset.ok_or(USAGE)?,
        output.ok_or(USAGE)?,
        cap.ok_or(USAGE)?,
    ))
}

fn validate(dataset: &Dataset, output: &Path) -> Result<(), String> {
    if dataset.provenance.description.trim().is_empty() || dataset.steps.is_empty() {
        return Err("Supply provenance and at least one step; synthetic fixtures must say synthetic_test_data.".into());
    }
    if !matches!(dataset.steps.first(), Some(Step::Send { .. })) {
        return Err("Start the dataset with a send step.".into());
    }
    for step in &dataset.steps {
        if let Step::Send { text } = step
            && (text.trim().is_empty() || text.len() > 4096)
        {
            return Err(
                "Each send needs 1–4096 bytes of nonblank text; split larger inputs explicitly."
                    .into(),
            );
        }
    }
    if dataset
        .expected
        .keys()
        .any(|key| !["actor_outcome", "constraints", "unresolved_deferred"].contains(&key.as_str()))
    {
        return Err(
            "Expected checks may name actor_outcome, constraints or unresolved_deferred only."
                .into(),
        );
    }
    let temp = std::env::temp_dir()
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let parent = output.parent().and_then(|p| p.canonicalize().ok());
    if !output.is_absolute() || output.exists() || parent.is_none_or(|p| !p.starts_with(temp)) {
        return Err("Output must be a new absolute directory under the system temporary directory, with an existing parent.".into());
    }
    Ok(())
}

fn settle(app: &mut BrainDump) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.running() && Instant::now() < deadline {
        app.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    if app.running() {
        Err("Fake run did not settle within five seconds.".into())
    } else {
        Ok(())
    }
}

fn originals_match(app: &BrainDump, supplied: &[&str]) -> bool {
    app.conversation()
        .sources()
        .iter()
        .map(|s| s.text.as_str())
        .eq(supplied.iter().copied())
}

fn run(dataset_path: &Path, output: &Path, cap: usize) -> Result<Value, String> {
    let input = std::fs::read_to_string(dataset_path)
        .map_err(|e| format!("Read the supplied dataset: {e}. {USAGE}"))?;
    let mut dataset: Dataset =
        serde_json::from_str(&input).map_err(|e| format!("Invalid dataset: {e}"))?;
    validate(&dataset, output)?;
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    let host = Arc::new(Fake {
        cap,
        state: Mutex::new(FakeState {
            replies: std::mem::take(&mut dataset.replies),
            ..Default::default()
        }),
    });
    let mut app = BrainDump::with_host(host.clone());
    capture::capture(&mut app, output, "00-entry");
    let mut supplied = vec![];
    let mut steps = vec![];
    for (index, step) in dataset.steps.iter().enumerate() {
        match step {
            Step::Send { text } => {
                app.paste(text);
                supplied.push(text.as_str());
            }
            Step::Retry => {
                if app.conversation().last_failure().is_none() {
                    return Err(
                        "A retry step requires a failed reading; use send for new words.".into(),
                    );
                }
            }
        }
        app.submit();
        settle(&mut app)?;
        let name = format!("{:02}-reading", index + 1);
        capture::capture(&mut app, output, &name);
        let accepted = originals_match(&app, &supplied);
        steps.push(json!({"capture": name, "notice": app.notice(),
            "input_accepted": accepted,
            "reading_applied": accepted && app.conversation().caught_up(),
            "failure": if accepted { app.conversation().last_failure() } else { Some(app.notice()) }}));
    }
    let exact = originals_match(&app, &supplied);
    let current = exact && app.conversation().caught_up();
    let board = app.conversation().board();
    let observed = json!({
        "actor_outcome": board.map(|b| json!({"who": b.parts.who, "outcome": b.parts.outcome})),
        "constraints": board.map(|b| json!({"must": b.parts.must, "must_not": b.parts.must_not})),
        "unresolved_deferred": board.map(|b| json!({"open": app.conversation().open_parts(),
            "questions": app.conversation().unresolved(), "notes": b.unresolved_notes,
            "deferred": b.deferred, "other_goals": b.other_goals})),
    });
    let mut checks = serde_json::Map::new();
    for name in ["actor_outcome", "constraints", "unresolved_deferred"] {
        let expected = dataset.expected.get(name);
        checks.insert(name.into(), json!({"status": match expected {
            Some(value) if board.is_some() && current => if *value == observed[name] { "pass" } else { "fail" },
            _ => "unknown",
        }, "basis": "Exact match to supplied fixture only; semantic correctness is unknown.",
            "expected": expected, "observed": observed[name]}));
    }
    checks.insert(
        "originals_exact".into(),
        json!({"status": if exact { "pass" } else { "fail" }}),
    );
    for (name, reason) in [
        (
            "injection_handling",
            "No model ran. Source preservation and decoder checks do not prove prompt-injection resistance.",
        ),
        (
            "exact_confirmation_handoff",
            "Not exercised: this runner never confirms or saves. Use the isolated goal_confirmation_terminal.py checks separately.",
        ),
    ] {
        checks.insert(name.into(), json!({"status": "unknown", "reason": reason}));
    }
    let state = host.state.lock().unwrap();
    let source = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty());
    let report = json!({
        "execution": "fake", "input": {"path": dataset_path, "hash": hash(&input), "provenance": dataset.provenance},
        "revision": {"checkout_at_run": source, "working_tree_dirty": dirty,
            "compiled_library_revision": null,
            "limit": "Runtime Git metadata describes the checkout, not necessarily this executable. Compiled library revision is unknown; rebuild at the revision being evaluated.",
            "compiled_runner_hash": hash(include_str!("fake_think_eval.rs")),
            "reading_prompt_file_hash": hash(include_str!("../src/think/brain_dump/prompts/reading.txt")),
            "review_prompt_file_hash": hash(include_str!("../src/think/brain_dump/prompts/decision_review.txt")),
            "prompts_executed": false},
        "requests": {"cap": cap, "used": state.requests.len(), "calls": state.requests,
            "refused_before_invocation": state.refused, "unused_replies": state.replies.len(),
            "shaping": state.requests.iter().filter(|r| r["kind"] == "shaping").count(),
            "decision_review": state.requests.iter().filter(|r| r["kind"] == "decision_review").count()},
        "steps": steps, "reading_current": current, "automated_checks": checks,
        "human_baseline": {"recognition": null, "active_attention_seconds": null,
            "waiting_seconds": null, "corrections": null, "cost": null,
            "note": "Unknown until measured by a person. Fake timings and replies are not a human baseline."},
        "provider_cost": {"amount": 0, "currency": "USD", "real_calls": 0},
        "think_record": null, "session_confirmed": false,
    });
    std::fs::write(
        output.join("report.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(report)
}

fn main() {
    let result = options(std::env::args().skip(1))
        .and_then(|(dataset, output, cap)| run(&dataset, &output, cap));
    match result {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
#[path = "support/fake_eval_tests.rs"]
mod tests;
