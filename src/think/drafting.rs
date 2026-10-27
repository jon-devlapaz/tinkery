//! Think's model host: Pi configured for the reading and the decision review.
//!
//! A request runs Pi with no tools, session, extensions or context files, so the model sees
//! only the prompt and the words sent. The process boundary (caps, timeout, cancellation) and the
//! call budget are shared in [`crate::pi`]; this module adds think's instructions and settings.
//! The app and evaluations attach a [`CallBudget`]; no request starts once its limit is reached.
//! Bundled guidance is verbatim from Seed Me c9d766760dbeac0c05c92108165f9e968bec0a66,
//! between its title and Size gate; no installed skill is read.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::{Deserialize, Serialize};

pub use crate::pi::{Attempt, CallBudget, LIMIT, Request};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// One way to reach the goal, as the reading proposes it. A proposal only: saving records it as
/// an option, never as a choice.
pub struct Approach {
    /// The approach in a few words.
    pub label: String,
    /// What it gains.
    pub benefit: String,
    /// What it costs to do.
    pub cost: String,
    /// What it costs to back out of later.
    pub undo_cost: String,
}

/// Pi, configured for think's shaping calls: an explicit model, a thinking level, and the
/// bundled working-draft guidance as standing instructions.
pub struct PiHost {
    program: PathBuf,
    pub(super) model: String,
    pub(super) thinking: String,
    timeout: Duration,
    budget: Option<Arc<CallBudget>>,
    pub(super) brain_history: std::sync::Mutex<Vec<String>>,
    pub(super) active_call: std::sync::Mutex<Option<Request>>,
}

impl PiHost {
    /// Pi at `program` with an explicit model and bundled working-draft guidance. Starts nothing.
    pub fn new(program: PathBuf, model: String) -> Result<Self, String> {
        if !crate::pi::explicit_model(&model) {
            return Err("Use an explicit --model provider/model-id.".into());
        }
        Ok(Self {
            program,
            model,
            thinking: "low".into(),
            timeout: Duration::from_secs(90),
            budget: None,
            brain_history: std::sync::Mutex::new(Vec::new()),
            active_call: std::sync::Mutex::new(None),
        })
    }

    /// Reuses the configuration for a separate goal, with an empty check history and the same
    /// budget. No process, provider request or file read occurs; an older goal's running job keeps
    /// its own history.
    pub fn new_session(&self) -> Self {
        Self {
            program: self.program.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            timeout: self.timeout,
            budget: self.budget.clone(),
            brain_history: std::sync::Mutex::new(Vec::new()),
            active_call: std::sync::Mutex::new(None),
        }
    }

    /// Sets Pi's thinking level; refuses a level Pi doesn't name. The default is `low`.
    pub fn with_thinking(mut self, level: &str) -> Result<Self, String> {
        crate::model_setup::check_thinking(level)?;
        self.thinking = level.into();
        Ok(self)
    }

    pub(super) fn working_instructions(&self) -> &str {
        include_str!("working-draft.txt")
            .split_once("### Shape the working draft")
            .expect("Bundled working-draft boundary")
            .1
    }

    /// Counts every request against `budget`, refusing any past its limit before Pi starts.
    /// Callers must supply the authorized allowance; the app defaults it to zero.
    pub fn with_budget(mut self, budget: Arc<CallBudget>) -> Self {
        self.budget = Some(budget);
        self
    }

    pub(super) fn budget(&self) -> Option<&CallBudget> {
        self.budget.as_deref()
    }

    /// Leaves the thinking level to Pi's own default.
    pub fn with_default_thinking(mut self) -> Self {
        self.thinking.clear();
        self
    }

    pub(super) fn complete(
        &self,
        kind: Request,
        input: Vec<u8>,
        prompt: String,
        cancelled: &AtomicBool,
    ) -> Result<String, String> {
        *self.active_call.lock().unwrap() = Some(kind);
        let result = crate::pi::Call {
            program: self.program.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            read_only_tools: false,
            folder: None,
            timeout: self.timeout,
        }
        .complete(self.budget.as_deref(), kind, input, prompt, cancelled);
        *self.active_call.lock().unwrap() = None;
        result
    }
}

pub(super) struct Job<T> {
    cancelled: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<T, String>>,
    worker: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Job<T> {
    pub(super) fn launch(
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancelled.clone();
        let (sender, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = sender.send(work(&worker_cancel));
        });
        Self {
            cancelled,
            result,
            worker: Some(worker),
        }
    }

    pub(super) fn poll(&self) -> Option<Result<T, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Draft worker stopped unexpectedly.".into()))
            }
        }
    }
}

impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[path = "drafting_tests.rs"]
mod tests;
