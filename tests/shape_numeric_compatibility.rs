//! Public Shape ledger actions preserve legacy numeric reads, exact refusals and publication data.

use std::{fs, path::PathBuf};
use tinkery::seed_me::{
    answering::{self, Refusal},
    ledger::{self, Value},
};

fn fixture(raw: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-W05-")
        .tempdir()
        .unwrap();
    let stamp = "2026-10-19T00:00:00+00:00";
    let session = ledger::initialize(&dir.path().join("sessions"), stamp).unwrap();
    let mut state = ledger::editable(&ledger::load(&session).unwrap())
        .unwrap()
        .to_json()
        .unwrap();
    state["goal"] = "TEST goal".into();
    state["origin"] = "goal".into();
    state["nodes"] = serde_json::json!([{"id":"goal","kind":"decision","status":"settled",
        "prerequisites":[],"evidence":[],"answer":"TEST goal","authority":"user",
        "authority_source":"TEST only","owner":"TEST operator","gate":"TEST gate",
        "label":"TEST_NUMERIC_LITERAL"}]);
    ledger::publish(
        &session,
        &Value::from(state),
        &Value::Int("0".into()),
        &"TEST goal".into(),
        None,
        stamp,
    )
    .unwrap();
    let path = session.join("ledger.json");
    let original = fs::read_to_string(&path).unwrap();
    let authored = original.replace("\"TEST_NUMERIC_LITERAL\"", raw);
    assert_ne!(authored, original);
    fs::write(path, authored).unwrap();
    (dir, session)
}

#[test]
fn editable_read_retains_legacy_float_projection_without_writing() {
    let (_dir, session) = fixture("2.08403533043113e+242");
    let path = session.join("ledger.json");
    let before = fs::read(&path).unwrap();
    let context = answering::read(&session)
        .unwrap()
        .context_without_findings();
    assert_eq!(
        context["nodes"][0]["label"].as_f64().unwrap().to_bits(),
        0x723f4114006c08c6
    );
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn publication_preserves_supported_numbers_order_and_stale_refusal() {
    for raw in [
        "2.08403533043113e+242",
        "1.5",
        "-0.0",
        "18446744073709551615",
        "18446744073709551616",
        r#"{"z":[null,true,-0.0,1.5,18446744073709551615],"a":"  TEST café\r\n👩‍💻  "}"#,
    ] {
        let (_dir, session) = fixture(raw);
        let before = answering::read(&session)
            .unwrap()
            .context_without_findings()["nodes"][0]["label"]
            .clone();
        assert_eq!(answering::probe(&session, 1).unwrap(), Some(2));
        let saved = ledger::load(&session).unwrap().to_json().unwrap();
        assert_eq!(saved["nodes"][0]["label"], before);
        let path = session.join("ledger.json");
        let bytes = fs::read(&path).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        if raw == "-0.0" {
            assert!(text.contains("\"label\": -0.0"));
        }
        if raw == "18446744073709551615" {
            assert_eq!(saved["nodes"][0]["label"].as_u64(), Some(u64::MAX));
        }
        if raw.starts_with('{') {
            assert!(text.find("\"a\":").unwrap() < text.find("\"z\":").unwrap());
        }
        assert_eq!(answering::probe(&session, 1), Err(Refusal::Stale));
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn unsupported_numbers_keep_exact_read_and_action_refusals_without_saving() {
    let cases = [
        (
            "NaN".to_owned(),
            "Cannot read this session: Out of range float values are not JSON compliant: nan",
        ),
        (
            "Infinity".into(),
            "Cannot read this session: Out of range float values are not JSON compliant: inf",
        ),
        (
            "-Infinity".into(),
            "Cannot read this session: Out of range float values are not JSON compliant: -inf",
        ),
        (
            "9".repeat(400),
            "Invalid session JSON: number out of range at line 20 column 415",
        ),
        (
            "9".repeat(4301),
            "Cannot read this session: Exceeds the limit (4300 digits) for integer string conversion: value has 4301 digits; use sys.set_int_max_str_digits() to increase the limit",
        ),
    ];
    for (raw, reason) in cases {
        let (_dir, session) = fixture(&raw);
        let path = session.join("ledger.json");
        let before = fs::read(&path).unwrap();
        assert_eq!(answering::read(&session).unwrap_err().to_string(), reason);
        assert_eq!(
            answering::probe(&session, 1),
            Err(Refusal::Refused(reason.into()))
        );
        assert_eq!(fs::read(path).unwrap(), before);
    }
}
