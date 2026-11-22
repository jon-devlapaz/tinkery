use super::*;

fn read_item(item: &serde_json::Value) -> Result<Attestation, String> {
    let run = Run::parse(&serde_json::json!({"checklist": [item]}));
    Attestation::read(&run.checklist.as_ref().unwrap()[0])
}
use serde_json::json;
use std::fs;

#[test]
fn awaiting_receipts_keep_pending_ids_out_of_untyped_metadata() {
    let source = json!({"verification_status":"awaiting-attestation", "verification":{
        "result":"awaiting-attestation", "passed":false, "pending_attestations":["seed-2", "seed-4"]
    }});
    let run = Run::parse(&source);
    assert!(run.verification.is(Verification::AwaitingAttestation));
    assert!(run.receipt_result.is(ReceiptResult::AwaitingAttestation));
    assert_eq!(run.pending_attestations.unwrap(), ["seed-2", "seed-4"]);
    assert!(
        run.data["verification"]
            .get("pending_attestations")
            .is_none()
    );
    assert_eq!(run.receipt, Receipt(source["verification"].clone()));
    for invalid in [
        Value::Null,
        json!([]),
        json!("seed-2"),
        json!([null]),
        json!([""]),
        json!(["seed-2", "seed-2"]),
        json!(["seed-2\ncommand"]),
    ] {
        let mut value = source.clone();
        value["verification"]["pending_attestations"] = invalid;
        assert!(Run::parse(&value).pending_attestations.is_err());
    }
    let mut missing = source;
    missing["verification"]
        .as_object_mut()
        .unwrap()
        .remove("pending_attestations");
    assert!(Run::parse(&missing).pending_attestations.is_err());
    assert!(
        Run::parse(&json!({"verification_status":"current"}))
            .pending_attestations
            .unwrap()
            .is_empty()
    );
    let checklist = Run::parse(&json!({"checklist":[{"status":"awaiting-attestation"}]}));
    assert!(checklist.checklist.unwrap()[0].status.known().is_none());
}

#[test]
fn checklist_obligations_fail_closed_only_for_manual_operator_items() {
    for owner in [None, Some(true), Some(false)] {
        for automatic in [false, true] {
            for status in ["pending", "failed", "unknown", "passed"] {
                for pending in [false, true] {
                    for recheck in [false, true] {
                        let mut item = json!({
                            "automatic": automatic, "status": status,
                            "pending": pending, "needs_recheck": recheck,
                        });
                        if let Some(owner) = owner {
                            item["operator_attestation"] = json!(owner);
                        }
                        let expected = if automatic || owner == Some(false) {
                            Attestation::Worker
                        } else {
                            Attestation::Operator {
                                owed: !pending && status != "passed",
                                recheck,
                            }
                        };
                        assert_eq!(read_item(&item).unwrap(), expected, "{item}");
                    }
                }
            }
        }
    }
    let valid = json!({"automatic": false, "status": "pending", "needs_recheck": false});
    assert_eq!(
        read_item(&valid).unwrap(),
        Attestation::Operator {
            owed: true,
            recheck: false
        }
    );
    for key in [
        "automatic",
        "needs_recheck",
        "status",
        "pending",
        "operator_attestation",
    ] {
        for value in [Value::Null, json!("true"), json!(1)] {
            let mut item = valid.clone();
            item[key] = value;
            assert!(read_item(&item).is_err(), "{item}");
        }
    }
    for key in ["automatic", "needs_recheck", "status"] {
        let mut item = valid.clone();
        item.as_object_mut().unwrap().remove(key);
        assert!(read_item(&item).is_err(), "{key}");
    }
}

#[test]
fn unsupported_envelopes_supply_no_runs() {
    for value in [
        json!({"protocol":"other","api_version":1,"runs":[]}),
        json!({"protocol":"tink-sdlc","api_version":"1","runs":[]}),
    ] {
        assert!(list_report(&value.to_string()).is_err());
        assert!(status_report(&value.to_string()).is_err());
    }
}

#[test]
fn a_failed_reader_start_says_what_to_check_without_os_internals() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-reader-")
        .tempdir()
        .unwrap();
    let why = run::run(
        &root.path().join("TEST-missing-checkout"),
        &["status".into(), "--json".into()],
        &AtomicBool::new(false),
        DEADLINE,
    )
    .unwrap_err();
    assert_eq!(
        why,
        "Couldn't start sdlc.py status. Check that it is installed and on PATH."
    );
}

#[test]
fn label_failure_names_label_not_the_script() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-label-command-")
        .tempdir()
        .unwrap();
    let scripts = root.path().join("_system/scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(
        scripts.join("sdlc.py"),
        "import sys\nassert sys.argv[1] == 'label'\nprint('Error: TEST label refused', file=sys.stderr)\nsys.exit(3)\n",
    )
    .unwrap();
    let why = run::run(
        root.path(),
        &["label".into()],
        &AtomicBool::new(false),
        DEADLINE,
    )
    .unwrap_err();
    assert_eq!(
        why,
        "sdlc.py label exited with exit status: 3: Error: TEST label refused"
    );
}

#[test]
fn every_refusal_prefers_json_then_sdlc_error_then_last_stderr_line() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-command-diagnostics-")
        .tempdir()
        .unwrap();
    let scripts = root.path().join("_system/scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(
        scripts.join("sdlc.py"),
        "import sys\nsys.stdout.write(sys.argv[2])\nsys.stderr.write(sys.argv[3])\nsys.exit(3)\n",
    )
    .unwrap();
    for (output, stderr, expected) in [
        (
            "",
            "sdlc.py: error: TEST option refused\nTEST last diagnostic\n",
            "sdlc.py: error: TEST option refused",
        ),
        (
            "",
            "TEST first diagnostic\nTEST last diagnostic\n\n",
            "TEST last diagnostic",
        ),
        (
            "{\"error\":{\"message\":\"TEST refusal\"}}",
            "sdlc.py: error: TEST option refused\n",
            "TEST refusal",
        ),
    ] {
        let why = run::run(
            root.path(),
            &["TEST".into(), output.into(), stderr.into()],
            &AtomicBool::new(false),
            DEADLINE,
        )
        .unwrap_err();
        assert_eq!(
            why,
            format!("sdlc.py TEST exited with exit status: 3: {expected}")
        );
    }
}

#[test]
fn generated_rules_are_dropped_only_from_well_formed_blocks() {
    let authored = b"TEST authored\n".as_slice();
    let rules = b"TEST authored\n\n<!-- tink:rules begin skillset=TEST-skillset -->\nTEST rules\n<!-- tink:rules end -->\n";
    assert_eq!(without_generated_rules(rules), authored);
    let trailing = b"TEST authored\n\n<!-- tink:rules begin -->\nTEST rules\n<!-- tink:rules end -->\nTEST after\n";
    assert_eq!(
        without_generated_rules(trailing),
        b"TEST authored\nTEST after\n"
    );
    for unbalanced in [
        b"TEST\n<!-- tink:rules begin -->\nTEST rules\n".as_slice(),
        b"TEST\n<!-- tink:rules end -->\n",
        b"<!-- tink:rules begin -->\n<!-- tink:rules begin -->\n<!-- tink:rules end -->\n",
    ] {
        assert_eq!(without_generated_rules(unbalanced), unbalanced);
    }
    assert_eq!(without_generated_rules(authored), authored);
}
