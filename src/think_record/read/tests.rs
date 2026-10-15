//! Synthetic saved sessions exercise attribution and refusal without operator files or providers.

use super::*;
use crate::think_record::{Origin, Session, Turn, write};
use std::time::{Duration, UNIX_EPOCH};

const GOAL: &str = "I TEST a calm morning.\nWhy: I need space.\nDone when: I can think.";
const WORDS: &str = "  Café 👩‍💻\n\"quoted\"\ttab  ";

fn session(root: &Path, branch: &str) -> PathBuf {
    let path = root.join(branch).join("TEST-sessions/same-name");
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("ledger.json"),
        serde_json::json!({"schema_version": 1, "session_id": format!("TEST-id-{branch}"), "status": "active", "goal": GOAL}).to_string(),
    )
    .unwrap();
    path.canonicalize().unwrap()
}

fn turns() -> Vec<Turn> {
    [
        Event::Words {
            source: 1,
            text: WORDS.into(),
        },
        Event::Asked {
            id: "q".into(),
            part: Some(crate::goal::Part::Why),
            question: "Why?".into(),
        },
        Event::Answered {
            id: "q".into(),
            question: "Why?".into(),
            source: 2,
            answer: "  TEST because.\n".into(),
        },
        Event::Skipped {
            id: "q2".into(),
            part: None,
            question: "When?".into(),
        },
        Event::Restored {
            id: "q2".into(),
            part: None,
            question: "When?".into(),
        },
        Event::ScopeAsked {
            id: "scope".into(),
            question: "Same goal?".into(),
            source: 3,
        },
        Event::ScopeAnswered {
            id: "scope".into(),
            question: "Same goal?".into(),
            source: 4,
            answer: "yes".into(),
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(n, event)| Turn {
        at: UNIX_EPOCH + Duration::from_secs(n as u64 + 1),
        event,
    })
    .collect()
}

fn record(root: &Path, session: &Path, seconds: u64) -> PathBuf {
    write(
        root,
        &Session {
            origin: &Origin::simulated(),
            turns: &turns(),
            checks: vec![serde_json::json!({"check": "TEST", "passed": true})],
            goal: GOAL,
            seed_me_session: session,
            repo: None,
            confirmed: UNIX_EPOCH + Duration::from_secs(seconds),
        },
    )
    .unwrap()
}

fn amend(path: &Path, change: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    change(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn reads_all_turns_verbatim_and_changes_no_saved_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    let original_record = fs::read(&path).unwrap();
    let original_ledger = fs::read(session.join("ledger.json")).unwrap();
    let history = read(&root, &session, GOAL).unwrap();
    assert_eq!(history.goal, GOAL);
    assert_eq!(
        history
            .turns
            .iter()
            .map(|turn| &turn.event)
            .collect::<Vec<_>>(),
        turns().iter().map(|turn| &turn.event).collect::<Vec<_>>()
    );
    assert_eq!(history.turns[0].at, "1970-01-01T00:00:01.000Z");
    assert!(history.details.contains(path.to_str().unwrap()));
    assert!(history.details.contains("Origin: simulated"));
    assert!(history.details.contains("\"check\": \"TEST\""));
    assert_eq!(fs::read(&path).unwrap(), original_record);
    assert_eq!(
        fs::read(session.join("ledger.json")).unwrap(),
        original_ledger
    );
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn same_folder_names_and_identical_goals_in_other_roots_never_match() {
    let dir = tempfile::tempdir().unwrap();
    let selected = session(dir.path(), "TEST-a");
    let other = session(dir.path(), "TEST-b");
    let root = dir.path().join("TEST-records");
    let other_record = record(&root, &other, 10);
    let missing = read(&root, &selected, GOAL).unwrap_err();
    assert!(missing.message.contains("No Think history"));
    let selected_record = record(&root, &selected, 20);
    let history = read(&root, &selected, GOAL).unwrap();
    assert!(history.details.contains(selected_record.to_str().unwrap()));
    assert!(!history.details.contains(other_record.to_str().unwrap()));
}

#[test]
fn duplicate_records_refuse_even_with_identical_contents() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    fs::copy(path, root.join("TEST-copy.json")).unwrap();
    let error = read(&root, &session, GOAL).unwrap_err();
    assert!(error.message.contains("More than one"), "{error:?}");
}

#[test]
fn missing_history_does_not_create_a_root() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-missing");
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("No Think history")
    );
    assert!(!root.exists());
}

#[test]
fn malformed_or_unsupported_files_cannot_hide_another_match() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    record(&root, &session, 10);
    let unknown = root.join("TEST-unknown.json");
    for (content, explanation) in [
        ("{", "could not be read"),
        ("{}", "could not be read"),
        ("{\"schema\":99}", "unsupported format"),
    ] {
        fs::write(&unknown, content).unwrap();
        let error = read(&root, &session, GOAL).unwrap_err();
        assert!(error.message.contains(explanation), "{error:?}");
    }
}

#[test]
fn old_matching_records_remain_unverifiable_without_migration() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    amend(&path, |value| {
        value["schema"] = 1.into();
        value
            .as_object_mut()
            .unwrap()
            .remove("seed_me_session_path");
    });
    let bytes = fs::read(&path).unwrap();
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("older Think history")
    );
    record(&root, &session, 20);
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("older Think history")
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn rejects_wrong_goal_and_incomplete_or_unknown_turns() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    let original = fs::read(&path).unwrap();
    amend(&path, |value| value["goal"] = "TEST changed".into());
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("disagrees")
    );
    for field in ["at", "source", "text"] {
        fs::write(&path, &original).unwrap();
        amend(&path, |value| {
            value["turns"][0].as_object_mut().unwrap().remove(field);
        });
        assert!(
            read(&root, &session, GOAL)
                .unwrap_err()
                .message
                .contains("could not be read"),
            "missing {field}"
        );
    }
    fs::write(&path, &original).unwrap();
    amend(&path, |value| {
        value["turns"][0]["kind"] = "unknown_event".into()
    });
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("could not be read")
    );
}

#[test]
fn stale_goal_or_removed_session_never_returns_history() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    record(&root, &session, 10);
    let ledger = session.join("ledger.json");
    fs::write(&ledger, "{\"status\":\"active\",\"goal\":\"TEST changed\"}").unwrap();
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("saved goal changed")
    );
    fs::remove_file(ledger).unwrap();
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .message
            .contains("session could not be read")
    );
}

#[test]
fn same_path_and_same_goal_with_a_replaced_session_identity_refuses_old_history() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let record_path = record(&root, &session, 10);
    let original_record = fs::read(&record_path).unwrap();
    amend(&session.join("ledger.json"), |value| {
        value["session_id"] = "TEST-replacement-id".into();
    });
    let error = read(&root, &session, GOAL).unwrap_err();
    assert!(
        error.message.contains("different saved session"),
        "{error:?}"
    );
    assert_eq!(fs::read(record_path).unwrap(), original_record);
}

#[test]
fn missing_session_identity_is_never_inferred_when_reading_or_writing() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    record(&root, &session, 10);
    amend(&session.join("ledger.json"), |value| {
        value.as_object_mut().unwrap().remove("session_id");
    });
    let error = read(&root, &session, GOAL).unwrap_err();
    assert!(
        error.message.contains("identity could not be checked"),
        "{error:?}"
    );
    let new_root = dir.path().join("TEST-new-records");
    let error = write(
        &new_root,
        &Session {
            origin: &Origin::simulated(),
            turns: &turns(),
            checks: vec![],
            goal: GOAL,
            seed_me_session: &session,
            repo: None,
            confirmed: UNIX_EPOCH,
        },
    )
    .unwrap_err();
    assert!(error.contains("no identity"), "{error}");
    assert!(!new_root.exists());
}

#[test]
fn invalid_session_reference_and_oversized_record_are_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    amend(&path, |value| {
        value["seed_me_session_path"] = "relative/TEST".into()
    });
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .details
            .contains("reference is invalid")
    );
    File::create(&path)
        .unwrap()
        .set_len(MAX_RECORD_BYTES + 1)
        .unwrap();
    assert!(
        read(&root, &session, GOAL)
            .unwrap_err()
            .details
            .contains("16 MiB")
    );
}

#[test]
fn unavailable_session_refuses_record_write_without_creating_record_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-records");
    let missing = dir.path().join("TEST-missing-session");
    let error = write(
        &root,
        &Session {
            origin: &Origin::simulated(),
            turns: &turns(),
            checks: vec![],
            goal: GOAL,
            seed_me_session: &missing,
            repo: None,
            confirmed: UNIX_EPOCH,
        },
    )
    .unwrap_err();
    assert!(error.contains("TEST-missing-session"));
    assert!(!root.exists());
}

#[test]
fn where_a_goal_was_made_reads_back_and_older_records_say_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    assert_eq!(read(&root, &session, GOAL).unwrap().made_in, MadeIn::NoRepo);
    amend(&path, |value| value["repo"] = "/TEST/repo-root".into());
    let history = read(&root, &session, GOAL).unwrap();
    assert_eq!(history.made_in, MadeIn::Repo("/TEST/repo-root".into()));
    assert!(history.details.contains("Repository: /TEST/repo-root"));
    amend(&path, |value| {
        value["schema"] = 2.into();
        value.as_object_mut().unwrap().remove("repo");
    });
    let history = read(&root, &session, GOAL).unwrap();
    assert_eq!(
        history.made_in,
        MadeIn::Unknown,
        "schema 2 is still readable"
    );
    assert!(history.details.contains("Repository: not recorded"));
}

#[test]
fn a_current_record_without_a_usable_repository_field_is_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    for change in [
        |v: &mut serde_json::Value| {
            v.as_object_mut().unwrap().remove("repo");
        },
        |v: &mut serde_json::Value| v["repo"] = "TEST-relative".into(),
        |v: &mut serde_json::Value| v["repo"] = 7.into(),
    ] {
        let original = fs::read(&path).unwrap();
        amend(&path, change);
        let error = read(&root, &session, GOAL).unwrap_err();
        assert!(error.details.contains("recorded repository"), "{error:?}");
        fs::write(&path, original).unwrap();
    }
}
