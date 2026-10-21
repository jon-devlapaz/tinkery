//! Resolution eligibility, atomic refusals, history and answer changes are ledger contracts.

use super::*;
use crate::seed_me::ledger::{self, value::array};

fn state() -> Value {
    Value::from(serde_json::json!({"nodes":[
        {"id":"gap","kind":"fact","status":"settled","claim":{"type":"unknown","scope":"TEST read-only"},"answer":"TEST not established","prerequisites":[],"authority":"evidence","authority_source":"TEST investigation","evidence":["TEST receipt"]},
        {"id":"answer","kind":"decision","status":"settled","authority":"user","answer":"Strictly read-only: never modify input files.","authority_source":"TEST operator","prerequisites":[],"owner":"User","gate":"TEST read-only","evidence":["TEST receipt"]}
    ]}))
}

fn links() -> Value {
    Value::from(serde_json::json!([{"unknown":"gap","resolved_by":"answer"}]))
}

#[test]
fn unknown_resolution_refuses_unsettled_answers_and_non_open_unknowns_atomically() {
    let original = state();
    let mut cases = vec![];
    for status in ["unresolved", "deferred", "superseded"] {
        let mut candidate = original.clone();
        candidate["nodes"].list_mut()[1].set("status", status.into());
        cases.push((candidate, links(), "settled user or delegated decision"));
    }
    for authority in ["evidence", "simulated"] {
        let mut candidate = original.clone();
        candidate["nodes"].list_mut()[1].set("authority", authority.into());
        cases.push((candidate, links(), "settled user or delegated decision"));
    }
    for kind in ["observation", "inference"] {
        let mut candidate = original.clone();
        candidate["nodes"].list_mut()[0]["claim"].set("type", kind.into());
        cases.push((candidate, links(), "open unknown fact"));
    }
    let mut resolved = original.clone();
    resolve_unknowns(&mut resolved, &links()).unwrap();
    cases.push((resolved, links(), "open unknown fact"));
    let mut superseded = original.clone();
    superseded["nodes"].list_mut()[0].set("status", "superseded".into());
    cases.push((superseded, links(), "open unknown fact"));
    for id in ["answer", "missing"] {
        let mut proposed = links();
        proposed.list_mut()[0].set("unknown", id.into());
        cases.push((original.clone(), proposed, "open unknown fact"));
    }
    let mut missing = links();
    missing.list_mut()[0].set("resolved_by", "missing".into());
    cases.push((
        original.clone(),
        missing,
        "settled user or delegated decision",
    ));
    let mut mixed = links();
    mixed.list_mut().push(Value::from(
        serde_json::json!({"unknown":"missing","resolved_by":"answer"}),
    ));
    cases.push((original.clone(), mixed, "open unknown fact"));
    cases.push((original.clone(), Value::Null, "resolves must be a list"));
    for (mut candidate, proposed, reason) in cases {
        let before = candidate.clone();
        let error = resolve_unknowns(&mut candidate, &proposed).unwrap_err();
        assert!(error.contains(reason), "{error}");
        assert_eq!(candidate, before, "refused links changed the facts");
    }
    assert!(!resolved_unknown(&original["nodes"].items()[0]));
}

#[test]
fn resolutions_preserve_history_and_reopen_when_the_covering_answer_changes() {
    let root = tempfile::Builder::new()
        .prefix("TEST-unknown-history-")
        .tempdir()
        .unwrap();
    ledger::fixtures::confirmed(root.path(), "TEST goal", "r1");
    let saved = ledger::load(root.path()).unwrap();
    let mut proposed = ledger::editable(&saved).unwrap();
    proposed["nodes"]
        .list_mut()
        .extend(state()["nodes"].items().iter().cloned());
    let saved = ledger::publish(
        root.path(),
        &proposed,
        &saved["version"],
        &"TEST record unknown".into(),
        None,
        "2026-10-21T00:00:00Z",
    )
    .unwrap();
    for authority in ["user", "delegated"] {
        let mut proposed = ledger::editable(&saved).unwrap();
        proposed["nodes"].list_mut()[2].set("authority", authority.into());
        resolve_unknowns(&mut proposed, &links()).unwrap();
        let resolved = ledger::transition(
            &saved,
            &proposed,
            &saved["version"],
            &"TEST answer covers gap".into(),
            None,
            "2026-10-21T00:00:01Z",
        )
        .unwrap();
        let gap = &resolved["nodes"].items()[1];
        assert!(resolved_unknown(gap));
        assert_eq!(gap["resolved_by"], "answer".into());
        assert_eq!(gap["answer"], "TEST not established".into());
        assert_eq!(gap["history"].items().len(), 1);
        assert!(!gap["history"].items()[0]["state"].has("resolved_by"));
        assert!(
            !ledger::decisions_block(&resolved)
                .unwrap()
                .contains("TEST not established")
        );
        for change in ["answer", "status"] {
            let mut revised = ledger::editable(&resolved).unwrap();
            let answer = &mut revised["nodes"].list_mut()[2];
            if change == "answer" {
                answer.set("answer", "TEST revised policy".into());
            } else {
                answer.set("status", "unresolved".into());
                answer.set("reopen_reason", "TEST changed premise".into());
                for field in ["answer", "authority", "authority_source"] {
                    answer.remove(field);
                }
            }
            let reopened = ledger::transition(
                &resolved,
                &revised,
                &resolved["version"],
                &"TEST revise answer".into(),
                None,
                "2026-10-21T00:00:02Z",
            )
            .unwrap();
            assert!(!resolved_unknown(&reopened["nodes"].items()[1]));
            assert_eq!(reopened["nodes"].items()[1]["history"].items().len(), 2);
            assert!(
                ledger::decisions_block(&reopened)
                    .unwrap()
                    .contains("TEST not established")
            );
        }
    }
    let mut empty = state();
    let before = empty.clone();
    resolve_unknowns(&mut empty, &array()).unwrap();
    assert_eq!(empty, before);
}
