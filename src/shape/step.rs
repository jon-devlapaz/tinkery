//! Shape's agent step: one operator key, then (when it says so) one model call through Pi with
//! read-only tools, and the shared ledger calls that record the result.
//!
//! The model only investigates and drafts words: what it found, with the next question or what the
//! seed confirms, plus separate check records validated only by Tinkery's ledger. It never formats or runs checks. It reads only the folder the operator was shown. Tinkery saves the operator's
//! answer, the question and the findings, builds the seed's evidence and decisions from
//! the ledger, and confirms a seed itself, each through the shared session boundary. The model is never asked to confirm
//! anything, and nothing it replies can confirm a seed. Each step counts against the call budget
//! before Pi starts; with none left it refuses before writing anything.

use crate::{
    pi::{self, Call, CallBudget, Request},
    seed_me::answering::{self, Alternative, Ask, Assumption, Found, Refusal},
    think_record::{self, Event, MadeIn},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const PROMPT: &str = include_str!("prompt.txt");

#[cfg(test)]
mod typed_tests;

/// Reply fields decoded here or delegated unchanged to Tinkery's record validator.
/// The prompt's examples name the same fields.
#[cfg(test)]
pub(super) const FIELDS: &[&str] = &[
    "next",
    "id",
    "label",
    "question",
    "gate",
    "found",
    "checked",
    "observed",
    "claim",
    "type",
    "scope",
    "limits",
    "supported_by",
    "contradicts",
    "suggestion",
    "assumed",
    "text",
    "why",
    "options",
    "choice",
    "tradeoff",
    "undo",
    "against",
    "confidence",
    "confirming",
    "checks",
    "statement",
    "kind",
    "source",
    "how_to_judge",
];

/// How Shape reaches a model: Pi with an explicit model and read-only tools, a call budget shared
/// for the whole run (zero unless the operator sets one), and where Think's records are.
pub struct Agent {
    call: Call,
    budget: Arc<CallBudget>,
    records: Option<PathBuf>,
}

impl Agent {
    /// Pi at `program` with `model` and `thinking` (empty for Pi's default), starting in `folder`
    /// unless Think recorded where a goal was made; `budget` limits every step. Refuses a vague
    /// model. Starts nothing.
    pub fn new(
        program: PathBuf,
        model: String,
        thinking: String,
        folder: Option<PathBuf>,
        budget: Arc<CallBudget>,
        records: Option<PathBuf>,
    ) -> Result<Self, String> {
        if !pi::explicit_model(&model) {
            return Err("Use an explicit --model provider/model-id.".into());
        }
        Ok(Self {
            call: Call {
                program,
                model,
                thinking,
                read_only_tools: true,
                folder,
                timeout: Duration::from_secs(300),
            },
            budget,
            records,
        })
    }

    /// Model calls this run may still make.
    pub fn remaining(&self) -> usize {
        self.budget.remaining()
    }

    /// The run's call limit.
    pub fn limit(&self) -> usize {
        self.budget.limit()
    }

    /// The folder a step for `session` reads unless the operator picks another: where Think
    /// recorded `goal` was made, else the folder this agent was given, else where Tinkery started.
    /// Reads Think's records; blocks, so run it off the paint path.
    pub(super) fn folder(&self, session: &Path, goal: &str) -> PathBuf {
        let made_in = self
            .records
            .as_deref()
            .and_then(|root| think_record::read(root, session, goal).ok())
            .and_then(|read| match read.made_in {
                MadeIn::Repo(path) => Some(path),
                _ => None,
            });
        made_in
            .or_else(|| self.call.folder.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// What one key asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Work {
    /// Save the operator's answer to question `id`, then look into the next question in `folder`.
    Answer {
        id: String,
        words: String,
        expected: u64,
        folder: PathBuf,
    },
    /// Look into the next question (or the seed) from version `expected`, reading `folder`.
    Look { expected: u64, folder: PathBuf },
    /// Confirm seed `revision` with the operator's own words.
    Confirm { revision: String, words: String },
}

/// What a finished step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Done {
    /// A new question is current, with the defaults its step assumed instead of asking.
    Asked(Asked),
    /// A seed draft was saved.
    Drafted,
    /// The seed is confirmed.
    Confirmed,
}

/// What the step that asked a question leaves for the screen, beyond what the ledger records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Asked {
    /// Defaults assumed instead of asking.
    pub(super) assumed: Vec<Assumption>,
    /// A hard-to-undo question's suggestion, not yet recorded, until the operator asks for it.
    pub(super) withheld: Option<String>,
}

/// Why a step stopped short, and whether the operator's answer was saved before it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Failure {
    pub(super) answer_saved: bool,
    pub(super) why: Why,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Why {
    /// Another writer changed the session first; nothing more was written.
    Stale,
    /// No model calls are left.
    Budget(usize),
    /// The model call failed or its reply couldn't be used.
    Model(String),
    /// Tinkery refused; the reason says whether anything saved.
    Refused(String),
    /// The operator stopped it.
    Stopped,
}

/// A running step, off the paint path. Dropping it asks the step to stop; a model call stops
/// promptly, and a helper call already started finishes.
pub(super) struct Running {
    cancelled: Arc<AtomicBool>,
    saved: Arc<AtomicBool>,
    started: Instant,
    result: mpsc::Receiver<Result<Done, Failure>>,
}

impl Running {
    /// Start `work` on `session`.
    pub(super) fn start(work: Work, skill: PathBuf, session: PathBuf, agent: Arc<Agent>) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));
        let (flag, answered) = (cancelled.clone(), saved.clone());
        let (sender, result) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(run(&work, &skill, &session, &agent, &flag, &answered));
        });
        Self {
            cancelled,
            saved,
            started: Instant::now(),
            result,
        }
    }

    /// Whether the operator's answer is saved yet; only an answer step ever saves one.
    pub(super) fn saved(&self) -> bool {
        self.saved.load(Ordering::Relaxed)
    }

    /// How long the step has run.
    pub(super) fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// The result, once it lands. Never blocks.
    pub(super) fn poll(&self) -> Option<Result<Done, Failure>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Failure {
                answer_saved: false,
                why: Why::Model("The step stopped unexpectedly.".into()),
            })),
        }
    }

    /// Ask the step to stop.
    pub(super) fn stop(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

fn refused(answer_saved: bool) -> impl Fn(Refusal) -> Failure {
    move |refusal| Failure {
        answer_saved,
        why: match refusal {
            Refusal::Stale => Why::Stale,
            Refusal::Refused(reason) => Why::Refused(reason),
        },
    }
}

fn run(
    work: &Work,
    skill: &Path,
    session: &Path,
    agent: &Agent,
    cancelled: &AtomicBool,
    saved: &AtomicBool,
) -> Result<Done, Failure> {
    match work {
        Work::Answer {
            id,
            words,
            expected,
            folder,
        } => {
            if agent.remaining() == 0 {
                return Err(Failure {
                    answer_saved: false,
                    why: Why::Budget(agent.limit()),
                });
            }
            let version =
                answering::answer(skill, session, *expected, id, words).map_err(refused(false))?;
            saved.store(true, Ordering::Relaxed);
            look(skill, session, version, folder, agent, cancelled, true)
        }
        Work::Look { expected, folder } => {
            look(skill, session, *expected, folder, agent, cancelled, false)
        }
        Work::Confirm { revision, words } => {
            answering::confirm_seed(skill, session, revision, words).map_err(refused(false))?;
            Ok(Done::Confirmed)
        }
    }
}

fn look(
    skill: &Path,
    session: &Path,
    expected: u64,
    folder: &Path,
    agent: &Agent,
    cancelled: &AtomicBool,
    answer_saved: bool,
) -> Result<Done, Failure> {
    let fail = |why| Failure { answer_saved, why };
    // A decision already recorded, such as an objection, is asked before the model looks for one,
    // and the pre-mortem is put to the operator before any seed, also without a model.
    if answering::promote(skill, session, expected)
        .map_err(refused(answer_saved))?
        .is_some()
        || answering::probe(skill, session, expected)
            .map_err(refused(answer_saved))?
            .is_some()
    {
        return Ok(Done::Asked(Asked::default()));
    }
    if agent.remaining() == 0 {
        return Err(fail(Why::Budget(agent.limit())));
    }
    let ledger = answering::read(skill, session).map_err(refused(answer_saved))?;
    if ledger.version() != expected {
        return Err(fail(Why::Stale));
    }
    let goal = ledger.goal().unwrap_or("").to_owned();
    let settled = think(agent.records.as_deref(), session, &goal);
    let input = json!({
        "goal": goal,
        "answered_in_think": settled,
        "found_so_far": ledger.findings(),
        "ledger": serde_json::from_str::<Value>(&ledger.context()).unwrap_or(Value::Null),
    });
    let call = Call {
        folder: Some(folder.to_path_buf()),
        ..agent.call.clone()
    };
    let contract = answering::contract(skill).map_err(refused(answer_saved))?;
    let instructions = format!("{}\n\nArtifact contract for Shape:\n{contract}", prompt());
    let reply = call
        .complete(
            Some(&agent.budget),
            Request::SeedStep,
            serde_json::to_vec(&input).unwrap_or_default(),
            instructions,
            cancelled,
        )
        .map_err(|error| {
            if cancelled.load(Ordering::Relaxed) {
                fail(Why::Stopped)
            } else {
                fail(Why::Model(error))
            }
        })?;
    match decode(&reply).map_err(|e| fail(Why::Model(e)))? {
        Next::Question(ask) => {
            answering::ask(skill, session, expected, &ask, folder)
                .map_err(refused(answer_saved))?;
            Ok(Done::Asked(Asked {
                withheld: ask.recommendation().filter(|_| ask.hard()),
                assumed: ask.assumed,
            }))
        }
        Next::Seed {
            confirming,
            checks,
            found,
        } => {
            answering::draft_seed(
                skill,
                session,
                expected,
                &confirming,
                &checks,
                &found,
                folder,
            )
            .map_err(refused(answer_saved))?;
            Ok(Done::Drafted)
        }
    }
}

/// Exactly two complete options, each with a known undo cost; anything else gives none.
fn alternatives(value: &Value) -> Vec<Alternative> {
    let options: Vec<Alternative> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| {
            let field =
                |key: &str| Some(o[key].as_str()?.trim().to_owned()).filter(|s| !s.is_empty());
            Some(Alternative {
                choice: field("choice")?,
                tradeoff: field("tradeoff")?,
                undo: field("undo")
                    .map(|u| u.to_lowercase())
                    .filter(|u| ["cheap", "moderate", "hard"].contains(&u.as_str()))?,
            })
        })
        .collect();
    if options.len() == 2 {
        options
    } else {
        Vec::new()
    }
}

/// The system prompt for one step.
pub(super) fn prompt() -> String {
    pi::assemble(PROMPT)
}

/// The questions Think's record says the operator answered for this exact session and goal.
/// Nothing when no record can be attributed.
fn think(records: Option<&Path>, session: &Path, goal: &str) -> Vec<Value> {
    let Some(read) = records.and_then(|root| think_record::read(root, session, goal).ok()) else {
        return Vec::new();
    };
    read.turns
        .iter()
        .filter_map(|turn| match &turn.event {
            Event::Answered {
                question, answer, ..
            }
            | Event::ScopeAnswered {
                question, answer, ..
            } => Some(json!({"question": question, "answer": answer})),
            _ => None,
        })
        .collect()
}

/// What the model proposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Next {
    /// The next question.
    Question(Ask),
    /// The seed's summary and canonical prose sections, and what the step found.
    Seed {
        confirming: String,
        checks: Value,
        found: Vec<Found>,
    },
}

/// The model's reply as one JSON object (surrounding prose or a code fence is tolerated).
/// Refuses anything else, including a reply that asks to confirm: only the operator confirms.
pub(super) fn decode(reply: &str) -> Result<Next, String> {
    let unusable = || "The model's reply wasn't a question or a seed draft.".to_owned();
    let (start, end) = (reply.find('{').ok_or_else(unusable)?, reply.rfind('}'));
    let value: Value =
        serde_json::from_str(&reply[start..=end.ok_or_else(unusable)?]).map_err(|_| unusable())?;
    let text = |key: &str| {
        value[key]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    if !matches!(value["next"].as_str(), Some("question" | "seed")) {
        return Err(unusable());
    }
    let found: Vec<Found> = serde_json::from_value(value["found"].clone()).map_err(|e|
        format!("Each finding needs an id, a typed claim, checked and observed; untyped receipts aren't accepted: {e}"))?;
    match value["next"].as_str() {
        Some("question") => {
            let question = text("question").ok_or_else(unusable)?;
            Ok(Next::Question(Ask {
                id: text("id").unwrap_or_else(|| "question".into()),
                label: text("label").unwrap_or_else(|| question.clone()),
                gate: text("gate").unwrap_or_else(|| question.clone()),
                question,
                found,
                supported_by: value.get("supported_by").cloned(),
                suggestion: text("suggestion"),
                assumed: value["assumed"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| {
                        Some(Assumption {
                            text: a["text"].as_str()?.trim().to_owned(),
                            why: a["why"].as_str()?.trim().to_owned(),
                        })
                    })
                    .filter(|a| !a.text.is_empty() && !a.why.is_empty())
                    .take(5)
                    .collect(),
                options: alternatives(&value["options"]),
                against: text("against"),
                confidence: text("confidence")
                    .filter(|c| ["low", "medium", "high"].contains(&c.as_str())),
            }))
        }
        Some("seed") => Ok(Next::Seed {
            confirming: text("confirming").ok_or_else(unusable)?,
            checks: value["checks"]
                .as_array()
                .filter(|checks| !checks.is_empty())
                .map(|checks| Value::Array(checks.clone()))
                .ok_or_else(unusable)?,
            found,
        }),
        _ => Err(unusable()),
    }
}
