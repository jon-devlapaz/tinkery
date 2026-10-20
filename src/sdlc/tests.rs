use super::*;
use serde_json::json;

fn fixture() -> (tempfile::TempDir, PathBuf, Value) {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-join-")
        .tempdir()
        .unwrap();
    let source = root
        .path()
        .canonicalize()
        .unwrap()
        .join("session/seed-contract.md");
    fs::create_dir(source.parent().unwrap()).unwrap();
    fs::write(&source, b"TEST confirmed seed\n").unwrap();
    fs::write(
        source.parent().unwrap().join("ledger.json"),
        json!({
            "schema_version":1, "goal":" TEST café\nexact goal  "
        })
        .to_string(),
    )
    .unwrap();
    let run = json!({"slug":"unrelated-name", "seed_contract":{
        "path":"runs/other/seed-contract.md", "source":source,
        "sha256": format!("{:x}", Sha256::digest(fs::read(&source).unwrap()))
    }});
    (root, source, run)
}

#[test]
fn canonical_source_and_current_bytes_alone_join_the_exact_goal() {
    let (_root, source, run) = fixture();
    let seed = Seed::parse(&run).unwrap().unwrap();
    assert!(started(&[seed], &source).unwrap());
    assert_eq!(goal(&run), Goal::Matched(" TEST café\nexact goal  ".into()));
    let mut alias = run.clone();
    alias["seed_contract"]["source"] = source
        .parent()
        .unwrap()
        .join("./seed-contract.md")
        .to_string_lossy()
        .into_owned()
        .into();
    assert_eq!(
        goal(&alias),
        Goal::Unknown,
        "a noncanonical record never joins"
    );
}

#[test]
fn edited_seed_is_unknown_not_started_and_never_lends_its_goal() {
    let (_root, source, run) = fixture();
    fs::write(&source, "TEST edited seed\n").unwrap();
    let seed = Seed::parse(&run).unwrap().unwrap();
    assert!(
        started(&[seed], &source).is_err(),
        "changed seed was called started"
    );
    assert_eq!(
        goal(&run),
        Goal::Unknown,
        "changed seed lent a goal to the run"
    );
}

#[test]
fn moved_and_missing_originals_never_join_even_with_identical_names_and_bytes() {
    let (root, source, run) = fixture();
    let moved = root.path().join("moved/seed-contract.md");
    fs::create_dir(moved.parent().unwrap()).unwrap();
    fs::rename(&source, &moved).unwrap();
    let seed = Seed::parse(&run).unwrap().unwrap();
    assert!(!started(std::slice::from_ref(&seed), &moved).unwrap());
    assert_eq!(goal(&run), Goal::Unknown);
    assert!(started(&[seed], &source).is_err());
    fs::remove_file(moved).unwrap();
    assert_eq!(goal(&run), Goal::Unknown);
    assert!(
        !started(&[], &source).unwrap(),
        "an ended session without a recorded seed isn't an unknown start"
    );
}

#[test]
fn missing_or_malformed_seed_records_cannot_establish_a_start() {
    let (_root, _source, run) = fixture();
    assert_eq!(goal(&json!({"seed_contract":null})), Goal::NoSeed);
    assert_eq!(goal(&json!({})), Goal::Unknown);
    for value in [json!({}), json!({"source":"relative", "sha256":"x"})] {
        let mut run = run.clone();
        run["seed_contract"] = value;
        assert_eq!(goal(&run), Goal::Unknown);
    }
    assert!(Seed::parse(&json!({})).is_err());
    for value in [
        json!({"protocol":"other","api_version":1}),
        json!({"protocol":"tink-sdlc","api_version":"1"}),
    ] {
        assert!(envelope(&value.to_string()).is_err());
    }
}

#[test]
fn recorded_but_unreadable_originals_stay_unknown_not_unseeded() {
    let (_root, source, run) = fixture();
    fs::write(
        source.parent().unwrap().join("ledger.json"),
        "TEST malformed ledger",
    )
    .unwrap();
    assert_eq!(goal(&run), Goal::Unknown);
    fs::remove_file(&source).unwrap();
    fs::create_dir(&source).unwrap();
    assert_eq!(goal(&run), Goal::Unknown);
}

#[test]
fn a_failed_reader_start_says_what_to_check_without_os_internals() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-reader-")
        .tempdir()
        .unwrap();
    let why = run::run(
        &root.path().join("TEST-missing-checkout"),
        &[],
        &AtomicBool::new(false),
        DEADLINE,
    )
    .unwrap_err();
    assert_eq!(
        why,
        "Couldn't start reading runs. Check that Python 3 is installed and the tink-sdlc checkout is accessible, then reopen this view to read again."
    );
}

#[test]
fn unreadable_runs_refuse_all_start_classification() {
    let (root, _source, run) = fixture();
    let scripts = root.path().join("_system/scripts");
    fs::create_dir_all(&scripts).unwrap();
    let cancel = AtomicBool::new(false);
    for row in [
        json!({"verification_status":"invalid", "seed_contract":null}),
        json!({"slug":"old-report"}),
    ] {
        let report =
            json!({"protocol":"tink-sdlc", "api_version":1, "runs":[run, row]}).to_string();
        fs::write(scripts.join("sdlc.py"), format!("print({report:?})\n")).unwrap();
        let why = seeds(root.path(), &cancel).unwrap_err();
        if row["verification_status"] == "invalid" {
            assert_eq!(
                why,
                "A run couldn't be read. Check it in tink-sdlc, then reopen this view to read again."
            );
        }
    }
}
