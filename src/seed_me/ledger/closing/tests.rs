//! Closing protections, without models or the operator's sessions.

use super::*;
use crate::seed_me::ledger::{self, TeachBackComparison, TeachBackRecord};

fn value(text: &str) -> Value {
    Value::parse(text).unwrap()
}

#[test]
fn confirmation_summary_names_risks_once_and_counts_exact_acceptance() {
    let ledger = value(
        r#"{"origin":"goal","nodes":[
        {"id":"goal","kind":"decision","status":"settled","answer":"goal"},
        {"id":"changed","kind":"decision","status":"settled","label":"Storage","answer":"remote","authority":"user",
         "history":[{"state":{"status":"settled","answer":"local"}}]},
        {"id":"delegate","kind":"decision","status":"settled","answer":"suggested","authority":"delegated","recommendation":"suggested"},
        {"id":"same","kind":"decision","status":"settled","answer":"yes","authority":"user","recommendation":"yes"},
        {"id":"different","kind":"decision","status":"settled","answer":"yes ","authority":"user","recommendation":"yes"},
        {"id":"fact","kind":"fact","status":"settled","answer":"yes","recommendation":"yes"},
        {"id":"open","kind":"decision","status":"unresolved","recommendation":"yes"}],
        "assumed":[{"text":"Markdown","why":"Either format works"}]}"#,
    );
    let summary = confirmation_summary(&ledger);
    assert_eq!(
        summary,
        vec![
            "Accepted as suggested: 2; chosen otherwise: 2.",
            "Storage: remote — reverses an earlier answer",
            "delegate: suggested — delegated; accepted exactly as suggested",
            "Assumed: Markdown — Either format works",
            "same: yes — accepted exactly as suggested",
        ]
    );
}

fn review() -> CompletionReview {
    serde_json::from_value(serde_json::json!({"contradictions":[],"decided":["TEST agreed scope"],
        "done":[],"failure_cases":[{"statement":"TEST exact words  ","case":"TEST wrong output must fail"}]})).unwrap()
}

#[test]
fn completion_review_requires_every_failure_case_and_refuses_blockers() {
    let checks = value(r#"[{"statement":"TEST exact words  ","kind":"judge","source":"TEST"}]"#);
    let good = review();
    good.validate(&checks).unwrap();
    for bad in [
        CompletionReview {
            contradictions: vec!["TEST conflicting choices".into()],
            ..good.clone()
        },
        CompletionReview {
            decided: vec![" ".into()],
            ..good.clone()
        },
        CompletionReview {
            failure_cases: vec![],
            ..good.clone()
        },
        CompletionReview {
            failure_cases: vec![good.failure_cases[0].clone(); 2],
            ..good.clone()
        },
        CompletionReview {
            failure_cases: vec![FailureCase {
                statement: "TEST exact words".into(),
                case: "TEST wrong".into(),
            }],
            ..good.clone()
        },
        CompletionReview {
            failure_cases: vec![FailureCase {
                case: " ".into(),
                ..good.failure_cases[0].clone()
            }],
            ..good.clone()
        },
    ] {
        assert!(bad.validate(&checks).is_err(), "accepted {bad:?}");
    }
}

fn draft() -> (tempfile::TempDir, String) {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-teach-back-")
        .tempdir()
        .unwrap();
    ledger::fixtures::confirmed(root.path(), "TEST goal", "r1");
    let mut ledger = ledger::load(root.path()).unwrap();
    ledger["nodes"].list_mut()[0].set("evidence", Value::Array(vec![]));
    ledger["nodes"].list_mut()[0].set("history", Value::Array(vec![]));
    std::fs::write(root.path().join("ledger.json"), ledger.pretty().unwrap()).unwrap();
    let seed = std::fs::read_to_string(root.path().join("seed-contract.md"))
        .unwrap()
        .replacen("status: confirmed for intake", "status: draft", 1)
        .lines()
        .filter(|line| !line.starts_with("Confirmed by:"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(root.path().join("seed-contract.md"), &seed).unwrap();
    (root, seed)
}
const AT: &str = "2026-10-20T00:00:00+00:00";

#[test]
fn completion_review_is_verbatim_byte_bound_append_only_and_not_goal_evidence() {
    let (root, seed) = draft();
    let before = ledger::load(root.path()).unwrap();
    let version = before["version"].pretty().unwrap().trim().parse().unwrap();
    let mut review = review();
    review.decided = vec!["TEST café scope.\r\nKeep words.  ".into()];
    review.done = vec!["TEST observed only.\r\nNot executed.  ".into()];
    review.failure_cases[0].case = "TEST wrong result\r\n  ".into();
    let next = ledger::record_teach_back(
        root.path(),
        version,
        "r1",
        &seed,
        &TeachBackRecord::Review(&review),
        AT,
    )
    .unwrap();
    let saved = ledger::load(root.path()).unwrap();
    let receipts = saved["nodes"].items()[0]["evidence"].items();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0]["artifact"].is("tinkery:review/completed"));
    assert_eq!(
        receipts[0]["checked"].string(),
        Some(
            format!(
                "Review r1 · {}",
                super::super::seed::sha256(seed.as_bytes())
            )
            .as_str()
        )
    );
    assert_eq!(
        serde_json::from_str::<CompletionReview>(receipts[0]["observed"].string().unwrap())
            .unwrap(),
        review
    );
    assert!(!ledger::evidence_differs(
        &before["nodes"].items()[0],
        &saved["nodes"].items()[0]
    ));
    assert!(ledger::completion_report(&saved, "r2", &seed).is_empty());
    assert!(ledger::completion_report(&saved, "r1", &(seed.clone() + "changed")).is_empty());
    let report = ledger::completion_report(&saved, "r1", &seed);
    assert!(report[0].contains(&review.decided[0]));
    assert!(report[1].contains(&review.done[0]));
    assert_eq!(report[2], "Drafting step guess:");
    for (lines, failure) in report[3..]
        .as_chunks::<2>()
        .0
        .iter()
        .zip(&review.failure_cases)
    {
        assert_eq!(lines[0], format!("Must fail: {}", failure.case));
        assert_eq!(lines[1], format!("  Check: {}", failure.statement));
    }
    assert_eq!(report.len(), 3 + review.failure_cases.len() * 2);
    let mut erased = ledger::editable(&saved).unwrap();
    erased["nodes"].list_mut()[0].set("evidence", Value::Array(vec![]));
    assert!(
        ledger::transition(
            &saved,
            &erased,
            &saved["version"],
            &"TEST erase review".into(),
            None,
            AT
        )
        .is_err()
    );
    review.done.clear();
    ledger::record_teach_back(
        root.path(),
        next,
        "r1",
        &seed,
        &TeachBackRecord::Review(&review),
        AT,
    )
    .unwrap();
    let latest = ledger::load(root.path()).unwrap();
    assert_eq!(
        latest["nodes"].items()[0]["evidence"].items()[0],
        receipts[0]
    );
    assert!(
        ledger::completion_report(&latest, "r1", &seed)
            .iter()
            .any(|line| line.ends_with("Nothing observed done."))
    );
    assert!(ledger::teach_back_report(&latest, "r1", &seed).is_empty());
    ledger::confirm_with_teach_back(root.path(), "r1", "TEST confirm review", AT).unwrap();
    assert!(ledger::verify_handoff(root.path(), &ledger::load(root.path()).unwrap()).unwrap()["verified"].truth());
}

#[test]
fn teach_back_preserves_words_and_mismatches_without_holding_confirmation() {
    let (root, seed) = draft();
    let before = ledger::load(root.path()).unwrap();
    let version: u64 = before["version"].pretty().unwrap().trim().parse().unwrap();
    let comparison = TeachBackComparison {
        mismatches: vec!["TEST no capture".into()],
    };
    assert!(
        ledger::record_teach_back(
            root.path(),
            version,
            "r1",
            &seed,
            &TeachBackRecord::Compared(&comparison),
            AT
        )
        .is_err()
    );
    let words = "TEST café will build sync.\r\nWill not delete.  ";
    let next = ledger::record_teach_back(
        root.path(),
        version,
        "r1",
        &seed,
        &TeachBackRecord::Words(words),
        AT,
    )
    .unwrap();
    let comparison = TeachBackComparison {
        mismatches: vec!["TEST sync is out.\r\nKeep this exactly.  ".into()],
    };
    ledger::record_teach_back(
        root.path(),
        next,
        "r1",
        &seed,
        &TeachBackRecord::Compared(&comparison),
        AT,
    )
    .unwrap();
    let saved = ledger::load(root.path()).unwrap();
    let receipts = saved["nodes"].items()[0]["evidence"].items();
    assert_eq!(receipts[0]["observed"].string(), Some(words));
    assert_eq!(
        receipts[1]["observed"].string(),
        Some(comparison.mismatches[0].as_str())
    );
    assert_eq!(saved["goal"], before["goal"]);
    assert_eq!(
        ledger::decisions_block(&saved).unwrap(),
        ledger::decisions_block(&before).unwrap()
    );
    assert!(!ledger::evidence_differs(
        &before["nodes"].items()[0],
        &saved["nodes"].items()[0]
    ));
    assert!(!ledger::review_flags(&saved["nodes"]).truth());
    let report = ledger::teach_back_report(&saved, "r1", &seed);
    assert!(report[0].contains(words));
    assert!(report[1].contains(&comparison.mismatches[0]));
    assert!(ledger::teach_back_report(&saved, "r1", &(seed.clone() + "changed")).is_empty());
    ledger::confirm_with_teach_back(root.path(), "r1", "TEST confirm despite mismatch", AT)
        .unwrap();
    assert!(ledger::verify_handoff(root.path(), &ledger::load(root.path()).unwrap()).unwrap()["verified"].truth());
    assert!(
        !ledger::load(root.path()).unwrap()["nodes"].items()[0]["evidence"]
            .items()
            .iter()
            .any(|r| r["observed"].is(ledger::TEACH_BACK_SKIPPED))
    );
}

#[test]
fn teach_back_does_not_reopen_goal_dependents_or_hide_real_evidence_changes() {
    let (root, seed) = draft();
    let before = ledger::load(root.path()).unwrap();
    let mut state = ledger::editable(&before).unwrap();
    let mut child = state["nodes"].items()[0].clone();
    child["id"] = "dependent".into();
    child["answer"] = "TEST dependent choice".into();
    child["prerequisites"] = Value::Array(vec!["goal".into()]);
    state["nodes"].list_mut().push(child);
    let saved = ledger::publish(
        root.path(),
        &state,
        &before["version"],
        &"TEST dependency".into(),
        None,
        AT,
    )
    .unwrap();
    let old_block = ledger::decisions_block(&before).unwrap();
    assert!(seed.contains(&old_block));
    let seed = seed.replace(&old_block, &ledger::decisions_block(&saved).unwrap());
    std::fs::write(root.path().join("seed-contract.md"), &seed).unwrap();
    let version = saved["version"].pretty().unwrap().trim().parse().unwrap();
    ledger::record_teach_back(
        root.path(),
        version,
        "r1",
        &seed,
        &TeachBackRecord::Words("TEST teach-back is not new goal evidence"),
        AT,
    )
    .unwrap();
    let captured = ledger::load(root.path()).unwrap();
    assert_eq!(
        captured["nodes"].items()[1]["status"].string(),
        Some("settled")
    );
    assert_eq!(
        captured["nodes"].items()[1]["premise_version"],
        saved["nodes"].items()[1]["premise_version"]
    );
    assert!(!ledger::review_flags(&captured["nodes"]).truth());
    let mut real_change = ledger::editable(&captured).unwrap();
    real_change["nodes"].list_mut()[0]["evidence"]
        .list_mut()
        .push("TEST genuine new evidence".into());
    let error = ledger::transition(
        &captured,
        &real_change,
        &captured["version"],
        &"TEST evidence change".into(),
        None,
        AT,
    )
    .unwrap_err();
    assert!(
        error.contains(
            "affected settlement requires explicit revalidation with evidence: dependent"
        ),
        "{error}"
    );
    ledger::confirm_with_teach_back(
        root.path(),
        "r1",
        "TEST confirmation with captured words and skip",
        AT,
    )
    .unwrap();
    let confirmed = ledger::load(root.path()).unwrap();
    assert_eq!(confirmed["goal"], before["goal"]);
    assert_eq!(
        confirmed["nodes"].items()[1]["status"].string(),
        Some("settled")
    );
    assert!(ledger::verify_handoff(root.path(), &confirmed).unwrap()["verified"].truth());
}

#[test]
fn teach_back_skip_is_recorded_verbatim_only_after_a_valid_confirmation() {
    let (root, seed) = draft();
    let before = std::fs::read(root.path().join("ledger.json")).unwrap();
    assert!(ledger::confirm_with_teach_back(root.path(), "r2", "TEST wrong revision", AT).is_err());
    assert_eq!(
        std::fs::read(root.path().join("ledger.json")).unwrap(),
        before
    );
    ledger::confirm_with_teach_back(root.path(), "r1", "TEST confirm without teach-back", AT)
        .unwrap();
    let saved = ledger::load(root.path()).unwrap();
    let receipts = saved["nodes"].items()[0]["evidence"].items();
    assert_eq!(receipts.len(), 1);
    assert_eq!(
        receipts[0]["observed"].string(),
        Some(ledger::TEACH_BACK_SKIPPED)
    );
    assert!(
        receipts[0]["checked"]
            .string()
            .unwrap()
            .contains("Teach-back r1")
    );
    assert!(
        !ledger::teach_back_report(&saved, "r1", &seed)
            .iter()
            .any(|s| s.contains("mismatch"))
    );
}

#[test]
fn teach_back_refuses_stale_drafts_secrets_and_erasure() {
    let (root, seed) = draft();
    let before = ledger::load(root.path()).unwrap();
    let version = before["version"].pretty().unwrap().trim().parse().unwrap();
    let bytes = std::fs::read(root.path().join("ledger.json")).unwrap();
    for (expected, text, words) in [
        (version - 1, seed.clone(), "TEST words"),
        (version, seed.clone() + "changed", "TEST words"),
        (version, seed.clone(), "AKIAABCDEFGHIJKLMNOP"),
    ] {
        assert!(
            ledger::record_teach_back(
                root.path(),
                expected,
                "r1",
                &text,
                &TeachBackRecord::Words(words),
                AT
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(root.path().join("ledger.json")).unwrap(),
            bytes
        );
    }
    ledger::record_teach_back(
        root.path(),
        version,
        "r1",
        &seed,
        &TeachBackRecord::Words("TEST original"),
        AT,
    )
    .unwrap();
    let saved = ledger::load(root.path()).unwrap();
    let mut state = ledger::editable(&saved).unwrap();
    state["nodes"].list_mut()[0].set("evidence", Value::Array(vec![]));
    let error = ledger::transition(
        &saved,
        &state,
        &saved["version"],
        &"TEST erase".into(),
        None,
        AT,
    )
    .unwrap_err();
    assert!(error.contains("Preserve teach-back"), "{error}");
}
