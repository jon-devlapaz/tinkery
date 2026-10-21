//! TEST-only confirmed seed fixtures use the real ledger, check renderer and confirmation writer.

use super::*;
use std::{fs, path::Path};

/// Adapt historical Markdown reply fixtures into prose fields without rewriting the evidence.
pub(crate) fn prose_reply(body: &str) -> serde_json::Value {
    let mut fields = serde_json::Map::new();
    let mut name = "You are confirming";
    let mut prose = String::new();
    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if !prose.trim().is_empty() {
                fields.insert(name.into(), prose.trim().into());
            }
            name = heading;
            prose.clear();
        } else {
            // Historical evidence stays intact; replay its probe readings as guesses, not answers.
            prose.push_str(
                &line
                    .replace("Probe:", "Probe reading (guess):")
                    .replace("— answer:", "— evidence reading:"),
            );
            prose.push('\n');
        }
    }
    if !prose.trim().is_empty() {
        fields.insert(name.into(), prose.trim().into());
    }
    serde_json::Value::Object(fields)
}

/// Save a valid human session and confirm its literal goal, without helpers or model calls.
pub(crate) fn confirmed(directory: &Path, goal: &str, revision: &str) {
    let mut ledger = Value::parse(r#"{"schema_version":1,"session_id":"TEST-session","version":1,"revision":0,
        "created_at":"2026-10-19T00:00:00+00:00","updated_at":"2026-10-19T00:00:00+00:00",
        "status":"active","draft":{"goal":"","outcome":"","options":[]},"goal":null,
        "origin":"goal","current_question":null,"frontier":[],"nodes":[],"assumed":[],"operator":"human"}"#).unwrap();
    ledger.set("goal", goal.into());
    let node = Value::parse(
        r#"{"id":"goal","kind":"decision","status":"settled","prerequisites":[],"evidence":[],
        "owner":"TEST","gate":"TEST","authority":"user","authority_source":"TEST fixture only"}"#,
    )
    .unwrap();
    let mut node = node;
    node.set("answer", goal.into());
    node.set("history", value::array());
    node.set("revision", value::int(0));
    ledger.set("nodes", Value::Array(vec![node]));
    fs::write(directory.join("ledger.json"), ledger.pretty().unwrap()).unwrap();
    let records =
        Value::parse(r#"[{"statement":"TEST exact café  ","kind":"judge","source":"TEST"}]"#)
            .unwrap();
    set_checks(directory, &records).unwrap();
    let mut seed = format!("status: draft\n# TEST\nrevision: {revision}\n\n");
    for heading in checks::LEAN {
        match *heading {
            "## Acceptance criteria" => seed.push_str(&checks_block(&records).unwrap()),
            "## Decisions (from the ledger)" => seed.push_str(&decisions_block(&ledger).unwrap()),
            "## Assumed (not confirmed by you)" => {}
            _ => seed.push_str(&format!("{heading}\nTEST\n\n")),
        }
    }
    fs::write(directory.join("seed-contract.md"), seed).unwrap();
    confirm_seed(
        directory,
        revision,
        "TEST confirmation only",
        "2026-10-19T00:00:00+00:00",
    )
    .unwrap();
}
