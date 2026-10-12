//! The only place think calls a model: one Pi process per request, bounded and cancellable.
//!
//! A request runs Pi with no tools, session, extensions or context files, so the model sees
//! only the prompt and the words sent. Input and output are capped at 32 KiB, each call times
//! out after 90 seconds, and nothing retries on its own. Callers never touch a process: they
//! get text back, or one line saying why not.

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

const LIMIT: usize = 32 * 1024;

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
/// working-draft section of the Seed Me skill as standing instructions.
pub struct PiHost {
    program: PathBuf,
    pub(super) model: String,
    pub(super) thinking: String,
    instructions: String,
    timeout: Duration,
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
            brain_history: std::sync::Mutex::new(Vec::new()),
        })
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

    /// Leaves the thinking level to Pi's own default.
    pub fn with_default_thinking(mut self) -> Self {
        self.thinking.clear();
        self
    }

    pub(super) fn complete(
        &self,
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
                    "Pi provider rate limit (429); retry later. Previous paper retained.".into(),
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
