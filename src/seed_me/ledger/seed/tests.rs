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
