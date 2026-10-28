//! Explicit legacy-session recovery preserves the deferred original and dependency ordering.
use super::*;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("TEST-deferred-recovery-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let mut state = read(&session).unwrap().state;
    state["nodes"].as_array_mut().unwrap().extend([
        json!({"id":"a","kind":"decision","status":"deferred","prerequisites":[],"evidence":[],
            "owner":"User","gate":"TEST A","label":"TEST A","question":"TEST exact question café?",
            "defer_reason":"  TEST reason café  ","revisit_condition":"TEST condition 中"}),
        json!({"id":"b","kind":"decision","status":"unresolved","prerequisites":["a"],"evidence":[],
            "owner":"User","gate":"TEST B","label":"TEST B","question":"TEST B?"}),
    ]);
    publish(&session, 1, "TEST legacy saved state", &state).unwrap();
    (root, session)
}

fn files(session: &Path) -> BTreeMap<std::ffi::OsString, Vec<u8>> {
    fs::read_dir(session)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.file_name(), fs::read(e.path()).unwrap())
        })
        .collect()
}

fn refuse(session: &Path, expected: u64, id: &str, words: &str) {
    let before = files(session);
    assert!(
        respond(
            session,
            expected,
            id,
            &ledger::AnswerAction::Revisit(words.into()),
            None
        )
        .is_err()
    );
    assert_eq!(files(session), before);
}

#[test]
fn deferred_recovery_preserves_originals_and_asks_the_unanswered_dependent() {
    let (_root, session) = fixture();
    let before = read(&session).unwrap();
    let shown = super::super::shaping::read(&session).unwrap();
    assert!(matches!(
        shown.stage,
        super::super::shaping::Shaped::Waiting
    ));
    let target = shown.deferred.unwrap();
    assert_eq!(target.id, "a");
    assert_eq!(target.question, "TEST exact question café?");
    assert_eq!(target.reason, "  TEST reason café  ");
    assert_eq!(target.revisit, "TEST condition 中");
    let words = "  TEST exact answer café\n中  ";
    respond(
        &session,
        before.version(),
        "a",
        &ledger::AnswerAction::Revisit(words.into()),
        None,
    )
    .unwrap();
    let after = read(&session).unwrap();
    assert_eq!(after.goal(), before.goal());
    assert_eq!(after.state["nodes"][2], before.state["nodes"][2]);
    assert_eq!(after.state["current_question"], "b");
    assert_eq!(after.state["nodes"][1]["answer"], words);
    assert_eq!(after.state["nodes"][1]["authority"], "user");
    let full = ledger::load(&session).unwrap().to_json().unwrap();
    let prior = full["nodes"][1]["history"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(prior["state"]["status"], "deferred");
    for field in ["question", "defer_reason", "revisit_condition"] {
        assert_eq!(prior["state"][field], before.state["nodes"][1][field]);
    }
    assert!(
        super::super::shaping::read(&session)
            .unwrap()
            .deferred
            .is_none()
    );
}

#[test]
fn deferred_recovery_refuses_wrong_goal_stale_empty_seed_and_ended_targets() {
    let (_root, session) = fixture();
    let version = read(&session).unwrap().version();
    for (expected, id, words) in [
        (version, "b", "TEST"),
        (version, "goal", "TEST"),
        (version - 1, "a", "TEST"),
        (version, "a", " \n "),
    ] {
        refuse(&session, expected, id, words);
    }
    fs::write(session.join("seed-contract.md"), "status: draft\n").unwrap();
    refuse(&session, version, "a", "TEST");
    fs::remove_file(session.join("seed-contract.md")).unwrap();
    end(&session, Ending::Stopped, None).unwrap();
    refuse(&session, read(&session).unwrap().version(), "a", "TEST");
}

#[test]
fn deferred_recovery_uses_existing_order_and_never_bypasses_own_readiness() {
    for blocked in ["multiple", "prerequisite", "predicate", "current"] {
        let (_root, session) = fixture();
        let mut state = read(&session).unwrap().state;
        if blocked == "predicate" {
            state["nodes"][1]["predicate"] = Value::Null;
        } else {
            let mut c = state["nodes"][1].clone();
            c["id"] = "c".into();
            if blocked == "multiple" {
                state["nodes"][2]["prerequisites"] = json!(["c", "a"]);
            } else {
                c["status"] = "unresolved".into();
                if blocked == "prerequisite" {
                    c["predicate"] = Value::Null;
                    state["nodes"][1]["prerequisites"] = json!(["c"]);
                } else {
                    state["current_question"] = "c".into();
                }
            }
            state["nodes"].as_array_mut().unwrap().push(c);
        }
        let version = publish(
            &session,
            read(&session).unwrap().version(),
            "TEST eligibility",
            &state,
        )
        .unwrap();
        if blocked == "multiple" {
            refuse(&session, version, "c", "TEST");
            respond(
                &session,
                version,
                "a",
                &ledger::AnswerAction::Revisit("TEST A".into()),
                None,
            )
            .unwrap();
            let shown = super::super::shaping::read(&session).unwrap();
            assert_eq!(shown.deferred.unwrap().id, "c");
            assert_eq!(
                read(&session).unwrap().state["nodes"][2]["status"],
                "unresolved"
            );
            assert!(
                promote(&session, read(&session).unwrap().version())
                    .unwrap()
                    .is_none()
            );
        } else {
            assert!(
                super::super::shaping::read(&session)
                    .unwrap()
                    .deferred
                    .is_none()
            );
            refuse(&session, version, "a", "TEST");
        }
    }
}

#[test]
fn deferred_recovery_keeps_existing_agent_authorization_and_writer_lock() {
    use std::os::fd::AsRawFd;
    let (_root, session) = fixture();
    let actor: crate::think_record::GoalConfirmation = serde_json::from_value(json!({
        "by":"agent","agent":"TEST-agent","authorizing_operator":"", "source":"TEST",
        "class":{"door":"two_way","door_reason":"TEST","blast_radius":"small","blast_radius_reason":"TEST"}
    })).unwrap();
    let version = read(&session).unwrap().version();
    let before = files(&session);
    assert!(
        respond(
            &session,
            version,
            "a",
            &ledger::AnswerAction::Revisit("TEST".into()),
            Some(&actor)
        )
        .is_err()
    );
    assert_eq!(files(&session), before);
    let lock = fs::File::open(session.join(".writer.lock")).unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    refuse(&session, version, "a", "TEST");
}
