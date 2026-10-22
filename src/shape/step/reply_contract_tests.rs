//! Captured #296 replies cross the fake-provider, decoder and real ledger boundaries.

use super::*;
use crate::seed_me::{Affirmed, Config, ledger::fixtures::prose_reply};
use std::{fs, os::unix::fs::PermissionsExt};

const MISSING_SOURCE: &str = include_str!("../../../tests/fixtures/shape-296/missing-source.json");
const RECHECKS: [&str; 3] = [
    include_str!("../../../tests/fixtures/shape-296/call-6.json"),
    include_str!("../../../tests/fixtures/shape-296/call-7.json"),
    include_str!("../../../tests/fixtures/shape-296/call-8.json"),
];

fn valid_reply() -> Value {
    let checks: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/shape-checks.json")).unwrap();
    json!({"next":"seed", "found":[], "checks":checks,
        "confirming":prose_reply(include_str!("../../../tests/fixtures/shape-seed-reply.txt")),
        "review":{"contradictions":[], "decided":[], "done":[],
            "failure_cases":checks.as_array().unwrap().iter().map(|_|
                json!("TEST wrong output must fail")).collect::<Vec<_>>()}})
}

fn ordered_reply(raw: &str) -> Value {
    let mut reply: Value = serde_json::from_str(raw).unwrap();
    reply["review"]["failure_cases"] = reply["review"]["failure_cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["case"].clone())
        .collect();
    reply
}

fn provider(root: &Path, reply: &str, calls: usize) -> Agent {
    let program = root.join("TEST-pi");
    fs::write(root.join("TEST-reply.json"), reply).unwrap();
    fs::write(&program, "#!/usr/bin/env python3\nimport pathlib, sys\nsys.stdin.read()\nprint(pathlib.Path(__file__).with_name('TEST-reply.json').read_text())\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    Agent::new(
        program,
        "test/stub".into(),
        String::new(),
        Some(root.into()),
        CallBudget::new(calls),
        None,
    )
    .unwrap()
}

fn look_again(session: &Path, root: &Path, agent: &Agent) -> Result<Done, Failure> {
    run(
        &Work::Look {
            expected: answering::read(session).unwrap().version(),
            folder: root.into(),
        },
        session,
        agent,
        &AtomicBool::new(false),
        &AtomicBool::new(false),
    )
}

fn assert_recovery(session: &Path, root: &Path, agent: &Agent) {
    let before: Value =
        serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap();
    fs::write(root.join("TEST-reply.json"), valid_reply().to_string()).unwrap();
    assert_eq!(look_again(session, root, agent), Ok(Done::Drafted));
    let after: Value =
        serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap();
    assert_eq!(after["goal"], before["goal"]);
    for node in before["nodes"].as_array().unwrap() {
        let current = after["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == node["id"])
            .unwrap();
        assert_eq!(current["answer"], node["answer"]);
    }
    assert!(!session.join("seed-confirmation.json").exists());
    assert_eq!(agent.remaining(), 0);
}

#[test]
fn captured_missing_source_preserves_the_answer_and_recovers_on_next_look() {
    let root = tempfile::Builder::new()
        .prefix("TEST-296-source-")
        .tempdir()
        .unwrap();
    let session = Config::new(Some(root.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: "TEST book loans".into(),
            outcome: "TEST two loans survive restart".into(),
            options: vec![],
            originals: vec![],
            context: "{}".into(),
            source: "TEST fixture".into(),
        })
        .unwrap()
        .session;
    let version = answering::read(&session).unwrap().version();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let agent = provider(root.path(), MISSING_SOURCE, 3);
    let saved = AtomicBool::new(false);
    let error = run(
        &Work::Answer {
            id: answering::PROBE.0.into(),
            words: "TEST keep my exact answer\r\n  ".into(),
            expected: version,
            folder: root.path().into(),
        },
        &session,
        &agent,
        &AtomicBool::new(false),
        &saved,
    )
    .unwrap_err();
    eprintln!("#296 missing-source: {error:?}");
    assert!(error.answer_saved && saved.load(Ordering::Relaxed));
    assert!(
        matches!(error.why, Why::Model(ref reason) if reason.contains("failure_cases must be a list of strings in check order"))
    );
    let captured: Value = serde_json::from_str(MISSING_SOURCE).unwrap();
    let check_error = crate::seed_me::ledger::validate_checks(
        &crate::seed_me::ledger::Value::from(captured["checks"].clone()),
    )
    .unwrap_err();
    eprintln!("#296 isolated missing-source checks: {check_error}");
    assert!(check_error.contains("check 1 missing fields: source"));
    assert!(check_error.contains("Source must name the confirmed goal"));
    assert!(!session.join("seed-contract.md").exists());
    let state: Value = serde_json::from_str(&answering::read(&session).unwrap().context()).unwrap();
    assert_eq!(
        state["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == answering::PROBE.0)
            .unwrap()["answer"],
        "TEST keep my exact answer\r\n  "
    );
    assert_eq!(agent.remaining(), 2, "no automatic retry");
    // With only transport updated, the original prose still hits the existing probe guard.
    let before = fs::read(session.join("ledger.json")).unwrap();
    fs::write(
        root.path().join("TEST-reply.json"),
        ordered_reply(MISSING_SOURCE).to_string(),
    )
    .unwrap();
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        matches!(error.why, Why::Refused(ref reason) if reason.contains("probe answers belong to the ledger"))
    );
    assert_eq!(fs::read(session.join("ledger.json")).unwrap(), before);
    assert_eq!(agent.remaining(), 1);
    assert_recovery(&session, root.path(), &agent);
}

#[test]
fn captured_rechecks_refuse_without_changing_saved_answers_then_recover() {
    for (index, reply) in RECHECKS.iter().enumerate() {
        let root = tempfile::Builder::new()
            .prefix("TEST-296-recheck-")
            .tempdir()
            .unwrap();
        let session = root.path().join("TEST-session");
        fs::create_dir(&session).unwrap();
        let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
        fs::write(session.join("ledger.json"), before).unwrap();
        let agent = provider(root.path(), reply, 2);
        let error = look_again(&session, root.path(), &agent).unwrap_err();
        eprintln!("#296 call-{}: {error:?}", index + 6);
        assert!(
            matches!(error.why, Why::Model(ref reason) if reason.contains("failure_cases must be a list of strings in check order")),
            "{error:?}"
        );
        assert_eq!(
            fs::read_to_string(session.join("ledger.json")).unwrap(),
            before
        );
        assert!(!session.join("seed-contract.md").exists());
        assert!(!session.join("seed-checks.json").exists());
        assert_eq!(agent.remaining(), 1, "no automatic retry");
        assert_recovery(&session, root.path(), &agent);
    }
}

#[test]
fn named_object_review_is_refused_without_writes_and_recovers_with_strings() {
    let root = tempfile::Builder::new()
        .prefix("TEST-296-named-refusal-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-session");
    fs::create_dir(&session).unwrap();
    let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
    fs::write(session.join("ledger.json"), before).unwrap();
    let mut reply = valid_reply();
    reply["review"]["failure_cases"] = reply["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| json!({"statement":check["statement"], "case":"TEST wrong output must fail"}))
        .collect();
    let agent = provider(root.path(), &reply.to_string(), 2);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        matches!(error.why, Why::Model(ref reason) if
        reason.contains("failure_cases must be a list of strings in check order")
        && !reason.contains("invalid type") && !reason.contains("untagged")),
        "{error:?}"
    );
    assert_eq!(
        fs::read_to_string(session.join("ledger.json")).unwrap(),
        before
    );
    assert!(!session.join("seed-contract.md").exists());
    assert!(!session.join("seed-checks.json").exists());
    assert_eq!(agent.remaining(), 1, "no automatic retry");
    assert_recovery(&session, root.path(), &agent);
}

#[test]
fn ordered_cases_pair_exact_statements_without_rewriting_and_keep_coverage_strict() {
    for (index, raw) in RECHECKS.iter().enumerate() {
        let root = tempfile::Builder::new()
            .prefix("TEST-296-ordered-")
            .tempdir()
            .unwrap();
        let session = root.path().join("TEST-session");
        fs::create_dir(&session).unwrap();
        let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
        fs::write(session.join("ledger.json"), before).unwrap();
        let reply = ordered_reply(raw);
        let cases = reply["review"]["failure_cases"].as_array().unwrap();
        let agent = provider(root.path(), &reply.to_string(), 1);
        let result = look_again(&session, root.path(), &agent);
        if index == 0 {
            assert_eq!(result, Ok(Done::Drafted));
            let state: Value =
                serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap();
            let origin = state["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == state["origin"])
                .unwrap();
            let receipt = origin["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["artifact"] == "tinkery:review/completed")
                .unwrap();
            let review: Value =
                serde_json::from_str(receipt["observed"].as_str().unwrap()).unwrap();
            assert_eq!(
                review["failure_cases"][0]["statement"],
                reply["checks"][0]["statement"]
            );
            assert_eq!(review["failure_cases"][0]["case"], cases[0]);
            let checks: Value =
                serde_json::from_slice(&fs::read(session.join("seed-checks.json")).unwrap())
                    .unwrap();
            assert_eq!(checks, reply["checks"]);
            // Drafting never rewrites the saved goal or fills its previously open outcome.
            assert_eq!(state["draft"]["outcome"], "");
            let old: Value = serde_json::from_str(before).unwrap();
            assert_eq!(state["goal"], old["goal"]);
            assert_eq!(state["nodes"][1], old["nodes"][1]);
        } else {
            let error = result.unwrap_err();
            let counts = if index == 1 {
                "received 3 for 2 checks"
            } else {
                "received 3 for 4 checks"
            };
            assert!(
                matches!(error.why, Why::Model(ref reason) if reason.contains(counts)),
                "{error:?}"
            );
            assert_eq!(
                fs::read_to_string(session.join("ledger.json")).unwrap(),
                before
            );
            assert!(!session.join("seed-contract.md").exists());
        }
        assert_eq!(agent.remaining(), 0);
    }
}

#[test]
fn captured_checks_with_valid_prose_refuse_missing_source_without_inventing_it() {
    let root = tempfile::Builder::new()
        .prefix("TEST-296-source-only-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-session");
    fs::create_dir(&session).unwrap();
    let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
    fs::write(session.join("ledger.json"), before).unwrap();
    let mut reply = ordered_reply(MISSING_SOURCE);
    // Isolate source validation from the independently guarded historical probe prose.
    reply["confirming"] = valid_reply()["confirming"].clone();
    let agent = provider(root.path(), &reply.to_string(), 2);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        matches!(error.why, Why::Refused(ref reason) if reason.contains("check 1 missing fields: source") && reason.contains("never invent evidence")),
        "{error:?}"
    );
    assert_eq!(
        fs::read_to_string(session.join("ledger.json")).unwrap(),
        before
    );
    assert!(!session.join("seed-checks.json").exists());
    assert!(!session.join("seed-contract.md").exists());
    assert_recovery(&session, root.path(), &agent);
}

#[test]
fn ordered_transport_preserves_bytes_and_refuses_mixed_or_incomplete_cases() {
    assert!(prompt().contains("one string per check in checks order"));
    let mut reply = valid_reply();
    reply["checks"] =
        json!([{"statement":"TEST café check\r\n  ", "kind":"judge", "source":"TEST goal"}]);
    reply["review"]["failure_cases"] = json!(["TEST wrong result\r\n  "]);
    let Next::Seed { review, checks, .. } = decode(&reply.to_string()).unwrap() else {
        panic!()
    };
    review
        .validate(&crate::seed_me::ledger::Value::from(checks))
        .unwrap();
    assert_eq!(review.failure_cases[0].statement, "TEST café check\r\n  ");
    assert_eq!(review.failure_cases[0].case, "TEST wrong result\r\n  ");
    for cases in [
        json!([null]),
        json!([42]),
        json!(["TEST", {"statement":"TEST", "case":"TEST"}]),
    ] {
        reply["review"]["failure_cases"] = cases;
        let error = decode(&reply.to_string()).unwrap_err();
        assert!(error.contains("typed completion review"));
        assert!(!error.contains("invalid type") && !error.contains("untagged"));
    }
    reply["review"]["failure_cases"] = json!(["  "]);
    let Next::Seed { review, checks, .. } = decode(&reply.to_string()).unwrap() else {
        panic!()
    };
    assert!(
        review
            .validate(&crate::seed_me::ledger::Value::from(checks))
            .unwrap_err()
            .contains("case 1 is blank")
    );
}
