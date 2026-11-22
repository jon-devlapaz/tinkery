//! Who answers a reading request: [`BoardHost`] is the boundary, met by [`ModelHost`] (Pi or
//! Claude Code: a shaping call, then the decision review) or by the practice simulation.
//!
//! A model request runs the host with no tools, session, extensions or context files, so the
//! model sees only the prompt and the words sent. The process boundary (caps, timeout,
//! cancellation) and the call budget are shared in [`crate::pi`]; this module adds Think's
//! instructions and settings. Construction requires a [`CallBudget`]; no request starts once
//! its limit is reached. Bundled guidance is verbatim from Seed Me
//! c9d766760dbeac0c05c92108165f9e968bec0a66, between its title and Size gate; no installed
//! skill is read.
use super::*;
use crate::pi::{CallBudget, Harness};
use crate::think_record as record;
use std::{path::PathBuf, time::Duration};

/// Answers a reading request. Runs off the paint thread; it must stop promptly once
/// `cancelled` is set, and it returns a decoded but unverified [`Guess`] or one line saying
/// why there isn't one.
pub trait BoardHost: Send + Sync {
    /// Reads `request` into a guess. A real host may make provider calls; the practice host
    /// makes none.
    fn reshape(&self, request: BoardRequest, cancelled: &AtomicBool) -> Result<Guess, String>;
    /// The current model-call phase, not a completed-call count. Stateless and simulated
    /// hosts report none; observing this never starts work.
    fn call_in_flight(&self) -> Option<Request> {
        None
    }
    /// A separate goal's host with the same settings and empty diagnostics. Stateful hosts
    /// return a fresh instance; the default `None` permits sharing a stateless host.
    fn new_session(&self) -> Option<Arc<dyn BoardHost>> {
        None
    }
    /// The launch allowance still left, in the words Shape uses; none for a host that makes no
    /// counted calls. Reading it never starts work.
    fn calls_left(&self) -> Option<String> {
        None
    }
    /// The host's own checks for "Why" in F1; none by default.
    fn diagnostics(&self) -> Vec<checks::Decision> {
        Vec::new()
    }
    /// Who this host is, for the think record. A host that names no model is `simulated`.
    fn origin(&self) -> record::Origin {
        record::Origin::simulated()
    }
}
/// Pi or Claude Code, configured for think's shaping calls: an explicit model, a thinking level,
/// and the bundled working-draft guidance as standing instructions.
pub struct ModelHost {
    harness: Harness,
    program: PathBuf,
    model: String,
    thinking: String,
    timeout: Duration,
    budget: Arc<CallBudget>,
    brain_history: std::sync::Mutex<Vec<checks::Decision>>,
    active_call: std::sync::Mutex<Option<Request>>,
}

impl ModelHost {
    /// `harness` at `program` with a model it accepts, bundled working-draft guidance and the
    /// authorized allowance. Starts nothing; zero refuses every request before the host starts.
    ///
    /// An unbudgeted host cannot be constructed:
    /// ```compile_fail,E0061
    /// use tinkery::{pi::Harness, think::brain_dump::ModelHost};
    /// let host = ModelHost::new(Harness::Pi, "TEST-pi".into(), "TEST/stub".into());
    /// ```
    pub fn new(
        harness: Harness,
        program: PathBuf,
        model: String,
        budget: Arc<CallBudget>,
    ) -> Result<Self, String> {
        harness.check_model(&model)?;
        Ok(Self {
            harness,
            program,
            model,
            thinking: "low".into(),
            timeout: Duration::from_secs(90),
            budget,
            brain_history: std::sync::Mutex::new(Vec::new()),
            active_call: std::sync::Mutex::new(None),
        })
    }

    /// Reuses the configuration for a separate goal, with an empty check history and the same
    /// budget. No process, provider request or file read occurs; an older goal's running job keeps
    /// its own history.
    pub fn new_session(&self) -> Self {
        Self {
            harness: self.harness,
            program: self.program.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            timeout: self.timeout,
            budget: self.budget.clone(),
            brain_history: std::sync::Mutex::new(Vec::new()),
            active_call: std::sync::Mutex::new(None),
        }
    }

    /// Sets the thinking level; refuses one the harness doesn't name. The default is `low`.
    pub fn with_thinking(mut self, level: &str) -> Result<Self, String> {
        self.harness.check_thinking(level)?;
        self.thinking = level.into();
        Ok(self)
    }

    fn working_instructions(&self) -> &str {
        include_str!("../working-draft.txt")
            .split_once("### Shape the working draft")
            .expect("Bundled working-draft boundary")
            .1
    }

    fn budget(&self) -> &CallBudget {
        &self.budget
    }

    /// Leaves the thinking level to the host's own default.
    pub fn with_default_thinking(mut self) -> Self {
        self.thinking.clear();
        self
    }

    fn complete(
        &self,
        kind: Request,
        input: Vec<u8>,
        prompt: String,
        cancelled: &AtomicBool,
    ) -> Result<String, String> {
        *self.active_call.lock().unwrap() = Some(kind);
        let result = crate::pi::Call {
            harness: self.harness,
            program: self.program.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            read_only_tools: false,
            folder: None,
            timeout: self.timeout,
        }
        .complete(&self.budget, kind.label(), input, prompt, cancelled);
        *self.active_call.lock().unwrap() = None;
        result
    }
}

impl BoardHost for ModelHost {
    fn call_in_flight(&self) -> Option<Request> {
        self.active_call.try_lock().ok().and_then(|call| *call)
    }
    fn new_session(&self) -> Option<Arc<dyn BoardHost>> {
        Some(Arc::new(ModelHost::new_session(self)))
    }
    fn calls_left(&self) -> Option<String> {
        Some(self.budget().left())
    }
    fn origin(&self) -> record::Origin {
        record::Origin {
            origin: "live".into(),
            harness: Some(self.harness),
            model: Some(self.model.clone()),
            thinking: (!self.thinking.is_empty()).then(|| self.thinking.clone()),
            prompt_hash: Some(record::hash(&format!(
                "{}\n{}",
                reading::prompt(self.working_instructions()),
                reading::assemble(review::PROMPT)
            ))),
        }
    }
    fn diagnostics(&self) -> Vec<checks::Decision> {
        self.brain_history
            .try_lock()
            .map(|h| h.clone())
            .unwrap_or_default()
    }
    fn reshape(&self, request: BoardRequest, cancelled: &AtomicBool) -> Result<Guess, String> {
        let started = std::time::Instant::now();
        let prompt = reading::prompt(self.working_instructions());
        let mut decision = checks::Decision::new(
            "structure-spans-history",
            checks::Verdict::Pass,
            "Readable board, exact claimed spans and application-owned history. Not a meaning judgment."
                .into(),
            started,
        );
        decision.request_key = checks::request_key(&request);
        let result: Result<Guess, String> = (|| {
            let input = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
            let text = self.complete(Request::Shaping, input, prompt, cancelled)?;
            // The decision review runs on the raw reply; when it finds nothing or cannot run,
            // the reading stays as it was.
            let review = review::input(&request, &text).and_then(|input| {
                match self.complete(
                    Request::DecisionReview,
                    input,
                    reading::assemble(review::PROMPT),
                    cancelled,
                ) {
                    Ok(review) => Review::decode(&review, &mut decision.normalizations),
                    Err(e) => {
                        decision
                            .normalizations
                            .push(format!("Decision review skipped: {e}"));
                        None
                    }
                }
            });
            decision.raw = Some(text.clone());
            let guess = Guess::decode_reviewed(
                &text,
                review.as_ref(),
                &request,
                &mut decision.normalizations,
            )?;
            decision.candidate = Some(guess.clone());
            decision.coverage = checks::coverage(&request, &guess);
            Ok(guess)
        })();
        if let Err(e) = &result {
            decision.decision = checks::Verdict::Reject;
            decision.reason = e.clone();
        }
        decision.elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        if let Ok(mut history) = self.brain_history.lock() {
            history.push(decision.clone());
        }
        result
    }
}
pub(super) struct Simulated;
impl BoardHost for Simulated {
    fn reshape(&self, request: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        Ok(Guess {
            parts: GoalParts {
                who: Some("I".into()),
                outcome: Some("try Simulated sensemaking (not inferred)".into()),
                when: Some("in this practice".into()),
                why: Some("I can try without saving".into()),
                done_when: Some("this is a placeholder, not an inferred criterion".into()),
                must: vec![],
                must_not: vec!["confirming anything".into()],
            },
            other_goals: vec![],
            deferred: vec![],
            open: vec![],
            unresolved_notes: vec![],
            decisions: vec![],
            uncertain: false,
            framings: vec![Framing {
                text: String::new(),
                supports: request
                    .sources
                    .first()
                    .map(|s| Anchor {
                        source: s.id,
                        quote: s.text.clone(),
                        occurrence: 0,
                    })
                    .into_iter()
                    .collect(),
            }],
            outcome: String::new(),
            misfits: vec![],
            questions: if request.answered.contains(&"priority".into())
                || request.skipped.iter().any(|q| q.id == "priority")
            {
                vec![]
            } else {
                vec![Question {
                    suggestion: None,
                    target: None,
                    id: "priority".into(),
                    text: "Simulated: which part matters most to you?".into(),
                }]
            },
            alternatives: [
                "Investigate the proposed approach",
                "Investigate a different route to the same experience",
            ]
            .into_iter()
            .map(|label| Approach {
                label: format!("Simulated: {label}"),
                benefit: "Placeholder, not an evaluated approach.".into(),
                cost: "Not evaluated.".into(),
                undo_cost: "Unknown.".into(),
            })
            .collect(),
        })
    }
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
