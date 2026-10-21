//! Unknown-resolution links preserve findings as history; only settled operator decisions cover them.
//! Links never settle an answer or prove that its words semantically cover the unknown.

use super::{Result, Value, find, one_of, python::text, require};
use std::collections::BTreeSet;

/// Whether a finding has been retired by a recorded answer, without deleting its original words.
pub fn resolved_unknown(node: &Value) -> bool {
    node.has("resolved_by")
}

pub(super) fn validate_resolution(node: &Value, nodes: &Value) -> Result<()> {
    require(
        node["kind"].is("fact")
            && node["status"].is("settled")
            && node["claim"]["type"].is("unknown")
            && text(&node["resolved_by"]),
        "resolved_by belongs only to an unknown fact and names a settled user or delegated decision",
    )?;
    let answer = find(nodes, node["resolved_by"].string().unwrap());
    require(
        answer["kind"].is("decision")
            && answer["status"].is("settled")
            && one_of(&answer["authority"], &["user", "delegated"]),
        "unknown resolution requires a settled user or delegated decision",
    )
}

fn open_unknown(node: &Value) -> bool {
    node["kind"].is("fact")
        && node["status"].is("settled")
        && node["claim"]["type"].is("unknown")
        && !resolved_unknown(node)
}

pub(super) fn validate_new_resolutions(old: &Value, proposed: &Value) -> Result<()> {
    for node in proposed["nodes"]
        .items()
        .iter()
        .filter(|n| resolved_unknown(n))
    {
        let previous = find(&old["nodes"], node["id"].string().unwrap_or(""));
        require(
            node["resolved_by"] == previous["resolved_by"] || open_unknown(previous),
            "unknown resolution requires an open unknown fact; do not replace an existing link",
        )?;
    }
    Ok(())
}

/// Apply a step's list of {unknown, resolved_by} links atomically to editable state.
/// Refuse missing IDs, repeat targets, facts other than open unknowns and unsettled answers.
pub fn resolve_unknowns(state: &mut Value, links: &Value) -> Result<()> {
    require(
        links.array().is_some(),
        "resolves must be a list of {unknown, resolved_by} links",
    )?;
    let mut proposed = state.clone();
    let mut seen = BTreeSet::new();
    for link in links.items() {
        require(
            super::exact(link, &["unknown", "resolved_by"])
                && text(&link["unknown"])
                && text(&link["resolved_by"]),
            "each resolves link requires unknown and resolved_by node IDs",
        )?;
        let id = link["unknown"].string().unwrap();
        require(seen.insert(id), "resolves repeats an unknown fact ID")?;
        let node = find(&proposed["nodes"], id);
        require(
            open_unknown(node),
            format!("unknown resolution requires an open unknown fact: {id}"),
        )?;
        let mut node = node.clone();
        node.set("resolved_by", link["resolved_by"].clone());
        validate_resolution(&node, &proposed["nodes"])?;
        *proposed["nodes"]
            .list_mut()
            .iter_mut()
            .find(|n| n["id"].is(id))
            .unwrap() = node;
    }
    *state = proposed;
    Ok(())
}

// A changed answer no longer covers an earlier unknown. Publication keeps the old link in history.
pub(super) fn reopen_changed_resolutions(old: &Value, proposed: &mut Value) {
    if proposed["nodes"].array().is_none() {
        return;
    }
    let nodes = proposed["nodes"].clone();
    for node in proposed["nodes"].list_mut() {
        let previous = find(&old["nodes"], node["id"].string().unwrap_or(""));
        if let Some(id) = previous["resolved_by"].string() {
            let before = find(&old["nodes"], id);
            let after = find(&nodes, id);
            if node["resolved_by"] == previous["resolved_by"]
                && ["answer", "status", "authority"]
                    .iter()
                    .any(|key| before[key] != after[key])
            {
                node.remove("resolved_by");
            }
        }
    }
}

#[cfg(test)]
mod tests;
