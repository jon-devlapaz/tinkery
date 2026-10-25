//! The one place Tinkery starts Pi: a bounded, cancellable process per request, and the call
//! budget every request is counted against.
//!
//! A request runs Pi with no session, extensions, skills or context files, so the model sees only
//! the prompt, the input and, when a caller asks for them, Pi's read-only tools (`read`, `grep`,
//! `find`, `ls`): it can read files inside the folder Pi starts in, never outside it, and runs no
//! commands. Input and output are capped at 32 KiB. Every completed or cancelled request stops its
//! process group and reaps the direct child; descendants that leave that group are outside this
//! ownership. Nothing retries on its own.
//! Callers get text back, or one line saying why not. Which phase asks is the caller's business; this module names none.

use std::{
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

const FOLDER_GUARD: &str = include_str!("../pi/folder.mjs");

/// Maximum encoded request, instructions or reply bytes accepted by the Pi boundary.
pub const LIMIT: usize = 32 * 1024;

/// Which kind of request a call is, for a [`CallBudget`]'s record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Request {
    /// The shaping call that reads the person's words.
    Shaping,
    /// The review that names decisions the reading settled silently.
    DecisionReview,
    /// One step of the seed interview: investigate, then draft the next question or the seed.
    SeedStep,
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

/// A hard limit on Pi requests, shared by every host that holds it. A request is counted
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
    /// Requests still allowed.
    pub fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.used())
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

/// How to start Pi: the executable, an explicit model, a thinking level (empty leaves Pi's
/// default), whether the model may read files, where it starts, and how long it may take.
#[derive(Clone, Debug)]
pub struct Call {
    /// The Pi executable.
    pub program: PathBuf,
    /// `provider/model-id`; never defaulted.
    pub model: String,
    /// Pi's thinking level, or empty for Pi's own default.
    pub thinking: String,
    /// `true` gives the model Pi's read-only tools; `false` gives it none.
    pub read_only_tools: bool,
    /// The folder Pi starts in, the only one its tools can read; `None` keeps Tinkery's.
    pub folder: Option<PathBuf>,
    /// The request stops after this long.
    pub timeout: Duration,
}

/// A prompt file as the model reads it. Notes (lines starting with `#`) are dropped, lines in a
/// paragraph are joined with one space, and each blank line becomes one newline.
pub fn assemble(file: &str) -> String {
    let mut prompt = String::new();
    let mut in_paragraph = false;
    for line in file.lines().filter(|l| !l.starts_with('#')) {
        if line.is_empty() {
            if !prompt.is_empty() {
                prompt.push('\n');
            }
            in_paragraph = false;
        } else {
            if in_paragraph {
                prompt.push(' ');
            }
            prompt.push_str(line);
            in_paragraph = true;
        }
    }
    prompt
}

/// Whether `model` names a provider and a model id with no spaces, as Pi needs.
pub fn explicit_model(model: &str) -> bool {
    model.split_once('/').is_some_and(|(provider, id)| {
        !provider.is_empty() && !id.is_empty() && !model.chars().any(char::is_whitespace)
    })
}

impl Call {
    /// Run Pi once with `prompt` as its system prompt and `input` on stdin. Refuses oversized input
    /// before starting; counts against `budget` (when given) before anything starts. Blocks until
    /// Pi answers, fails, times out or `cancelled` is set; run it off the paint path.
    pub fn complete(
        &self,
        budget: Option<&CallBudget>,
        kind: Request,
        input: Vec<u8>,
        prompt: String,
        cancelled: &AtomicBool,
    ) -> Result<String, String> {
        if input.len() > LIMIT {
            return Err(
                "This is too much to send at once. Nothing was sent; your words are kept. Start a smaller goal, keeping this one as a reference.".into(),
            );
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err("Request cancelled.".into());
        }
        let Some(budget) = budget else {
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
        command.args(["--print", "--no-session"]);
        // Held until Pi exits: the extension that keeps every tool call inside the folder.
        let mut _folder_guard = None;
        if self.read_only_tools {
            let guard = tempfile::Builder::new()
                .prefix("tinkery-pi-folder-")
                .suffix(".mjs")
                .tempfile()
                .and_then(|mut file| file.write_all(FOLDER_GUARD.as_bytes()).map(|()| file))
                .map_err(|error| format!("Cannot prepare Pi's folder guard: {error}"))?;
            command.args(["--tools", "read,grep,find,ls", "--extension"]);
            command.arg(guard.path());
            _folder_guard = Some(guard);
        } else {
            command.arg("--no-tools");
        }
        command.args([
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
        if let Some(folder) = &self.folder {
            command.current_dir(folder);
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
            .process_group(0)
            .spawn()
            .map_err(|error| format!("Cannot start Pi: {error}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = thread::spawn(move || stdin.write_all(&input));
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout = read_pipe(child.stdout.take().unwrap(), overflow.clone());
        let stderr = read_pipe(child.stderr.take().unwrap(), overflow.clone());
        let started = Instant::now();
        let mut status = None;
        let result = loop {
            let failure = if cancelled.load(Ordering::Relaxed) {
                Some("Request cancelled.".to_owned())
            } else if overflow.load(Ordering::Relaxed) {
                Some("Pi output exceeded 32 KiB.".to_owned())
            } else if started.elapsed() >= self.timeout {
                Some(format!(
                    "Pi timed out after {} seconds.",
                    self.timeout.as_secs()
                ))
            } else {
                None
            };
            if let Some(error) = failure {
                break Err(error);
            }
            match child.try_wait() {
                Ok(Some(exited)) => status = Some(exited),
                Ok(None) => {}
                Err(error) => {
                    break Err(format!("Cannot wait for Pi: {error}"));
                }
            }
            if let Some(status) = status
                && writer.is_finished()
                && stdout.is_finished()
                && stderr.is_finished()
            {
                break Ok(status);
            }
            thread::sleep(Duration::from_millis(20));
        };
        stop(&mut child);
        // Unfinished pipe workers must not extend a failed call's deadline.
        let status = result?;
        let input_result = writer.join().map_err(|_| "Pi input writer failed.")?;
        let output = stdout.join().map_err(|_| "Pi output reader failed.")??;
        let diagnostics = stderr.join().map_err(|_| "Pi error reader failed.")??;
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
        input_result.map_err(|error| format!("Cannot send the request to Pi: {error}"))?;
        if cancelled.load(Ordering::Relaxed) {
            return Err("Request cancelled.".into());
        }
        let text = std::str::from_utf8(&output).map_err(|_| "Pi returned invalid UTF-8.")?;
        Ok(text.to_owned())
    }
}

// Stop descendants even if Pi already exited, then reap the immediate child.
fn stop(child: &mut Child) {
    // SAFETY: this group was created for the owned child with process_group(0).
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    mod descendant {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/command_descendant.rs"
        ));
    }

    #[test]
    fn successful_requests_stop_descendants_and_preserve_results_and_budget() {
        completed_request(0);
    }

    #[test]
    fn failed_requests_stop_descendants_and_preserve_results_and_budget() {
        completed_request(7);
    }

    fn completed_request(exit: u8) {
        let fixture = descendant::Fixture::new(exit);
        let call = Call {
            program: fixture.program(),
            model: "TEST/fake".into(),
            thinking: String::new(),
            read_only_tools: false,
            folder: Some(fixture.root().into()),
            timeout: Duration::from_secs(10),
        };
        let budget = CallBudget::new(1);
        let result = call.complete(
            Some(&budget),
            Request::Shaping,
            b" TEST exact input\n ".to_vec(),
            "TEST instructions".into(),
            &AtomicBool::new(false),
        );
        fixture.assert_stopped();
        assert_eq!(fixture.input(), b" TEST exact input\n ");
        assert_eq!(budget.used(), 1);
        assert_eq!(budget.remaining(), 0);
        if exit == 0 {
            assert_eq!(result.unwrap(), " TEST reply café  \n");
            assert_eq!(budget.attempts()[0].outcome, "reply");
        } else {
            let error = result.unwrap_err();
            assert!(
                error.starts_with("Pi exited with exit status: 7"),
                "{error}"
            );
            assert!(
                error.ends_with("check the configured model and authentication."),
                "{error}"
            );
            assert_eq!(budget.attempts()[0].outcome, "failure");
            assert_eq!(budget.attempts()[0].error.as_deref(), Some(error.as_str()));
        }
    }

    #[test]
    fn oversized_input_keeps_words_and_offers_a_next_step_before_spending_budget() {
        let call = Call {
            program: "/TEST-never-start-pi".into(),
            model: "TEST/fake".into(),
            thinking: String::new(),
            read_only_tools: true,
            folder: None,
            timeout: Duration::from_secs(1),
        };
        let budget = CallBudget::new(1);
        let words = vec![b'x'; LIMIT + 1];
        let error = call
            .complete(
                Some(&budget),
                Request::SeedStep,
                words.clone(),
                "TEST instructions".into(),
                &AtomicBool::new(false),
            )
            .unwrap_err();
        assert_eq!(
            error,
            "This is too much to send at once. Nothing was sent; your words are kept. Start a smaller goal, keeping this one as a reference."
        );
        assert_eq!(words.len(), LIMIT + 1);
        assert!(words.iter().all(|b| *b == b'x'));
        assert!(budget.attempts().is_empty());
    }

    #[test]
    fn stopped_requests_kill_descendants_without_extending_the_deadline() {
        let mut failures = Vec::new();
        for mode in ["timeout", "exited", "cancel", "overflow"] {
            let directory = tempfile::Builder::new()
                .prefix("tinkery-TEST-pi-")
                .tempdir()
                .unwrap();
            let program = directory.path().join("pi");
            std::fs::write(
                &program,
                format!(
                    r#"#!/usr/bin/env python3
import pathlib, subprocess, sys, time
child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(10)'])
pathlib.Path('TEST-child-pid.tmp').write_text(str(child.pid))
pathlib.Path('TEST-child-pid.tmp').rename('TEST-child-pid')
if {mode:?} == 'exited':
    sys.exit(0)
if {mode:?} == 'overflow':
    print('x' * 40000, flush=True)
time.sleep(10)
"#
                ),
            )
            .unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            let call = Call {
                program,
                model: "TEST/fake".into(),
                thinking: String::new(),
                read_only_tools: false,
                folder: Some(directory.path().into()),
                timeout: if matches!(mode, "cancel" | "overflow") {
                    Duration::from_secs(3)
                } else {
                    Duration::from_millis(250)
                },
            };
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancel = if mode == "cancel" {
                let cancelled = cancelled.clone();
                let ready = directory.path().join("TEST-child-pid");
                Some(thread::spawn(move || {
                    let until = Instant::now() + Duration::from_secs(2);
                    while !ready.exists() && Instant::now() < until {
                        thread::sleep(Duration::from_millis(10));
                    }
                    cancelled.store(true, Ordering::Relaxed);
                }))
            } else {
                None
            };
            let budget = CallBudget::new(1);
            let started = Instant::now();
            let result = call.complete(
                Some(&budget),
                Request::Shaping,
                vec![],
                "TEST".into(),
                &cancelled,
            );
            let elapsed = started.elapsed();
            if let Some(cancel) = cancel {
                cancel.join().unwrap();
            }
            let pid: i32 = std::fs::read_to_string(directory.path().join("TEST-child-pid"))
                .expect("fake Pi started its descendant")
                .parse()
                .unwrap();
            // An orphan may briefly remain a zombie until init reaps it. It cannot run.
            let running = || {
                let output = Command::new("ps")
                    .args(["-o", "stat=", "-p", &pid.to_string()])
                    .output()
                    .unwrap();
                let state = String::from_utf8_lossy(&output.stdout);
                !state.trim().is_empty() && !state.trim().starts_with('Z')
            };
            let until = Instant::now() + Duration::from_secs(1);
            while running() && Instant::now() < until {
                thread::sleep(Duration::from_millis(10));
            }
            let survived = running();
            if survived {
                // SAFETY: this PID belongs to the fake descendant, still alive above.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
                failures.push(format!("{mode}: descendant survived"));
            }
            assert!(
                elapsed < Duration::from_millis(650),
                "{mode}: waited {elapsed:?}"
            );
            let error = result.unwrap_err();
            let expected = match mode {
                "cancel" => "Request cancelled.",
                "overflow" => "Pi output exceeded 32 KiB.",
                _ => "Pi timed out",
            };
            assert!(error.starts_with(expected), "{mode}: {error}");
            assert_eq!(budget.used(), 1);
            assert_eq!(budget.attempts()[0].outcome, "failure");
        }
        assert!(failures.is_empty(), "{}", failures.join("; "));
    }
}
