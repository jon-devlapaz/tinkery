//! `seed wait`: block until the agent driving one Tink is needed, then name what happened and
//! the next allowed action as one JSON object. Reads only: it starts, resumes, verifies,
//! finishes and stops nothing, writes no file, and calls no model or GitHub.
//!
//! The agent is needed at once when its Tink is confirmed and not started, or when a started
//! worker is neither running nor being verified. While a worker runs or verification runs, it
//! waits for that to end. Before a run exists, it waits for the session to change, which is
//! how a Shape step taken elsewhere finishes. It never claims an outcome the records do not
//! show: a timeout says so and exits non-zero.

use crate::{
    sdlc::Verification,
    seed_me::{files, ledger},
    worker::{self, Observed, State, Turn},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// How long a wait blocks when `--wait-seconds` is not given.
pub const DEFAULT_SECONDS: usize = 1800;
// Each look runs SDLC's status reader twice, so a look costs about a second at most.
const TICK: Duration = Duration::from_secs(2);

/// What the wait ended on. `timeout` is the only result that is not an event.
pub struct Outcome {
    /// The JSON result: `event`, `next`, and what was read last.
    pub result: Value,
    /// False only for a timeout.
    pub happened: bool,
}

/// Wait on the session in `directory` and, with `root`, on the run that records its Tink.
/// Refuses a folder that is not a readable session and a run that cannot be read.
pub fn session(directory: &Path, root: Option<PathBuf>, seconds: usize) -> Result<Outcome, String> {
    let directory = directory
        .canonicalize()
        .map_err(|e| format!("{}: {e}", directory.display()))?;
    let look = || -> Result<(Value, Option<Observed>), String> {
        let status = ledger::status(&directory)?.to_json()?;
        let run = match &root {
            Some(root) => worker::observe(root, &directory.join(files::SEED))?,
            None => None,
        };
        Ok((status, run))
    };
    let started = Instant::now();
    let deadline = started + Duration::from_secs(seconds as u64);
    let (first, _) = look()?;
    loop {
        let (status, run) = look()?;
        let event = match &run {
            Some(run) => worker_event(run),
            None if status["handoff"]["verified"] == true => Some((
                "confirmed",
                "The Tink is confirmed and no run records it. Run seed start DIR.".to_owned(),
            )),
            None if status != first => Some(if status["current_question"].is_null() {
                (
                    "shape",
                    "The session changed. Read seed show DIR.".to_owned(),
                )
            } else {
                (
                    "decision",
                    "A Shape question is due. Read seed show DIR, then answer it.".to_owned(),
                )
            }),
            None => None,
        };
        let now = Instant::now();
        if event.is_some() || now >= deadline {
            let happened = event.is_some();
            let (event, next) = event.unwrap_or((
                "timeout",
                "Nothing changed before the time limit. Run seed wait again, or read seed show DIR."
                    .to_owned(),
            ));
            return Ok(Outcome {
                result: json!({
                    "event": event,
                    "next": next,
                    "session": directory,
                    "status": status["status"],
                    "current_question": status["current_question"],
                    "run": run.as_ref().map(|r| &r.run),
                    "worker": run.as_ref().map(|r| r.state.words()),
                    "verification": run.as_ref().and_then(|r| r.verification.as_str()),
                    "waited_seconds": now.duration_since(started).as_secs(),
                }),
                happened,
            });
        }
        std::thread::sleep(TICK.min(deadline - now));
    }
}

/// `None` while the worker or verification still runs; otherwise the event and SDLC's next step.
fn worker_event(run: &Observed) -> Option<(&'static str, String)> {
    let event = match (run.state, run.verification.known()) {
        (_, Some(Verification::Running))
        | (
            State::Active {
                turn: Turn::Running,
            },
            _,
        ) => return None,
        (State::Active { turn: Turn::AtCap } | State::Stopped { cap: true }, _) => "cap",
        (State::Active { turn: Turn::Ended }, _) => "worker-finished",
        (_, Some(Verification::Failed | Verification::Interrupted)) => "check-failed",
        (_, Some(Verification::AwaitingAttestation)) => "decision",
        (State::Completed, Some(Verification::Current)) => "verified",
        (State::Completed, _) => "worker-completed",
        _ => "inspect",
    };
    Some((
        event,
        format!("Worker: {}. SDLC: {}", run.state.words(), run.next),
    ))
}
