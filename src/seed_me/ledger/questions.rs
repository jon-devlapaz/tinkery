//! Question scheduling and proposed prerequisites. Only decisions can gate a new question;
//! readiness and ordering come from the ledger validator, never a phase's parallel rules.

use super::*;

/// Add an open question without settling or replacing any earlier decision. Invalid prerequisites
/// or findings refuse before publication; held questions remain recorded and the first ready
/// decision becomes current only when no question is already current.
pub fn queue_question(state: &mut Value, question: Value) -> Result<()> {
    require(
        question["kind"].is("decision") && question["status"].is("unresolved"),
        "A proposed question must be an open decision. Retry the step.",
    )?;
    let id = question["id"].string().unwrap_or("").to_owned();
    require(
        state["nodes"].array().is_some(),
        "The session has no decisions list. Reopen the session before trying again.",
    )?;
    state["nodes"].list_mut().push(question);
    let ready = validate(state).map_err(question_refusal)?;
    let node = find(&state["nodes"], &id);
    require(
        node["prerequisites"]
            .items()
            .iter()
            .all(|parent| find(&state["nodes"], parent.string().unwrap())["kind"].is("decision")),
        "A question can wait only for earlier decisions, not findings. Cite findings in supported_by and retry the step.",
    )?;
    if state["current_question"] == Value::Null {
        let current = first_question(state, &ready).map_or(Value::Null, Value::from);
        state.set("current_question", current);
    }
    Ok(())
}

/// The first ready decision in ledger order. Validates the editable state; a held decision is
/// never returned, even when it appears before a ready one.
pub fn next_question(state: &Value) -> Result<Option<String>> {
    let ready = validate(state).map_err(question_refusal)?;
    Ok(first_question(state, &ready).map(str::to_owned))
}

fn first_question<'a>(state: &'a Value, ready: &'a [String]) -> Option<&'a str> {
    ready.iter().find_map(|id| {
        find(&state["nodes"], id)["kind"]
            .is("decision")
            .then_some(id.as_str())
    })
}

/// Unsettled prerequisite IDs of a validated node, in its recorded order. Empty means no
/// prerequisite holds it; other readiness rules, such as its predicate, still apply.
pub fn waiting_on(nodes: &Value, id: &str) -> Vec<String> {
    find(nodes, id)["prerequisites"]
        .items()
        .iter()
        .filter(|parent| !find(nodes, parent.string().unwrap_or(""))["status"].is("settled"))
        .filter_map(|parent| parent.string().map(str::to_owned))
        .collect()
}

fn question_refusal(reason: String) -> String {
    match reason.as_str() {
        "invalid prerequisites" => "Question prerequisites must be a list of decision IDs. Retry the step.",
        "duplicate prerequisite" => "A question repeats a prerequisite. Name each earlier decision once and retry the step.",
        "missing prerequisite" => "A question waits for a decision that isn't recorded. Use existing decision IDs and retry the step.",
        "dependency cycle" => "The questions depend on each other in a circle. Remove the circular prerequisite and retry the step.",
        _ => return reason,
    }.into()
}
