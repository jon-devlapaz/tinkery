//! Captured #296 replies cross the fake-provider, decoder and real ledger boundaries.

use super::typed_tests::review_error;
use super::*;
use crate::pi::{CallBudget, Harness};
use crate::seed_me::{Affirmed, Config, ledger::fixtures::prose_reply};
use std::{fs, os::unix::fs::PermissionsExt};

/// The whole reason a refused step gives, a broken ledger rule included.
fn refused(why: &Why) -> Option<String> {
    match why {
        Why::Refused(reason) => Some(reason.clone()),
        Why::Rule(rule) => Some(rule.to_string()),
        _ => None,
    }
}

/// One step through the screen's controller, as its keys start it.
fn run(
    work: &Work,
    session: &Path,
    agent: &Agent,
    cancelled: &AtomicBool,
    saved: &AtomicBool,
) -> Result<Done, Failure> {
    Steps::new(Some(agent), &GoalConfirmation::default()).run(work, session, cancelled, saved)
}

const MISSING_SOURCE: &str = include_str!("../../../tests/fixtures/shape-296/missing-source.json");
const RECHECKS: [&str; 3] = [
    include_str!("../../../tests/fixtures/shape-296/call-6.json"),
    include_str!("../../../tests/fixtures/shape-296/call-7.json"),
    include_str!("../../../tests/fixtures/shape-296/call-8.json"),
];

/// #296 call 8's provisionals all proposed the whole suite (#469).
const SHARED_SUITE: &str =
    "check 1 proposes `python3 -m unittest discover`, which check 2 also uses in cwd `.`.";

fn valid_reply() -> Value {
    let checks: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/shape-checks.json")).unwrap();
    json!({"next":"seed", "readings": [], "assumed": [], "found":[], "checks":checks,
        "confirming":prose_reply(include_str!("../../../tests/fixtures/shape-seed-reply.txt")),
        "review":{"contradictions":[], "decided":[], "done":[],
            "failure_cases":checks.as_array().unwrap().iter().map(|_|
                json!("TEST wrong output must fail")).collect::<Vec<_>>()}})
}

/// A valid reply after the operator's own TEST pre-mortem answer, which every such reply must
/// cover (#1010); these tests are not about that coverage.
fn answered_reply() -> Value {
    let mut reply = valid_reply();
    reply["risks"] =
        json!([{"risk": "TEST", "out_of_scope": "TEST fixture, not about the pre-mortem"}]);
    reply
}

/// The checks as Shape saves them: each keeps its own failure case.
fn with_cases(checks: &Value, cases: &[Value]) -> Value {
    let mut saved = checks.clone();
    for (check, case) in saved.as_array_mut().unwrap().iter_mut().zip(cases) {
        check["failure_case"] = case.clone();
    }
    saved
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

fn attached_reply() -> Value {
    let mut reply = valid_reply();
    reply["review"]
        .as_object_mut()
        .unwrap()
        .remove("failure_cases");
    for (index, check) in reply["checks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        check["failure_case"] = format!("TEST failure {index} café\r\n  ").into();
    }
    reply
}

fn provider(root: &Path, reply: &str, calls: usize) -> Agent {
    let program = root.join("TEST-pi");
    fs::write(root.join("TEST-reply.json"), reply).unwrap();
    fs::write(&program, "#!/usr/bin/env python3\nimport pathlib, sys\nsys.stdin.read()\nprint(pathlib.Path(__file__).with_name('TEST-reply.json').read_text())\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    Agent::new(
        crate::pi::Drafting::new(
            crate::pi::Harness::Pi,
            program,
            "test/stub".into(),
            String::new(),
            CallBudget::new(calls),
        )
        .unwrap(),
        Some(root.into()),
        None,
    )
}

fn look_again(session: &Path, root: &Path, agent: &Agent) -> Result<Done, Failure> {
    run(
        &Work::Look {
            feedback: None,
            expected: answering::read(session).unwrap().version(),
            reads: std::path::PathBuf::from(root).into(),
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
    let reply = if answering::read(session)
        .unwrap()
        .pre_mortem()
        .is_some_and(|p| p.operator)
    {
        answered_reply()
    } else {
        valid_reply()
    };
    fs::write(root.join("TEST-reply.json"), reply.to_string()).unwrap();
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
    let version = answering::answer(
        &session,
        version,
        answering::SAMPLE.0,
        "TEST sample: one pasted line",
        Actor::OPERATOR,
    )
    .unwrap();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let agent = provider(root.path(), MISSING_SOURCE, 3);
    let saved = AtomicBool::new(false);
    let error = run(
        &Work::Answer {
            relay: None,
            id: answering::PROBE.0.into(),
            words: "TEST keep my exact answer\r\n  ".into(),
            expected: version,
            reads: std::path::PathBuf::from(root.path()).into(),
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
        refused(&error.why).is_some_and(|reason| reason
            .contains("Source must name the confirmed goal")
            && reason.contains("Also: The Tink draft reply needs a typed completion review")
            && reason.contains("failure_cases must be a list of strings in check order")),
        "#432: both broken rules in one refusal: {error:?}"
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
    // With transport updated, pure check validation reports the missing source first.
    let before = fs::read(session.join("ledger.json")).unwrap();
    fs::write(
        root.path().join("TEST-reply.json"),
        ordered_reply(MISSING_SOURCE).to_string(),
    )
    .unwrap();
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        refused(&error.why).is_some_and(|reason| reason.contains("check 1 missing fields: source"))
    );
    // The same captured prose remains forbidden once check records are supplied.
    let reply = ordered_reply(MISSING_SOURCE);
    let prose_error = answering::draft_seed(
        &session,
        answering::read(&session).unwrap().version(),
        &reply["confirming"],
        &valid_reply()["checks"],
        answering::Reported {
            found: &[],
            assumed: &[],
            resolves: None,
        },
        root.path(),
    )
    .unwrap_err();
    assert!(
        matches!(prose_error, Refusal::Rejected(ref reason) if reason.contains("probe answers belong to the ledger"))
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
        let review = "failure_cases must be a list of strings in check order";
        // #469: call 8 proposed the whole suite for three provisionals; both faults are heard.
        assert!(
            match &error.why {
                Why::Rule(rule) if index == 2 => {
                    let reason = rule.to_string();
                    reason.contains(SHARED_SUITE)
                        && reason
                            .contains("Also: The Tink draft reply needs a typed completion review")
                        && reason.contains(review)
                }
                Why::Model(reason) => index != 2 && reason.contains(review),
                _ => false,
            },
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
            assert_eq!(checks, with_cases(&reply["checks"], cases));
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
                match &error.why {
                    Why::Rule(rule) if index == 2 =>
                        rule.rule.contains(SHARED_SUITE) && rule.rule.contains(counts),
                    Why::Model(reason) =>
                        index == 1 && reason.contains(counts) && reason.contains("failure mode"),
                    _ => false,
                },
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
        refused(&error.why).is_some_and(|reason| reason.contains("check 1 missing fields: source")
            && reason.contains("never invent evidence")),
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

/// Recorded 2026-10-23 clinic replies are unchanged, including call 19's extra closing brace.
#[test]
fn recorded_361_refusals_name_unsafe_coverage_and_preserve_saved_answers() {
    for (raw, counts) in [
        (
            include_str!("../../../tests/fixtures/shape-361/call-19.json"),
            "received 4 for 2 checks",
        ),
        (
            include_str!("../../../tests/fixtures/shape-361/call-20.json"),
            "received 1 for 2 checks",
        ),
    ] {
        let root = tempfile::Builder::new()
            .prefix("TEST-361-refusal-")
            .tempdir()
            .unwrap();
        let session = Config::new(Some(root.path().join("TEST-sessions")))
            .confirm(Affirmed {
                goal: "TEST clinic game".into(),
                outcome: String::new(),
                options: vec![],
                originals: vec![],
                context: "{}".into(),
                source: "TEST fixture".into(),
            })
            .unwrap()
            .session;
        let version = answering::read(&session).unwrap().version();
        let version = answering::probe(&session, version).unwrap().unwrap();
        let version = answering::answer(
            &session,
            version,
            answering::SAMPLE.0,
            "TEST sample: one pasted line",
            Actor::OPERATOR,
        )
        .unwrap();
        let version = answering::probe(&session, version).unwrap().unwrap();
        let agent = provider(root.path(), raw, 1);
        let words = "TEST preserve my answer café\r\n  ";
        let saved = AtomicBool::new(false);
        let error = run(
            &Work::Answer {
                relay: None,
                id: answering::PROBE.0.into(),
                words: words.into(),
                expected: version,
                reads: std::path::PathBuf::from(root.path()).into(),
            },
            &session,
            &agent,
            &AtomicBool::new(false),
            &saved,
        )
        .unwrap_err();
        assert!(
            matches!(error.why, Why::Model(ref reason) if reason.contains(counts)),
            "{error:?}"
        );
        assert!(error.answer_saved && saved.load(Ordering::Relaxed));
        let ledger: Value =
            serde_json::from_str(&answering::read(&session).unwrap().context()).unwrap();
        assert_eq!(ledger["goal"], "TEST clinic game");
        assert_eq!(
            ledger["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["id"] == answering::PROBE.0)
                .unwrap()["answer"],
            words
        );
        assert!(!session.join("seed-contract.md").exists());
        assert!(!session.join("seed-checks.json").exists());
        assert!(!session.join("seed-confirmation.json").exists());
        assert_eq!(
            fs::read_to_string(root.path().join("TEST-reply.json")).unwrap(),
            raw
        );
        assert_eq!(agent.remaining(), 0, "one call, never an automatic retry");
    }
}

#[test]
fn recoverable_361_extra_closing_brace_drafts_in_one_call_without_rewriting_content() {
    let root = tempfile::Builder::new()
        .prefix("TEST-361-repair-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-session");
    fs::create_dir(&session).unwrap();
    let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
    fs::write(session.join("ledger.json"), before).unwrap();
    // Synthetic coverage-correct control: the recorded cases cannot safely be paired or filled in.
    let reply = valid_reply();
    let raw = format!("{reply}}}");
    let agent = provider(root.path(), &raw, 1);
    assert_eq!(look_again(&session, root.path(), &agent), Ok(Done::Drafted));
    let after: Value =
        serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap();
    let before: Value = serde_json::from_str(before).unwrap();
    assert_eq!(after["goal"], before["goal"]);
    for node in before["nodes"].as_array().unwrap() {
        assert_eq!(
            after["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == node["id"])
                .unwrap()["answer"],
            node["answer"]
        );
    }
    let checks: Value =
        serde_json::from_slice(&fs::read(session.join("seed-checks.json")).unwrap()).unwrap();
    assert_eq!(
        checks,
        with_cases(
            &reply["checks"],
            reply["review"]["failure_cases"].as_array().unwrap()
        )
    );
    assert_eq!(
        fs::read_to_string(root.path().join("TEST-reply.json")).unwrap(),
        raw
    );
    assert!(!session.join("seed-confirmation.json").exists());
    assert_eq!(agent.remaining(), 0);
}

#[test]
fn reply_framing_repairs_only_complete_objects_and_names_remaining_errors() {
    let question = json!({"next":"question", "question":"TEST braces } { and escaped quote \" stay intact?", "found":[]});
    for raw in [
        format!("{question}}}"),
        format!("preface\n```json\n{question}}}\n```\nafterword"),
    ] {
        let Next::Question(ask) = decode(&raw, "").unwrap() else {
            panic!()
        };
        assert_eq!(ask.question, question["question"].as_str().unwrap());
    }
    for (raw, reason) in [
        ("no JSON", "no JSON object"),
        (
            "{\"next\":\"question\"",
            "ended before its JSON object was complete",
        ),
        ("{\"next\":}", "invalid JSON"),
        (
            "stray } before {",
            "ended before its JSON object was complete",
        ),
        (
            "{\"next\":\"confirm\"}",
            "field next must be question or seed",
        ),
        (
            "{\"next\":\"question\",\"found\":[]}",
            "field question must contain non-empty words",
        ),
    ] {
        assert!(decode(raw, "").unwrap_err().contains(reason), "{raw}");
    }
    for raw in [
        format!("{question}{question}"),
        format!("{question}{{}}"),
        format!("{question}{{"),
        format!("{question},}}"),
        format!("{question}]}}"),
        format!("{question}unexpected}}"),
    ] {
        assert!(
            decode(&raw, "")
                .unwrap_err()
                .contains("extra content after its JSON object"),
            "{raw}"
        );
    }
}

#[test]
fn prompt_example_pairs_provisional_and_judge_checks_without_inventing_coverage() {
    let prompt = prompt();
    assert!(prompt.contains("one complete JSON object with no extra closing brace"));
    let example = PROMPT
        .lines()
        .find(|line| line.starts_with("{\"next\":\"seed\""))
        .unwrap();
    assert!(prompt.contains(example));
    let Next::Seed { review, checks, .. } = decode(example, "").unwrap() else {
        panic!()
    };
    let review = review.unwrap();
    assert_eq!(checks.as_array().unwrap().len(), 2);
    assert_eq!(checks[0]["kind"], "provisional");
    assert_eq!(
        checks[0]["cmd"],
        "python -m pytest 'tests/test_<new_behavior>.py'"
    );
    assert_eq!(checks[0]["source"], "pyproject.toml:<runner config lines>");
    assert_eq!(
        checks[0]["missing"],
        "tests/test_<new_behavior>.py does not exist yet; add an assertion that the requested behavior holds"
    );
    assert_eq!(checks[1]["kind"], "judge");
    let checks = crate::seed_me::ledger::Value::from(checks);
    crate::seed_me::ledger::validate_checks(&checks).unwrap();
    review.validate(&checks).unwrap();
}

#[test]
fn ordered_transport_preserves_bytes_and_refuses_mixed_or_incomplete_cases() {
    let mut reply = valid_reply();
    reply["checks"] =
        json!([{"statement":"TEST café check\r\n  ", "kind":"judge", "source":"TEST goal"}]);
    reply["review"]["failure_cases"] = json!(["TEST wrong result\r\n  "]);
    let Next::Seed { review, checks, .. } = decode(&reply.to_string(), "").unwrap() else {
        panic!()
    };
    let review = review.unwrap();
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
        let error = review_error(&reply);
        assert!(error.contains("typed completion review"));
        assert!(!error.contains("invalid type") && !error.contains("untagged"));
    }
    reply["review"]["failure_cases"] = json!(["  "]);
    let Next::Seed { review, checks, .. } = decode(&reply.to_string(), "").unwrap() else {
        panic!()
    };
    let review = review.unwrap();
    assert!(
        review
            .validate(&crate::seed_me::ledger::Value::from(checks))
            .unwrap_err()
            .contains("case 1 is blank")
    );
}

/// The #296 ledger with a structured Done when line, as Think composes one.
fn done_when_session(root: &Path, criterion: &str) -> PathBuf {
    let session = root.join("TEST-session");
    fs::create_dir_all(&session).unwrap();
    let mut ledger: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/shape-296/ledger.json"
    ))
    .unwrap();
    let goal = format!(
        "{}\nDone when: {criterion}",
        ledger["goal"].as_str().unwrap()
    );
    // The ledger keeps the confirmed goal in three places; change them together.
    ledger["goal"] = goal.clone().into();
    ledger["draft"]["goal"] = goal.clone().into();
    ledger["nodes"][0]["answer"] = goal.into();
    fs::write(session.join("ledger.json"), ledger.to_string()).unwrap();
    session
}

#[test]
fn a_done_when_line_changed_only_in_letter_case_is_saved_in_the_operator_words() {
    // #432, recorded contradict call 7: "Valid-row" came back as "valid-row" and was refused.
    let root = tempfile::Builder::new()
        .prefix("TEST-432-case-")
        .tempdir()
        .unwrap();
    let criterion =
        "Bad rows print a stderr warning; Valid-row output stays byte-for-byte identical.";
    let session = done_when_session(root.path(), criterion);
    let mut reply = valid_reply();
    reply["checks"][0]["statement"] =
        "bad rows print a stderr warning; valid-row output stays byte-for-byte identical.".into();
    let agent = provider(root.path(), &reply.to_string(), 1);
    assert_eq!(look_again(&session, root.path(), &agent), Ok(Done::Drafted));
    let checks: Value =
        serde_json::from_slice(&fs::read(session.join("seed-checks.json")).unwrap()).unwrap();
    assert_eq!(checks[0]["statement"], criterion);
    assert_eq!(
        checks[1],
        with_cases(
            &reply["checks"],
            &reply["review"]["failure_cases"].as_array().unwrap().clone()
        )[1],
        "other checks keep the model's words"
    );

    // A changed word is not a case difference: it still refuses and writes nothing.
    let session = done_when_session(&root.path().join("reworded"), criterion);
    reply["checks"][0]["statement"] =
        "bad rows print a warning; valid-row output stays byte-for-byte identical.".into();
    let agent = provider(root.path(), &reply.to_string(), 1);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        refused(&error.why)
            .is_some_and(|reason| reason.contains("keep this complete Done when line unchanged")),
        "{error:?}"
    );
    assert!(error.feedback.is_some());
    assert!(!session.join("seed-checks.json").exists());
}

#[test]
fn a_reply_breaking_done_when_and_case_count_hears_both_in_one_refusal() {
    // #432, recorded trap call 1: one Done when line split in two checks, cases for neither shape.
    let root = tempfile::Builder::new()
        .prefix("TEST-432-both-")
        .tempdir()
        .unwrap();
    let session = done_when_session(
        root.path(),
        "A wrong answer prints ✗ and the existing explanation; Tests still pass.",
    );
    let mut reply = valid_reply();
    reply["checks"][0]["statement"] =
        "A wrong answer prints ✗ and the existing explanation.".into();
    reply["checks"][1]["statement"] = "Tests still pass.".into();
    reply["review"]["failure_cases"] = json!(["TEST only one case"]);
    let agent = provider(root.path(), &reply.to_string(), 1);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    let Some(reason) = refused(&error.why) else {
        panic!("{error:?}")
    };
    assert!(
        reason.contains("keep this complete Done when line unchanged"),
        "{reason}"
    );
    assert!(reason.contains("Also: Completion review needs exactly one failure case per check: received 1 for 3 checks"), "{reason}");
    let feedback = error
        .feedback
        .expect("an explicit retry hears this refusal");
    assert!(
        feedback.input()["reason"]
            .as_str()
            .unwrap()
            .contains("Also: Completion review")
    );
    assert!(!session.join("seed-checks.json").exists());
}

#[test]
fn a_ledger_rule_the_reply_broke_becomes_retry_feedback_and_storage_never_does() {
    // #432, recorded contradict2 call 13: a finding reused the probe's node id; call 14 ran blind.
    let root = tempfile::Builder::new()
        .prefix("TEST-432-ledger-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-session");
    fs::create_dir(&session).unwrap();
    let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
    fs::write(session.join("ledger.json"), before).unwrap();
    let taken = serde_json::from_str::<Value>(before).unwrap()["nodes"][0]["id"].clone();
    let mut reply = valid_reply();
    reply["found"] = json!([{"id": taken, "claim": {"type": "observation", "scope": "TEST line"},
        "checked": "README.md:1", "observed": "TEST observed"}]);
    let agent = provider(root.path(), &reply.to_string(), 1);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        refused(&error.why).is_some_and(
            |reason| reason.contains(&format!("duplicate node id {}", taken.as_str().unwrap()))
        ),
        "{error:?}"
    );
    assert!(
        error.feedback.is_some(),
        "the retry must hear which rule broke"
    );
    assert_eq!(
        fs::read_to_string(session.join("ledger.json")).unwrap(),
        before
    );

    // A session another writer holds is a storage refusal: nothing for the model to fix.
    let lock = fs::File::create(session.join(".writer.lock")).unwrap();
    // SAFETY: the descriptor stays owned by `lock` for this whole test.
    assert_eq!(
        unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&lock), libc::LOCK_EX) },
        0
    );
    let agent = provider(root.path(), &valid_reply().to_string(), 1);
    let error = look_again(&session, root.path(), &agent).unwrap_err();
    assert!(
        refused(&error.why).is_some_and(|reason| reason.contains("already has a writer")),
        "{error:?}"
    );
    assert!(error.feedback.is_none());
}

#[test]
fn one_case_per_failure_mode_hears_that_one_check_takes_one_string() {
    // #432, re-measured contradict calls 2-4: two failure modes of one check, sent as two cases.
    let mut reply = valid_reply();
    reply["checks"] =
        json!([{"statement":"TEST bad rows are skipped", "kind":"judge", "source":"TEST goal"}]);
    reply["review"]["failure_cases"] = json!(["TEST still crashes", "TEST counts data rows"]);
    let error = review_error(&reply);
    assert!(error.contains("received 2 for 1 check. "), "{error}");
    assert!(
        error.contains("put every failure mode of one check in its one string"),
        "{error}"
    );
}

#[test]
fn a_claude_code_step_reads_only_the_shown_folder_and_drafts_from_its_json_result() {
    let root = tempfile::Builder::new()
        .prefix("TEST-claude-step-")
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
    let version = answering::answer(
        &session,
        version,
        answering::SAMPLE.0,
        "TEST sample: one pasted line",
        Actor::OPERATOR,
    )
    .unwrap();
    let version = answering::probe(&session, version).unwrap().unwrap();
    // Claude Code gets a clean environment, so the fake's paths are baked in.
    let log = root.path().join("TEST-started.json");
    let reply = json!({"type":"result", "is_error":false, "result":answered_reply().to_string(),
        "modelUsage":{"claude-TEST":{}}});
    let program = root.path().join("claude");
    fs::write(
        &program,
        format!(
            "#!/usr/bin/env python3\nimport json, os, sys\nsys.stdin.read()\n\
             open({log:?}, 'w').write(json.dumps(dict(argv=sys.argv[1:], cwd=os.getcwd())))\n\
             print({reply:?})\n",
            log = log.display().to_string(),
            reply = reply.to_string(),
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let agent = Agent::new(
        crate::pi::Drafting::new(
            Harness::ClaudeCode,
            program,
            "claude-TEST".into(),
            "max".into(),
            CallBudget::new(1),
        )
        .unwrap(),
        None,
        None,
    );
    let done = run(
        &Work::Answer {
            relay: None,
            id: answering::PROBE.0.into(),
            words: "TEST answer".into(),
            expected: version,
            reads: std::path::PathBuf::from(root.path()).into(),
        },
        &session,
        &agent,
        &AtomicBool::new(false),
        &AtomicBool::new(false),
    );
    assert_eq!(done, Ok(Done::Drafted));
    let started: Value = serde_json::from_str(&fs::read_to_string(log).unwrap()).unwrap();
    let argv: Vec<String> = serde_json::from_value(started["argv"].clone()).unwrap();
    let after = |flag: &str| argv[argv.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(after("--tools"), "Read,Grep,Glob");
    assert_eq!(after("--permission-mode"), "dontAsk");
    assert_eq!(after("--effort"), "max");
    assert_eq!(
        Path::new(started["cwd"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        root.path().canonicalize().unwrap()
    );
    assert!(
        crate::pi::Drafting::new(
            Harness::ClaudeCode,
            "claude".into(),
            "claude-TEST".into(),
            "minimal".into(),
            CallBudget::new(0)
        )
        .is_err(),
        "a level Claude Code doesn't name is refused before anything starts"
    );
}

#[test]
fn a_provisional_proposing_another_checks_command_is_refused_with_a_plain_reason() {
    // #469: pi-skimmer and humanize provisionals proposed the whole suite another check ran.
    let root = tempfile::Builder::new()
        .prefix("TEST-469-shared-")
        .tempdir()
        .unwrap();
    let suite: fn(&mut Value) = |reply| reply["checks"][1]["cmd"] = "./TEST-check".into();
    let twin: fn(&mut Value) = |reply| {
        reply["checks"][2] = json!({"statement":"TEST other behavior","kind":"provisional",
            "cmd":"./TEST-new-check","expect":"TEST","cwd":".","missing":"TEST","source":"TEST"})
    };
    let cases = [
        (
            "suite",
            suite,
            "check 2 proposes `./TEST-check`, which check 1 also uses in cwd `.`.",
        ),
        (
            "twin",
            twin,
            "check 2 proposes `./TEST-new-check`, which check 3 also uses in cwd `.`.",
        ),
    ];
    for (name, change, expected) in cases {
        let session = root.path().join(name).join("TEST-session");
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("ledger.json"),
            include_str!("../../../tests/fixtures/shape-296/ledger.json"),
        )
        .unwrap();
        let mut reply = valid_reply();
        change(&mut reply);
        let agent = provider(root.path(), &reply.to_string(), 1);
        let error = look_again(&session, root.path(), &agent).unwrap_err();
        let Some(reason) = refused(&error.why) else {
            panic!("{error:?}")
        };
        assert!(reason.contains(expected), "{reason}");
        assert!(
            reason.contains(
                "must select its missing regression by name; otherwise make it a judge check"
            ),
            "{reason}"
        );
        assert!(error.feedback.is_some(), "a retry hears the refusal");
        assert!(!session.join("seed-checks.json").exists());
    }

    // A command that selects only the missing regression drafts, and so does the same command
    // run from another directory, which selects a different test.
    let mut elsewhere = valid_reply();
    elsewhere["checks"][2] = json!({"statement":"TEST other package","kind":"provisional",
        "cmd":"./TEST-new-check","expect":"TEST","cwd":"TEST-package-b","missing":"TEST","source":"TEST"});
    for (name, reply) in [("targeted", valid_reply()), ("elsewhere", elsewhere)] {
        let session = root.path().join(name).join("TEST-session");
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("ledger.json"),
            include_str!("../../../tests/fixtures/shape-296/ledger.json"),
        )
        .unwrap();
        let agent = provider(root.path(), &reply.to_string(), 1);
        assert_eq!(
            look_again(&session, root.path(), &agent),
            Ok(Done::Drafted),
            "{name}"
        );
    }
}

#[test]
fn each_saved_check_keeps_its_own_case_when_statements_repeat() {
    for attached in [false, true] {
        let root = tempfile::Builder::new()
            .prefix("TEST-469-paired-")
            .tempdir()
            .unwrap();
        let session = root.path().join("TEST-session");
        fs::create_dir(&session).unwrap();
        let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
        fs::write(session.join("ledger.json"), before).unwrap();
        let mut reply = attached_reply();
        reply["checks"][1]["statement"] = reply["checks"][0]["statement"].clone();
        let paired = reply["checks"].clone();
        let mut canonical = reply["checks"].clone();
        let cases: Vec<_> = canonical
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .map(|check| {
                check
                    .as_object_mut()
                    .unwrap()
                    .remove("failure_case")
                    .unwrap()
            })
            .collect();
        if !attached {
            reply["checks"] = canonical.clone();
            reply["review"]["failure_cases"] = json!(cases);
        }
        let agent = provider(root.path(), &reply.to_string(), 1);
        assert_eq!(look_again(&session, root.path(), &agent), Ok(Done::Drafted));
        let saved: Value =
            serde_json::from_slice(&fs::read(session.join("seed-checks.json")).unwrap()).unwrap();
        assert_eq!(saved, paired, "each saved check keeps its own case");
        let state: Value =
            serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap();
        let old: Value = serde_json::from_str(before).unwrap();
        assert_eq!(state["goal"], old["goal"]);
        let receipt = state["nodes"][0]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["artifact"] == "tinkery:review/completed")
            .unwrap();
        let review: Value = serde_json::from_str(receipt["observed"].as_str().unwrap()).unwrap();
        for (index, case) in cases.iter().enumerate() {
            assert_eq!(
                review["failure_cases"][index],
                json!({"statement":canonical[index]["statement"], "case":case})
            );
        }
        assert!(!session.join("seed-confirmation.json").exists());
        assert_eq!(agent.remaining(), 0);
    }
}

#[test]
fn attached_cases_refuse_missing_malformed_or_mixed_coverage_without_writes() {
    let mut replies = Vec::new();
    for bad in [Value::Null, json!(42), json!([]), json!("  ")] {
        let mut reply = attached_reply();
        reply["checks"][1]["failure_case"] = bad;
        replies.push((reply, "Check 2 needs a non-empty failure_case"));
    }
    let mut partial = attached_reply();
    partial["checks"][1]
        .as_object_mut()
        .unwrap()
        .remove("failure_case");
    replies.push((partial, "Check 2 needs a non-empty failure_case"));
    for legacy in [Value::Null, json!([]), json!(["TEST extra case"])] {
        let mut mixed = attached_reply();
        mixed["review"]["failure_cases"] = legacy;
        replies.push((mixed, "not also in review.failure_cases"));
    }
    let mut foreign = attached_reply();
    foreign["checks"][0]["foreign"] = "TEST extra field".into();
    replies.push((foreign, "check 1 has unknown fields: foreign"));
    let mut shared = attached_reply();
    shared["checks"][1]["cmd"] = shared["checks"][0]["cmd"].clone();
    shared["checks"][1]["failure_case"] = Value::Null;
    replies.push((
        shared,
        "check 2 proposes `./TEST-check`, which check 1 also uses",
    ));
    for (reply, reason) in replies {
        let root = tempfile::Builder::new()
            .prefix("TEST-469-cases-")
            .tempdir()
            .unwrap();
        let session = root.path().join("TEST-session");
        fs::create_dir(&session).unwrap();
        let before = include_str!("../../../tests/fixtures/shape-296/ledger.json");
        fs::write(session.join("ledger.json"), before).unwrap();
        let agent = provider(root.path(), &reply.to_string(), 1);
        let failure = look_again(&session, root.path(), &agent).unwrap_err();
        let detail = match &failure.why {
            Why::Model(detail) => detail.clone(),
            other => refused(other).unwrap_or_else(|| panic!("{other:?}")),
        };
        assert!(detail.contains(reason), "{detail}");
        if reason.starts_with("check 2 proposes") {
            assert!(
                detail.contains("Also: Check 2 needs a non-empty failure_case"),
                "{detail}"
            );
        }
        assert!(failure.feedback.is_some());
        assert_eq!(
            fs::read_to_string(session.join("ledger.json")).unwrap(),
            before
        );
        assert!(!session.join("seed-contract.md").exists());
        assert!(!session.join("seed-checks.json").exists());
        assert_eq!(agent.remaining(), 0, "no automatic retry");
    }
}

#[test]
fn a_goal_past_32_kib_still_reaches_the_model() {
    // #593: a 59,795-byte goal was refused after seven Shape answers under the old 32 KiB cap.
    let root = tempfile::Builder::new()
        .prefix("TEST-593-size-")
        .tempdir()
        .unwrap();
    let session = Config::new(Some(root.path().join("TEST-sessions")))
        .confirm(Affirmed {
            goal: "TEST book loans".into(),
            outcome: "TEST two loans survive restart".into(),
            options: vec![],
            originals: vec![],
            context: "TEST Think context: ".to_owned() + &"c".repeat(60_000),
            source: "TEST fixture".into(),
        })
        .unwrap()
        .session;
    let agent = provider(root.path(), &answered_reply().to_string(), 1);
    let program = root.path().join("TEST-pi");
    fs::write(&program, "#!/usr/bin/env python3\nimport pathlib, sys\nhere = pathlib.Path(__file__)\nhere.with_name('TEST-sent').write_text(str(len(sys.stdin.buffer.read())))\nprint(here.with_name('TEST-reply.json').read_text())\n").unwrap();
    let version = answering::read(&session).unwrap().version();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let version = answering::answer(
        &session,
        version,
        answering::SAMPLE.0,
        "TEST sample: one pasted line",
        Actor::OPERATOR,
    )
    .unwrap();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let step = run(
        &Work::Answer {
            relay: None,
            id: answering::PROBE.0.into(),
            words: "TEST the answer".into(),
            expected: version,
            reads: std::path::PathBuf::from(root.path()).into(),
        },
        &session,
        &agent,
        &AtomicBool::new(false),
        &AtomicBool::new(false),
    );
    assert_eq!(step, Ok(Done::Drafted));
    let sent: usize = fs::read_to_string(root.path().join("TEST-sent"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        sent > 60_000 && sent <= crate::pi::REQUEST_LIMIT,
        "{sent} bytes sent"
    );
}

#[test]
fn a_draft_reply_without_assumed_is_refused_so_defaults_are_never_silently_dropped() {
    // #931: 2 of 2 real drafts left the optional list out while choosing routine details.
    let reply = |assumed: Option<serde_json::Value>| {
        let mut reply = serde_json::json!({"next":"seed", "readings": [], "found":[], "confirming":{},
            "checks":[{"statement":"TEST", "kind":"judge", "source":"TEST"}],
            "review":{"contradictions":[],"decided":[],"done":[],"failure_cases":["TEST wrong output must fail"]}});
        if let Some(assumed) = assumed {
            reply["assumed"] = assumed;
        }
        decode(&reply.to_string(), "")
    };
    let refused = reply(None).unwrap_err();
    assert!(
        refused.contains("field assumed must be a list"),
        "{refused}"
    );
    assert!(reply(Some(serde_json::json!("none"))).is_err());
    assert!(reply(Some(serde_json::json!([{"text": "output is CSV"}]))).is_err());
    assert!(reply(Some(serde_json::json!(["output is CSV"]))).is_err());
    let one = serde_json::json!({"text": "TEST default", "why": "TEST routine"});
    assert!(reply(Some(serde_json::json!(vec![one.clone(); 6]))).is_err());
    assert!(reply(Some(serde_json::json!(vec![one; 5]))).is_ok());
    assert!(reply(Some(serde_json::json!([]))).is_ok());
}

#[test]
fn a_draft_that_picks_one_of_two_readings_asks_it_instead_and_stays_gray() {
    // #942: two real drafts chose confirm-first over undo-after without asking, and a grant started.
    let root = tempfile::Builder::new()
        .prefix("TEST-942-reading-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-session");
    fs::create_dir_all(&session).unwrap();
    fs::write(
        session.join("ledger.json"),
        include_str!("../../../tests/fixtures/shape-296/ledger.json"),
    )
    .unwrap();
    let saved = || -> Value {
        serde_json::from_slice(&fs::read(session.join("ledger.json")).unwrap()).unwrap()
    };
    let gray_before = ledger::gray(&saved());
    let mut reply = valid_reply();
    reply["readings"] = json!([{"chosen": "TEST ask before erasing.", "other": "TEST erase and offer undo",
        "why": "TEST a confirm step and an undo build different things"}]);
    let agent = provider(root.path(), &reply.to_string(), 1);
    assert!(matches!(
        look_again(&session, root.path(), &agent),
        Ok(Done::Asked(_))
    ));
    assert!(
        !session.join("seed-checks.json").exists(),
        "the draft is not saved"
    );
    let ledger = saved();
    let gray = ledger::gray(&ledger);
    assert_eq!(gray.len(), gray_before.len() + 1, "{gray:?}");
    let asked = gray.last().unwrap();
    assert_eq!(
        asked["label"],
        "TEST ask before erasing, or TEST erase and offer undo"
    );
    assert_eq!(asked["status"], "unresolved");
    let node = ledger["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == asked["id"])
        .unwrap()
        .clone();
    assert!(
        node["question"]
            .as_str()
            .unwrap()
            .contains("TEST ask before erasing, or TEST erase and offer undo"),
        "{node}"
    );
    assert!(
        node.get("suggestion").is_none_or(Value::is_null),
        "the operator chooses: {node}"
    );

    // A draft that leaves readings out is refused, so a reading is never silently picked.
    let missing = valid_reply()
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| *k != "readings")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<serde_json::Map<_, _>>();
    let refused = decode(&Value::Object(missing).to_string(), "").unwrap_err();
    assert!(refused.contains("field readings must list"), "{refused}");
    let mut half = valid_reply();
    half["readings"] = json!([{"chosen": "TEST one way"}]);
    assert!(decode(&half.to_string(), "").is_err());
}

/// A session at its pre-mortem, answered with `words` through the step that then drafts with
/// `reply`; returns the session and that step's result.
fn drafted_after(root: &Path, words: &str, reply: &Value) -> (PathBuf, Result<Done, Failure>) {
    drafted_by(&GoalConfirmation::default(), root, words, reply)
}

/// [`drafted_after`], with every step taken by the one `confirmation` names.
fn drafted_by(
    confirmation: &GoalConfirmation,
    root: &Path,
    words: &str,
    reply: &Value,
) -> (PathBuf, Result<Done, Failure>) {
    let session = Config::new(Some(root.join("TEST-sessions")))
        .confirm_as(
            Affirmed {
                goal: "TEST rank the menu".into(),
                outcome: "TEST the order holds".into(),
                options: vec![],
                originals: vec![],
                context: "{}".into(),
                source: confirmation.source(),
            },
            confirmation.actor().unwrap(),
        )
        .unwrap()
        .session;
    let version = answering::read(&session).unwrap().version();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let version = answering::answer(
        &session,
        version,
        answering::SAMPLE.0,
        "TEST sample: one pasted line",
        Actor::OPERATOR,
    )
    .unwrap();
    let version = answering::probe(&session, version).unwrap().unwrap();
    let agent = provider(root, &reply.to_string(), 1);
    let done = Steps::new(Some(&agent), confirmation).run(
        &Work::Answer {
            relay: None,
            id: answering::PROBE.0.into(),
            words: words.into(),
            expected: version,
            reads: PathBuf::from(root).into(),
        },
        &session,
        &AtomicBool::new(false),
        &AtomicBool::new(false),
    );
    (session, done)
}

fn seed_text(session: &Path) -> String {
    fs::read_to_string(session.join("seed-contract.md")).unwrap()
}

#[test]
fn each_pre_mortem_risk_needs_a_check_or_an_out_of_scope_line_before_a_draft() {
    // #1010: the pilot's pre-mortem named both risks below; Tink r1 checked neither.
    let words = "None of the above moves out of last place, or the order changes on reload.";
    let with = |risks: Value| {
        let mut reply = valid_reply();
        reply["risks"] = risks;
        reply
    };
    for (risks, refusal) in [
        (None, "field risks is empty"),
        (Some(json!([])), "field risks is empty"),
        (
            Some(json!([{"risk": "the order changes on reload"}])),
            "field risks must list each risk",
        ),
        (
            Some(json!([{"risk": "the order is random", "out_of_scope": "TEST"}])),
            "is not copied from the settled pre-mortem answer",
        ),
        (
            Some(json!([{"risk": "the order changes on reload", "check": "Reloading keeps it."}])),
            "which is not one check's exact statement",
        ),
    ] {
        let root = tempfile::Builder::new()
            .prefix("TEST-1010-")
            .tempdir()
            .unwrap();
        let reply = risks.map_or_else(valid_reply, with);
        let (session, done) = drafted_after(root.path(), words, &reply);
        let error = done.unwrap_err();
        let reason = refused(&error.why).or(match &error.why {
            Why::Model(reason) => Some(reason.clone()),
            _ => None,
        });
        assert!(
            reason.as_deref().is_some_and(|r| r.contains(refusal)),
            "{refusal}: {error:?}"
        );
        assert!(!session.join("seed-contract.md").exists());
    }
    let root = tempfile::Builder::new()
        .prefix("TEST-1010-")
        .tempdir()
        .unwrap();
    let reply = with(json!([
        {"risk": "the order changes on reload", "check": "existing output is correct."},
        {"risk": "None of the above moves out of last place", "out_of_scope": "TEST the ranking is unchanged here"},
    ]));
    let (session, done) = drafted_after(root.path(), words, &reply);
    assert_eq!(done, Ok(Done::Drafted));
    let seed = seed_text(&session);
    let risks = seed.split_once("## Risks and verification").unwrap().1;
    for line in [
        "Pre-mortem risks, in the operator's words, each with its check or an out-of-scope line confirmed with this Tink:",
        "- Risk: \"the order changes on reload\". Check: Existing output is correct.",
        "- Risk: \"None of the above moves out of last place\". Out of scope: TEST the ranking is unchanged here",
    ] {
        assert!(risks.contains(line), "{line}\n{seed}");
    }
    // A skipped pre-mortem names no risk, so no list is owed.
    let root = tempfile::Builder::new()
        .prefix("TEST-1010-")
        .tempdir()
        .unwrap();
    let (session, done) = drafted_after(root.path(), "skip", &valid_reply());
    assert_eq!(done, Ok(Done::Drafted));
    assert!(!seed_text(&session).contains("Pre-mortem risks"));
}

#[test]
fn a_delegated_pre_mortem_bounds_the_risks_a_draft_copies_but_owes_none() {
    // The 2026-11-26 Pi Durable dogfood: Shape refused three agent-operated drafts that copied
    // their risks from the agent's own pre-mortem answer.
    let agent: GoalConfirmation = serde_json::from_value(json!({
        "by":"agent", "agent":"TEST-agent", "authorizing_operator":"TEST-person",
        "source":"TEST permission to plan this disposable menu",
        "class":{"door":"two_way", "door_reason":"TEST disposable plan",
        "blast_radius":"small", "blast_radius_reason":"TEST root only"}
    }))
    .unwrap();
    let words = "None of the above moves out of last place, or the order changes on reload.";
    let with = |risks: Value| {
        let mut reply = valid_reply();
        reply["risks"] = risks;
        reply
    };
    let draft = |reply: &Value| {
        let root = tempfile::Builder::new()
            .prefix("TEST-delegated-pre-mortem-")
            .tempdir()
            .unwrap();
        let (session, done) = drafted_by(&agent, root.path(), words, reply);
        (
            done,
            session
                .join("seed-contract.md")
                .exists()
                .then(|| seed_text(&session)),
        )
    };
    let (done, seed) = draft(&with(json!([
        {"risk": "the order changes on reload", "check": "existing output is correct."},
    ])));
    assert_eq!(done, Ok(Done::Drafted));
    let seed = seed.unwrap();
    for line in [
        "Pre-mortem risks, in the delegated agent's words, each with its check or an out-of-scope line confirmed with this Tink:",
        "- Risk: \"the order changes on reload\". Check: Existing output is correct.",
    ] {
        assert!(seed.contains(line), "{line}\n{seed}");
    }
    let (done, seed) = draft(&valid_reply());
    assert_eq!(
        done,
        Ok(Done::Drafted),
        "a delegated answer owes no risk list"
    );
    assert!(!seed.unwrap().contains("Pre-mortem risks"));
    let (done, seed) = draft(&with(
        json!([{"risk": "the order is random", "out_of_scope": "TEST"}]),
    ));
    let error = done.unwrap_err();
    assert!(
        refused(&error.why)
            .is_some_and(|r| r.contains("is not copied from the settled pre-mortem answer")),
        "{error:?}"
    );
    assert!(seed.is_none());
}

#[test]
fn a_reversion_that_edits_source_text_or_reads_git_history_refuses_the_draft() {
    // #1012: a pilot probe removed code with a regex, and the next read `git show origin/main`,
    // which failed on CI's shallow clone.
    for (field, words) in [
        (
            "cmd",
            "git show origin/main:src/menu.js > TEST-old.js && ./TEST-check",
        ),
        (
            "missing",
            "TEST the probe deletes the shuffle from the source text with a regex",
        ),
        (
            "cmd",
            "sed -i 's/shuffle//' src/menu.js && ./TEST-new-check",
        ),
    ] {
        let root = tempfile::Builder::new()
            .prefix("TEST-1012-")
            .tempdir()
            .unwrap();
        let mut reply = valid_reply();
        reply["checks"][1][field] = words.into();
        let (session, done) = drafted_after(root.path(), "skip", &reply);
        let error = done.unwrap_err();
        assert!(
            refused(&error.why).is_some_and(|r| r.contains(&format!("check 2's {field} names"))
                && r.contains("baseline built in the test or the base commit's files")),
            "{error:?}"
        );
        assert!(!session.join("seed-contract.md").exists());
    }
    // Words that only resemble those forms, and git commands that read no history, still draft.
    for (field, words) in [
        (
            "expect",
            "TEST the CORS origin/host header is used -i style",
        ),
        ("cmd", "git diff --exit-code && ./TEST-new-check"),
    ] {
        let root = tempfile::Builder::new()
            .prefix("TEST-1012-")
            .tempdir()
            .unwrap();
        let mut reply = valid_reply();
        reply["checks"][1][field] = words.into();
        let (_, done) = drafted_after(root.path(), "skip", &reply);
        assert_eq!(done, Ok(Done::Drafted), "{words}");
    }
}

#[test]
fn a_judged_check_asks_for_a_preview_unless_the_repository_declares_one() {
    // #1014: two pilot runs had no preview to open, and a hand-run preview found a layout bug.
    let ask = "Commit tinkery.preview at the project root";
    let root = tempfile::Builder::new()
        .prefix("TEST-1014-")
        .tempdir()
        .unwrap();
    let (session, done) = drafted_after(root.path(), "skip", &valid_reply());
    assert_eq!(done, Ok(Done::Drafted));
    let handoff = seed_text(&session);
    let handoff = handoff.split_once("## Downstream handoff").unwrap().1;
    assert!(handoff.contains(ask), "{handoff}");
    let root = tempfile::Builder::new()
        .prefix("TEST-1014-")
        .tempdir()
        .unwrap();
    fs::write(
        root.path().join("tinkery.preview"),
        r#"{"command":"TEST","url":"http://127.0.0.1:5173/"}"#,
    )
    .unwrap();
    let (session, done) = drafted_after(root.path(), "skip", &valid_reply());
    assert_eq!(done, Ok(Done::Drafted));
    assert!(!seed_text(&session).contains(ask));
    // Without a judged or browser check there is nothing to look at.
    let root = tempfile::Builder::new()
        .prefix("TEST-1014-")
        .tempdir()
        .unwrap();
    let mut reply = valid_reply();
    reply["checks"].as_array_mut().unwrap().pop();
    reply["review"]["failure_cases"]
        .as_array_mut()
        .unwrap()
        .pop();
    let (session, done) = drafted_after(root.path(), "skip", &reply);
    assert_eq!(done, Ok(Done::Drafted));
    assert!(!seed_text(&session).contains(ask));
    // An objective browser check is looked at too.
    let root = tempfile::Builder::new()
        .prefix("TEST-1014-")
        .tempdir()
        .unwrap();
    reply["checks"][0]["expect"] = "TEST open the page in a browser at 375px".into();
    let (session, done) = drafted_after(root.path(), "skip", &reply);
    assert_eq!(done, Ok(Done::Drafted));
    assert!(seed_text(&session).contains(ask));
}
