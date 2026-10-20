//! Explicit answer actions through the real ledger, without a helper or provider call.

use super::*;
use ledger::AnswerAction;
use std::fs;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("TEST-answer-actions-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let skill = root.path().join("TEST-no-helper");
    let mut state = read(&skill, &session).unwrap().state;
    state["nodes"].as_array_mut().unwrap().extend([
        json!({"id":"read","kind":"fact","status":"settled","prerequisites":[],
            "claim":{"type":"observation","scope":"TEST file"},"answer":"TEST local",
            "authority":"evidence","authority_source":"TEST fixture","evidence":["TEST file says local"]}),
        json!({"id":"choice","kind":"decision","status":"unresolved","prerequisites":[],
            "owner":"User","gate":"TEST storage gate","question":"TEST local or synced?",
            "recommendation":"TEST local","supported_by":["read"],"evidence":[]}),
    ]);
    state["current_question"] = "choice".into();
    publish(&session, 1, "TEST question", &state).unwrap();
    (root, session, skill)
}

fn respond_now(session: &Path, skill: &Path, action: &AnswerAction) -> Result<u64, Refusal> {
    respond(
        session,
        read(skill, session).unwrap().version(),
        "choice",
        action,
    )
}

#[test]
fn deferral_records_both_fields_and_never_confirms() {
    let (_root, session, skill) = fixture();
    let reason = "TEST wait\r\nfor the operator ";
    let revisit = "TEST before implementation\nwhen scope is known ";
    respond_now(
        &session,
        &skill,
        &AnswerAction::Defer {
            reason: reason.into(),
            revisit: revisit.into(),
        },
    )
    .unwrap();
    let state = read(&skill, &session).unwrap().state;
    let node = &state["nodes"][2];
    assert_eq!(node["status"], "deferred");
    assert_eq!(node["defer_reason"], reason);
    assert_eq!(node["revisit_condition"], revisit);
    assert!(node["answer"].is_null());
    assert!(state["current_question"].is_null());
    assert!(!session.join("seed-confirmation.json").exists());
    let rendered = ledger::decisions_block(&ledger::load(&session).unwrap()).unwrap();
    assert!(
        rendered.contains("deferred") && rendered.contains("TEST before implementation"),
        "{rendered}"
    );
}

#[test]
fn deferral_without_revisit_is_refused_without_writing() {
    let (_root, session, skill) = fixture();
    for (reason, revisit) in [("TEST wait", ""), ("", "TEST later")] {
        let before = fs::read(session.join("ledger.json")).unwrap();
        let error = respond_now(
            &session,
            &skill,
            &AnswerAction::Defer {
                reason: reason.into(),
                revisit: revisit.into(),
            },
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("reason and revisit condition"),
            "{error}"
        );
        assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    }
}

#[test]
fn delegation_records_the_suggestion_facts_and_operator_key() {
    let (_root, session, skill) = fixture();
    respond_now(&session, &skill, &AnswerAction::Delegate).unwrap();
    let state = read(&skill, &session).unwrap().state;
    let node = &state["nodes"][2];
    assert_eq!(node["answer"], "TEST local");
    assert_eq!(node["authority"], "delegated");
    assert_eq!(node["evidence"], state["nodes"][1]["evidence"]);
    assert_eq!(
        today(&receipts(&state, true))
            .matches("TEST file says local")
            .count(),
        1
    );
    assert_eq!(read(&skill, &session).unwrap().findings().len(), 1);
    assert!(
        node["authority_source"]
            .as_str()
            .unwrap()
            .contains("'F12 delegate suggestion'")
    );
    let rendered = ledger::decisions_block(&ledger::load(&session).unwrap()).unwrap();
    assert!(rendered.contains("[delegated] TEST local"), "{rendered}");
}

#[test]
fn delegation_leaves_confidence_in_the_suggestion_not_the_answer() {
    for (recommendation, expected) in [
        ("TEST local. Confidence: low.", "TEST local."),
        ("TEST local. Confidence: medium.", "TEST local."),
        ("TEST local. Confidence: high.", "TEST local."),
        (
            "TEST literal Confidence: medium. stays inside.",
            "TEST literal Confidence: medium. stays inside.",
        ),
        (
            "TEST local. Confidence: unknown.",
            "TEST local. Confidence: unknown.",
        ),
    ] {
        let (_root, session, skill) = fixture();
        let mut saved = read(&skill, &session).unwrap();
        saved.state["nodes"][2]["recommendation"] = recommendation.into();
        publish(
            &session,
            saved.version,
            "TEST confidence tail",
            &saved.state,
        )
        .unwrap();
        respond_now(&session, &skill, &AnswerAction::Delegate).unwrap();
        let state = read(&skill, &session).unwrap().state;
        assert_eq!(state["nodes"][2]["answer"], expected);
        assert_eq!(state["nodes"][2]["recommendation"], recommendation);
        assert_eq!(state["nodes"][2]["authority"], "delegated");
        assert_eq!(state["nodes"][2]["evidence"], state["nodes"][1]["evidence"]);
        let decisions = ledger::decisions_block(&ledger::load(&session).unwrap()).unwrap();
        assert!(
            decisions.contains(&format!("[delegated] {expected} (source:")),
            "{decisions}"
        );
    }
}

#[test]
fn delegation_without_facts_or_suggestion_is_refused_without_writing() {
    for missing in ["supported_by", "recommendation"] {
        let (_root, session, skill) = fixture();
        let mut read = read(&skill, &session).unwrap();
        read.state["nodes"][2]
            .as_object_mut()
            .unwrap()
            .remove(missing);
        publish(&session, read.version, "TEST ungrounded", &read.state).unwrap();
        let before = fs::read(session.join("ledger.json")).unwrap();
        let error = respond_now(&session, &skill, &AnswerAction::Delegate).unwrap_err();
        assert!(
            error.to_string().contains(if missing == "supported_by" {
                "no supporting facts"
            } else {
                "no suggestion"
            }),
            "{error}"
        );
        assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    }
}

#[test]
fn revision_reopens_all_settled_dependents_with_the_changed_answer_named() {
    let (_root, session, skill) = fixture();
    respond_now(&session, &skill, &AnswerAction::Delegate).unwrap();
    let mut saved = read(&skill, &session).unwrap();
    for (id, parent) in [("child", "choice"), ("grandchild", "child")] {
        saved.state["nodes"].as_array_mut().unwrap().push(json!({
            "id":id,"kind":"decision","status":"settled","prerequisites":[parent],"evidence":[],
            "owner":"User","gate":"TEST dependent","answer":"TEST old dependent",
            "authority":"user","authority_source":"TEST fixture"
        }));
    }
    publish(
        &session,
        saved.version,
        "TEST dependent chain",
        &saved.state,
    )
    .unwrap();
    let words = "TEST synced\r\nonly with permission ";
    respond_now(&session, &skill, &AnswerAction::Revise(words.into())).unwrap();
    let state = read(&skill, &session).unwrap().state;
    assert_eq!(state["nodes"][2]["answer"], words);
    assert_eq!(state["nodes"][2]["authority"], "user");
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(session.join("ledger.json")).unwrap())
            .unwrap();
    let history = stored["nodes"][2]["history"].as_array().unwrap();
    assert_eq!(
        history.last().unwrap()["reason"],
        "The operator revised choice in Tinkery Shape."
    );
    for node in &state["nodes"].as_array().unwrap()[3..] {
        assert_eq!(node["status"], "unresolved", "{node}");
        assert!(node["answer"].is_null() && node["authority"].is_null());
        assert!(node["reopen_reason"].as_str().unwrap().contains("choice"));
        assert!(node["reopen_reason"].as_str().unwrap().contains(words));
    }
    assert_eq!(state["current_question"], "child");
    let version = read(&skill, &session).unwrap().version();
    let version = answer(&skill, &session, version, "child", "TEST reanswered").unwrap();
    assert!(promote(&skill, &session, version).unwrap().is_some());
    assert_eq!(
        read(&skill, &session).unwrap().state["current_question"],
        "grandchild"
    );
}

#[test]
fn revision_resolves_its_own_flag_without_revalidating_findings() {
    for target in ["choice", "read"] {
        let (_root, session, skill) = fixture();
        respond_now(&session, &skill, &AnswerAction::Delegate).unwrap();
        let mut saved = read(&skill, &session).unwrap();
        saved.state["nodes"].as_array_mut().unwrap().push(json!({
            "id":"conflict","kind":"fact","status":"settled","prerequisites":[],
            "claim":{"type":"observation","scope":"TEST conflict"},"answer":"TEST conflicting fact",
            "authority":"evidence","authority_source":"TEST fixture","evidence":["TEST conflict"],"contradicts":target,
        }));
        publish(&session, saved.version, "TEST flag", &saved.state).unwrap();
        assert!(
            ledger::review_flags(&ledger::load(&session).unwrap()["nodes"])
                .get("choice")
                .is_some()
        );
        respond_now(
            &session,
            &skill,
            &AnswerAction::Revise("TEST replacement".into()),
        )
        .unwrap();
        let saved = ledger::load(&session).unwrap();
        let flags = ledger::review_flags(&saved["nodes"]);
        assert!(flags.get("choice").is_none(), "{}", flags.pretty().unwrap());
        assert_eq!(flags.get("read").is_some(), target == "read");
        assert_eq!(
            read(&skill, &session).unwrap().state["nodes"][1]["answer"],
            "TEST local"
        );
    }
}

#[test]
fn answer_action_read_and_lock_failures_use_plain_recovery_words() {
    let (root, session, _skill) = fixture();
    for (path, expected) in [
        (root.path().join("TEST-missing"), "cannot be locked"),
        (session.clone(), "could not be read"),
    ] {
        if path == session {
            fs::write(path.join("ledger.json"), "{ TEST broken JSON").unwrap();
        }
        let error = respond(&path, 1, "choice", &AnswerAction::Delegate)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
        for technical in ["JSON", "line 1", "os error", "No such file", "Expecting"] {
            assert!(!error.contains(technical), "{error}");
        }
        assert!(
            error.contains("try again") || error.contains("trying again"),
            "{error}"
        );
    }
}

#[test]
fn revision_refuses_findings_goal_blank_words_and_stale_display() {
    let (_root, session, skill) = fixture();
    respond_now(&session, &skill, &AnswerAction::Delegate).unwrap();
    let saved = read(&skill, &session).unwrap();
    let before = fs::read(session.join("ledger.json")).unwrap();
    for id in ["goal", "read"] {
        assert!(
            respond(
                &session,
                saved.version,
                id,
                &AnswerAction::Revise("TEST replacement".into())
            )
            .is_err()
        );
    }
    assert!(respond_now(&session, &skill, &AnswerAction::Revise(" ".into())).is_err());
    assert_eq!(
        respond(
            &session,
            saved.version - 1,
            "choice",
            &AnswerAction::Revise("TEST new".into())
        )
        .unwrap_err(),
        Refusal::Stale
    );
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
}
