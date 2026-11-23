//! Shape's agent step: one operator key, then (when it says so) one model call through the chosen
//! harness with read-only tools, and the shared ledger calls that record the result.
//!
//! The model only investigates and drafts words: what it found, with the next question or what the
//! seed confirms, plus separate check records validated only by Tinkery's ledger. It never formats or runs checks. It reads only the folder the operator was shown. Tinkery saves the operator's
//! answer, the question and the findings, builds the seed's evidence and decisions from
//! the ledger, and confirms a seed itself, each through the shared session boundary. The model is never asked to confirm
//! anything, and nothing it replies can confirm a seed. Each step counts against the call budget
//! before the model starts; with none left it refuses before writing anything.

use crate::{
    confirmation::{Actor, GoalConfirmation},
    pi::{self, Call, Drafting},
    seed_me::ledger,
    shape::answering::{self, Alternative, Ask, Assumption, Found, Refusal, Rule, Undo},
    think_record::{self, MadeIn, ShapeContext},
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

#[cfg(all(test, unix))]
mod reply_contract_tests;
#[cfg(test)]
mod typed_tests;
#[cfg(test)]
mod unknown_tests;

mod review_reply;
mod teach_back;

/// One bounded host diagnostic for an explicit retry against the same saved context.
/// It is neither an operator answer nor ledger evidence, and is never persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RetryFeedback {
    session: PathBuf,
    version: u64,
    folder: PathBuf,
    reason: String,
    truncated: bool,
}

impl RetryFeedback {
    const LIMIT: usize = 2048;

    /// Whether this rejection still belongs to the session version and folder being read.
    pub(super) fn applies(&self, session: &Path, version: u64, folder: &Path) -> bool {
        self.session == session && self.version == version && self.folder == folder
    }

    fn new(session: &Path, version: u64, folder: &Path, reason: &str) -> Option<Self> {
        let line = reason
            .lines()
            .next()
            .filter(|line| !line.trim().is_empty())?;
        let truncated = line.len() > Self::LIMIT || reason.lines().nth(1).is_some();
        let mut reason = line.to_owned();
        if reason.len() > Self::LIMIT {
            let mut end = Self::LIMIT - '…'.len_utf8();
            while !reason.is_char_boundary(end) {
                end -= 1;
            }
            reason.truncate(end);
            reason.push('…');
        }
        Some(Self {
            session: session.into(),
            version,
            folder: folder.into(),
            reason,
            truncated,
        })
    }

    fn input(&self) -> Value {
        json!({"reason": self.reason, "truncated": self.truncated})
    }
}

/// Current reply fields decoded here or delegated unchanged to Tinkery's record validator.
/// The prompt's examples name the same fields; `confidence` is asked for and never read.
#[cfg(test)]
pub(super) const FIELDS: &[&str] = &[
    "next",
    "id",
    "label",
    "question",
    "gate",
    "found",
    "resolves",
    "unknown",
    "resolved_by",
    "checked",
    "observed",
    "claim",
    "type",
    "scope",
    "limits",
    "supported_by",
    "prerequisites",
    "contradicts",
    "suggestion",
    "assumed",
    "text",
    "why",
    "readings",
    "chosen",
    "other",
    "options",
    "choice",
    "tradeoff",
    "undo",
    "against",
    "confidence",
    "confirming",
    "review",
    "contradictions",
    "decided",
    "done",
    "failure_case",
    "checks",
    "statement",
    "kind",
    "source",
    "how_to_judge",
    "cmd",
    "expect",
    "cwd",
    "missing",
];

/// How Shape reaches a model, and what a step reads besides the ledger: the launch's
/// [`Drafting`] transport, where Think's records are, and the folder a step reads when nothing
/// says where the goal was made.
pub struct Agent {
    drafting: Drafting,
    folder: Option<PathBuf>,
    records: Option<PathBuf>,
}

impl Agent {
    /// Shape's steps through `drafting`, reading `folder` unless Think or Tend recorded where a
    /// goal was made, with Think's records in `records`. Starts and reads nothing.
    pub fn new(drafting: Drafting, folder: Option<PathBuf>, records: Option<PathBuf>) -> Self {
        Self {
            drafting,
            folder,
            records,
        }
    }

    /// Model calls this run may still make.
    pub fn remaining(&self) -> usize {
        self.drafting.budget.remaining()
    }

    /// The run's call limit.
    pub fn limit(&self) -> usize {
        self.drafting.budget.limit()
    }

    /// The allowance sentence Think shows too, so both phases count the same way.
    pub fn left(&self) -> String {
        self.drafting.budget.left()
    }

    /// What a step for `session` reads unless the operator picks another folder: where Tend or
    /// Think recorded `goal` was made, else the folder this agent was given, else where Tinkery
    /// started; and the questions Think's record says the operator answered. Reads Think's
    /// record once. A record that exists but cannot be attributed sends no answers and says
    /// why; a broken Tend origin refuses. Blocks, so run it off the paint path.
    pub(super) fn reads(&self, session: &Path, goal: &str) -> Result<Reads, String> {
        let creation = crate::seed_me::connection::read(session, goal)?;
        let context = self
            .records
            .as_deref()
            .map(|root| think_record::read_for_shape(root, session, goal))
            .transpose()
            .map(Option::flatten);
        let made_in = if creation.is_some() {
            let root = self
                .records
                .as_deref()
                .ok_or("The Tend origin cannot be checked without the records folder.")?;
            match crate::seed_me::connection::origins(root, &[(session, goal)])
                .map_err(|e| e.message)?
                .remove(0)
            {
                MadeIn::Repo(path) if path.is_dir() => Some(path),
                MadeIn::Invalid(reason) => return Err(reason),
                _ => {
                    return Err(
                        "The Tend repository is unavailable. Restore it before drafting.".into(),
                    );
                }
            }
        } else {
            match &context {
                Ok(Some(ShapeContext {
                    made_in: MadeIn::Repo(path),
                    ..
                })) => Some(path.clone()),
                _ => None,
            }
        };
        let folder = made_in
            .or_else(|| self.folder.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(match context {
            Ok(context) => Reads {
                think: context.map_or_else(Vec::new, |context| {
                    context
                        .answered
                        .into_iter()
                        .map(|a| json!({"question": a.question, "answer": a.answer}))
                        .collect()
                }),
                ..Reads::from(folder)
            },
            Err(unread) => Reads {
                unread: Some(unread.message),
                ..Reads::from(folder)
            },
        })
    }
}

/// What a model step reads besides the ledger: one folder, which the operator can change, and
/// what Think's record for this exact session and goal holds.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Reads {
    pub(super) folder: PathBuf,
    /// The questions Think says the operator answered, in recorded order.
    think: Vec<Value>,
    /// Why a Think record could not be attributed; no answers are sent then.
    unread: Option<String>,
}

impl From<PathBuf> for Reads {
    /// `folder` with nothing from Think.
    fn from(folder: PathBuf) -> Self {
        Self {
            folder,
            think: Vec::new(),
            unread: None,
        }
    }
}

impl Reads {
    /// What the operator must hear about Think's record: why its answers are not sent, if so.
    pub(super) fn unread(&self) -> Option<String> {
        self.unread
            .as_ref()
            .map(|why| format!("{why} The step sends no Think answers."))
    }

    /// The same, reading `folder` instead.
    pub(super) fn reading(&self, folder: PathBuf) -> Self {
        Self {
            folder,
            ..self.clone()
        }
    }
}

/// A correction stays bound to what was shown, even if a failed save refreshes the draft.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CheckEdit {
    pub(super) expected: u64,
    pub(super) seed: String,
    pub(super) target: ledger::DraftCheck,
    pub(super) method: ledger::CheckMethod,
}

/// What one key asked for: a model step, a confirmation, or a change the ledger makes without a
/// model. Every one runs through [`Steps::run`].
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Work {
    /// Save the operator's answer to question `id`, then look into the next question through `reads`.
    Answer {
        id: String,
        words: String,
        /// The operator's relayed words and their origin, recorded as the operator's own answer.
        relay: Option<answering::Relay>,
        expected: u64,
        reads: Reads,
    },
    /// Publish the operator's explicit action, then look into the next.
    Respond {
        id: String,
        action: ledger::AnswerAction,
        expected: u64,
        reads: Reads,
    },
    /// Look into the next question (or the seed) from version `expected`, through `reads`.
    Look {
        expected: u64,
        reads: Reads,
        feedback: Option<RetryFeedback>,
    },
    /// Compare the operator's teach-back with the exact displayed draft, using one budgeted call.
    TeachBack {
        revision: String,
        words: String,
        expected: u64,
        seed: String,
    },
    /// Save the operator's change to seed draft `revision` from `expected`, then redraft by looking
    /// through `reads`. Never confirms.
    Change {
        revision: String,
        words: String,
        expected: u64,
        reads: Reads,
    },
    /// Confirm only the draft bytes and ledger version the operator read.
    Confirm {
        revision: String,
        words: String,
        expected: u64,
        seed: String,
    },
    /// Replace one draft check's command or how-to-judge words. No model runs.
    RepairCheck { edit: CheckEdit, words: String },
    /// Answer a deferred prerequisite. No model runs.
    Revisit {
        id: String,
        expected: u64,
        words: String,
    },
    /// Keep flagged node `id` with the operator's reason. No model runs.
    Keep {
        id: String,
        expected: u64,
        reason: String,
    },
    /// End the session for good. No model runs.
    End(answering::Ending),
    /// Make an assumed default an open question. No model runs.
    Object { expected: u64, text: String },
    /// Record the withheld suggestion on question `id`. No model runs.
    Recommend {
        id: String,
        expected: u64,
        text: String,
    },
}

impl Work {
    /// Whether this work calls a model, and so needs an agent with responses left.
    pub(super) fn calls_model(&self) -> bool {
        matches!(
            self,
            Self::Answer { .. }
                | Self::Respond { .. }
                | Self::Look { .. }
                | Self::TeachBack { .. }
                | Self::Change { .. }
        )
    }
}

/// What a finished step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Done {
    /// A new question is current, with the defaults its step assumed instead of asking.
    Asked(Asked),
    /// A seed draft was saved.
    Drafted,
    /// Teach-back words and the model's mismatches were saved; confirming stays available.
    Compared,
    /// The seed is confirmed.
    Confirmed,
    /// A change without a model was saved.
    Wrote,
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
    pub(super) feedback: Option<Box<RetryFeedback>>,
}

impl Failure {
    fn rejected(
        mut self,
        session: &Path,
        version: u64,
        folder: &Path,
        cancelled: &AtomicBool,
    ) -> Self {
        let reason = match &self.why {
            Why::Model(reason) | Why::Refused(reason) => Some(reason.clone()),
            Why::Rule(rule) => Some(rule.to_string()),
            _ => None,
        };
        if !cancelled.load(Ordering::Relaxed)
            && let Some(reason) = reason
        {
            self.feedback = RetryFeedback::new(session, version, folder, &reason).map(Box::new);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Why {
    /// Another writer changed the session first; nothing more was written.
    Stale,
    /// No drafting responses are left.
    Budget(usize),
    /// The model call failed or its reply couldn't be used.
    Model(String),
    /// Tinkery refused; the reason says whether anything saved.
    Refused(String),
    /// The model's reply broke a ledger rule; nothing from it was saved.
    Rule(Rule),
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
    /// Start `work` on `session` off the calling thread, through [`Steps`] attributed to
    /// `confirmation`.
    pub(super) fn start(
        work: Work,
        session: PathBuf,
        agent: Option<Arc<Agent>>,
        confirmation: GoalConfirmation,
    ) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let saved = Arc::new(AtomicBool::new(false));
        let (flag, answered) = (cancelled.clone(), saved.clone());
        let (sender, result) = mpsc::channel();
        std::thread::spawn(move || {
            let steps = Steps::new(agent.as_deref(), &confirmation);
            let _ = sender.send(steps.run(&work, &session, &flag, &answered));
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
            Ok(mut result) => {
                if self.cancelled.load(Ordering::Relaxed)
                    && let Err(failure) = &mut result
                {
                    failure.feedback = None;
                }
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Failure {
                answer_saved: false,
                why: Why::Model("The step stopped unexpectedly.".into()),
                feedback: None,
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
        feedback: None,
        why: match refusal {
            Refusal::Stale => Why::Stale,
            Refusal::Refused(reason) | Refusal::Rejected(reason) => Why::Refused(reason),
            Refusal::Rule(rule) => Why::Rule(rule),
        },
    }
}

/// The refusal when a model step has no drafting agent.
pub(super) const NO_AGENT: &str = "This step needs drafting: put --shape-pi, --model and --shape-budget before seed. Nothing was changed.";

/// One Shape session's controller, shared by the screen and the command: who every write is
/// attributed to, the folder a step reads, the one budget check and the one runner for each
/// [`Work`]. It holds no screen state and decides no wording.
#[derive(Clone, Copy)]
pub(super) struct Steps<'a> {
    agent: Option<&'a Agent>,
    confirmation: &'a GoalConfirmation,
}

impl<'a> Steps<'a> {
    /// Steps through `agent`, with every write attributed to the launch's declared
    /// `confirmation`, never to the drafting agent alone. An undeclared launch is the operator.
    pub(super) fn new(agent: Option<&'a Agent>, confirmation: &'a GoalConfirmation) -> Self {
        Self {
            agent,
            confirmation,
        }
    }

    /// What a step for `session` reads: the session's base checkout, else where its `goal` was
    /// made, with Think's answers (see [`Agent::reads`]). Without an agent or goal, only the
    /// base. Where the goal was made is always checked with an agent, so a broken origin
    /// refuses. Blocks.
    pub(super) fn reads(
        self,
        session: &Path,
        goal: Option<&str>,
        base: Option<PathBuf>,
    ) -> Result<Option<Reads>, String> {
        let made = self
            .agent
            .zip(goal)
            .map(|(agent, goal)| agent.reads(session, goal))
            .transpose()?;
        Ok(match (base, made) {
            (Some(base), Some(made)) => Some(made.reading(base)),
            (base, made) => made.or(base.map(Reads::from)),
        })
    }

    /// Whether `work` may start: a model step needs an agent with a response left. Checked
    /// before anything is written; other work always may.
    pub(super) fn admit(self, work: &Work) -> Result<(), Failure> {
        if !work.calls_model() {
            return Ok(());
        }
        match self.agent {
            Some(agent) => left(agent, false),
            None => Err(Failure {
                answer_saved: false,
                why: Why::Refused(NO_AGENT.into()),
                feedback: None,
            }),
        }
    }

    /// Run `work` to its end on the calling thread. It makes at most the calls the agent's
    /// budget allows and writes nothing after a refusal.
    pub(super) fn run(
        self,
        work: &Work,
        session: &Path,
        cancelled: &AtomicBool,
        saved: &AtomicBool,
    ) -> Result<Done, Failure> {
        let actor = self.confirmation.actor().map_err(|reason| Failure {
            answer_saved: false,
            why: Why::Refused(reason),
            feedback: None,
        })?;
        // A look checks the budget only after the ledger's own questions, which need no model.
        if !matches!(work, Work::Look { .. }) {
            self.admit(work)?;
        }
        let agent = || {
            self.agent.ok_or_else(|| Failure {
                answer_saved: false,
                why: Why::Refused(NO_AGENT.into()),
                feedback: None,
            })
        };
        let wrote =
            |result: Result<(), Refusal>| result.map(|()| Done::Wrote).map_err(refused(false));
        match work {
            Work::Answer {
                id,
                words,
                relay,
                expected,
                reads,
            } => {
                let version = match relay {
                    Some(relay) => answering::answer_relayed(session, *expected, id, relay, actor),
                    None => answering::answer(session, *expected, id, words, actor),
                }
                .map_err(refused(false))?;
                saved.store(true, Ordering::Relaxed);
                look(
                    session,
                    version,
                    reads,
                    (agent()?, actor),
                    cancelled,
                    true,
                    None,
                )
            }
            Work::Respond {
                id,
                action,
                expected,
                reads,
            } => {
                let version = answering::respond(session, *expected, id, action, actor)
                    .map_err(refused(false))?;
                saved.store(true, Ordering::Relaxed);
                look(
                    session,
                    version,
                    reads,
                    (agent()?, actor),
                    cancelled,
                    true,
                    None,
                )
            }
            Work::Look {
                expected,
                reads,
                feedback,
            } => look(
                session,
                *expected,
                reads,
                (agent()?, actor),
                cancelled,
                false,
                feedback.as_ref(),
            ),
            Work::TeachBack {
                revision,
                words,
                expected,
                seed,
            } => teach_back::run(
                (revision, words, *expected, seed),
                session,
                agent()?,
                actor,
                cancelled,
                saved,
            ),
            Work::Change {
                revision,
                words,
                expected,
                reads,
            } => {
                let version = answering::change_draft(session, *expected, revision, words, actor)
                    .map_err(refused(false))?;
                saved.store(true, Ordering::Relaxed);
                look(
                    session,
                    version,
                    reads,
                    (agent()?, actor),
                    cancelled,
                    true,
                    None,
                )
            }
            Work::Confirm {
                revision,
                words,
                expected,
                seed,
            } => {
                answering::confirm_seed(session, *expected, revision, seed, words, actor)
                    .map_err(refused(false))?;
                Ok(Done::Confirmed)
            }
            Work::RepairCheck { edit, words } => wrote(answering::repair_check(
                session,
                edit.expected,
                &edit.seed,
                (&edit.target, edit.method, words),
                actor,
            )),
            Work::Revisit {
                id,
                expected,
                words,
            } => wrote(
                answering::respond(
                    session,
                    *expected,
                    id,
                    &ledger::AnswerAction::Revisit(words.clone()),
                    actor,
                )
                .map(|_| ()),
            ),
            Work::Keep {
                id,
                expected,
                reason,
            } => wrote(answering::keep(session, *expected, id, reason, actor)),
            Work::End(ending) => wrote(answering::end(session, *ending, actor)),
            Work::Object { expected, text } => {
                wrote(answering::object(session, *expected, text, actor).map(|_| ()))
            }
            Work::Recommend { id, expected, text } => {
                wrote(answering::recommend(session, *expected, id, text, actor).map(|_| ()))
            }
        }
    }
}

/// The one budget check: refuses when `agent` has no response left.
fn left(agent: &Agent, answer_saved: bool) -> Result<(), Failure> {
    if agent.remaining() == 0 {
        return Err(Failure {
            answer_saved,
            why: Why::Budget(agent.limit()),
            feedback: None,
        });
    }
    Ok(())
}

fn look(
    session: &Path,
    expected: u64,
    reads: &Reads,
    (agent, actor): (&Agent, Actor<'_>),
    cancelled: &AtomicBool,
    answer_saved: bool,
    feedback: Option<&RetryFeedback>,
) -> Result<Done, Failure> {
    let folder = reads.folder.as_path();
    let fail = |why| Failure {
        answer_saved,
        why,
        feedback: None,
    };
    let rejected = |failure: Failure| failure.rejected(session, expected, folder, cancelled);
    // A ledger rule the reply broke is retry feedback too; storage refusals never are (#432).
    let reply_refused = |refusal: Refusal| match refusal {
        Refusal::Rejected(_) | Refusal::Rule(_) => rejected(refused(answer_saved)(refusal)),
        refusal => refused(answer_saved)(refusal),
    };
    let ledger = answering::read(session).map_err(refused(answer_saved))?;
    if ledger.version() != expected {
        return Err(fail(Why::Stale));
    }
    if ledger.has_current_question() {
        return Ok(Done::Asked(Asked::default()));
    }
    // A decision already recorded, such as an objection, is asked before the model looks for one,
    // and the pre-mortem and real-sample probes are put to the operator before any seed, also
    // without a model.
    if answering::promote(session, expected)
        .map_err(refused(answer_saved))?
        .is_some()
        || answering::probe(session, expected)
            .map_err(refused(answer_saved))?
            .is_some()
    {
        return Ok(Done::Asked(Asked::default()));
    }
    left(agent, answer_saved)?;
    let goal = ledger.goal().unwrap_or("").to_owned();
    let mut input = input(&ledger, reads.think.clone());
    if let Some(feedback) = feedback.filter(|f| f.applies(session, expected, folder)) {
        input["previous_refusal"] = feedback.input();
    }
    let call = Call {
        folder: Some(folder.to_path_buf()),
        ..agent.drafting.call.clone()
    };
    let contract = answering::contract().map_err(refused(answer_saved))?;
    let instructions = format!("{}\n\nSeed reply guidance for Shape:\n{contract}", prompt());
    let reply = call
        .complete(
            &agent.drafting.budget,
            "seed step",
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
    match decode(&reply, &goal).map_err(|e| rejected(fail(Why::Model(e))))? {
        Next::Question(ask) => {
            answering::ask(session, expected, &ask, folder).map_err(reply_refused)?;
            Ok(Done::Asked(Asked {
                withheld: ask.recommendation().filter(|_| ask.hard()),
                assumed: ask.assumed,
            }))
        }
        Next::Seed {
            confirming,
            checks,
            review,
            found,
            assumed,
            resolves,
        } => {
            let records = crate::seed_me::ledger::Value::from(checks.clone());
            // #432: a reply breaking a check rule and its review hears both, so a retry can't
            // trade one fault for the other. Only pure reply checks become retry feedback.
            let review = review.and_then(|r| r.validate(&records).map(|()| r));
            let review = match (ledger::validate_new_draft_checks(&goal, &records), review) {
                (Ok(()), review) => review.map_err(|why| rejected(fail(Why::Model(why))))?,
                (Err(rule), review) => {
                    let also = review
                        .err()
                        .map(|e| format!(" Also: {e}"))
                        .unwrap_or_default();
                    return Err(rejected(fail(Why::Rule(Rule {
                        node: None,
                        rule: format!("{rule}{also}"),
                    }))));
                }
            };
            let revision = answering::draft_seed(
                session,
                expected,
                &confirming,
                &checks,
                answering::Reported {
                    found: &found,
                    assumed: &assumed,
                    resolves: resolves.as_ref(),
                },
                folder,
            )
            .map_err(reply_refused)?;
            let seed = crate::regular_file::read_text(
                &session.join(crate::seed_me::files::SEED),
                "not a regular file",
            )
            .map_err(|_| {
                fail(Why::Model(
                    "The draft could not be read. Reopen Shape before trying again.".into(),
                ))
            })?;
            answering::teach_back(
                session,
                answering::read(session)
                    .map_err(refused(answer_saved))?
                    .version(),
                &revision,
                &seed,
                &crate::seed_me::ledger::TeachBackRecord::Review(&review),
                actor,
            )
            .map_err(refused(answer_saved))?;
            Ok(Done::Drafted)
        }
    }
}

fn input(ledger: &answering::Ledger, settled: Vec<Value>) -> Value {
    json!({
        "goal": ledger.goal().unwrap_or(""),
        "answered_in_think": settled,
        "found_so_far": ledger.findings(),
        "ledger": ledger.context_without_findings(),
    })
}

/// Restore the operator's exact Done when line where one check kept it whole but changed only its
/// letter case (#432). Other words, and checks that split or reword a line, are left to refuse.
fn restore_case(goal: &str, checks: &mut [Value]) {
    for criterion in ledger::goal_criteria(goal) {
        let keeps = |check: &Value, ignore_case| {
            check["statement"]
                .as_str()
                .is_some_and(|statement| ledger::keeps_criterion(statement, criterion, ignore_case))
        };
        if !checks.iter().any(|check| keeps(check, false))
            && let Some(check) = checks.iter_mut().find(|check| keeps(check, true))
        {
            check["statement"] = criterion.into();
        }
    }
}

/// At most five complete defaults, trimmed; incomplete entries are dropped, never repaired.
fn assumptions(value: &Value) -> Vec<Assumption> {
    value
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
        .collect()
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
                undo: Undo::of(&field("undo")?.to_lowercase())?,
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

/// What the model proposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Next {
    /// The next question.
    Question(Box<Ask>),
    /// The seed's summary and canonical prose sections, and what the step found.
    Seed {
        confirming: Value,
        checks: Value,
        /// Unusable reviews wait for the check rules, so one refusal names every broken rule.
        review: Result<Box<crate::seed_me::ledger::CompletionReview>, String>,
        found: Vec<Found>,
        /// Defaults the draft chose instead of asking; the operator sees them, never as answers.
        assumed: Vec<Assumption>,
        resolves: Option<Value>,
    },
}

/// The model's reply as one JSON object (surrounding prose or a code fence is tolerated).
/// Tolerates redundant closing braces after one complete object, never missing content or a
/// second object. Refuses a reply that asks to confirm: only the operator confirms. A seed's
/// checks must keep each of `goal`'s Done when lines whole; every broken rule is named at once.
pub(super) fn decode(reply: &str, goal: &str) -> Result<Next, String> {
    let start = reply.find('{').ok_or_else(|| {
        "The reply contains no JSON object. Return one question or Tink draft.".to_owned()
    })?;
    let end = reply[start..]
        .rfind('}')
        .map_or(reply.len(), |i| start + i + 1);
    let object = &reply[start..end];
    let mut stream = serde_json::Deserializer::from_str(object).into_iter::<Value>();
    let value = stream.next().expect("Object starts with an opening brace").map_err(|error| {
        if error.is_eof() {
            "The reply ended before its JSON object was complete. No question or Tink draft was applied.".to_owned()
        } else {
            format!("The reply has invalid JSON at line {}, column {}. Return one question or Tink draft.", error.line(), error.column())
        }
    })?;
    if !object[stream.byte_offset()..]
        .chars()
        .all(|c| c == '}' || c.is_whitespace())
        || reply[end..].contains('{')
    {
        return Err("The reply contains extra content after its JSON object. Return only one question or Tink draft.".into());
    }
    let text = |key: &str| {
        value[key]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    if !matches!(value["next"].as_str(), Some("question" | "seed")) {
        return Err(
            "Reply field next must be question or seed; a model reply cannot confirm a Tink."
                .into(),
        );
    }
    let found: Vec<Found> = serde_json::from_value(value["found"].clone()).map_err(|_| {
        "The model's findings are incomplete or use an unsupported format. Each finding needs an id, a typed claim, checked and observed. Retry the step.".to_owned()
    })?;
    match value["next"].as_str() {
        Some("question") => {
            let question = text("question").ok_or_else(|| {
                "Question reply field question must contain non-empty words.".to_owned()
            })?;
            Ok(Next::Question(Box::new(Ask {
                id: text("id").unwrap_or_else(|| "question".into()),
                label: text("label").unwrap_or_else(|| question.clone()),
                gate: text("gate").unwrap_or_else(|| question.clone()),
                question,
                found,
                resolves: value.get("resolves").cloned(),
                supported_by: value.get("supported_by").cloned(),
                prerequisites: value.get("prerequisites").cloned(),
                suggestion: text("suggestion"),
                assumed: assumptions(&value["assumed"]),
                options: alternatives(&value["options"]),
                against: text("against"),
            })))
        }
        Some("seed") => {
            // #942: in 2 of 2 real drafts the model picked one of two readings that build different
            // things without asking, so a draft must name each such reading, and one becomes the question.
            let readings = value["readings"].as_array().filter(|r| {
                r.iter().all(|r| {
                    ["chosen", "other", "why"]
                        .iter()
                        .all(|k| r[*k].as_str().is_some_and(|s| !s.trim().is_empty()))
                })
            });
            let Some(readings) = readings else {
                return Err(concat!(
                    "Tink draft reply field readings must list each place the goal reads two ways that build ",
                    "different things, each with chosen, other and why; use [] only when the goal settles them all."
                )
                .to_owned());
            };
            if let Some(reading) = readings.first() {
                let part = |k: &str| {
                    let words = reading[k].as_str().unwrap_or_default().trim();
                    words.trim_end_matches(['.', '?', '!', ';', ',']).to_owned()
                };
                let (chosen, other) = (part("chosen"), part("other"));
                return Ok(Next::Question(Box::new(Ask {
                    id: "reading".into(),
                    label: format!("{chosen}, or {other}"),
                    question: format!(
                        "The goal reads two ways that build different things: {chosen}, or {other}. Which one?"
                    ),
                    gate: part("why"),
                    found,
                    resolves: value.get("resolves").cloned(),
                    supported_by: None,
                    prerequisites: None,
                    suggestion: None,
                    assumed: assumptions(&value["assumed"]),
                    options: Vec::new(),
                    against: None,
                })));
            }
            let mut checks = value["checks"]
                .as_array()
                .filter(|checks| !checks.is_empty())
                .cloned()
                .ok_or_else(|| {
                    "Tink draft reply field checks must be a non-empty list of check records."
                        .to_owned()
                })?;
            // #931: an optional list was left out in 2 of 2 real drafts, so a draft must say "none".
            let listed = value["assumed"].as_array().map_or(0, Vec::len);
            if !value["assumed"].is_array()
                || listed > 5
                || assumptions(&value["assumed"]).len() != listed
            {
                return Err(concat!(
                    "Tink draft reply field assumed must be a list of at most five defaults the draft ",
                    "chose instead of asking, each with text and why; use [] only when it chose none."
                )
                .to_owned());
            }
            restore_case(goal, &mut checks);
            let review = review_reply::decode(value["review"].clone(), &mut checks);
            let checks = Value::Array(checks);
            Ok(Next::Seed {
                review,
                confirming: value.get("confirming").cloned().ok_or_else(||
                    "Tink draft reply field confirming is missing; supply the named summary and prose fields.".to_owned())?,
                checks,
                found,
                assumed: assumptions(&value["assumed"]),
                resolves: value.get("resolves").cloned(),
            })
        }
        _ => unreachable!("Reply kind checked above"),
    }
}
