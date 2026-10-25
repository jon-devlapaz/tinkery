//! Typed findings and review actions through the real session boundary, with no provider.

use super::*;
use std::fs;

fn fact(id: &str, kind: &str) -> Found {
    Found {
        id: id.into(),
        claim: json!({"type":kind,"scope":"TEST README"}),
        checked: "README.md:1".into(),
        observed: format!("TEST {kind}"),
        supported_by: None,
        contradicts: None,
    }
}

fn question(found: Vec<Found>, supported_by: Option<Value>) -> Ask {
    Ask {
        id: "choice".into(),
        label: "TEST choice".into(),
        question: "TEST choose?".into(),
        gate: "TEST gate".into(),
        found,
        resolves: None,
        supported_by,
        prerequisites: None,
        suggestion: None,
        assumed: vec![],
        options: vec![],
        against: None,
        confidence: None,
    }
}

#[test]
fn typed_facts_ground_questions_and_inferences_without_settling_a_decision() {
    let root = tempfile::Builder::new()
        .prefix("TEST-typed-shape-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    fs::write(root.path().join("README.md"), "TEST local\n").unwrap();
    let observation = fact("read", "observation");
    let mut inference = fact("inferred", "inference");
    inference.claim["limits"] = "TEST not runtime proof".into();
    inference.supported_by = Some(json!(["read"]));
    let unknown = fact("gap", "unknown");
    let ask = question(
        vec![observation, inference, unknown],
        Some(json!(["read", "inferred"])),
    );
    ask_step(&session, &ask, root.path());
    let state = read(&session).unwrap().state;
    let all = nodes(&state);
    for node in &all[1..4] {
        assert_eq!(node["kind"], "fact");
        assert_eq!(node["authority"], "evidence");
        assert_eq!(node["evidence"].as_array().unwrap().len(), 1);
    }
    assert_eq!(all[2]["supported_by"], json!(["read"]));
    assert_eq!(all[4]["supported_by"], json!(["read", "inferred"]));
    assert_eq!(all[4]["status"], "unresolved");
    assert!(all[4].get("answer").is_none());
    let text = today(&receipts(&state, true));
    for expected in [
        "observation; scope:",
        "inference; scope:",
        "limits: TEST not runtime proof",
        "unknown; scope:",
    ] {
        assert!(text.contains(expected), "{text}");
    }
}

fn ask_step(session: &Path, question: &Ask, folder: &Path) {
    let version = read(session).unwrap().version();
    ask(session, version, question, folder).unwrap();
}

#[test]
fn ledger_refuses_bad_facts_and_support_without_saving_any_part_of_a_reply() {
    let root = tempfile::Builder::new()
        .prefix("TEST-fact-refusal-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let mut unsupported = fact("inferred", "inference");
    unsupported.claim["limits"] = "TEST bounded".into();
    let mut no_limits = fact("inferred", "inference");
    no_limits.supported_by = Some(json!(["read"]));
    let mut bad_target = fact("read", "observation");
    bad_target.contradicts = Some(json!("missing"));
    let null_support: Found = serde_json::from_value(json!({"id":"read",
        "claim":{"type":"observation","scope":"TEST"},"checked":"README.md:1",
        "observed":"TEST read","supported_by":null}))
    .unwrap();
    let mut unknown_support = fact("inferred", "inference");
    unknown_support.claim["limits"] = "TEST bounded".into();
    unknown_support.supported_by = Some(json!(["gap"]));
    for (question, reason) in [
        (question(vec![null_support], None), "invalid supported_by"),
        (
            question(vec![fact("gap", "unknown"), unknown_support], None),
            "not a decision or an unknown",
        ),
        (
            question(vec![fact("read", "observation"), unsupported], None),
            "must cite the observations",
        ),
        (
            question(vec![fact("read", "observation"), no_limits], None),
            "must state its limits",
        ),
        (
            question(vec![fact("gap", "unknown")], Some(json!(["gap"]))),
            "not a decision or an unknown",
        ),
        (
            question(vec![bad_target], None),
            "contradicts must name another node",
        ),
    ] {
        let before = fs::read(session.join("ledger.json")).unwrap();
        let error = ask(&session, 1, &question, root.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains(reason), "{error}");
        assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
        let checks = json!([{"statement":"TEST","kind":"judge","source":"TEST","how_to_judge":"TEST inspect"}]);
        if question.supported_by.is_none() {
            assert!(
                draft_seed(
                    &session,
                    1,
                    &ledger::fixtures::prose_reply(include_str!(
                        "../../../tests/fixtures/shape-seed-reply.txt"
                    )),
                    &checks,
                    &question.found,
                    root.path(),
                    None,
                )
                .is_err()
            );
            assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
            assert!(!session.join("seed-checks.json").exists());
            assert!(!session.join("seed-contract.md").exists());
        }
    }
    let mut state = read(&session).unwrap().state;
    facts(&mut state, &[fact("read", "observation")], root.path());
    state["nodes"][0]["contradicts"] = "read".into();
    let error = publish(&session, 1, "TEST invalid decision contradiction", &state)
        .unwrap_err()
        .to_string();
    assert!(error.contains("only allowed on a fact"), "{error}");
}

#[test]
fn contradicting_a_fact_flags_it_and_its_settled_support_dependents() {
    let root = tempfile::Builder::new()
        .prefix("TEST-review-chain-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let mut inferred = fact("inferred", "inference");
    inferred.claim["limits"] = "TEST no runtime proof".into();
    inferred.supported_by = Some(json!(["read"]));
    ask_step(
        &session,
        &question(
            vec![fact("read", "observation"), inferred],
            Some(json!(["inferred"])),
        ),
        root.path(),
    );
    let version = read(&session).unwrap().version();
    let version = answer(&session, version, "choice", "TEST unchanged choice", None).unwrap();
    let mut conflict = fact("conflict", "observation");
    conflict.contradicts = Some(json!("read"));
    ask(
        &session,
        version,
        &question(vec![conflict], None),
        root.path(),
    )
    .unwrap();
    for expected in ["read", "inferred", "choice"] {
        let saved = ledger::load(&session).unwrap();
        let flags = ledger::review_flags(&saved["nodes"]);
        assert!(flags.get(expected).is_some(), "{}", flags.pretty().unwrap());
        let shaping = super::super::shaping::read(&session).unwrap();
        assert!(
            matches!(shaping.stage, super::super::shaping::Shaped::Review { ref id, revisable, .. } if id == expected && revisable == (expected == "choice"))
        );
        keep(
            &session,
            read(&session).unwrap().version(),
            expected,
            "TEST checked the conflict",
            None,
        )
        .unwrap();
    }
    assert!(
        matches!(super::super::shaping::read( &session).unwrap().stage,
        super::super::shaping::Shaped::Question { ref id, .. } if id == "choice-2")
    );
}

#[test]
fn a_contradicted_session_cannot_confirm_until_the_operator_keeps_it_with_a_reason() {
    let root = tempfile::Builder::new()
        .prefix("TEST-contradicted-confirm-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let mut conflict = fact("conflict", "observation");
    conflict.contradicts = Some(json!("goal"));
    let checks =
        json!([{"statement":"TEST","kind":"judge","source":"TEST","how_to_judge":"TEST inspect"}]);
    let revision = draft_seed(
        &session,
        1,
        &ledger::fixtures::prose_reply(include_str!(
            "../../../tests/fixtures/shape-seed-reply.txt"
        )),
        &checks,
        &[conflict],
        root.path(),
        None,
    )
    .unwrap();
    let before = fs::read(session.join("ledger.json")).unwrap();
    let error = confirm_seed(
        &session,
        crate::seed_me::answering::read(&session).unwrap().version(),
        &revision,
        &std::fs::read_to_string(session.join("seed-contract.md")).unwrap(),
        "TEST confirm",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("nodes flagged for review remain: goal"),
        "{error}"
    );
    assert!(!session.join("seed-confirmation.json").exists());
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    let version = read(&session).unwrap().version();
    assert!(keep(&session, version, "goal", " ", None).is_err());
    assert!(keep(&session, version - 1, "goal", "TEST reason", None).is_err());
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    keep(
        &session,
        version,
        "goal",
        "TEST retain the goal despite this finding",
        None,
    )
    .unwrap();
    let saved = ledger::load(&session).unwrap();
    assert_eq!(
        saved["nodes"].array().unwrap()[0]["answer"].string(),
        Some("TEST goal")
    );
    assert!(
        saved["nodes"].array().unwrap()[0]["history"]
            .pretty()
            .unwrap()
            .contains("TEST retain the goal")
    );
    assert!(
        ledger::review_flags(&saved["nodes"])
            .object()
            .unwrap()
            .is_empty()
    );
    confirm_seed(
        &session,
        crate::seed_me::answering::read(&session).unwrap().version(),
        &revision,
        &std::fs::read_to_string(session.join("seed-contract.md")).unwrap(),
        "TEST confirm",
        None,
    )
    .unwrap();
    end(&session, Ending::Completed, None).unwrap();
}
