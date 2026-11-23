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

use super::step::{Agent, Done, NO_AGENT, Steps, Why, Work};
use super::{
    answering,
    shaping::{self, Shaped},
};
use crate::confirmation::GoalConfirmation;
use crate::seed_me::{
    self,
    ledger::{self, AnswerAction},
};

const STALE: &str = "Another writer changed this session first; nothing more was saved.";

/// The one thing a `seed step` call does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Look into the next question, or draft the seed, when nothing is being asked.
    Look,
    /// Save these words as the answer to the current question, then look.
    Answer(String),
    /// Save the operator's relayed words as their own answer to the current question, then look.
    Relayed(answering::Relay),
    /// Accept the current question's recorded suggestion, then look.
    Delegate,
    /// Confirm the draft whose exact bytes have this SHA-256.
    Confirm(String),
}

/// Save `goal` as a new session's confirmed goal under `root`, attributed as `confirmation`
/// declares, and return the session. The words are kept exactly, as the goal and as its one
/// original; `origin` says where they came from. The record says a command saved the goal and
/// nobody was shown it. Calls no model and confirms no seed.
pub fn new_goal(
    root: PathBuf,
    goal: &str,
    origin: &str,
    confirmation: &GoalConfirmation,
) -> Result<PathBuf, String> {
    if goal.trim().is_empty() {
        return Err("The goal is empty; nothing was saved.".into());
    }
    // A goal that states its own check keeps it; the ledger reads the same `Done when:` line.
    let outcome = goal
        .lines()
        .find_map(|line| line.trim().strip_prefix("Done when:"))
        .map_or("", str::trim);
    let actor = confirmation.actor()?;
    let declared = actor.session_operator();
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
        actor,
    )?;
    Ok(receipt.session)
}

/// What Shape would show for `session`, as JSON for a caller without a screen: the goal exactly as
/// recorded, the ledger version, the open questions, the one current thing (question, draft with
/// the SHA-256 that `Action::Confirm` names, ready seed, or a plain other state), and
/// `decided_without_you`: every settled answer not in the operator's own words and every assumed
/// default, so the operator can overrule any of them, and `gray`, as [`ledger::gray`] defines
/// it. Read-only.
pub fn show(session: &Path) -> Result<serde_json::Value, String> {
    let shaping = shaping::read(session)?;
    let now = match &shaping.stage {
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
            "sha256": text.as_deref().map(sha256),
            "text": text, "needs_check": shaping.checks.iter().find(|check| check.gap().is_some()).map(|check| check.statement()), "signoff": shaping.signoff,
            "risks": shaping.review_lines(false).collect::<Vec<_>>(),
        }),
        Shaped::Ready { revision, ended } => {
            serde_json::json!({"stage": "ready", "revision": revision, "ended": ended})
        }
        other => serde_json::json!({"stage": "other", "what": format!("{other:?}")}),
    };
    let ledger: serde_json::Value =
        serde_json::from_str(&seed_me::ledger::load(session)?.pretty()?)
            .map_err(|e| e.to_string())?;
    let decided_without_you = shaping
        .decided
        .iter()
        .filter(|d| d.authority != super::answering::Authority::User)
        .map(
            |d| serde_json::json!({"what": d.title, "decided": d.answer, "authority": d.authority.word()}),
        )
        .chain(ledger["assumed"].as_array().into_iter().flatten().map(
            |a| serde_json::json!({"what": a["text"], "why": a["why"], "authority": "assumed"}),
        ))
        .collect::<Vec<_>>();
    let gray = ledger::gray(&ledger);
    Ok(serde_json::json!({
        "goal": shaping.goal, "version": shaping.version, "open": shaping.open, "now": now,
        "decided_without_you": decided_without_you, "gray": gray,
    }))
}

/// Take `action` on `session` from a fresh read, through the same step the screen runs. Returns
/// what the caller must read that the ledger does not hold: the folder a model step reads, why
/// Think's answers are not sent if so,
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
    let mut notes = Vec::new();
    let expected = shaping
        .version
        .ok_or("This session's ledger version is unknown; nothing was changed.")?;
    let steps = Steps::new(agent, confirmation);
    let work = if let Action::Confirm(sha) = &action {
        let Shaped::Drafted {
            revision: Some(revision),
            text: Some(seed),
        } = shaping.stage
        else {
            return Err("There is no draft to confirm; read `tinkery seed show DIR`. Nothing was confirmed.".into());
        };
        if sha256(&seed) != *sha {
            return Err(
                "That SHA-256 is not the current draft's; read the draft again. Nothing was confirmed."
                    .into(),
            );
        }
        Work::Confirm {
            revision,
            // The source names the real act: a command, not typed words.
            words: format!("tinkery seed step confirm {sha}"),
            expected,
            seed,
        }
    } else {
        if agent.is_none() {
            return Err(NO_AGENT.into());
        }
        let goal = shaping.goal.as_deref().unwrap_or_default();
        let reads = steps
            .reads(session, Some(goal), base.map(|base| base.checkout))?
            .unwrap_or_else(|| PathBuf::new().into());
        notes.push(format!(
            "A model step reads only this folder: {}",
            reads.folder.display()
        ));
        notes.extend(reads.unread());
        match (action, shaping.stage) {
            (Action::Look, Shaped::Waiting) => Work::Look {
                expected,
                reads,
                feedback: None,
            },
            (Action::Answer(words), Shaped::Question { id, .. }) if !words.trim().is_empty() => {
                Work::Answer {
                    id,
                    words,
                    relay: None,
                    expected,
                    reads,
                }
            }
            (Action::Relayed(relay), Shaped::Question { id, .. }) => Work::Answer {
                id,
                words: relay.words().to_owned(),
                relay: Some(relay),
                expected,
                reads,
            },
            (Action::Delegate, Shaped::Question { id, .. }) => Work::Respond {
                id,
                action: AnswerAction::Delegate,
                expected,
                reads,
            },
            (Action::Answer(_), Shaped::Question { .. }) => {
                return Err("The answer is empty; nothing was saved.".into());
            }
            _ => {
                return Err(
                    "That action does not fit the status of this Tink; read `tinkery seed show DIR`. Nothing was changed."
                        .into(),
                );
            }
        }
    };
    let (stopped, saved) = (AtomicBool::new(false), AtomicBool::new(false));
    match steps.run(&work, session, &stopped, &saved) {
        Ok(Done::Asked(asked)) => Ok(notes
            .into_iter()
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
        Ok(_) => Ok(notes),
        Err(failure) => Err(format!(
            "{}{}",
            if failure.answer_saved {
                "Your answer is saved. "
            } else {
                ""
            },
            match failure.why {
                Why::Stale => STALE.into(),
                Why::Budget(limit) => format!(
                    "No drafting responses are left: this launch's allowance is {limit}. Relaunch with --shape-budget."
                ),
                Why::Model(reason) => format!("The model step failed: {reason}"),
                Why::Refused(reason) => reason,
                Why::Rule(rule) => rule.to_string(),
                Why::Stopped => "The step was stopped.".into(),
            }
        )),
    }
}

/// The SHA-256 that names a draft's exact bytes, as `show` prints it and `Action::Confirm` names it.
fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
