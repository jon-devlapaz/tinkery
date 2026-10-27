//! Synthetic saved sessions exercise attribution and refusal without operator files or providers.

use super::*;
use crate::think_record::{Origin, Session, Turn, write};
use std::time::{Duration, UNIX_EPOCH};

const GOAL: &str = "I TEST a calm morning.\nWhy: I need space.\nDone when: I can think.";
const WORDS: &str = "  Café 👩‍💻\n\"quoted\"\ttab  ";

fn session(root: &Path, branch: &str) -> PathBuf {
    let path = root.join("TEST-sessions").join(format!("TEST-id-{branch}"));
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
        Event::Suggested {
            id: "q".into(),
            answer: "TEST proposal".into(),
            reason: "TEST assumption".into(),
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
        Event::DraftSuggested {
            text: "  TEST deterministic proposal\r\n".into(),
            provenance: "TEST run / PR / head; external acceptance unverified".into(),
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
fn history_uses_ledger_ids_in_one_root_and_refuses_ambiguous_live_sessions() {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-history-id-")
        .tempdir()
        .unwrap();
    let selected = session(dir.path(), "TEST-a");
    let other = session(dir.path(), "TEST-b");
    let root = dir.path().join("TEST-records");
    let selected_record = record(&root, &selected, 10);
    record(&root, &other, 20);
    let moved = selected
        .parent()
        .unwrap()
        .join("TEST-different-folder-name");
    fs::rename(&selected, &moved).unwrap();
    assert!(
        read(&root, &moved, GOAL)
            .unwrap()
            .details
            .contains(selected_record.to_str().unwrap())
    );
    amend(&other.join("ledger.json"), |v| {
        v["session_id"] = "TEST-id-TEST-a".into()
    });
    let error = read(&root, &moved, GOAL).unwrap_err();
    assert!(
        error.message.contains("More than one saved session"),
        "{error:?}"
    );
    assert_eq!(
        read_origins(&root, &[(moved.as_path(), GOAL), (other.as_path(), GOAL)]).unwrap(),
        vec![MadeIn::Unknown, MadeIn::Unknown]
    );
}

#[test]
fn default_root_move_preserves_history_bytes_and_matches_ledger_identity() {
    for path_only in [false, true] {
        let temporary = tempfile::Builder::new()
            .prefix("tinkery-TEST-history-move-")
            .tempdir()
            .unwrap();
        let home = temporary.path().canonicalize().unwrap();
        let old = home.join(".local/share/seed-me/sessions");
        let new = home.join(".local/share/tinkery/sessions");
        let selected = old.join("TEST-moved-id");
        fs::create_dir_all(&selected).unwrap();
        fs::write(
            selected.join("ledger.json"),
            serde_json::json!({
                "schema_version":1, "session_id":"TEST-moved-id", "status":"active", "goal":GOAL
            })
            .to_string(),
        )
        .unwrap();
        let records = home.join(".local/share/tinkery/think");
        let record_path = record(&records, &selected, 10);
        if path_only {
            amend(&record_path, |v| {
                v.as_object_mut().unwrap().remove("seed_me_session_id");
            });
        }
        let original = fs::read(&record_path).unwrap();
        let original_ledger = fs::read(selected.join("ledger.json")).unwrap();
        assert!(
            crate::seed_me::migration::migrate(Some(home.as_os_str()))
                .unwrap()
                .is_some()
        );
        let moved = new.join("TEST-moved-id");
        let history = read(&records, &moved, GOAL).unwrap();
        assert_eq!(history.turns[0].event, turns()[0].event);
        assert_eq!(
            fs::read(moved.join("ledger.json")).unwrap(),
            original_ledger
        );
        assert_eq!(
            fs::read(&record_path).unwrap(),
            original,
            "move must never rewrite Think records"
        );
        assert_eq!(
            read_origins(&records, &[(moved.as_path(), GOAL)]).unwrap(),
            vec![MadeIn::NoRepo]
        );
        assert_eq!(fs::read(record_path).unwrap(), original);
        assert!(!old.exists());
        assert!(
            crate::seed_me::migration::migrate(Some(home.as_os_str()))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn missing_path_fallback_compares_ledger_id_not_the_live_folder_name() {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-history-path-")
        .tempdir()
        .unwrap();
    let selected = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &selected, 10);
    amend(&path, |v| {
        v.as_object_mut().unwrap().remove("seed_me_session_id");
        v["seed_me_session_path"] = dir
            .path()
            .join("TEST-absent/TEST-not-the-id")
            .to_str()
            .unwrap()
            .into();
        v["seed_me_session"] = "TEST-not-the-id".into();
    });
    let moved = selected.parent().unwrap().join("TEST-not-the-id");
    fs::rename(selected, &moved).unwrap();
    assert!(
        read(&root, &moved, GOAL)
            .unwrap_err()
            .message
            .contains("No Think history")
    );
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
    assert!(error.message.contains("No Think history"), "{error:?}");
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
fn batch_origins_match_individual_strict_history_attribution() {
    for case in [
        "valid",
        "schema2",
        "schema1-match",
        "schema1-unrelated",
        "duplicate",
        "wrong-id",
        "wrong-goal",
        "missing-record",
        "missing-root",
        "root-is-file",
        "missing-session",
        "missing-id",
        "changed-goal",
        "unrelated-missing-session",
        "malformed",
        "unsupported",
        "missing-repo",
        "relative-repo",
        "bad-reference",
        "missing-path",
        "wrong-folder",
        "bad-turn",
        "oversized",
        "json-directory",
        "stray-text",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let selected = session(dir.path(), "TEST-a");
        let other = session(dir.path(), "TEST-b");
        let outside = session(dir.path(), "TEST-outside");
        let root = dir.path().join("TEST-records");
        let path = record(&root, &selected, 10);
        let other_record = record(&root, &other, 20);
        record(&root, &outside, 30);
        amend(&path, |v| v["repo"] = "/TEST/repo-a".into());
        amend(&other_record, |v| v["repo"] = "/TEST/repo-b".into());
        match case {
            "schema2" => amend(&path, |v| {
                v["schema"] = 2.into();
                v.as_object_mut().unwrap().remove("repo");
            }),
            "schema1-match" => amend(&path, |v| {
                v["schema"] = 1.into();
                v.as_object_mut().unwrap().remove("seed_me_session_path");
            }),
            "schema1-unrelated" => {
                let old = root.join("TEST-old.json");
                fs::copy(&path, &old).unwrap();
                amend(&old, |v| {
                    v["schema"] = 1.into();
                    v["seed_me_session"] = "TEST-unrelated".into();
                });
            }
            "duplicate" => {
                fs::copy(&path, root.join("TEST-duplicate.json")).unwrap();
            }
            "wrong-id" => amend(&path, |v| v["seed_me_session_id"] = "TEST-replaced".into()),
            "wrong-goal" => amend(&path, |v| v["goal"] = "TEST different".into()),
            "missing-record" => fs::remove_file(&path).unwrap(),
            "missing-root" => {
                fs::rename(&root, dir.path().join("TEST-moved-records")).unwrap();
            }
            "root-is-file" => {
                fs::rename(&root, dir.path().join("TEST-moved-records")).unwrap();
                fs::write(&root, "TEST not a directory").unwrap();
            }
            "missing-session" => fs::remove_file(selected.join("ledger.json")).unwrap(),
            "missing-id" => amend(&selected.join("ledger.json"), |v| {
                v.as_object_mut().unwrap().remove("session_id");
            }),
            "changed-goal" => amend(&selected.join("ledger.json"), |v| {
                v["goal"] = "TEST changed".into()
            }),
            "unrelated-missing-session" => {
                let unrelated = root.join("TEST-unrelated.json");
                fs::copy(&path, &unrelated).unwrap();
                amend(&unrelated, |v| {
                    v["seed_me_session_path"] = dir
                        .path()
                        .join("TEST-absent")
                        .to_string_lossy()
                        .into_owned()
                        .into();
                    v["seed_me_session"] = "TEST-absent".into();
                });
            }
            "malformed" => {
                fs::write(root.join("TEST-broken.json"), "not JSON").unwrap();
            }
            "unsupported" => {
                fs::write(root.join("TEST-future.json"), "{\"schema\":99}").unwrap();
            }
            "missing-repo" => amend(&path, |v| {
                v.as_object_mut().unwrap().remove("repo");
            }),
            "relative-repo" => amend(&path, |v| v["repo"] = "TEST-relative".into()),
            "bad-reference" => amend(&path, |v| {
                v["seed_me_session_path"] = "TEST-relative".into()
            }),
            "missing-path" => amend(&path, |v| {
                v.as_object_mut().unwrap().remove("seed_me_session_path");
            }),
            "wrong-folder" => amend(&path, |v| v["seed_me_session"] = "TEST-wrong-folder".into()),
            "bad-turn" => amend(&path, |v| v["turns"][0]["at"] = "".into()),
            "oversized" => File::create(&path)
                .unwrap()
                .set_len(MAX_RECORD_BYTES + 1)
                .unwrap(),
            "json-directory" => fs::create_dir(root.join("TEST-directory.json")).unwrap(),
            "stray-text" => {
                fs::write(root.join("TEST-ignore.txt"), "not JSON").unwrap();
            }
            "valid" => {}
            _ => unreachable!(),
        }
        let sessions = [
            (selected.as_path(), GOAL),
            (other.as_path(), GOAL),
            (outside.as_path(), GOAL),
        ];
        let individual: Vec<_> = sessions
            .iter()
            .map(|(path, goal)| read(&root, path, goal).map_or(MadeIn::Unknown, |h| h.made_in))
            .collect();
        let batch = read_origins(&root, &sessions)
            .unwrap_or_else(|_| vec![MadeIn::Unknown; sessions.len()]);
        assert_eq!(batch, individual, "TEST case {case}");
        if case == "valid" {
            assert_eq!(
                batch,
                vec![
                    MadeIn::Repo("/TEST/repo-a".into()),
                    MadeIn::Repo("/TEST/repo-b".into()),
                    MadeIn::NoRepo
                ]
            );
        }
        if case == "duplicate" {
            assert_eq!(batch[0], MadeIn::Unknown);
            assert_eq!(batch[1], MadeIn::Repo("/TEST/repo-b".into()));
        }
    }
}

#[cfg(unix)]
#[test]
fn batch_and_history_refuse_noncanonical_sessions_and_symlink_records() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let selected = session(dir.path(), "TEST-a");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &selected, 10);
    amend(&path, |v| v["repo"] = "/TEST/repo-a".into());
    let original = fs::read(&path).unwrap();
    let alias = dir.path().join("TEST-alias");
    symlink(&selected, &alias).unwrap();
    amend(&path, |v| {
        v["seed_me_session_path"] = alias.to_string_lossy().into_owned().into();
        v["seed_me_session"] = "TEST-alias".into();
    });
    assert!(read(&root, &selected, GOAL).is_err());
    assert!(read_origins(&root, &[(selected.as_path(), GOAL)]).is_err());
    fs::write(&path, original).unwrap();
    symlink(&path, root.join("TEST-link.json")).unwrap();
    assert!(read(&root, &selected, GOAL).is_err());
    assert!(read_origins(&root, &[(selected.as_path(), GOAL)]).is_err());
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

#[test]
fn optional_reading_metadata_cannot_invalidate_independent_originals_or_attribution() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-context");
    let root = dir.path().join("TEST-records");
    let path = record(&root, &session, 10);
    amend(&path, |v| {
        v.as_object_mut().unwrap().remove("reading_context");
        v.as_object_mut().unwrap().remove("tinkery_source");
    });
    let history = read(&root, &session, GOAL).unwrap();
    assert!(history.details.contains("Reading context: not recorded"));
    assert!(history.details.contains("Tinkery source: unknown"));
    amend(&path, |v| {
        v["reading_context"] = serde_json::json!({"events":[{"kind":"submitted","source":999}]});
        v["tinkery_source"] = serde_json::json!({"commit":"invalid","dirty":false});
    });
    let history = read(&root, &session, GOAL).unwrap();
    assert_eq!(history.turns[0].event, turns()[0].event);
    assert_eq!(history.goal, GOAL);
    assert!(history.details.contains("Reading context: unavailable"));
    assert!(history.details.contains("Tinkery source: unavailable"));
    assert_eq!(
        read_origins(&root, &[(session.as_path(), GOAL)]).unwrap(),
        vec![MadeIn::NoRepo]
    );
}

#[test]
fn oversized_confirmed_record_is_refused_before_creating_an_unreadable_file() {
    let dir = tempfile::tempdir().unwrap();
    let session = session(dir.path(), "TEST-large-context");
    let root = dir.path().join("TEST-records");
    let mut context = crate::think_record::ReadingContext::default();
    context.submitted(1);
    context.presented(
        serde_json::json!({
            "framings":[{"text":"T".repeat(MAX_RECORD_BYTES as usize)}],"other_goals":[],"deferred":[]
        }),
        UNIX_EPOCH,
    );
    context.submitted(2);
    context.submitted(4);
    let original = fs::read(session.join("ledger.json")).unwrap();
    let result = crate::think_record::write_with_context(
        &root,
        &Session {
            origin: &Origin::simulated(),
            turns: &turns(),
            checks: vec![],
            goal: GOAL,
            seed_me_session: &session,
            repo: None,
            confirmed: UNIX_EPOCH,
        },
        &GoalConfirmation::default(),
        Some(&context),
    );
    assert!(result.unwrap_err().contains("16 MiB"));
    assert!(!root.exists(), "writer published an unreadable record");
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), original);
}
