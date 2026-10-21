//! Reply transport tests; semantic fact rules remain in the shared ledger.

use super::*;

#[test]
fn session_sized_step_sends_findings_once_without_changing_saved_context() {
    use crate::seed_me::{Affirmed, Config, Original};
    let root = tempfile::Builder::new()
        .prefix("TEST-send-once-")
        .tempdir()
        .unwrap();
    let context = "TEST Think context: ".to_owned() + &"c".repeat(3920);
    let goal = "TEST goal: ".to_owned() + &"g".repeat(1015);
    let actor: crate::think_record::GoalConfirmation = serde_json::from_value(json!({
        "by":"agent", "agent":"TEST-agent", "authorizing_operator":"TEST-person",
        "source":"TEST scoped authorization: ".to_owned() + &"s".repeat(200),
        "class":{"door":"two_way", "door_reason":"TEST reversible scratch session",
        "blast_radius":"small", "blast_radius_reason":"TEST isolated scratch roots"}
    }))
    .unwrap();
    let session = Config::new(Some(root.path().join("TEST-sessions")))
        .confirm_as(
            Affirmed {
                goal: goal.clone(),
                outcome: "TEST outcome".into(),
                options: vec![],
                originals: vec![Original {
                    id: 1,
                    in_reply_to: None,
                    text: "TEST original: ".to_owned() + &"o".repeat(700),
                }],
                context: context.clone(),
                source: actor.source(),
            },
            &actor,
        )
        .unwrap()
        .session;
    let path = session.join("ledger.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let origin = saved["nodes"][0].clone();
    let actor = actor.actor().unwrap();
    let receipt = json!({"checked":"Caller-declared authorization, not authenticated identity",
        "at":"2026-10-21T00:00:00Z", "observed":actor.context().unwrap().details()});
    // The dogfood had nine facts and four agent answers; these payloads are slightly larger.
    let all = saved["nodes"].as_array_mut().unwrap();
    for index in 0..9 {
        let observed = format!("TEST finding {index}: {}", "f".repeat(300));
        all.push(json!({"id":format!("fact-{index}"), "kind":"fact", "status":"settled",
            "prerequisites":[], "answer":observed, "authority":"evidence", "authority_source":"TEST read-only investigation",
            "claim":{"type":"observation", "scope":"TEST README.md:1"},
            "evidence":[{"checked":"TEST README.md:1", "at":"2026-10-21T00:00:00Z", "observed":observed}],
            "history":[], "revision":0}));
    }
    for index in 0..4 {
        all.push(json!({"id":format!("answer-{index}"), "kind":"decision", "status":"settled",
            "prerequisites":[], "owner":"User", "gate":"TEST settled choice", "label":format!("TEST choice {index}"),
            "question":"TEST question: ".to_owned() + &"q".repeat(300),
            "recommendation":"TEST suggestion: ".to_owned() + &"r".repeat(400),
            "answer":"TEST answer: ".to_owned() + &"a".repeat(700), "authority":actor.authority(),
            "authority_source":actor.reason("answered with these words in Tinkery Shape."),
            "evidence":[receipt], "supported_by":["fact-0"], "history":[], "revision":0}));
    }
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let ledger = answering::read(&session).unwrap();
    let think = vec![json!({"question":"TEST Think question", "answer":"t".repeat(250)}); 2];
    let request = input(&ledger, think.clone());
    let mut duplicated = request.clone();
    duplicated["ledger"] = serde_json::from_str(&ledger.context()).unwrap();
    let before_size = serde_json::to_vec(&duplicated).unwrap().len();
    let after_size = serde_json::to_vec(&request).unwrap().len();
    eprintln!("TEST session-sized step: duplicated={before_size}, send-once={after_size}");
    assert!(
        before_size >= 37451,
        "fixture must cover the measured dogfood size"
    );
    assert!(
        after_size <= pi::LIMIT,
        "{after_size} exceeds the unchanged request limit"
    );
    assert_eq!(request["goal"], goal);
    assert_eq!(request["answered_in_think"], json!(think));
    let decisions = request["ledger"]["nodes"].as_array().unwrap();
    assert_eq!(decisions.len(), 5);
    assert!(decisions.iter().all(|n| n["kind"] == "decision"));
    let mut original = origin;
    for field in ["history", "revision", "premise_version", "reviewed_version"] {
        original.as_object_mut().unwrap().remove(field);
    }
    assert_eq!(decisions[0], original, "goal provenance must stay verbatim");
    assert!(
        decisions[0]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["observed"] == context)
    );
    assert!(decisions[1..].iter().all(|n| n["evidence"] == json!([])));
    let findings = request["found_so_far"].as_array().unwrap();
    assert_eq!(
        findings.len(),
        10,
        "nine facts and one deduplicated attribution receipt"
    );
    for index in 0..9 {
        assert_eq!(
            findings
                .iter()
                .filter(|n| n["id"] == format!("fact-{index}"))
                .count(),
            1
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(
        ledger.context(),
        answering::read(&session).unwrap().context()
    );
}

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
        run(&work, root.path(), &agent, &AtomicBool::new(false), &saved),
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
        seed: "TEST draft".into(),
    };
    let saved = AtomicBool::new(false);
    assert_eq!(
        run(&work, root.path(), &agent, &AtomicBool::new(false), &saved),
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
