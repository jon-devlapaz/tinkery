//! Shape without a screen: one command saves a goal, and one command takes one explicit step.
//!
//! For an agent or a script, a key bar is friction, so the same ledger and step run from named
//! actions. Each call is one explicit action under the launch's declared authority and call
//! budget; nothing loops, retries or confirms on its own. Confirming names the SHA-256 of the
//! draft bytes the caller read, which is this interface's "read to the end". It adds no rule:
//! the ledger refuses what it refuses on screen.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use super::step::{self, Agent, Done, Why, Work};
use crate::seed_me::{
    self,
    ledger::AnswerAction,
    shaping::{self, Shaped},
};
use crate::think_record::GoalConfirmation;

/// The one thing a `seed step` call does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Look into the next question, or draft the seed, when nothing is being asked.
    Look,
    /// Save these words as the answer to the current question, then look.
    Answer(String),
    /// Accept the current question's recorded suggestion, then look.
    Delegate,
    /// Confirm the draft whose exact bytes have this SHA-256.
    Confirm(String),
}

/// Save `goal` as a new session's confirmed goal under `root`, attributed as `confirmation`
/// declares, and return the session. The words are kept exactly, as the goal and as its one
/// original; `origin` says where they came from. Calls no model and confirms no seed.
pub fn new_goal(
    root: Option<PathBuf>,
    goal: &str,
    origin: &str,
    confirmation: &GoalConfirmation,
) -> Result<PathBuf, String> {
    if goal.trim().is_empty() {
        return Err("The goal is empty; nothing was saved.".into());
    }
    // A goal that states its own check keeps it; Shape reads the same `Done when:` line.
    let outcome = goal.split_once("Done when:").map_or("", |(_, rest)| {
        rest.split("\nMust").next().unwrap_or(rest).trim()
    });
    let receipt = seed_me::Config::new(root).confirm_as(
        seed_me::Affirmed {
            goal: goal.to_owned(),
            outcome: outcome.to_owned(),
            options: Vec::new(),
            originals: vec![seed_me::Original {
                id: 1,
                in_reply_to: None,
                text: goal.to_owned(),
            }],
            context: serde_json::json!({"origin": origin, "entry": "tinkery seed new"}).to_string(),
            source: format!(
                "Goal saved by `tinkery seed new` from {origin}; caller-declared attribution: {}; \
                 goal only, not seed or implementation approval.",
                confirmation.details()
            ),
        },
        confirmation,
    )?;
    Ok(receipt.session)
}

/// What Shape would show for `session`, as JSON for a caller without a screen: the goal exactly as
/// recorded, the ledger version, and the one current thing (question, draft with the SHA-256
/// that `Action::Confirm` names, ready seed, or a plain other state). Read-only.
pub fn show(session: &Path) -> Result<serde_json::Value, String> {
    let shaping = shaping::read(session)?;
    let now = match shaping.stage {
        Shaped::Question {
            id,
            title,
            ask,
            found,
            suggestion,
            options,
            against,
            hard,
            ..
        } => serde_json::json!({
            "stage": "question", "id": id, "label": title, "question": ask, "found": found,
            "suggestion": suggestion, "options": options, "against": against, "hard_to_undo": hard,
        }),
        Shaped::Review {
            id,
            title,
            answer,
            found,
            ..
        } => serde_json::json!({
            "stage": "review", "id": id, "label": title, "answer": answer, "found": found,
            "next": "A flagged answer needs the operator's reason; open Shape's screen.",
        }),
        Shaped::Waiting => serde_json::json!({"stage": "waiting", "next": "seed step DIR look"}),
        Shaped::Drafted { revision, text } => serde_json::json!({
            "stage": "draft", "revision": revision,
            "sha256": text.as_ref().map(|text| format!("{:x}", Sha256::digest(text.as_bytes()))),
            "text": text, "needs_check": shaping.commandless_check, "signoff": shaping.signoff,
            "risks": shaping.confirmation,
        }),
        Shaped::Ready { revision, ended } => {
            serde_json::json!({"stage": "ready", "revision": revision, "ended": ended})
        }
        other => serde_json::json!({"stage": "other", "what": format!("{other:?}")}),
    };
    Ok(serde_json::json!({
        "goal": shaping.goal, "version": shaping.version, "open": shaping.open, "now": now,
    }))
}

/// Take `action` on `session` from a fresh read, through the same step the screen runs. Returns
/// what the step left unrecorded for the caller to read: assumed defaults and a withheld
/// suggestion. An action that does not fit where the seed stands changes nothing.
pub fn step(session: &Path, agent: &Agent, action: Action) -> Result<Vec<String>, String> {
    let (shaping, base) = shaping::read_with_base(session)?;
    let expected = shaping
        .version
        .ok_or("This session's ledger version is unknown; nothing was changed.")?;
    let goal = shaping.goal.as_deref().unwrap_or_default();
    let folder = || base.map_or_else(|| agent.folder(session, goal), |base| base.checkout);
    let work = match (action, shaping.stage) {
        (Action::Look, Shaped::Waiting) => Work::Look {
            expected,
            folder: folder(),
            feedback: None,
        },
        (Action::Answer(words), Shaped::Question { id, .. }) if !words.trim().is_empty() => {
            Work::Answer {
                id,
                words,
                expected,
                folder: folder(),
            }
        }
        (Action::Delegate, Shaped::Question { id, .. }) => Work::Respond {
            id,
            action: AnswerAction::Delegate,
            expected,
            folder: folder(),
        },
        (
            Action::Confirm(sha256),
            Shaped::Drafted {
                revision: Some(revision),
                text: Some(seed),
            },
        ) if format!("{:x}", Sha256::digest(seed.as_bytes())) == sha256 => Work::Confirm {
            revision,
            words: "confirm".into(),
            expected,
            seed,
        },
        (Action::Confirm(_), Shaped::Drafted { .. }) => {
            return Err(
                "That SHA-256 is not the current draft's; read the draft again. Nothing was confirmed."
                    .into(),
            );
        }
        (Action::Answer(_), Shaped::Question { .. }) => {
            return Err("The answer is empty; nothing was saved.".into());
        }
        _ => {
            return Err(
                "That action does not fit where this seed stands; read `tinkery seed show DIR`. Nothing was changed."
                    .into(),
            );
        }
    };
    let (stopped, saved) = (AtomicBool::new(false), AtomicBool::new(false));
    match step::run(&work, session, agent, &stopped, &saved) {
        Ok(Done::Asked(asked)) => Ok(asked
            .assumed
            .into_iter()
            .map(|a| format!("Assumed: {} ({})", a.text, a.why))
            .chain(
                asked
                    .withheld
                    .map(|s| format!("Suggestion, not recorded: {s}")),
            )
            .collect()),
        Ok(_) => Ok(Vec::new()),
        Err(failure) => Err(format!(
            "{}{}",
            if failure.answer_saved {
                "Your answer is saved. "
            } else {
                ""
            },
            match failure.why {
                Why::Stale =>
                    "Another writer changed this session first; nothing more was saved.".into(),
                Why::Budget(limit) => format!(
                    "No drafting runs are left: this launch's allowance is {limit}. Relaunch with --shape-budget."
                ),
                Why::Model(reason) => format!("The model step failed: {reason}"),
                Why::Refused(reason) => reason,
                Why::Stopped => "The step was stopped.".into(),
            }
        )),
    }
}
