//! Rust-only ledger contracts retained after retiring the migration's Python oracle.
use tinkery::seed_me::ledger::{self, Value};

const STAMP: &str = "2026-10-19T00:00:00+00:00";

fn fixture() -> (tempfile::TempDir, Value, serde_json::Value) {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-ledger-rules-")
        .tempdir()
        .unwrap();
    let session = ledger::initialize(root.path(), STAMP).unwrap();
    let initial = ledger::load(&session).unwrap();
    let state = serde_json::json!({
        "status":"active","operator":"human","draft":{"goal":"","outcome":"","options":[]},
        "goal":"TEST goal","origin":"goal","current_question":null,"assumed":[],
        "nodes":[{"id":"goal","kind":"decision","status":"settled","prerequisites":[],
            "evidence":[],"answer":"TEST goal","authority":"user","authority_source":"TEST only",
            "owner":"TEST operator","gate":"TEST confirmation"},
            {"id":"question","kind":"decision","status":"unresolved","prerequisites":[],"evidence":[],
             "owner":"TEST operator","gate":"TEST choice","question":"TEST?"}]
    });
    let saved = ledger::transition(
        &initial,
        &Value::from(state),
        &Value::Int("0".into()),
        &"TEST origin".into(),
        None,
        STAMP,
    )
    .unwrap();
    let editable = ledger::editable(&saved).unwrap().to_json().unwrap();
    (root, saved, editable)
}

fn refusal(ledger: &Value, state: serde_json::Value, version: u64) -> String {
    ledger::transition(
        ledger,
        &state.into(),
        &Value::Int(version.to_string()),
        &"TEST change".into(),
        None,
        STAMP,
    )
    .unwrap_err()
}

#[test]
fn stale_versions_goal_drift_deletion_and_kind_changes_refuse() {
    let (_root, saved, state) = fixture();
    let mut stale = state.clone();
    stale["draft"]["outcome"] = "TEST stale write".into();
    assert!(refusal(&saved, stale, 0).contains("stale"));
    let mut changed = state.clone();
    changed["goal"] = "TEST drift".into();
    assert!(refusal(&saved, changed, 1).contains("goal"));
    let mut changed = state.clone();
    changed["nodes"].as_array_mut().unwrap().pop();
    assert!(refusal(&saved, changed, 1).contains("preserve nodes"));
    let mut changed = state;
    changed["nodes"][1]["kind"] = "fact".into();
    assert!(refusal(&saved, changed, 1).contains("kind"));
}

#[test]
fn prerequisites_must_exist_be_acyclic_and_settle_first() {
    let (_root, _, mut state) = fixture();
    state["nodes"].as_array_mut().unwrap().pop();
    let question = |id: &str, prerequisites: serde_json::Value| {
        serde_json::json!({
            "id":id,"kind":"decision","status":"unresolved","prerequisites":prerequisites,
            "evidence":[],"owner":"User","gate":"TEST choice","question":"TEST?"
        })
    };
    state["nodes"]
        .as_array_mut()
        .unwrap()
        .push(question("a", serde_json::json!(["missing"])));
    assert!(ledger::validate(&state.clone().into()).is_err());
    state["nodes"][1]["prerequisites"] = serde_json::json!(["b"]);
    state["nodes"]
        .as_array_mut()
        .unwrap()
        .push(question("b", serde_json::json!(["a"])));
    assert!(ledger::validate(&state.clone().into()).is_err());
    state["nodes"][2]["prerequisites"] = serde_json::json!([]);
    let ready = ledger::validate(&state.clone().into()).unwrap();
    assert_eq!(ready, ["b"]);
    state["nodes"][1]["status"] = "settled".into();
    state["nodes"][1]["answer"] = "TEST premature".into();
    state["nodes"][1]["authority"] = "user".into();
    state["nodes"][1]["authority_source"] = "TEST only".into();
    assert!(ledger::validate(&state.into()).is_err());
}

#[test]
fn receipt_times_are_strict_and_secrets_are_refused() {
    for stamp in [
        STAMP,
        "2026-10-19T03:00:00+03:00",
        "2026-10-19T00:00:00.123Z",
    ] {
        let receipt =
            serde_json::json!({"checked":"TEST file:1","observed":"TEST text","at":stamp});
        ledger::validate_receipt(&receipt.into()).unwrap();
    }
    for stamp in [
        "20261019T000000+0000",
        "2026-10-19T00:00:00",
        "2026-10-19T00:00:00junk+00:00",
        "2026-02-30T00:00:00Z",
    ] {
        let receipt =
            serde_json::json!({"checked":"TEST file:1","observed":"TEST text","at":stamp});
        assert!(
            ledger::validate_receipt(&receipt.into()).is_err(),
            "{stamp}"
        );
    }
    let receipt = serde_json::json!({"checked":"TEST file:1","observed":"password=TEST-fixture-secret","at":STAMP});
    assert!(
        ledger::validate_receipt(&receipt.into())
            .unwrap_err()
            .contains("secret")
    );
}

fn receipt(text: &str) -> Value {
    serde_json::json!({"checked":"TEST file:1", "observed":text, "at":STAMP}).into()
}

#[test]
fn secret_receipts_keep_normalization_whitespace_and_overlapping_token_boundaries() {
    for text in [
        "TEST -----BEGIN RSA PRIVATE KEY-----",
        "TEST AKIAABCDEFGHIJKLMNOP",
        "TEST ghp_abcdefghijklmnopqrst",
        "TEST sk-abcdefghijklmnopqrst",
        "TEST xoxb-abcdefghij",
        "TEST https://name:fixture@host/",
        "TEST password=fixture",
        "TEST api_key\u{1c}=\u{85}fixture",
        "TEST bearer\u{a0}abcdefghijklmnop",
        "TEST aKİaABCDEFGHIJKLMNOP",
        "TEST ſK-abcdefghijklmnopqrst",
        // A rejected candidate must not consume the separator before a valid one.
        "TEST xAKIAABCDEFGHIJKLMNOP-AKIAABCDEFGHIJKLMNOP",
        "TEST AKIAABCDEFGHIJKLMNOPx-AKIAABCDEFGHIJKLMNOP",
        "TEST xghp_abcdefghijklmnopqrst-ghp_abcdefghijklmnopqrst",
        // These characters are not alphanumeric or underscore, though regex \\w includes them.
        "TEST \u{301}AKIAABCDEFGHIJKLMNOP\u{301}",
        "TEST \u{200c}AKIAABCDEFGHIJKLMNOP\u{200c}",
        "TEST ‿AKIAABCDEFGHIJKLMNOP‿",
    ] {
        assert!(
            ledger::validate_receipt(&receipt(text))
                .unwrap_err()
                .contains("secret"),
            "accepted {text:?}"
        );
    }
    for text in [
        "TEST xAKIAABCDEFGHIJKLMNOP",
        "TEST AKIAABCDEFGHIJKLMNOPx",
        "TEST _AKIAABCDEFGHIJKLMNOP_",
        "TEST éAKIAABCDEFGHIJKLMNOP",
        "TEST AKIAABCDEFGHIJKLMNOP中",
        "TEST ²AKIAABCDEFGHIJKLMNOP",
        "TEST \u{345}AKIAABCDEFGHIJKLMNOP",
        // Newer Unicode letters recognized by Rust may be absent from regex's tables.
        "TEST \u{88f}AKIAABCDEFGHIJKLMNOP",
        "TEST AKIAABCDEFGHIJKLMNOP\u{c5c}",
        "TEST \u{a7ce}AKIAABCDEFGHIJKLMNOP",
        "TEST AKIAABCDEFGHIJKLMNOP\u{10940}",
        "TEST xghp_abcdefghijklmnopqrst",
        "TEST xsk-abcdefghijklmnopqrst",
        "TEST xxoxb-abcdefghij",
        "TEST xpassword=fixture",
        "TEST xbearer abcdefghijklmnop",
        "TEST password=short",
        "TEST ghp_short",
        "TEST ordinary text",
    ] {
        ledger::validate_receipt(&receipt(text))
            .unwrap_or_else(|reason| panic!("refused {text:?}: {reason}"));
    }
}

#[test]
fn long_ordinary_and_rejected_candidate_receipts_finish_promptly() {
    // Warm the compiled patterns before measuring. A wide deadline allows slow CI;
    // the old repeated suffix searches exceed it even for modest receipt sizes.
    ledger::validate_receipt(&receipt("TEST warm-up")).unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for size in [4_096, 8_192, 32_768, 131_072] {
            for phrase in ["TEST ordinary text ", "xsk-abcdefghijklmnopqrst-"] {
                let text = phrase.repeat(size / phrase.len());
                ledger::validate_receipt(&receipt(&text)).unwrap();
            }
        }
        let _ = sender.send(());
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("receipt screening stalled on ordinary text or rejected token candidates");
}

#[test]
fn bundled_guidance_keeps_the_pinned_source_bytes() {
    use sha2::{Digest, Sha256};
    // Seed Me c9d766760dbeac0c05c92108165f9e968bec0a66; extraction boundary is in drafting.rs.
    for (text, expected) in [
        (
            include_str!("../src/think/working-draft.txt"),
            "65f268e900cbef0fe9ddd733facdf8b6cecd274f7ee625b830b157c27a7c82b5",
        ),
        (
            include_str!("../src/seed_me/seed-contract.txt"),
            "6d52c8771f935e89420680d0c97ef62d45939ce13175ef937aa5a298bd678199",
        ),
    ] {
        assert_eq!(format!("{:x}", Sha256::digest(text.as_bytes())), expected);
    }
}

#[test]
fn typed_json_adaptation_preserves_supported_values_and_refuses_nonfinite_numbers() {
    let value = serde_json::json!({"text":"  TEST café\r\n👩‍💻  ","array":[null,true,1,2.5],"nested":{"x":-1}});
    assert_eq!(Value::from(value.clone()).to_json().unwrap(), value);
    assert!(Value::Float(f64::NAN).to_json().is_err());
}
