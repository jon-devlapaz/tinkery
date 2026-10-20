//! Operator answers beyond typing: grounded delegation, explicit deferral and revision with
//! dependent reopening. No model reply can invoke these actions; publication holds the writer lock.

use super::{
    Value, array, descendants, editable, find, object, require, review_flags, text, validate,
};

/// An explicit operator action on one displayed question or settled decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerAction {
    /// Leave the question unconfirmed until the operator's stated condition holds.
    Defer {
        /// Why the operator chooses to leave it open.
        reason: String,
        /// When it must be revisited.
        revisit: String,
    },
    /// Accept the recorded suggestion on the strength of its supporting facts.
    Delegate,
    /// Replace an operator-answered decision; settled dependents must be answered again.
    Revise(String),
}

/// Whether a settled answer is the operator's revisable decision, never a finding or goal.
/// A screen may offer revision only when this decision is also flagged for review.
pub fn can_revise(ledger: &Value, id: &str) -> bool {
    let node = find(&ledger["nodes"], id);
    node["kind"].is("decision")
        && node["status"].is("settled")
        && matches!(node["authority"].string(), Some("user" | "delegated"))
        && node["id"] != ledger["origin"]
}

pub(super) fn change(
    ledger: &Value,
    id: &str,
    action: &AnswerAction,
) -> Result<(Value, Value), String> {
    let mut state = editable(ledger)?;
    let before = find(&state["nodes"], id).clone();
    require(
        before != Value::Null,
        "This answer is no longer recorded. Reopen the session.",
    )?;
    let mut node = before.clone();
    let mut reviewed = object(&[]);
    match action {
        AnswerAction::Defer { reason, revisit } => {
            open_question(&state, &before, id)?;
            node.set("status", "deferred".into());
            node.set("defer_reason", reason.as_str().into());
            node.set("revisit_condition", revisit.as_str().into());
            state.set("current_question", Value::Null);
        }
        AnswerAction::Delegate => {
            open_question(&state, &before, id)?;
            require(
                text(&before["recommendation"]),
                "This question has no suggestion to delegate. Type your answer instead.",
            )?;
            let mut evidence = array();
            for source in before["supported_by"].items() {
                let fact = find(&state["nodes"], source.string().unwrap_or(""));
                if fact["kind"].is("fact") && fact["status"].is("settled") {
                    evidence
                        .list_mut()
                        .extend(fact["evidence"].items().iter().cloned());
                }
            }
            node.set("status", "settled".into());
            node.set("answer", before["recommendation"].clone());
            node.set("authority", "delegated".into());
            node.set(
                "authority_source",
                "the operator pressed 'F12 delegate suggestion' in Tinkery Shape".into(),
            );
            node.set("evidence", evidence);
            state.set("current_question", Value::Null);
        }
        AnswerAction::Revise(words) => {
            require(
                can_revise(ledger, id),
                "Only an operator-answered decision can be revised. Keep this finding or goal with a reason instead.",
            )?;
            require(
                text(&words.as_str().into()),
                "Type the replacement answer first; nothing was saved.",
            )?;
            node.set("answer", words.as_str().into());
            node.set("authority", "user".into());
            node.set(
                "authority_source",
                "the operator's replacement answer, typed in Tinkery Shape".into(),
            );
            state.set("current_question", Value::Null);
            let affected = descendants(&state["nodes"], &[id.to_owned()]);
            for dependent in state["nodes"].list_mut() {
                if affected.contains(dependent["id"].string().unwrap_or(""))
                    && dependent["status"].is("settled")
                {
                    dependent.set("status", "unresolved".into());
                    for field in ["answer", "authority", "authority_source"] {
                        dependent.remove(field);
                    }
                    dependent.set(
                        "reopen_reason",
                        format!("Answer {id} changed to: {words}").as_str().into(),
                    );
                }
            }
            if review_flags(&ledger["nodes"]).has(id) {
                reviewed.set(id, words.as_str().into());
            }
        }
    }
    *state["nodes"]
        .list_mut()
        .iter_mut()
        .find(|n| n["id"].is(id))
        .unwrap() = node;
    let ready = validate(&state).map_err(|reason| {
        if reason == "deferral requires reason and revisit condition" {
            format!("{reason}. Fill in both fields, then save again.")
        } else if reason == "evidence or delegated settlement requires evidence"
            && matches!(action, AnswerAction::Delegate)
        {
            "This question has no supporting facts to delegate. Type your answer instead.".into()
        } else {
            reason
        }
    })?;
    if matches!(action, AnswerAction::Revise(_)) {
        let affected = descendants(&state["nodes"], &[id.to_owned()]);
        let current = ready
            .iter()
            .find(|id| affected.contains(*id) && find(&state["nodes"], id)["kind"].is("decision"))
            .or_else(|| {
                ready
                    .iter()
                    .find(|id| find(&state["nodes"], id)["kind"].is("decision"))
            });
        state.set(
            "current_question",
            current.map_or(Value::Null, |id| id.as_str().into()),
        );
    }
    Ok((state, reviewed))
}

fn open_question(state: &Value, node: &Value, id: &str) -> Result<(), String> {
    require(
        node["kind"].is("decision")
            && node["status"].is("unresolved")
            && state["current_question"].is(id),
        "This question is no longer current. Reopen the session before answering.",
    )
}
