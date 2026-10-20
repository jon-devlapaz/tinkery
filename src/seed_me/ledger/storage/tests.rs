//! Persistence fault outcomes and Python/Rust writer exclusion in TEST sessions.

use super::*;
use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

const STAMP: &str = "2026-10-19T00:00:00+00:00";

fn fixture() -> (tempfile::TempDir, Value, Value) {
    let root = tempfile::Builder::new()
        .prefix("TEST-ledger-storage-")
        .tempdir()
        .unwrap();
    let ledger = Value::parse(r#"{"schema_version":1,"session_id":"TEST-session","version":0,"revision":0,
        "created_at":"2026-10-19T00:00:00+00:00","updated_at":"2026-10-19T00:00:00+00:00",
        "status":"active","draft":{"goal":"","outcome":"","options":[]},"goal":null,
        "origin":null,"current_question":null,"frontier":[],"nodes":[],"assumed":[],"operator":"human"}"#).unwrap();
    atomic_write(root.path(), &ledger.pretty().unwrap()).unwrap();
    let mut state = super::super::editable(&ledger).unwrap();
    state["draft"].set("goal", "TEST changed".into());
    (root, ledger, state)
}

fn helper() -> std::path::PathBuf {
    Path::new(&std::env::var_os("SEED_ME_TEST_SKILL").expect("pinned fixture required"))
        .parent()
        .unwrap()
        .join("scripts/session.py")
}

#[test]
fn python_writer_blocks_rust_and_rust_writer_blocks_python() {
    let (root, ledger, state) = fixture();
    let mut python = Command::new("python3")
        .args([
            "-u",
            "-c",
            r#"
import importlib.util,sys
spec=importlib.util.spec_from_file_location('TEST_session',sys.argv[1])
s=importlib.util.module_from_spec(spec); spec.loader.exec_module(s)
with s.writer(sys.argv[2]):
    print('TEST locked',flush=True)
    sys.stdin.read(1)
"#,
        ])
        .arg(helper())
        .arg(root.path())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(python.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line.trim(), "TEST locked");
    let result = publish(
        root.path(),
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    );
    python.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(python.wait().unwrap().success());
    assert_eq!(result.unwrap_err().reason, "session already has a writer");
    assert_eq!(load(root.path()).unwrap(), ledger);

    let writer = Writer::acquire(root.path()).unwrap();
    let payload = root.path().join("TEST-payload.json");
    let mut value = Value::parse(r#"{"expected_version":0,"reason":"TEST update"}"#).unwrap();
    value.set("state", state.clone());
    fs::write(&payload, value.pretty().unwrap()).unwrap();
    let output = Command::new("python3")
        .arg(helper())
        .arg("publish")
        .arg(root.path())
        .arg(payload)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).lines().next(),
        Some("session already has a writer")
    );
    drop(writer);
    publish(
        root.path(),
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    )
    .unwrap();
    let _released = Writer::acquire(root.path()).unwrap();
}

#[test]
fn draft_saves_share_the_lock_and_refuse_stale_or_replaced_inputs_before_writes() {
    let (root, _, _) = fixture();
    let records = Value::parse(r#"[{"statement":"TEST","kind":"judge","source":"TEST"}]"#).unwrap();
    let draft = "status: draft\nTEST unconfirmed draft\n";
    let writer = Writer::acquire(root.path()).unwrap();
    assert_eq!(
        save_draft(root.path(), 0, None, &records, draft).unwrap_err(),
        "session already has a writer"
    );
    drop(writer);
    assert!(!root.path().join(super::super::checks::FILE).exists());
    assert_eq!(
        save_draft(root.path(), 1, None, &records, draft).unwrap_err(),
        "stale publication version"
    );
    assert!(!root.path().join(super::super::checks::FILE).exists());
    save_draft(root.path(), 0, None, &records, draft).unwrap();
    let checks = fs::read(root.path().join(super::super::checks::FILE)).unwrap();
    assert!(
        save_draft(root.path(), 0, None, &records, draft)
            .unwrap_err()
            .contains("seed changed")
    );
    fs::write(
        root.path().join(super::super::seed::FILE),
        "status: confirmed for intake\nTEST",
    )
    .unwrap();
    assert!(
        save_draft(
            root.path(),
            0,
            Some("status: confirmed for intake\nTEST"),
            &records,
            draft
        )
        .unwrap_err()
        .contains("never replace")
    );
    assert_eq!(
        fs::read(root.path().join(super::super::checks::FILE)).unwrap(),
        checks
    );
}

#[test]
fn load_and_transition_refusals_release_the_writer() {
    let (root, ledger, state) = fixture();
    assert!(
        publish(
            root.path(),
            &state,
            &Value::Int("99".into()),
            &"TEST update".into(),
            None,
            STAMP
        )
        .is_err()
    );
    drop(Writer::acquire(root.path()).unwrap());
    fs::write(root.path().join("ledger.json"), "TEST invalid JSON").unwrap();
    assert!(
        publish(
            root.path(),
            &state,
            &ledger["version"],
            &"TEST update".into(),
            None,
            STAMP
        )
        .is_err()
    );
    drop(Writer::acquire(root.path()).unwrap());
}

#[test]
fn failed_writes_report_not_saved_or_unknown_from_read_back() {
    let (root, ledger, state) = fixture();
    let result = transition(
        &ledger,
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    )
    .unwrap();
    let failed = commit(
        root.path(),
        &ledger,
        result.clone(),
        |_, _| Err(io::Error::from_raw_os_error(libc::EACCES)),
        load,
    )
    .unwrap_err();
    assert_eq!(failed.save_state, SaveState::NotSaved);
    assert_eq!(load(root.path()).unwrap(), ledger);
    for current in [
        Ok(ledger.clone()),
        Ok(state.clone()),
        Err("TEST unreadable".into()),
    ] {
        let unchanged = current.as_ref().is_ok_and(|v| *v == ledger);
        let failed = commit(
            root.path(),
            &ledger,
            result.clone(),
            |_, _| Err(io::Error::from_raw_os_error(libc::EACCES)),
            |_| current,
        )
        .unwrap_err();
        assert_eq!(
            failed.save_state,
            if unchanged {
                SaveState::NotSaved
            } else {
                SaveState::Unknown
            }
        );
        assert_eq!(
            failed.reason,
            format!(
                "could not save the ledger: {}",
                io::Error::from_raw_os_error(libc::EACCES)
            )
        );
    }
    // A rename can have succeeded before an error was reported; trust only the read-back.
    let saved = commit(
        root.path(),
        &ledger,
        result.clone(),
        |_, _| Err(io::Error::other("TEST after rename")),
        |_| Ok(result.clone()),
    )
    .unwrap();
    assert_eq!(saved, result);
}

#[test]
fn successful_write_with_unreadable_read_back_never_claims_saved() {
    let (root, ledger, state) = fixture();
    let result = transition(
        &ledger,
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    )
    .unwrap();
    let failed = commit(
        root.path(),
        &ledger,
        result,
        |directory, result| atomic_write(directory, &result.pretty().unwrap()),
        |_| Err("TEST unreadable after rename".into()),
    )
    .unwrap_err();
    assert_eq!(failed.save_state, SaveState::Unknown);
    assert_eq!(
        failed.reason,
        "could not save the ledger: TEST unreadable after rename"
    );
}

#[test]
fn serialization_refusal_precedes_io_without_a_save_failure_prefix() {
    let (root, _, state) = fixture();
    let path = root.path().join("ledger.json");
    let raw = fs::read_to_string(&path).unwrap().replace(
        "\"created_at\": \"2026-10-19T00:00:00+00:00\"",
        "\"created_at\": NaN",
    );
    fs::write(&path, &raw).unwrap();
    let failed = publish(
        root.path(),
        &state,
        &Value::Int("0".into()),
        &"TEST update".into(),
        None,
        STAMP,
    )
    .unwrap_err();
    assert_eq!(
        failed.reason,
        "Out of range float values are not JSON compliant: nan"
    );
    assert_eq!(failed.save_state, SaveState::NotSaved);
    assert_eq!(fs::read_to_string(path).unwrap(), raw);
    drop(Writer::acquire(root.path()).unwrap());
}

#[test]
fn os_permission_denied_diagnostic_is_rust_not_python_oserror() {
    // A deterministic OS error seam works for root-run CI too; no real permissions are left changed.
    let rust = io::Error::from_raw_os_error(libc::EACCES).to_string();
    let output = Command::new("python3").args(["-c", "import errno; print(str(PermissionError(errno.EACCES, 'Permission denied', 'TEST-ledger.json')))"])
        .output().unwrap();
    let python = String::from_utf8(output.stdout).unwrap();
    assert!(rust.contains("os error 13"), "{rust}");
    assert!(python.starts_with("[Errno 13]"), "{python}");
    assert_ne!(rust, python.trim());
}

#[test]
fn atomic_replace_is_private_and_leaves_no_temporary_file_and_noop_keeps_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let (root, ledger, state) = fixture();
    let path = root.path().join("ledger.json");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    publish(
        root.path(),
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    )
    .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!fs::read_dir(root.path()).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ledger-")
    }));
    let ledger = load(root.path()).unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    publish(
        root.path(),
        &state,
        &ledger["version"],
        &"".into(),
        None,
        STAMP,
    )
    .unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}
