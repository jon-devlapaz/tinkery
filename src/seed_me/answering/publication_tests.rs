//! Shape's real ledger boundary against the pinned writer, without provider calls.

use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command, time::Duration};

const STAMP: &str = "2026-10-19T00:00:00+00:00";

fn temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("TEST-shape-publication-")
        .tempdir()
        .unwrap()
}

#[test]
fn pinned_python_publications_match_through_shape_boundary() {
    let root = temp();
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ledger/publication.py"))
        .arg(root.path())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONHASHSEED", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut count = 0;
    let mut through_shape = 0;
    let mut nonfinite = 0;
    for line in String::from_utf8(output.stdout).unwrap().lines() {
        let answer = ledger::Value::parse(line).unwrap();
        let case = &answer["case"];
        let pair = Path::new(answer["pair"].string().unwrap());
        let rust = pair.join("rust");
        let python = pair.join("python");
        let name = case["name"].string().unwrap();
        if let Some(expected_read) = answer.get("read") {
            let skill = PathBuf::from(std::env::var_os("SEED_ME_TEST_SKILL").unwrap());
            let actual = read(&skill, &rust).unwrap();
            assert_eq!(
                actual.version().to_string(),
                expected_read["expected_version"].pretty().unwrap().trim(),
                "{name}"
            );
            assert_eq!(
                serde_json::from_str::<Value>(&actual.context()).unwrap(),
                serde_json::from_str::<Value>(&expected_read["state"].pretty().unwrap()).unwrap(),
                "{name}"
            );
        }
        if let Some(message) = answer.get("read_message") {
            nonfinite += 1;
            assert_eq!(answer["real_path"], ledger::Value::Bool(true), "{name}");
            let skill = PathBuf::from(std::env::var_os("SEED_ME_TEST_SKILL").unwrap());
            assert_eq!(
                read(&skill, &rust).unwrap_err(),
                Refusal::Refused(format!(
                    "Cannot read this session: {}",
                    message.string().unwrap()
                )),
                "{name}"
            );
        }
        let result = if answer["real_path"] == ledger::Value::Bool(true) {
            through_shape += 1;
            let state = serde_json::from_str(&case["state"].pretty().unwrap()).unwrap();
            let version = case["expected_version"]
                .pretty()
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            // Exactly the production path, with only the clock supplied rather than sampled.
            publish_at(
                &rust,
                version,
                case["reason"].string().unwrap(),
                &state,
                STAMP,
            )
        } else {
            ledger::publish(
                &rust,
                &case["state"],
                &case["expected_version"],
                &case["reason"],
                case.get("revalidated"),
                STAMP,
            )
            .map(|v| v["version"].pretty().unwrap().trim().parse().unwrap())
            .map_err(published)
        };
        match answer["verdict"].string().unwrap() {
            "accept" => assert_eq!(
                result.unwrap_or_else(|e| panic!("{name}: {e}")),
                answer["version"]
                    .pretty()
                    .unwrap()
                    .trim()
                    .parse::<u64>()
                    .unwrap(),
                "{name}"
            ),
            "refuse" => assert_eq!(
                result.unwrap_err(),
                not_saved(answer["message"].string().unwrap().into()),
                "{name}"
            ),
            _ => panic!("unknown verdict"),
        }
        assert_eq!(
            fs::read(rust.join("ledger.json")).unwrap(),
            fs::read(python.join("ledger.json")).unwrap(),
            "{name}"
        );
        assert_eq!(
            fs::metadata(rust.join("ledger.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            fs::metadata(python.join("ledger.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            "{name}"
        );
        assert!(
            !rust.join("ledger-view.html").exists(),
            "Rust must not write the view"
        );
        count += 1;
    }
    assert_eq!(
        nonfinite, 6,
        "all non-finite read and publication cases must run"
    );
    assert!(
        count >= 1069 && through_shape >= 457,
        "incomplete publication corpus: {count}, Shape {through_shape}"
    );
    println!("publication differential: {count} cases, {through_shape} through answering.rs");
}

#[test]
fn publication_clock_matches_python_including_zero_microseconds() {
    let instants = [
        (0, 0),
        (1, 1),
        (951_782_400, 123_456),
        (1_792_368_000, 999_999),
        (4_102_444_800, 0),
    ];
    for (secs, micros) in instants {
        let ours = publication_time(
            UNIX_EPOCH + Duration::from_secs(secs) + Duration::from_micros(micros),
        )
        .unwrap();
        let output = Command::new("python3").args(["-c", &format!(
            "from datetime import datetime,timezone,timedelta; print((datetime.fromtimestamp({secs},timezone.utc)+timedelta(microseconds={micros})).isoformat())"
        )]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(ours, String::from_utf8(output.stdout).unwrap().trim());
    }
    let submicro = UNIX_EPOCH + Duration::from_nanos(999);
    assert_eq!(
        publication_time(submicro).unwrap(),
        "1970-01-01T00:00:00+00:00"
    );
}

pub(super) fn session(root: &Path) -> PathBuf {
    let session = root.join("TEST-session");
    fs::create_dir(&session).unwrap();
    let initial = ledger::Value::parse(r#"{
        "schema_version":1,"session_id":"TEST-session","version":0,"revision":0,
        "created_at":"2026-10-19T00:00:00+00:00","updated_at":"2026-10-19T00:00:00+00:00",
        "status":"active","draft":{"goal":"","outcome":"","options":[]},"goal":null,
        "origin":null,"current_question":null,"frontier":[],"nodes":[],"assumed":[],"operator":"human"
    }"#).unwrap();
    fs::write(session.join("ledger.json"), initial.pretty().unwrap()).unwrap();
    let mut state: Value =
        serde_json::from_str(&ledger::editable(&initial).unwrap().pretty().unwrap()).unwrap();
    state["goal"] = "TEST goal".into();
    state["origin"] = "goal".into();
    state["nodes"] = json!([{"id":"goal","kind":"decision","status":"settled","prerequisites":[],
        "evidence":[],"answer":"TEST goal","authority":"user","authority_source":"TEST only",
        "owner":"TEST operator","gate":"TEST gate"}]);
    publish_at(&session, 0, "TEST goal", &state, STAMP).unwrap();
    session
}

#[test]
fn every_shape_ledger_action_works_without_launching_the_helper() {
    let root = temp();
    let session = session(root.path());
    let skill = root.path().join("SKILL.md");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".ci-seed-me/skills/seed-me/SKILL.md"),
        &skill,
    )
    .unwrap();
    fs::create_dir(root.path().join("scripts")).unwrap();
    fs::write(
        root.path().join("scripts/session.py"),
        "raise SystemExit('TEST helper must never run')",
    )
    .unwrap();
    let version = read(&skill, &session).unwrap().version();
    let version = probe(&skill, &session, version).unwrap().unwrap();
    let version = answer(&skill, &session, version, PROBE.0, "TEST exact\r\nanswer").unwrap();
    let version = ask(
        &skill,
        &session,
        version,
        &Ask {
            prerequisites: None,
            supported_by: None,
            id: "test-question".into(),
            label: "TEST".into(),
            question: "TEST?".into(),
            gate: "TEST".into(),
            found: vec![],
            suggestion: None,
            assumed: vec![Assumption {
                text: "TEST default".into(),
                why: "TEST same".into(),
            }],
            options: vec![],
            against: None,
            confidence: None,
        },
        root.path(),
    )
    .unwrap();
    let version = recommend(
        &skill,
        &session,
        version,
        "test-question",
        "TEST suggestion",
    )
    .unwrap();
    let version = object(&skill, &session, version, "TEST default").unwrap();
    let version = answer(&skill, &session, version, "test-question", "TEST answer").unwrap();
    let version = promote(&skill, &session, version).unwrap().unwrap();
    assert_eq!(read(&skill, &session).unwrap().version(), version);
    assert_eq!(read(&skill, &session).unwrap().goal(), Some("TEST goal"));
    let saved = fs::read_to_string(session.join("ledger.json")).unwrap();
    assert!(saved.contains("TEST exact\\r\\nanswer"));
    assert_eq!(
        fs::metadata(session.join("ledger.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn seed_drafting_confirmation_status_and_ending_need_no_installed_helper() {
    let root = temp();
    let session = session(root.path());
    let skill = root.path().join("TEST-no-installed-skill/SKILL.md");
    let checks =
        serde_json::from_str(include_str!("../../../tests/fixtures/shape-checks.json")).unwrap();
    let revision = draft_seed(
        &skill,
        &session,
        1,
        include_str!("../../../tests/fixtures/shape-seed-reply.txt"),
        &checks,
        &[],
        root.path(),
    )
    .unwrap();
    confirm_seed(&skill, &session, &revision, "TEST yes to this seed only").unwrap();
    assert!(matches!(
        super::super::shaping::read(&skill, &session).unwrap().stage,
        super::super::shaping::Shaped::Ready { ended: false, .. }
    ));
    end(&skill, &session, Ending::Completed).unwrap();
    assert!(matches!(
        super::super::shaping::read(&skill, &session).unwrap().stage,
        super::super::shaping::Shaped::Ready { ended: true, .. }
    ));
    assert_eq!(read(&skill, &session).unwrap().goal(), Some("TEST goal"));
    for name in ["viewer.json", "ledger-view.html"] {
        assert!(!session.join(name).exists());
    }
}

#[test]
fn stale_shape_publication_refuses_without_changing_bytes() {
    let root = temp();
    let session = session(root.path());
    let ledger = ledger::load(&session).unwrap();
    let mut state: Value =
        serde_json::from_str(&ledger::editable(&ledger).unwrap().pretty().unwrap()).unwrap();
    state["draft"]["goal"] = "TEST changed from stale read".into();
    let bytes = fs::read(session.join("ledger.json")).unwrap();
    assert_eq!(
        publish_at(&session, 0, "TEST stale", &state, STAMP),
        Err(Refusal::Stale)
    );
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), bytes);
}

#[test]
fn completion_through_publish_is_refused() {
    let root = temp();
    let session = session(root.path());
    let ledger = ledger::load(&session).unwrap();
    let mut state: Value =
        serde_json::from_str(&ledger::editable(&ledger).unwrap().pretty().unwrap()).unwrap();
    state["status"] = "completed".into();
    assert_eq!(publish_at(&session, 1, "TEST complete", &state, STAMP), Err(Refusal::Refused(
        "Tinkery refused: publish cannot complete a session: use `end --status completed`, which checks the confirmed seed".into())));
    assert_eq!(ledger::load(&session).unwrap(), ledger);
}

#[test]
fn an_unknown_save_asks_the_operator_to_look() {
    let error = |save_state| ledger::PublishError {
        reason: "could not save the ledger: TEST disk".into(),
        save_state,
    };
    assert_eq!(
        published(error(ledger::SaveState::NotSaved)),
        Refusal::Refused("Tinkery refused: could not save the ledger: TEST disk".into())
    );
    let Refusal::Refused(unknown) = published(error(ledger::SaveState::Unknown)) else {
        panic!("an unknown save must be a refusal with words");
    };
    assert!(
        unknown.starts_with("This may or may not have saved (TEST disk)."),
        "{unknown}"
    );
    assert!(unknown.contains("Reopen this session"), "{unknown}");
    assert!(!unknown.contains("refused"), "{unknown}");
}

#[test]
fn os_missing_file_diagnostic_is_rust_not_python_oserror() {
    let root = temp();
    let ours = ledger::load(root.path()).unwrap_err();
    let output = Command::new("python3").args(["-c", "import pathlib,sys\ntry: pathlib.Path(sys.argv[1]).read_text()\nexcept OSError as e: print(str(e))"])
        .arg(root.path().join("ledger.json")).output().unwrap();
    let python = String::from_utf8(output.stdout).unwrap();
    assert!(python.starts_with("[Errno 2]"), "{python}");
    assert!(ours.contains("os error 2"), "{ours}");
    assert_ne!(ours, python.trim());
}
