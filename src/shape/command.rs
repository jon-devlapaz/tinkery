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
    answering::{self, Refusal},
    ledger::AnswerAction,
    shaping::{self, Shaped},
};
use crate::think_record::GoalConfirmation;

const STALE: &str = "Another writer changed this session first; nothing more was saved.";

/// The one thing a `seed step` call does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Look into the next question, or draft the seed, when nothing is being asked. On a draft
    /// it redrafts, which is the way on after a saved change whose redraft failed.
    Look,
    /// Save these words as the answer to the current question, then look.
    Answer(String),
    /// Accept the current question's recorded suggestion, then look.
    Delegate,
    /// Save these words as a change to the current draft, then redraft. Never confirms.
    Revise(String),
    /// Confirm the draft whose exact bytes have this SHA-256.
    Confirm(String),
}

/// Save `goal` as a new session's confirmed goal under `root`, attributed as `confirmation`
/// declares, and return the session. The words are kept exactly, as the goal and as its one
/// original; `origin` says where they came from. The record says a command saved the goal and
/// nobody was shown it. Also returns the `Done when` lines the ledger reads from the goal: the
/// seed must keep each one whole, and with none the checks are the model's. Calls no model and confirms
/// no seed.
pub fn new_goal(
    root: PathBuf,
    goal: &str,
    origin: &str,
    confirmation: &GoalConfirmation,
) -> Result<(PathBuf, Vec<String>), String> {
    if goal.trim().is_empty() {
        return Err("The goal is empty; nothing was saved.".into());
    }
    // A goal that states its own check keeps it; the ledger reads the same `Done when:` line.
    let outcome = goal
        .lines()
        .find_map(|line| line.trim().strip_prefix("Done when:"))
        .map_or("", str::trim);
    let declared = match confirmation.actor()? {
        crate::think_record::Actor::Agent(_) => "agent",
        crate::think_record::Actor::Operator => "human",
    };
    let receipt = seed_me::Config::new(Some(root)).confirm_as(
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
                "Goal saved by the command `tinkery seed new` from {origin}, launched with \
                 --operator {declared} (a declaration, not verified identity). No goal was \
                 displayed and nobody typed confirm. Goal only, not seed or implementation approval."
            ),
        },
        confirmation,
    )?;
    let kept = seed_me::ledger::goal_criteria(goal);
    Ok((
        receipt.session,
        kept.into_iter().map(str::to_owned).collect(),
    ))
}

/// What Shape would show for `session`, as JSON for a caller without a screen: the goal exactly as
/// recorded, the `Done when` lines the seed must keep whole, the ledger version, and the one
/// current thing (question, draft with the SHA-256
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
    let must_keep = seed_me::ledger::goal_criteria(shaping.goal.as_deref().unwrap_or_default());
    Ok(serde_json::json!({
        "goal": shaping.goal, "must_keep": must_keep, "version": shaping.version,
        "open": shaping.open, "now": now,
    }))
}

/// Take `action` on `session` from a fresh read, through the same step the screen runs. Returns
/// what the caller must read that the ledger does not hold: the folder a model step reads,
/// assumed defaults and a withheld suggestion. Confirming calls no model and needs no `agent`;
/// every other action refuses without one. An action that does not fit where the seed stands
/// changes nothing.
pub fn step(
    session: &Path,
    agent: Option<&Agent>,
    confirmation: &GoalConfirmation,
    action: Action,
) -> Result<Vec<String>, String> {
    let (shaping, base) = shaping::read_with_base(session)?;
    let expected = shaping
        .version
        .ok_or("This session's ledger version is unknown; nothing was changed.")?;
    if let Action::Confirm(sha256) = &action {
        let Shaped::Drafted {
            revision: Some(revision),
            text: Some(seed),
        } = &shaping.stage
        else {
            return Err("There is no draft to confirm; read `tinkery seed show DIR`. Nothing was confirmed.".into());
        };
        if format!("{:x}", Sha256::digest(seed.as_bytes())) != *sha256 {
            return Err(
                "That SHA-256 is not the current draft's; read the draft again. Nothing was confirmed."
                    .into(),
            );
        }
        // The source names the real act: a command, not typed words.
        let source = format!("tinkery seed step confirm {sha256}");
        return match answering::confirm_seed(
            session,
            expected,
            revision,
            seed,
            &source,
            Some(confirmation),
        ) {
            Ok(()) => Ok(Vec::new()),
            Err(Refusal::Stale) => Err(STALE.into()),
            Err(Refusal::Refused(reason) | Refusal::Rejected(reason)) => Err(reason),
        };
    }
    let agent = agent.ok_or(
        "This step needs drafting: put --shape-pi, --model and --shape-budget before seed. Nothing was changed.",
    )?;
    let goal = shaping.goal.as_deref().unwrap_or_default();
    let folder = base.map_or_else(|| agent.folder(session, goal), |base| base.checkout);
    let reads = format!("A model step reads only this folder: {}", folder.display());
    let work = match (action, shaping.stage) {
        (Action::Look, Shaped::Waiting | Shaped::Drafted { .. }) => Work::Look {
            expected,
            folder,
            feedback: None,
        },
        (Action::Answer(words), Shaped::Question { id, .. }) if !words.trim().is_empty() => {
            Work::Answer {
                id,
                words,
                expected,
                folder,
            }
        }
        (Action::Delegate, Shaped::Question { id, .. }) => Work::Respond {
            id,
            action: AnswerAction::Delegate,
            expected,
            folder,
        },
        (
            Action::Revise(words),
            Shaped::Drafted {
                revision: Some(revision),
                ..
            },
        ) if !words.trim().is_empty() => Work::Change {
            revision,
            words,
            expected,
            folder,
        },
        (Action::Answer(_), Shaped::Question { .. })
        | (Action::Revise(_), Shaped::Drafted { .. }) => {
            return Err("The words are empty; nothing was saved.".into());
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
        Ok(Done::Asked(asked)) => Ok(std::iter::once(reads)
            .chain(
                asked
                    .assumed
                    .into_iter()
                    .map(|a| format!("Assumed: {} ({})", a.text, a.why)),
            )
            .chain(
                asked
                    .withheld
                    .map(|s| format!("Suggestion, not recorded: {s}")),
            )
            .collect()),
        Ok(_) => Ok(vec![reads]),
        Err(failure) => Err(format!(
            "{}{}",
            if failure.answer_saved {
                "Your words are saved. "
            } else {
                ""
            },
            match failure.why {
                Why::Stale => STALE.into(),
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
