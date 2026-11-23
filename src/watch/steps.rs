//! Worker-step display and attention from typed status, in the worker's own state words, without
//! API-word recognition or execution.

use super::{Worker, sdlc};
use crate::worker::{State, Turn};

/// tink-sdlc's pinned build steps as detail lines, and what they say about a worker.
/// An older tink-sdlc omits them; anything present but unfamiliar is refused, not guessed.
pub(super) fn render(run: &sdlc::Run) -> Result<Option<(Vec<String>, Worker)>, String> {
    use sdlc::{StepState as S, StopReason as R, TurnStatus as T};
    if !run.steps_present {
        return Ok(None);
    }
    let refuse = || "A worker step Watch doesn't know; nothing is shown rather than a guess.";
    let mut lines = Vec::new();
    let mut worker = Worker::Idle;
    for step in run.steps.as_ref().ok_or_else(refuse)? {
        let name = step.data["name"].as_str().ok_or_else(refuse)?;
        let cap_reached = match step.data.get("response_cap_reached") {
            Some(value) => value.as_bool().ok_or_else(refuse)?,
            None => false,
        };
        if cap_reached {
            if !step.state.is(S::Active) || !step.turn.is(T::Running) || step.stopped_by.present() {
                return Err(refuse().into());
            }
            worker = Worker::AtCap;
        }
        if let Some(stop) = step.data.get("cap_stop") {
            match (step.state.known(), step.stopped_by.known()) {
                (Some(S::Active), None) if step.turn.is(T::Running) => worker = Worker::AtCap,
                (Some(S::Stopped), Some(R::Cap)) if !step.turn.present() => (),
                _ => return Err(refuse().into()),
            }
            let cap = stop["cap"]
                .as_u64()
                .filter(|cap| *cap > 0)
                .ok_or_else(refuse)?;
            lines.push(format!(
                "Worker step {name}: stopped at its response cap ({cap})."
            ));
            let files = match &stop["changed_files"] {
                serde_json::Value::Null => "unavailable".into(),
                serde_json::Value::Array(files) if files.is_empty() => "none".into(),
                serde_json::Value::Array(files) if files.len() <= 12 => files
                    .iter()
                    .map(|path| excerpt(path, 180).ok_or_else(refuse))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", "),
                _ => return Err(refuse().into()),
            };
            let omitted = stop["omitted_files"].as_u64().ok_or_else(refuse)?;
            let more = if omitted > 0 {
                format!(" (+{omitted} more)")
            } else {
                String::new()
            };
            lines.push(format!("Local changes at stop: {files}{more}"));
            if let Some(shell) = stop.get("shell").filter(|value| !value.is_null()) {
                let result = match shell["error"].as_bool().ok_or_else(refuse)? {
                    true => "error",
                    false => "no error reported",
                };
                let command = excerpt(&shell["command"], 240).ok_or_else(refuse)?;
                let output = excerpt(&shell["output"], 600).ok_or_else(refuse)?;
                lines.push(format!(
                    "Last shell result ({result}, not Verify): {command}"
                ));
                lines.push(output.into());
            }
            continue;
        }
        if cap_reached {
            lines.push(format!(
                "Worker step {name}: reached its response cap. Saved evidence is not available yet."
            ));
            continue;
        }
        let state = State::read(&step.state, &step.turn, &step.stopped_by).map_err(|_| refuse())?;
        let seen = match state {
            State::Active { turn } => {
                let words = step.data["observed_turn"].as_str().ok_or_else(refuse)?;
                if worker == Worker::Idle || turn == Turn::Ended {
                    worker = Worker::Step(state);
                }
                format!(" · {words}")
            }
            // A step that is not active reports no turn; one that does is not guessed at.
            _ if step.turn.present() => return Err(refuse().into()),
            // Only the newest uncertain step owes reconciliation; an older tink-sdlc
            // doesn't say which is newest, so every uncertain step is treated as owed.
            State::Uncertain => {
                let owed = match step.data.get("owed") {
                    None => true,
                    Some(owed) => owed.as_bool().ok_or_else(refuse)?,
                };
                if owed && !worker.ended() {
                    worker = Worker::Step(state);
                }
                String::new()
            }
            _ => String::new(),
        };
        lines.push(format!("Worker step {name}: {}{seen}", state.words()));
    }
    if lines.is_empty() {
        lines.push("No worker step has been started for this run.".into());
    }
    Ok(Some((lines, worker)))
}

fn excerpt(value: &serde_json::Value, limit: usize) -> Option<&str> {
    value
        .as_str()
        .filter(|text| text.chars().count() <= limit && !text.chars().any(char::is_control))
}
