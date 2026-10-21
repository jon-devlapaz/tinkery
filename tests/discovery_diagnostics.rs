//! #197: the advertised Shape F1 route must expose real read failures without running anything.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    fs, thread,
    time::{Duration, Instant},
};
use tinkery::{
    menu::{Home, snapshot},
    think::brain_dump::BrainDump,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn fixture() -> (tempfile::TempDir, Home) {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-197-")
        .tempdir()
        .unwrap();
    let broken = dir.path().join("TEST-broken");
    fs::create_dir(&broken).unwrap();
    fs::write(broken.join("ledger.json"), "not JSON").unwrap();
    let mut home = Home::new(
        Box::new(BrainDump::default),
        Ok(dir.path().to_path_buf()),
        Err("TEST links disabled".into()),
    );
    home.handle_key(key(KeyCode::Char('3')));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        home.tick();
        let text = snapshot(100, 30, &mut home, true).unwrap();
        if text.contains("couldn't be read") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "discovery did not settle: {text}"
        );
        thread::sleep(Duration::from_millis(1));
    }
    (dir, home)
}

#[test]
fn scan_counts_every_failure_but_retains_only_eight_path_reason_samples() {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-197-cap-")
        .tempdir()
        .unwrap();
    for index in 0..11 {
        let session = dir.path().join(format!("TEST-broken-{index}"));
        fs::create_dir(&session).unwrap();
        fs::write(session.join("ledger.json"), "not JSON").unwrap();
    }
    fs::write(dir.path().join("TEST-stray.txt"), "not a session").unwrap();
    let found = tinkery::tend::scan(dir.path()).unwrap();
    assert_eq!(found.unreadable, 11);
    assert!(!found.complete());
    assert!(found.sessions.is_empty());
    assert_eq!(found.failures.len(), 8);
    for failure in found.failures {
        assert!(failure.path.ends_with("ledger.json"));
        assert!(failure.reason.starts_with("Invalid JSON:"));
        assert!(failure.reason.contains("line 1"));
    }
}

#[test]
fn read_failure_reasons_preserve_session_classification_and_seed_unknowns() {
    use tinkery::seed_me::{Record, Stage, read};
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(read(dir.path()), Record::NotASession));
    let ledger = dir.path().join("ledger.json");
    fs::write(&ledger, "{}").unwrap();
    let Record::Unreadable(failure) = read(dir.path()) else {
        panic!("missing status must refuse");
    };
    assert_eq!(failure.path, ledger);
    assert!(failure.reason.contains("status"));
    fs::write(&ledger, r#"{"status":"active","goal":"TEST goal"}"#).unwrap();
    fs::create_dir(dir.path().join("seed-contract.md")).unwrap();
    let Record::Session(session) = read(dir.path()) else {
        panic!("seed read failure must not hide the session");
    };
    assert_eq!(session.stage, Stage::Unknown);
}

#[test]
fn notice_names_only_the_help_route_that_has_diagnostics() {
    let (_dir, mut home) = fixture();
    let text = snapshot(100, 30, &mut home, true).unwrap();
    assert!(text.contains("F1 shows why."), "{text}");
    assert!(!text.contains("or Tend shows why"), "{text}");
}

#[test]
fn shape_f1_shows_the_actual_failed_ledger_and_parse_reason() {
    let (dir, mut home) = fixture();
    home.handle_key(key(KeyCode::F(1)));
    let mut seen = String::new();
    for _ in 0..12 {
        seen.push_str(&snapshot(100, 30, &mut home, true).unwrap());
        home.handle_key(key(KeyCode::PageDown));
    }
    let compact: String = seen.chars().filter(|c| !c.is_whitespace()).collect();
    let path = dir.path().join("TEST-broken/ledger.json");
    assert!(compact.contains(path.to_str().unwrap()), "{seen}");
    assert!(seen.contains("Invalid JSON"), "{seen}");
    assert_eq!(fs::read(path).unwrap(), b"not JSON");
    assert!(!dir.path().join("TEST-records").exists());
}
