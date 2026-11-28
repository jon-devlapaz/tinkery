//! One goal's trace: every record of its work, from Think to Taste, joined by the goal's
//! `session_id` and returned as lines in reading order. Reads only. It writes no file, runs no
//! command and calls no model. Operator words are copied verbatim. A transcript or other bulky
//! record appears only as its path and SHA-256, never as content. A record that is missing or
//! cannot be read is an `unknown` line with its reason, never a silent gap or a pass.

use crate::{
    regular_file, sdlc,
    seed_me::{self, ledger},
    think_record,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The line format's name and version. Renaming or removing a field changes it.
pub const FORMAT: &str = "tinkery.trace/1";

// The largest record this reader hashes or parses; a larger one is an unknown line.
const LIMIT: u64 = 256 * 1024 * 1024;

/// The lines for the confirmed goal saved in `session`. Think comes from `records`. Runs come
/// from `checkout`, or else from the repository the Think record names. Refuses only a session
/// whose ledger cannot be read, because then no identity joins the rest.
pub fn session(
    session: &Path,
    checkout: Option<&Path>,
    records: Result<PathBuf, String>,
) -> Result<Vec<Value>, String> {
    let session = session
        .canonicalize()
        .map_err(|e| format!("{}: {e}", session.display()))?;
    let ledger = ledger::load(&session)?.to_json()?;
    let id = ledger["session_id"]
        .as_str()
        .ok_or("The saved session has no identity.")?;
    let goal = ledger["goal"].as_str().unwrap_or_default();
    let mut trace = Trace {
        session: id.into(),
        lines: vec![],
    };
    trace.event(
        "goal",
        "session",
        None,
        None,
        json!({
            "format": FORMAT, "goal": goal, "status": ledger["status"],
            "path": session, "created_at": ledger["created_at"],
        }),
    );

    let mut repo = None;
    match records.map_err(|why| (why.clone(), why)).and_then(|root| {
        think_record::read(&root, &session, goal).map_err(|e| (e.message, e.details))
    }) {
        Ok(history) => {
            if let think_record::MadeIn::Repo(root) = &history.made_in {
                repo = Some(root.clone());
            }
            let source = history.record.as_deref().map(link);
            trace.event("think", "record", None, None, json!({"source": source}));
            for turn in history.turns {
                let mut fields = serde_json::to_value(&turn.event).expect("events serialize");
                let kind = fields["kind"].as_str().unwrap_or_default().to_owned();
                if let Some(f) = fields.as_object_mut() {
                    f.remove("kind");
                }
                let actor = match kind.as_str() {
                    "words" | "answered" | "scope_answered" | "skipped" | "restored" => "operator",
                    "suggested" => "model",
                    _ => "tinkery",
                };
                trace.event("think", &kind, Some(turn.at), Some(actor), fields);
            }
        }
        Err((message, details)) => {
            trace.unknown("think", "record", &format!("{message} {details}"))
        }
    }
    trace.unknown(
        "think",
        "model transcript",
        "Tinkery does not keep Think model transcripts; it counts responses, then deletes them.",
    );

    trace.event(
        "shape",
        "ledger",
        ledger["updated_at"].as_str().map(str::to_owned),
        None,
        json!({
            "revision": ledger["revision"], "source": link(&session.join(seed_me::files::LEDGER)),
        }),
    );
    let mut retried = vec![];
    for node in ledger["nodes"].as_array().into_iter().flatten() {
        for entry in node["history"].as_array().into_iter().flatten() {
            let at = entry["superseded_at"].as_str().map(str::to_owned);
            trace.event(
                "shape",
                "superseded",
                at,
                None,
                json!({
                    "id": node["id"], "status": entry["state"]["status"], "reason": entry["reason"],
                }),
            );
        }
        let user = node["authority"] == "user";
        let mut fields = json!({
            "id": node["id"], "node": node["kind"], "status": node["status"],
            "authority": node["authority"], "question": node["question"], "answer": node["answer"],
        });
        if let Some(f) = fields.as_object_mut() {
            f.retain(|_, v| !v.is_null());
        }
        trace.event("shape", "node", None, user.then_some("operator"), fields);
        for receipt in node["evidence"].as_array().into_iter().flatten() {
            if receipt["checked"] == "Taste rejection" {
                let at = receipt["at"].as_str().map(str::to_owned);
                trace.event(
                    "retry",
                    "rejected",
                    at,
                    Some("operator"),
                    json!({
                        "run_id": receipt["artifact"], "words": receipt["observed"],
                    }),
                );
                retried.extend(receipt["artifact"].as_str().map(str::to_owned));
            }
        }
    }
    trace.unknown(
        "shape",
        "model transcript",
        "Tinkery does not keep Shape model transcripts; it counts responses, then deletes them.",
    );

    let confirmation = session.join(seed_me::files::CONFIRMATION);
    match read_json(&confirmation) {
        Ok(value) => trace.event(
            "confirm",
            "seed confirmed",
            value["confirmed_at"].as_str().map(str::to_owned),
            value["operator"].as_str(),
            json!({
                "revision": value["revision"], "source": value["source"],
                "seed_sha256": value["seed_sha256"], "record": link(&confirmation),
            }),
        ),
        Err(why) => trace.unknown("confirm", "seed confirmed", &why),
    }

    let Some(checkout) = checkout.map(Path::to_path_buf).or(repo) else {
        trace.unknown(
            "run",
            "runs",
            "No checkout to read runs from; pass --sdlc-root ROOT.",
        );
        return Ok(trace.lines);
    };
    let runs = runs(&checkout);
    for run_id in retried {
        let prior = runs
            .iter()
            .find(|(_, run)| run["run_id"] == run_id.as_str());
        match prior.and_then(|(_, run)| sdlc::Seed::parse(run).ok().flatten()) {
            Some(seed) => trace.event(
                "retry",
                "prior session",
                None,
                None,
                json!({
                    "run_id": run_id, "prior_session": seed.source().parent(),
                }),
            ),
            None => trace.unknown(
                "retry",
                "prior session",
                &format!("Run {run_id} is not in {}.", checkout.display()),
            ),
        }
    }
    let seed = session.join(seed_me::files::SEED);
    let mut found = false;
    for (folder, run) in &runs {
        let Ok(Some(bound)) = sdlc::Seed::parse(run) else {
            continue;
        };
        if bound.source() != seed {
            continue;
        }
        found = true;
        trace.run(
            folder,
            run,
            seed_me::runs::matches(&bound, &seed).unwrap_or(false),
        );
    }
    if !found {
        trace.unknown(
            "run",
            "runs",
            &format!(
                "No run in {} was built from this session.",
                checkout.display()
            ),
        );
    }
    Ok(trace.lines)
}

struct Trace {
    session: String,
    lines: Vec<Value>,
}

impl Trace {
    fn line(
        &mut self,
        kind: &str,
        stage: &str,
        name: &str,
        at: Option<String>,
        actor: Option<&str>,
        fields: Value,
    ) {
        // The shared keys come last, so a record's own field can never replace them.
        let mut line = match fields {
            Value::Object(fields) => fields,
            _ => Map::new(),
        };
        line.insert("session".into(), self.session.clone().into());
        line.insert("kind".into(), kind.into());
        line.insert("stage".into(), stage.into());
        line.insert("name".into(), name.into());
        line.insert("at".into(), at.into());
        line.insert("actor".into(), actor.into());
        self.lines.push(Value::Object(line));
    }

    fn event(
        &mut self,
        stage: &str,
        name: &str,
        at: Option<String>,
        actor: Option<&str>,
        fields: Value,
    ) {
        self.line("event", stage, name, at, actor, fields);
    }

    fn unknown(&mut self, stage: &str, name: &str, reason: &str) {
        self.line(
            "unknown",
            stage,
            name,
            None,
            None,
            json!({ "reason": reason }),
        );
    }

    /// One run folder, in the order its records were made.
    fn run(&mut self, folder: &Path, run: &Value, seed_matches: bool) {
        let slug = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let at_run = |mut fields: Value| {
            fields["run"] = slug.into();
            fields
        };
        self.event("run", "run", None, None, at_run(json!({
            "run_id": run["run_id"], "seed_matches": seed_matches, "source": link(&folder.join("run.json")),
        })));
        for (name, gate) in files(&folder.join("decisions"), "json") {
            let at = name
                .split('-')
                .nth(1)
                .and_then(|ns| ns.parse().ok())
                .map(from_ns);
            match read_json(&gate) {
                Ok(value) => self.event("gate", "decision", at, value["actor"].as_str().or(value["reviewer"].as_str()), at_run(json!({
                    "gate": name.split('-').next(), "decision": value["decision"],
                    "reviewer": value["reviewer"], "source": value["source"], "reason": value["reason"],
                }))),
                Err(why) => self.unknown("gate", "decision", &why),
            }
        }
        for value in jsonl(&folder.join("worker-authorization.jsonl")) {
            match value {
                Ok(value) => self.event("worker", "authorization", None, Some("agent"), at_run(json!({
                    "step": value["step"], "action": value["action"], "authority": value["authority"],
                    "authorization": value["authorization"],
                }))),
                Err(why) => self.unknown("worker", "authorization", &why),
            }
        }
        let steps = files(&folder.join("steps"), "json");
        if steps.is_empty() {
            self.unknown(
                "worker",
                "step",
                "No worker step was prepared for this run.",
            );
        }
        for (name, contract) in steps {
            let step = name.trim_end_matches(".json");
            let value = match read_json(&contract) {
                Ok(value) => value,
                Err(why) => {
                    self.unknown("worker", "step", &why);
                    continue;
                }
            };
            self.event("worker", "step", value["prepared_ns"].as_u64().map(from_ns), Some("worker"), at_run(json!({
                "step": step, "state": value["state"], "model": value["model"],
                "max_responses": value["max_responses"], "pi_session": value["session"], "source": link(&contract),
            })));
            match transcript(&contract, value["session"].as_str()) {
                Ok(path) => self.event(
                    "worker",
                    "transcript",
                    None,
                    Some("worker"),
                    at_run(json!({
                        "step": step, "source": link(&path),
                    })),
                ),
                Err(why) => self.unknown("worker", "transcript", &why),
            }
        }
        self.unknown(
            "worker",
            "driver session",
            "The driver's session is not recorded with the run.",
        );
        let output = folder.join("04-test/output");
        match read_json(&output.join("verification.json")) {
            Ok(value) => {
                self.event("verify", "verification", None, None, at_run(json!({
                    "result": value["result"], "error": value["error"], "candidate": value["candidate"]["head"],
                    "source": link(&output.join("verification.json")), "log": link(&output.join("test-log.md")),
                })));
                for group in ["seed_checks", "checklist_items"] {
                    for (id, state) in value[group].as_object().into_iter().flatten() {
                        self.line("score", "verify", "check", None, None, at_run(json!({
                            "group": group, "id": id, "value": state, "candidate": value["candidate"]["head"],
                        })));
                    }
                }
            }
            Err(why) => self.unknown("verify", "verification", &why),
        }
        for value in jsonl(&output.join("probes.jsonl")).into_iter().flatten() {
            self.event(
                "verify",
                "probe",
                None,
                None,
                at_run(json!({ "probe": value })),
            );
        }
        let review = folder.join("05-deploy/output/REVIEW-findings.md");
        match review.exists() {
            true => self.event(
                "review",
                "findings",
                None,
                Some("reviewer"),
                at_run(json!({ "source": link(&review) })),
            ),
            false => self.unknown(
                "review",
                "findings",
                "No review findings are recorded for this run.",
            ),
        }
        match read_json(&folder.join("pr.json")) {
            Ok(value) => self.event(
                "pr",
                "pull request",
                value["time_ns"].as_u64().map(from_ns),
                None,
                at_run(json!({
                    "url": value["url"], "candidate": value["candidate"],
                })),
            ),
            Err(why) => self.unknown("pr", "pull request", &why),
        }
        let labels = jsonl(&folder.join("labels.jsonl"));
        if labels.is_empty() {
            self.unknown(
                "taste",
                "verdict",
                "No Taste verdict is recorded for this run.",
            );
        }
        for value in labels {
            match value {
                Ok(value) => {
                    let commit = value["source"]
                        .as_str()
                        .and_then(|s| s.strip_prefix("taste@"));
                    self.line("score", "taste", "verdict", value["time_ns"].as_u64().map(from_ns), Some("operator"), at_run(json!({
                        "value": value["kind"], "words": value["words"], "reviewer": value["reviewer"],
                        "source": value["source"], "commit": commit,
                    })));
                }
                Err(why) => self.unknown("taste", "verdict", &why),
            }
        }
    }
}

/// The path and SHA-256 of a file, or why it has none. Never its content.
fn link(path: &Path) -> Value {
    match regular_file::read_bounded(path, LIMIT, "not a regular file") {
        Ok(regular_file::Bounded {
            bytes: Some(bytes), ..
        }) => {
            json!({ "path": path, "sha256": format!("{:x}", Sha256::digest(bytes)) })
        }
        Ok(_) => json!({ "path": path, "unknown": "larger than the trace reads" }),
        Err(error) => json!({ "path": path, "unknown": error.to_string() }),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = regular_file::read_bounded(path, LIMIT, "not a regular file")
        .map_err(|e| format!("{}: {e}", path.display()))?
        .bytes
        .ok_or_else(|| format!("{}: larger than the trace reads", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Each line of a JSON-lines file; a missing file has none.
fn jsonl(path: &Path) -> Vec<Result<Value, String>> {
    if !path.exists() {
        return vec![];
    }
    match regular_file::read_text(path, "not a regular file") {
        Ok(text) => text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(|e| format!("{}: {e}", path.display())))
            .collect(),
        Err(error) => vec![Err(format!("{}: {error}", path.display()))],
    }
}

/// The files in `folder` with this extension, by name.
fn files(folder: &Path, extension: &str) -> Vec<(String, PathBuf)> {
    let mut found: Vec<_> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == extension) && path.is_file())
        .filter_map(|path| Some((path.file_name()?.to_str()?.to_owned(), path)))
        .collect();
    found.sort();
    found
}

/// Each `runs/<slug>/run.json` in `checkout` that parses, by slug.
fn runs(checkout: &Path) -> Vec<(PathBuf, Value)> {
    let mut found: Vec<_> = std::fs::read_dir(checkout.join("runs"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|folder| Some((read_json(&folder.join("run.json")).ok()?, folder)))
        .map(|(run, folder)| (folder, run))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// The worker's Pi session file: the folder launch recorded beside the step contract, and the
/// file whose header names the step's session.
fn transcript(contract: &Path, session: Option<&str>) -> Result<PathBuf, String> {
    let session = session.ok_or("The step records no Pi session.")?;
    let recorded = contract.with_extension("session-dir");
    let folder = regular_file::read_text(&recorded, "not a regular file")
        .map_err(|e| format!("{}: {e}", recorded.display()))?;
    let folder = PathBuf::from(folder.trim_end_matches('\n'));
    files(&folder, "jsonl")
        .into_iter()
        .map(|(_, path)| path)
        .find(|path| {
            let first = regular_file::read_text(path, "not a regular file").ok();
            let header = first
                .as_deref()
                .and_then(|text| text.lines().next())
                .and_then(|line| serde_json::from_str::<Value>(line).ok());
            header.is_some_and(|h| h["type"] == "session" && h["id"] == session)
        })
        .ok_or_else(|| format!("No Pi session {session} in {}.", folder.display()))
}

/// Nanoseconds since 1970 as an RFC 3339 UTC time.
fn from_ns(ns: u64) -> String {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_nanos(ns);
    let (stamp, nanos) = crate::calendar::utc(time, ':');
    format!("{stamp}.{nanos:09}+00:00")
}
