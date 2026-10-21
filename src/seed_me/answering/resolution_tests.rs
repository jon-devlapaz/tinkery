//! Refused resolution links save no part of a question; copied receipts never revive retired unknowns.

use super::*;
use std::fs;

#[test]
fn refused_resolution_links_keep_facts_open_and_save_no_question_or_findings() {
    let root = tempfile::Builder::new()
        .prefix("TEST-resolution-refusal-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let mut ask = Ask {
        id: "policy".into(),
        label: "TEST policy".into(),
        question: "TEST read-only?".into(),
        gate: "TEST policy".into(),
        found: vec![Found {
            id: "gap".into(),
            claim: json!({"type":"unknown","scope":"TEST read-only"}),
            checked: "TEST repository".into(),
            observed: "TEST not established".into(),
            supported_by: None,
            contradicts: None,
        }],
        resolves: None,
        supported_by: None,
        prerequisites: None,
        suggestion: None,
        assumed: vec![],
        options: vec![],
        against: None,
        confidence: None,
    };
    let version = super::ask(&session, 1, &ask, root.path()).unwrap();
    let before = fs::read(session.join("ledger.json")).unwrap();
    ask.id = "next".into();
    ask.found[0].id = "new-gap".into();
    for (links, reason) in [
        (
            json!([{"unknown":"gap","resolved_by":"policy"}]),
            "settled user or delegated decision",
        ),
        (
            json!([{"unknown":"goal","resolved_by":"goal"}]),
            "open unknown fact",
        ),
        (
            json!([{"unknown":"missing","resolved_by":"goal"}]),
            "open unknown fact",
        ),
    ] {
        ask.resolves = Some(links);
        let error = super::ask(&session, version, &ask, root.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains(reason), "{error}");
        assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
        assert!(
            read(&session)
                .unwrap()
                .findings()
                .iter()
                .any(|n| n["id"] == "gap")
        );
        assert!(!session.join("seed-contract.md").exists());
    }
}

#[test]
fn resolved_unknown_receipts_are_omitted_even_when_copied_into_decision_evidence() {
    let receipt = json!({"checked":"TEST repository", "observed":"TEST retired gap", "at":"2026-10-21T00:00:00Z"});
    let state = json!({"origin":"goal", "nodes":[
        {"id":"gap", "kind":"fact", "status":"settled", "claim":{"type":"unknown","scope":"TEST"}, "resolved_by":"answer", "evidence":[receipt]},
        {"id":"answer", "kind":"decision", "status":"settled", "evidence":[receipt]}
    ]});
    assert!(receipts(&state, true).is_empty());
    assert!(receipts(&state, false).is_empty());
    let ledger = Ledger { version: 1, state };
    assert!(ledger.findings().is_empty());
}
