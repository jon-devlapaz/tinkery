//! Explicit-data evaluation of think. Replies are scripted by default; `--live` sends the supplied
//! words to Pi only with operator data, a written permission and a positive call limit, all counted
//! by one budget at the Pi boundary. Never saves a session or confirms a goal.
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
        BoardHost, BoardRequest, BrainDump, DecisionReview, Guess, decision_review_input,
    },
    drafting::{CallBudget, PiHost, Request},
    record::hash,
};

const USAGE: &str = "Supply --dataset FILE --output NEW_DIRECTORY --max-requests N. Replies are scripted. \
Add --live --model provider/model-id --seed-me SKILL.md [--pi-command PATH] [--thinking LEVEL] to call Pi; \
there is no default dataset, model or limit.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    provenance: Provenance,
    /// Who allowed these words to go to a provider, in their own terms. Required for `--live`.
    #[serde(default)]
    permission: Option<String>,
    steps: Vec<Step>,
    #[serde(default)]
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    kind: Request,
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
    budget: Arc<CallBudget>,
    replies: Mutex<VecDeque<Reply>>,
}
impl Fake {
    fn request(&self, kind: Request) -> Result<String, String> {
        // The same budget as Pi: charged before the scripted reply is read.
        let slot = self.budget.reserve(kind)?;
        let result = match self.replies.lock().unwrap().pop_front() {
            Some(reply) if reply.kind == kind => match reply.result {
                ReplyResult::Text(text) => Ok(text),
                ReplyResult::Failure(error) => Err(error),
            },
            Some(_) => Err("Fake reply kind does not match the next request.".into()),
            None => Err("No scripted fake reply remains.".into()),
        };
        self.budget.finish(slot, &result);
        result
    }
}
impl BoardHost for Fake {
    fn reshape(&self, request: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        let raw = self.request(Request::Shaping)?;
        let mut changes = vec![];
        // Like PiHost, readable invalid boards still get a review before decoding.
        let review = decision_review_input(&request, &raw).and_then(|_| {
            self.request(Request::DecisionReview)
                .ok()
                .and_then(|review| DecisionReview::decode(&review, &mut changes))
        });
        Guess::decode_reviewed(&raw, review.as_ref(), &request, &mut changes)
    }
}

/// Pi settings for a live run; absent for a scripted one.
#[derive(Debug)]
struct Live {
    pi_command: PathBuf,
    model: String,
    seed_me: PathBuf,
    thinking: String,
}
#[derive(Debug)]
struct Options {
    dataset: PathBuf,
    output: PathBuf,
    cap: usize,
    live: Option<Live>,
}

fn options(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut args = args;
    let mut live = false;
    let mut values = std::collections::BTreeMap::new();
    while let Some(arg) = args.next() {
        if arg == "--live" && !live {
            live = true;
            continue;
        }
        let known = [
            "--dataset",
            "--output",
            "--max-requests",
            "--model",
            "--seed-me",
            "--pi-command",
            "--thinking",
        ];
        let value = args.next().ok_or(USAGE)?;
        if !known.contains(&arg.as_str()) || values.insert(arg, value).is_some() {
            return Err(USAGE.into());
        }
    }
    let mut take = |name: &str| values.remove(name);
    let (dataset, output) = (
        take("--dataset").ok_or(USAGE)?,
        take("--output").ok_or(USAGE)?,
    );
    let cap = take("--max-requests")
        .ok_or(USAGE)?
        .parse()
        .map_err(|_| USAGE)?;
    let live = if live {
        Some(Live {
            model: take("--model").ok_or(USAGE)?,
            seed_me: take("--seed-me").ok_or(USAGE)?.into(),
            pi_command: take("--pi-command").unwrap_or_else(|| "pi".into()).into(),
            thinking: take("--thinking").unwrap_or_else(|| "low".into()),
        })
    } else {
        None
    };
    if !values.is_empty() {
        return Err(format!("Pi options need --live. {USAGE}"));
    }
    Ok(Options {
        dataset: dataset.into(),
        output: output.into(),
        cap,
        live,
    })
}

fn validate(dataset: &Dataset, output: &Path, cap: usize, live: bool) -> Result<(), String> {
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
    let parent = output.parent().and_then(|p| p.canonicalize().ok());
    if !output.is_absolute() || output.exists() {
        return Err("Output must be a new absolute directory with an existing parent.".into());
    }
    let Some(parent) = parent else {
        return Err("Output must be a new absolute directory with an existing parent.".into());
    };
    if !live {
        let temp = std::env::temp_dir()
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !parent.starts_with(temp) {
            return Err("A scripted run writes under the system temporary directory only.".into());
        }
        return Ok(());
    }
    if !matches!(dataset.provenance.kind, InputKind::OperatorSupplied) {
        return Err(
            "A live run sends operator_supplied words only; synthetic fixtures stay scripted."
                .into(),
        );
    }
    if dataset
        .permission
        .as_deref()
        .is_none_or(|p| p.trim().is_empty())
    {
        return Err("A live run needs the dataset's permission: who allowed these words to reach a provider.".into());
    }
    if !dataset.replies.is_empty() {
        return Err("A live run takes no scripted replies; remove them or drop --live.".into());
    }
    if cap == 0 {
        return Err("A live run needs a positive --max-requests; nothing was sent.".into());
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let operator_data = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share"))
        .into_iter()
        .flat_map(|share| [share.join("seed-me"), share.join("tinkery")])
        .filter_map(|p| p.canonicalize().ok());
    if parent.starts_with(&repository) || operator_data.into_iter().any(|p| parent.starts_with(p)) {
        return Err(
            "Live output stays outside the repository and the operator's Seed Me and Tinkery data."
                .into(),
        );
    }
    Ok(())
}

fn settle(app: &mut BrainDump, limit: Duration) -> Result<(), String> {
    let deadline = Instant::now() + limit;
    while app.running() && Instant::now() < deadline {
        app.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    if app.running() {
        Err(format!(
            "The run did not settle within {} seconds.",
            limit.as_secs()
        ))
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

fn run(options: &Options) -> Result<Value, String> {
    let (dataset_path, output, cap) = (&options.dataset, &options.output, options.cap);
    let input = std::fs::read_to_string(dataset_path)
        .map_err(|e| format!("Read the supplied dataset: {e}. {USAGE}"))?;
    let mut dataset: Dataset =
        serde_json::from_str(&input).map_err(|e| format!("Invalid dataset: {e}"))?;
    validate(&dataset, output, cap, options.live.is_some())?;
    let budget = CallBudget::new(cap);
    let fake = Arc::new(Fake {
        budget: budget.clone(),
        replies: Mutex::new(std::mem::take(&mut dataset.replies)),
    });
    let (host, settle_limit): (Arc<dyn BoardHost>, _) = match &options.live {
        None => (fake.clone(), Duration::from_secs(5)),
        Some(live) => (
            Arc::new(
                PiHost::new(
                    live.pi_command.clone(),
                    live.model.clone(),
                    live.seed_me.clone(),
                )?
                .with_thinking(&live.thinking)?
                .with_budget(budget.clone()),
            ),
            // Two 90-second Pi calls per send, plus room to settle.
            Duration::from_secs(200),
        ),
    };
    let provider = options
        .live
        .as_ref()
        .map(|live| json!({"pi_command": live.pi_command, "origin": host.origin()}));
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    let mut app = BrainDump::with_host(host);
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
        settle(&mut app, settle_limit)?;
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
            "Not judged automatically. Source preservation and decoder checks do not prove prompt-injection resistance.",
        ),
        (
            "exact_confirmation_handoff",
            "Not exercised: this runner never confirms or saves. Use the isolated goal_confirmation_terminal.py checks separately.",
        ),
    ] {
        checks.insert(name.into(), json!({"status": "unknown", "reason": reason}));
    }
    let attempts = budget.attempts();
    let (refused, calls): (Vec<_>, Vec<_>) = attempts.iter().partition(|a| a.outcome == "refused");
    let count = |kind| calls.iter().filter(|a| a.kind == kind).count();
    let live = options.live.is_some();
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
        "execution": if live { "live" } else { "fake" },
        "input": {"path": dataset_path, "hash": hash(&input), "provenance": dataset.provenance,
            "permission": dataset.permission},
        "provider": provider,
        "revision": {"checkout_at_run": source, "working_tree_dirty": dirty,
            "compiled_library_revision": null,
            "limit": "Runtime Git metadata describes the checkout, not necessarily this executable. Compiled library revision is unknown; rebuild at the revision being evaluated.",
            "compiled_runner_hash": hash(include_str!("think_eval.rs")),
            "reading_prompt_file_hash": hash(include_str!("../src/think/brain_dump/prompts/reading.txt")),
            "review_prompt_file_hash": hash(include_str!("../src/think/brain_dump/prompts/decision_review.txt")),
            "prompts_executed": live},
        "requests": {"cap": budget.limit(), "used": calls.len(), "calls": calls,
            "refused_before_invocation": refused,
            "unused_replies": (!live).then(|| fake.replies.lock().unwrap().len()),
            "shaping": count(Request::Shaping), "decision_review": count(Request::DecisionReview)},
        "steps": steps, "reading_current": current, "automated_checks": checks,
        "human_baseline": {"recognition": null, "active_attention_seconds": null,
            "waiting_seconds": null, "corrections": null, "cost": null,
            "note": "Unknown until measured by a person. Fake timings and replies are not a human baseline."},
        "provider_cost": if live {
            json!({"amount": null, "currency": null, "real_calls": calls.len(),
                "note": "Charges are unknown here; read them from the provider account."})
        } else {
            json!({"amount": 0, "currency": "USD", "real_calls": 0})
        },
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
    let result = options(std::env::args().skip(1)).and_then(|options| run(&options));
    match result {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
#[path = "support/think_eval_tests.rs"]
mod tests;
