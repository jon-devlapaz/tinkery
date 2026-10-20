//! Prerequisite transport, atomic refusal and ledger-owned ready ordering, without a provider.

use super::*;
use std::fs;

fn proposal(id: &str, prerequisites: Option<Value>) -> Ask {
    Ask {
        id: id.into(),
        label: format!("TEST {id}"),
        question: format!("TEST choose {id}?"),
        gate: "TEST choice".into(),
        found: vec![],
        supported_by: None,
        prerequisites,
        suggestion: None,
        assumed: vec![],
        options: vec![],
        against: None,
        confidence: None,
    }
}

#[test]
fn question_prerequisites_are_preserved_and_settled_parents_allow_current() {
    let root = tempfile::Builder::new()
        .prefix("TEST-prerequisites-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let question = proposal("b", Some(json!(["goal"])));
    ask(&skill, &session, 1, &question, root.path()).unwrap();
    let saved = read(&skill, &session).unwrap();
    assert_eq!(saved.state["nodes"][1]["prerequisites"], json!(["goal"]));
    assert_eq!(saved.state["current_question"], "b");
}

#[test]
fn held_question_waits_for_ready_decisions_in_ledger_order_then_promotes() {
    let root = tempfile::Builder::new()
        .prefix("TEST-held-question-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let version = ask(&skill, &session, 1, &proposal("a", None), root.path()).unwrap();
    // No current question, as after a saved answer; A must be selected from the frontier.
    let mut state = read(&skill, &session).unwrap().state;
    state["current_question"] = Value::Null;
    let version = publish(&session, version, "TEST leave A open", &state).unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &proposal("b", Some(json!(["a"]))),
        root.path(),
    )
    .unwrap();
    let saved = read(&skill, &session).unwrap();
    assert_eq!(saved.state["current_question"], "a");
    assert_eq!(saved.state["nodes"][2]["prerequisites"], json!(["a"]));
    let shown = super::super::shaping::read(&skill, &session).unwrap();
    assert_eq!(shown.open, ["TEST a", "TEST b — waits for: TEST a"]);
    let version = answer(&skill, &session, version, "a", "TEST A answered").unwrap();
    promote(&skill, &session, version).unwrap().unwrap();
    assert_eq!(
        read(&skill, &session).unwrap().state["current_question"],
        "b"
    );
    assert_eq!(
        super::super::shaping::read(&skill, &session).unwrap().open,
        ["TEST b"]
    );
}

#[test]
fn held_questions_with_deferred_parents_save_without_becoming_current() {
    let root = tempfile::Builder::new()
        .prefix("TEST-deferred-parent-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let version = ask(&skill, &session, 1, &proposal("a", None), root.path()).unwrap();
    let version = respond(
        &session,
        version,
        "a",
        &ledger::AnswerAction::Defer {
            reason: "TEST wait".into(),
            revisit: "TEST revisit".into(),
        },
    )
    .unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &proposal("b", Some(json!(["a"]))),
        root.path(),
    )
    .unwrap();
    assert!(!read(&skill, &session).unwrap().has_current_question());
    assert_eq!(promote(&skill, &session, version).unwrap(), None);
    assert_eq!(
        super::super::shaping::read(&skill, &session).unwrap().open,
        ["TEST b — waits for: TEST a"]
    );
}

#[test]
fn prerequisite_refusals_save_no_question_findings_or_assumptions() {
    let root = tempfile::Builder::new()
        .prefix("TEST-refused-prerequisites-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    fs::write(root.path().join("README.md"), "TEST fact\n").unwrap();
    for (prerequisites, expected) in [
        (json!(["missing"]), "isn't recorded"),
        (json!(["b"]), "circle"),
        (json!(["read"]), "not findings"),
        (Value::Null, "list of decision IDs"),
        (json!("goal"), "list of decision IDs"),
        (json!([7]), "list of decision IDs"),
        (json!(["goal", "goal"]), "once"),
    ] {
        let mut question = proposal("b", Some(prerequisites));
        question.found.push(Found {
            id: "read".into(),
            claim: json!({"type":"observation","scope":"TEST"}),
            checked: "README.md:1".into(),
            observed: "TEST fact".into(),
            supported_by: None,
            contradicts: None,
        });
        question.assumed.push(Assumption {
            text: "TEST default".into(),
            why: "TEST equivalent".into(),
        });
        let before = fs::read(session.join("ledger.json")).unwrap();
        let error = ask(&skill, &session, 1, &question, root.path())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(expected) && error.to_lowercase().contains("retry the step"),
            "{error}"
        );
        for forbidden in [
            "dependency cycle",
            "missing prerequisite",
            "invalid prerequisites",
            "Error(",
            "serde",
            "io::",
        ] {
            assert!(!error.contains(forbidden), "{error}");
        }
        assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    }
}

#[test]
fn current_question_is_not_replaced_and_multiple_prerequisites_keep_their_order() {
    let root = tempfile::Builder::new()
        .prefix("TEST-current-prerequisites-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let version = ask(&skill, &session, 1, &proposal("a", None), root.path()).unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &proposal("b", Some(json!(["goal", "a"]))),
        root.path(),
    )
    .unwrap();
    let saved = read(&skill, &session).unwrap();
    assert_eq!(saved.state["current_question"], "a");
    assert_eq!(saved.state["nodes"][1]["prerequisites"], json!([]));
    assert_eq!(
        saved.state["nodes"][2]["prerequisites"],
        json!(["goal", "a"])
    );
    assert_eq!(promote(&skill, &session, version).unwrap(), None);
    assert_eq!(
        super::super::shaping::read(&skill, &session).unwrap().open,
        ["TEST a", "TEST b — waits for: TEST a"]
    );
}

#[test]
fn ready_selection_skips_inactive_decisions_and_held_questions() {
    let root = tempfile::Builder::new()
        .prefix("TEST-ready-frontier-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let version = ask(&skill, &session, 1, &proposal("a", None), root.path()).unwrap();
    let mut state = read(&skill, &session).unwrap().state;
    state["nodes"][1]["predicate"] = Value::Null;
    state["current_question"] = Value::Null;
    let version = publish(&session, version, "TEST unknown predicate", &state).unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &proposal("b", Some(json!(["a"]))),
        root.path(),
    )
    .unwrap();
    assert_eq!(promote(&skill, &session, version).unwrap(), None);
    ask(&skill, &session, version, &proposal("c", None), root.path()).unwrap();
    assert_eq!(
        read(&skill, &session).unwrap().state["current_question"],
        "c"
    );
}

#[test]
fn revising_an_answer_reopens_its_model_proposed_dependent() {
    let root = tempfile::Builder::new()
        .prefix("TEST-revise-prerequisite-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let version = ask(&skill, &session, 1, &proposal("a", None), root.path()).unwrap();
    let version = answer(&skill, &session, version, "a", "TEST old A").unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &proposal("b", Some(json!(["a"]))),
        root.path(),
    )
    .unwrap();
    let version = answer(&skill, &session, version, "b", "TEST old B").unwrap();
    let version = respond(
        &session,
        version,
        "a",
        &ledger::AnswerAction::Revise("TEST new A".into()),
    )
    .unwrap();
    let saved = read(&skill, &session).unwrap();
    assert_eq!(saved.state["nodes"][2]["status"], "unresolved");
    assert!(saved.state["nodes"][2]["answer"].is_null());
    assert_eq!(saved.state["nodes"][2]["prerequisites"], json!(["a"]));
    assert_eq!(saved.state["current_question"], "b");
    assert_eq!(promote(&skill, &session, version).unwrap(), None);
}
