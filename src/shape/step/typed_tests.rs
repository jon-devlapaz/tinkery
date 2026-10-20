//! Reply transport tests; semantic fact rules remain in the shared ledger.

use super::*;

#[test]
fn decodes_each_claim_and_keeps_support_and_contradiction_references() {
    let reply = json!({"next":"question","id":"choice","question":"TEST choose?",
    "supported_by":["read","inferred"], "found":[
        {"id":"read","claim":{"type":"observation","scope":"README:1"},
            "checked":"README.md:1","observed":"TEST local"},
        {"id":"inferred","claim":{"type":"inference","scope":"TEST local path","limits":"not a runtime check"},
            "checked":"README.md:1","observed":"TEST likely local","supported_by":["read"],"contradicts":"earlier"},
        {"id":"gap","claim":{"type":"unknown","scope":"TEST sync"},
            "checked":"README.md:1","observed":"This investigation has not established sync."}
    ]});
    let Next::Question(ask) = decode(&reply.to_string()).unwrap() else {
        panic!("not a question")
    };
    assert_eq!(ask.supported_by, Some(json!(["read", "inferred"])));
    for (found, expected) in ask
        .found
        .iter()
        .zip(["observation", "inference", "unknown"])
    {
        assert_eq!(found.claim["type"], expected);
    }
    assert_eq!(ask.found[1].supported_by, Some(json!(["read"])));
    assert_eq!(ask.found[1].contradicts, Some(json!("earlier")));
    assert_eq!(ask.found[1].claim["limits"], "not a runtime check");
}

#[test]
fn untyped_or_partial_findings_are_refused_never_dropped_or_guessed() {
    for found in [
        json!([{"checked":"README.md:1","observed":"TEST local"}]),
        json!([{"id":"read","claim":{"type":"observation","scope":"TEST"},"checked":"README.md:1"}]),
        json!([{"id":"read","claim":{"type":"observation","scope":"TEST"},"checked":"README.md:1","observed":"TEST", "authority":"user"}]),
        json!({}),
    ] {
        let parser_error = serde_json::from_value::<Vec<Found>>(found.clone())
            .unwrap_err()
            .to_string();
        let error =
            decode(&json!({"next":"question","question":"TEST?","found":found}).to_string())
                .unwrap_err();
        assert!(error.contains("typed claim"), "{error}");
        assert!(!error.contains(&parser_error), "{error}");
        assert!(error.ends_with("Retry the step."), "{error}");
    }
}
