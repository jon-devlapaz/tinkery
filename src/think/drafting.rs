//! The only place think calls a model: one Pi process per request, bounded and cancellable.
//!
//! A request runs Pi with no tools, session, extensions or context files, so the model sees
//! only the prompt and the words sent. Input and output are capped at 32 KiB, each call times
//! out after 90 seconds, and nothing retries on its own. Callers never touch a process: they
//! get text back, or one line saying why not. An evaluation can give Pi a [`CallBudget`]; then
//! no request starts once its limit is reached.

use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// Maximum encoded request, instructions or reply bytes accepted by the Pi boundary.
pub(super) const LIMIT: usize = 32 * 1024;

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

/// Which of think's two requests a call is: the reading, or the decision review after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Request {
    /// The shaping call that reads the person's words.
    Shaping,
    /// The review that names decisions the reading settled silently.
    DecisionReview,
}

/// One request a [`CallBudget`] saw: started (then `reply` or `failure`) or refused unstarted.
#[derive(Clone, Debug, Serialize)]
pub struct Attempt {
    /// Which request it was.
    pub kind: Request,
    /// `started`, `reply`, `failure` or `refused`.
    pub outcome: &'static str,
    /// Why it failed or was refused.
    pub error: Option<String>,
}

/// A hard limit on provider requests, shared by every host that holds it. A request is counted
/// before anything starts, whatever happens after; once the limit is reached the next request is
/// refused unstarted. The count never resets, so a new session on the same budget can't spend
/// it twice.
#[derive(Debug)]
pub struct CallBudget {
    limit: usize,
    attempts: std::sync::Mutex<Vec<Attempt>>,
}

impl CallBudget {
    /// A budget allowing at most `limit` requests. Zero refuses every request.
    pub fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            attempts: Default::default(),
        })
    }
    /// The configured limit.
    pub fn limit(&self) -> usize {
        self.limit
    }
    /// Every request seen so far, in order, refusals included.
    pub fn attempts(&self) -> Vec<Attempt> {
        self.attempts.lock().unwrap().clone()
    }
    /// Requests that counted against the limit.
    pub fn used(&self) -> usize {
        self.attempts()
            .iter()
            .filter(|a| a.outcome != "refused")
            .count()
    }
    /// Counts one `kind` request, or refuses it when the limit is reached. Returns the slot that
    /// [`CallBudget::finish`] fills in.
    pub fn reserve(&self, kind: Request) -> Result<usize, String> {
        let mut attempts = self.attempts.lock().unwrap();
        let used = attempts.iter().filter(|a| a.outcome != "refused").count();
        if used >= self.limit {
            let error = format!(
                "Call limit of {} reached; the next request was refused before it started.",
                self.limit
            );
            attempts.push(Attempt {
                kind,
                outcome: "refused",
                error: Some(error.clone()),
            });
            return Err(error);
        }
        attempts.push(Attempt {
            kind,
            outcome: "started",
            error: None,
        });
        Ok(attempts.len() - 1)
    }
    /// Records how a reserved request ended.
    pub fn finish<T>(&self, slot: usize, result: &Result<T, String>) {
        if let Some(attempt) = self.attempts.lock().unwrap().get_mut(slot) {
            attempt.outcome = if result.is_ok() { "reply" } else { "failure" };
            attempt.error = result.as_ref().err().cloned();
        }
    }
}

/// Pi, configured for think's shaping calls: an explicit model, a thinking level, and the
/// working-draft section of the Seed Me skill as standing instructions.
pub struct PiHost {
    program: PathBuf,
    pub(super) model: String,
    pub(super) thinking: String,
    instructions: String,
    timeout: Duration,
    budget: Option<Arc<CallBudget>>,
    pub(super) brain_history: std::sync::Mutex<Vec<String>>,
}

impl PiHost {
    /// Pi at `program` with `model` (`provider/model-id`, no default) and the Seed Me skill at
    /// `skill`. Refuses a vague model or a skill whose working-draft section it can't find,
    /// rather than shaping with the wrong instructions. Starts nothing.
    pub fn new(program: PathBuf, model: String, skill: PathBuf) -> Result<Self, String> {
        if !model.split_once('/').is_some_and(|(provider, id)| {
            !provider.is_empty() && !id.is_empty() && !model.chars().any(char::is_whitespace)
        }) {
            return Err("Use an explicit --model provider/model-id.".into());
        }
        let text = std::fs::read_to_string(&skill).map_err(|error| {
            format!(
                "Cannot read Seed Me instructions at {}: {error}",
                skill.display()
            )
        })?;
        let (_, instructions) = text
            .split_once("# Seed Me\n")
            .ok_or("Unsupported Seed Me instructions: missing title.")?;
        let (instructions, _) = instructions
            .split_once("### Size gate")
            .ok_or("Unsupported Seed Me instructions: missing working-draft boundary.")?;
        if !instructions.contains("### Shape the working draft") || instructions.len() > LIMIT {
            return Err("Unsupported Seed Me working-draft instructions.".into());
        }
        Ok(Self {
            program,
            model,
            thinking: "low".into(),
            instructions: instructions.into(),
            timeout: Duration::from_secs(90),
            budget: None,
            brain_history: std::sync::Mutex::new(Vec::new()),
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
            instructions: self.instructions.clone(),
            timeout: self.timeout,
            budget: self.budget.clone(),
            brain_history: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Sets Pi's thinking level; refuses a level Pi doesn't name. The default is `low`.
    pub fn with_thinking(mut self, level: &str) -> Result<Self, String> {
        if !matches!(
            level,
            "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
        ) {
            return Err("Use --thinking off|minimal|low|medium|high|xhigh|max.".into());
        }
        self.thinking = level.into();
        Ok(self)
    }

    pub(super) fn working_instructions(&self) -> &str {
        self.instructions
            .split_once("### Shape the working draft")
            .expect("Constructor validates the shaping boundary")
            .1
    }

    /// Counts every request against `budget`, refusing any past its limit before Pi starts.
    /// Without one, as in the app, requests are unlimited.
    pub fn with_budget(mut self, budget: Arc<CallBudget>) -> Self {
        self.budget = Some(budget);
        self
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
        if input.len() > LIMIT {
            return Err(
                "Encoded request exceeds 32 KiB; originals and local words retained. No provider request sent.".into(),
            );
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err("Request cancelled.".into());
        }
        let Some(budget) = &self.budget else {
            return self.run(input, prompt, cancelled);
        };
        let slot = budget.reserve(kind)?;
        let result = self.run(input, prompt, cancelled);
        budget.finish(slot, &result);
        result
    }

    fn run(
        &self,
        input: Vec<u8>,
        prompt: String,
        cancelled: &AtomicBool,
    ) -> Result<String, String> {
        let mut command = Command::new(&self.program);
        command.args([
            "--print",
            "--no-session",
            "--no-tools",
            "--no-extensions",
            "--no-mcp",
            "--no-skills",
            "--no-prompt-templates",
            "--no-themes",
            "--no-context-files",
            "--no-approve",
            "--offline",
        ]);
        if !self.thinking.is_empty() {
            command.arg("--thinking").arg(&self.thinking);
        }
        let mut child = command
            .arg("--model")
            .arg(&self.model)
            .arg("--system-prompt")
            .arg(prompt)
            .env_remove("PI_SESSION_ID")
            .env_remove("PI_SESSION_FILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Cannot start Pi: {error}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = thread::spawn(move || stdin.write_all(&input));
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout = read_pipe(child.stdout.take().unwrap(), overflow.clone());
        let stderr = read_pipe(child.stderr.take().unwrap(), overflow.clone());
        let started = Instant::now();
        let result = loop {
            let failure = if cancelled.load(Ordering::Relaxed) {
                Some("Request cancelled.")
            } else if overflow.load(Ordering::Relaxed) {
                Some("Pi output exceeded 32 KiB.")
            } else if started.elapsed() >= self.timeout {
                Some("Pi timed out after 90 seconds.")
            } else {
                None
            };
            if let Some(error) = failure {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error.to_owned());
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!("Cannot wait for Pi: {error}"));
                }
            }
        };
        let input_result = writer.join().map_err(|_| "Pi input writer failed.")?;
        let output = stdout.join().map_err(|_| "Pi output reader failed.")??;
        let diagnostics = stderr.join().map_err(|_| "Pi error reader failed.")??;
        let status = result?;
        if !status.success() {
            let diagnostics = String::from_utf8_lossy(&diagnostics);
            if diagnostics.contains("429") && diagnostics.contains("rate_limit") {
                return Err(
                    "Pi provider rate limit (429); retry later. No automatic retry.".into(),
                );
            }
            let lower = diagnostics.to_ascii_lowercase();
            if [
                "context_length_exceeded",
                "context window",
                "input too long",
                "context rejected",
                "context length",
                "input/context rejected",
            ]
            .iter()
            .any(|s| lower.contains(s))
            {
                return Err(
                    "Pi provider rejected the input/context size. No automatic retry.".into(),
                );
            }
            return Err(format!(
                "Pi exited with {status}; check the configured model and authentication."
            ));
        }
        input_result.map_err(|error| format!("Cannot send thoughts to Pi: {error}"))?;
        if cancelled.load(Ordering::Relaxed) {
            return Err("Request cancelled.".into());
        }
        let text = std::str::from_utf8(&output).map_err(|_| "Pi returned invalid UTF-8.")?;
        Ok(text.to_owned())
    }
}

fn read_pipe(
    reader: impl Read + Send + 'static,
    overflow: Arc<AtomicBool>,
) -> JoinHandle<Result<Vec<u8>, String>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("Cannot read Pi output: {error}"))?;
        if bytes.len() > LIMIT {
            overflow.store(true, Ordering::Relaxed);
            return Err("Pi output exceeded 32 KiB.".into());
        }
        Ok(bytes)
    })
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
