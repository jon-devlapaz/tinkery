//! Reply transport tests; semantic fact rules remain in the shared ledger.

use super::*;

#[test]
fn zero_budget_refuses_revision_before_reading_or_publishing() {
    let root = tempfile::Builder::new()
        .prefix("TEST-revise-budget-")
        .tempdir()
        .unwrap();
    let ledger = root.path().join("ledger.json");
    std::fs::write(&ledger, "TEST untouched ledger").unwrap();
    let agent = Agent::new(
        root.path().join("TEST-never-run"),
        "test/model".into(),
        String::new(),
        Some(root.path().into()),
        CallBudget::new(0),
        None,
    )
    .unwrap();
    let work = Work::Respond {
        id: "TEST-choice".into(),
        expected: 1,
        action: crate::seed_me::ledger::AnswerAction::Revise("TEST replacement".into()),
        folder: root.path().into(),
    };
    let saved = AtomicBool::new(false);
    assert_eq!(
        run(
            &work,
            &root.path().join("TEST-skill"),
            root.path(),
            &agent,
            &AtomicBool::new(false),
            &saved
        ),
        Err(Failure {
            answer_saved: false,
            why: Why::Budget(0)
        })
    );
    assert!(!saved.load(Ordering::Relaxed));
    assert_eq!(
        std::fs::read_to_string(ledger).unwrap(),
        "TEST untouched ledger"
    );
}

#[test]
fn teach_back_zero_budget_refuses_before_capture_or_any_read() {
    let root = tempfile::Builder::new()
        .prefix("TEST-teach-back-budget-")
        .tempdir()
        .unwrap();
    let path = root.path().join("ledger.json");
    std::fs::write(&path, "TEST untouched").unwrap();
    let agent = Agent::new(
        root.path().join("TEST-never-run"),
        "test/model".into(),
        String::new(),
        Some(root.path().into()),
        CallBudget::new(0),
        None,
    )
    .unwrap();
    let work = Work::TeachBack {
        revision: "r1".into(),
        words: "TEST exact words\r\n".into(),
        expected: 1,
        folder: root.path().into(),
        seed: "TEST draft".into(),
    };
    let saved = AtomicBool::new(false);
    assert_eq!(
        run(
            &work,
            &root.path().join("TEST-skill"),
            root.path(),
            &agent,
            &AtomicBool::new(false),
            &saved
        ),
        Err(Failure {
            answer_saved: false,
            why: Why::Budget(0)
        })
    );
    assert!(!saved.load(Ordering::Relaxed));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "TEST untouched");
}

#[test]
fn a_seed_reply_requires_a_typed_completion_review_without_parser_diagnostics() {
    let mut reply = json!({"next":"seed", "found":[], "confirming":{},
        "checks":[{"statement":"TEST exact", "kind":"judge", "source":"TEST"}]});
    for review in [
        Value::Null,
        json!("reviewed"),
        json!({"contradictions":[]}),
        json!({"contradictions":[], "decided":[], "done":[], "failure_cases":[], "extra":true}),
    ] {
        reply["review"] = review;
        let error = decode(&reply.to_string()).unwrap_err();
        assert!(error.contains("typed completion review"), "{error}");
        assert!(
            !error.contains("invalid type")
                && !error.contains("missing field")
                && !error.contains("unknown field")
        );
    }
    reply.as_object_mut().unwrap().remove("review");
    assert!(
        decode(&reply.to_string())
            .unwrap_err()
            .contains("typed completion review")
    );
    reply["review"] = json!({"contradictions":[], "decided":["TEST choice"], "done":[],
        "failure_cases":[{"statement":"TEST exact", "case":"TEST wrong result must fail"}]});
    let Next::Seed { review, checks, .. } = decode(&reply.to_string()).unwrap() else {
        panic!()
    };
    review
        .validate(&crate::seed_me::ledger::Value::parse(&checks.to_string()).unwrap())
        .unwrap();
    let prompt = prompt();
    for rule in [
        "Before drafting in this same step",
        "Separate decided",
        "one case that must fail",
        "a new blocker returns as the next question, never a seed",
        "never a separate call",
    ] {
        assert!(prompt.contains(rule), "{rule}");
    }
}

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
fn question_prerequisites_pass_unchanged_to_the_ledger_boundary() {
    let mut reply = json!({"next":"question","question":"TEST?","found":[]});
    let Next::Question(legacy) = decode(&reply.to_string()).unwrap() else {
        panic!()
    };
    assert_eq!(legacy.prerequisites, None);
    for proposed in [
        json!(["b", "a"]),
        Value::Null,
        json!("not a list"),
        json!([42]),
    ] {
        reply["prerequisites"] = proposed.clone();
        let Next::Question(ask) = decode(&reply.to_string()).unwrap() else {
            panic!()
        };
        assert_eq!(ask.prerequisites, Some(proposed));
    }
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
