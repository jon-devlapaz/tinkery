//! Confirmation and ending fail closed on changed bytes, changed decisions and absent consent.

use super::*;
use crate::seed_me::ledger::{self, fixtures};
use std::os::unix::fs::PermissionsExt;

const STAMP: &str = "2026-10-19T00:00:00+00:00";
fn fixture() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("TEST-seed-protection-")
        .tempdir()
        .unwrap();
    fixtures::confirmed(root.path(), "TEST original goal", "r1");
    root
}

fn draft() -> tempfile::TempDir {
    let root = fixture();
    let path = root.path().join(FILE);
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace(CONFIRMED, DRAFT)
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("Confirmed by:"))
        .collect::<String>();
    fs::write(path, text).unwrap();
    fs::remove_file(root.path().join(CONFIRMATION)).unwrap();
    root
}

fn terminal_agent() -> crate::think_record::GoalConfirmation {
    serde_json::from_value(serde_json::json!({
        "by": "terminal", "declaration": "declared: agent",
        "action": "typed confirm at this terminal", "identity": "declaration is not verified identity"
    }))
    .unwrap()
}

fn saved_files(directory: &Path) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn confirmation_keeps_source_whitespace_for_people_and_agents() {
    let agent = terminal_agent();
    for actor in [None, Some(&agent)] {
        for source in ["  TEST source café  ", "\u{2003}\tTEST source 中\u{a0}"] {
            let root = draft();
            ledger::confirm_as(root.path(), "r1", source, STAMP, actor).unwrap();
            let receipt =
                Value::parse(&fs::read_to_string(root.path().join(CONFIRMATION)).unwrap()).unwrap();
            assert_eq!(receipt["source"].string(), Some(source));
            let (schema, operator, attribution) = if actor.is_some() {
                (
                    3,
                    "agent",
                    "; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity",
                )
            } else {
                (2, "human", "")
            };
            assert_eq!(receipt["schema_version"], int(schema));
            assert!(receipt["operator"].is(operator));
            let note = format!(
                "Confirmed by: \"{source}{attribution}\" (revision r1, 2026-10-19). Not approved for implementation."
            );
            assert!(
                fs::read_to_string(root.path().join(FILE))
                    .unwrap()
                    .split('\n')
                    .any(|line| line == note)
            );
            let before = saved_files(root.path());
            assert!(ledger::status(root.path()).unwrap()["handoff"]["verified"].truth());
            assert_eq!(
                saved_files(root.path()),
                before,
                "verification rewrote a receipt"
            );
        }
    }
}

#[test]
fn confirmation_still_refuses_blank_and_multiline_sources_without_writes() {
    let agent = terminal_agent();
    for actor in [None, Some(&agent)] {
        for source in ["", " \t\u{2003}\u{a0}", "TEST\nwords", "TEST\rwords"] {
            let root = draft();
            let before = saved_files(root.path());
            let error = ledger::confirm_as(root.path(), "r1", source, STAMP, actor).unwrap_err();
            assert!(
                error.contains(if source.contains(['\n', '\r']) {
                    "--source must be one line"
                } else {
                    "--source is required"
                }),
                "{error}"
            );
            assert_eq!(saved_files(root.path()), before);
        }
    }
}

#[test]
fn historical_trimmed_confirmation_notes_and_receipts_still_verify_unchanged() {
    let agent = terminal_agent();
    for actor in [None, Some(&agent)] {
        let root = draft();
        ledger::confirm_as(root.path(), "r1", "TEST confirmation only", STAMP, actor).unwrap();
        let mut receipt =
            Value::parse(&fs::read_to_string(root.path().join(CONFIRMATION)).unwrap()).unwrap();
        // Freeze the old source/note pair explicitly. Historical writers discarded
        // surrounding whitespace before storing both fields; reads cannot recover it.
        let legacy_note = if actor.is_some() {
            "Confirmed by: \"TEST confirmation only; caller-declared declared: agent; typed confirm at this terminal; declaration is not verified identity\" (revision r1, 2026-10-19). Not approved for implementation."
        } else {
            "Confirmed by: \"TEST confirmation only\" (revision r1, 2026-10-19). Not approved for implementation."
        };
        let seed = fs::read_to_string(root.path().join(FILE)).unwrap();
        let legacy_seed = seed
            .split_inclusive('\n')
            .map(|line| {
                if line.starts_with("Confirmed by:") {
                    format!("{legacy_note}\n")
                } else {
                    line.into()
                }
            })
            .collect::<String>();
        receipt.set("source", "TEST confirmation only".into());
        receipt.set(
            "seed_sha256",
            sha256(legacy_seed.as_bytes()).as_str().into(),
        );
        fs::write(root.path().join(FILE), legacy_seed).unwrap();
        fs::write(root.path().join(CONFIRMATION), receipt.pretty().unwrap()).unwrap();
        let before = saved_files(root.path());
        assert!(ledger::status(root.path()).unwrap()["handoff"]["verified"].truth());
        assert_eq!(saved_files(root.path()), before);
    }
}
fn assert_blocked(directory: &Path, expected: &str) {
    let before = fs::read(directory.join("ledger.json")).unwrap();
    let result = verify_handoff(directory, &load(directory).unwrap()).unwrap();
    assert_eq!(result["verified"], Value::Bool(false));
    assert!(
        result["reason"].string().unwrap().contains(expected),
        "{result:?}"
    );
    assert!(ledger::end(directory, "completed", "TEST complete", STAMP).is_err());
    assert_eq!(fs::read(directory.join("ledger.json")).unwrap(), before);
}
#[test]
fn agent_sessions_require_verified_confirmation_to_complete() {
    let root = fixture();
    let directory = root.path();
    let mut saved = load(directory).unwrap();
    saved.set("operator", "agent".into());
    fs::write(directory.join("ledger.json"), saved.pretty().unwrap()).unwrap();
    let seed_path = directory.join(FILE);
    let draft = fs::read_to_string(&seed_path)
        .unwrap()
        .replace(CONFIRMED, DRAFT)
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("Confirmed by:"))
        .collect::<String>();
    fs::write(&seed_path, draft).unwrap();
    assert_blocked(directory, "not confirmed");
    let actor = serde_json::from_value(serde_json::json!({
        "by":"agent", "agent":"TEST-agent", "authorizing_operator":"TEST-person",
        "source":"TEST scoped seed permission", "class":{"door":"two_way",
        "door_reason":"TEST reversible", "blast_radius":"small", "blast_radius_reason":"TEST isolated"}
    })).unwrap();
    ledger::confirm_as(directory, "r1", "TEST seed only", STAMP, Some(&actor)).unwrap();
    assert!(verify_handoff(directory, &load(directory).unwrap()).unwrap()["verified"].truth());
    let confirmed = fs::read(&seed_path).unwrap();
    fs::write(&seed_path, [confirmed.as_slice(), b" "].concat()).unwrap();
    assert_blocked(directory, "seed changed");
    fs::write(seed_path, confirmed).unwrap();
    ledger::end(directory, "completed", "TEST complete", STAMP).unwrap();
    let saved = load(directory).unwrap();
    assert!(saved["operator"].is("agent"));
    assert!(saved["status"].is("completed"));
}

#[test]
fn changed_seed_bytes_cannot_verify_or_complete() {
    let root = fixture();
    let path = root.path().join(FILE);
    let seed = fs::read_to_string(&path).unwrap();
    fs::write(path, seed + " ").unwrap();
    assert_blocked(root.path(), "seed changed");
}
#[test]
fn changed_decisions_cannot_verify_or_complete() {
    let root = fixture();
    let path = root.path().join("ledger.json");
    let mut ledger = load(root.path()).unwrap();
    ledger["draft"].set("outcome", "TEST different decision context".into());
    fs::write(path, ledger.pretty().unwrap()).unwrap();
    assert_blocked(root.path(), "decisions changed");
}
#[test]
fn ending_requires_confirmation_record() {
    let root = fixture();
    fs::remove_file(root.path().join(CONFIRMATION)).unwrap();
    let before = fs::read(root.path().join("ledger.json")).unwrap();
    let result = ledger::end(root.path(), "completed", "TEST complete", STAMP);
    assert!(result.is_err(), "unconfirmed completion was accepted");
    assert!(
        result
            .unwrap_err()
            .reason
            .contains("no confirmation record")
    );
    assert_eq!(fs::read(root.path().join("ledger.json")).unwrap(), before);
}
#[test]
fn confirming_stale_decisions_refuses_without_writes() {
    let root = fixture();
    let path = root.path().join(FILE);
    let draft = fs::read_to_string(&path)
        .unwrap()
        .replace(CONFIRMED, DRAFT)
        .split_inclusive('\n')
        .filter(|s| !s.starts_with("Confirmed by:"))
        .collect::<String>();
    fs::write(&path, &draft).unwrap();
    let mut ledger = load(root.path()).unwrap();
    ledger.set(
        "assumed",
        Value::parse(r#"[{"text":"TEST new default","why":"TEST"}]"#).unwrap(),
    );
    fs::write(root.path().join("ledger.json"), ledger.pretty().unwrap()).unwrap();
    let receipt = fs::read(root.path().join(CONFIRMATION)).unwrap();
    let result = ledger::confirm_seed(root.path(), "r1", "TEST yes", STAMP);
    assert!(result.unwrap_err().contains("decisions do not match"));
    assert_eq!(fs::read_to_string(path).unwrap(), draft);
    assert_eq!(fs::read(root.path().join(CONFIRMATION)).unwrap(), receipt);
}
#[test]
fn status_ignores_viewer_records_and_corrupt_snapshots_without_mutation() {
    let root = fixture();
    fs::write(
        root.path().join("viewer.json"),
        r#"{"url":"http://127.0.0.1:1","pid":1}"#,
    )
    .unwrap();
    fs::write(root.path().join("ledger-view.html"), b"\xff").unwrap();
    let before: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let result = status(root.path()).unwrap();
    assert_eq!(result["handoff"]["verified"], Value::Bool(true));
    for key in ["viewer", "snapshot", "snapshot_current"] {
        assert!(result.get(key).is_none());
    }
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn confirming_preserves_seed_mode_and_refuses_a_busy_writer() {
    use std::os::fd::AsRawFd;
    let root = fixture();
    let lock = fs::File::open(root.path().join(".writer.lock")).unwrap();
    // SAFETY: `lock` owns the descriptor for the lifetime of both refused calls.
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    assert_eq!(
        ledger::confirm_seed(root.path(), "r1", "TEST yes", STAMP).unwrap_err(),
        "session already has a writer"
    );
    assert_eq!(
        ledger::set_checks(root.path(), &checks::read_checks(root.path()).unwrap()).unwrap_err(),
        "session already has a writer"
    );
    assert_eq!(
        ledger::end(root.path(), "completed", "TEST complete", STAMP)
            .unwrap_err()
            .reason,
        "session already has a writer"
    );
    drop(lock);
    let path = root.path().join(FILE);
    let draft = fs::read_to_string(&path)
        .unwrap()
        .replace(CONFIRMED, DRAFT)
        .split_inclusive('\n')
        .filter(|s| !s.starts_with("Confirmed by:"))
        .collect::<String>();
    fs::write(&path, draft).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    ledger::confirm_seed(root.path(), "r1", "TEST yes", STAMP).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}
