//! Byte-level goldens for the stored ledger: schema-1 files written before the typed ledger
//! must read and republish to exactly these bytes.

use super::*;
use std::fs;

const STAMP: &str = "2026-10-19T00:00:00+00:00";

/// Publish a changed draft goal over `stored` and return the new file bytes or the refusal.
fn republish(stored: &str) -> std::result::Result<String, String> {
    let root = tempfile::Builder::new()
        .prefix("TEST-ledger-golden-")
        .tempdir()
        .unwrap();
    fs::write(root.path().join("ledger.json"), stored).unwrap();
    let ledger = load(root.path())?;
    let mut state = super::super::editable(&ledger)?;
    state["draft"].set("goal", "TEST changé — \u{1F600} \"q\" \\ \u{7}".into());
    publish(
        root.path(),
        &state,
        &ledger["version"],
        &"TEST update".into(),
        None,
        STAMP,
    )
    .map_err(|e| e.to_string())?;
    Ok(fs::read_to_string(root.path().join("ledger.json")).unwrap())
}

fn base(extra: &str, version: &str, nodes: &str) -> String {
    format!(
        r#"{{"session_id":"TEST-session","schema_version":1,"version":{version},"revision":0,{extra}
        "created_at":"2026-10-19T00:00:00+00:00","updated_at":"2026-10-19T00:00:00+00:00",
        "status":"active","draft":{{"goal":"","outcome":"","options":[]}},"goal":null,
        "origin":null,"current_question":null,"frontier":[],"nodes":{nodes},"assumed":[],"operator":"human"}}"#
    )
}

#[test]
fn republished_ledgers_keep_their_golden_bytes() {
    let goal = r#"{"id":"goal","kind":"decision","status":"settled","prerequisites":[],"evidence":[],"owner":"TEST","gate":"TEST","authority":"user","authority_source":"TEST fixture only","answer":"TEST goal ü","history":[],"revision":0}"#;
    let probe = format!(
        r#"[{goal},{{"id":"probe-pre-mortem","kind":"decision","status":"unresolved","prerequisites":[],"evidence":[],"owner":"user","gate":"commitment","label":"Pre-mortem","question":"TEST what fails?\n\nOptions: A — a; B — b\nAgainst my suggestion: TEST","recommendation":"A","history":[],"revision":0}}]"#
    );
    let probe = base("", "3", &probe)
        .replace(r#""goal":null"#, r#""goal":"TEST goal ü""#)
        .replace(r#""origin":null"#, r#""origin":"goal""#)
        .replace(r#""frontier":[]"#, r#""frontier":["probe-pre-mortem"]"#);
    let swap = |from: &str, to: &str| base("", "0", "[]").replace(from, to);
    for (name, stored) in [
        ("goal_float", swap(r#""goal":null"#, r#""goal":1.0"#)),
        ("goal_nan", swap(r#""goal":null"#, r#""goal":NaN"#)),
        ("goal_string_int", swap(r#""goal":null"#, r#""goal":"7""#)),
        (
            "question_float",
            swap(r#""current_question":null"#, r#""current_question":1e16"#),
        ),
        (
            "question_nan",
            swap(r#""current_question":null"#, r#""current_question":NaN"#),
        ),
        (
            "frontier_float",
            swap(r#""frontier":[]"#, r#""frontier":[2E3]"#),
        ),
        (
            "revision_bool",
            swap(r#""revision":0"#, r#""revision":false"#),
        ),
        ("version_string", swap(r#""version":0"#, r#""version":"0""#)),
        (
            "schema_float",
            swap(r#""schema_version":1"#, r#""schema_version":1.0"#),
        ),
        (
            "dup_key",
            swap(
                r#""status":"active""#,
                r#""status":"stopped","status":"active""#,
            ),
        ),
        ("crlf", swap(",", ",\r\n")),
        ("plain", base("", "0", "[]")),
        (
            "floats",
            base(
                r#""extra":[1.0,1e16,1.5e-7,-0.0,0.1,123456789012345678901234567890,2E3],"#,
                "0",
                "[]",
            ),
        ),
        ("nan", base(r#""extra":NaN,"#, "0", "[]")),
        ("infinity", base(r#""extra":-Infinity,"#, "0", "[]")),
        ("big_version", base("", "18446744073709551616", "[]")),
        ("float_version", base("", "0.0", "[]")),
        (
            "unicode",
            swap(
                r#""outcome":"""#,
                r#""outcome":"é\u00e9\ud83d\ude00\u001f\/\t\u2028\u007f""#,
            ),
        ),
        ("probe", probe),
    ] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/seed_me/ledger/storage/goldens");
        let (file, actual) = match republish(&stored) {
            Ok(bytes) => (format!("{name}.json"), bytes),
            Err(reason) => (format!("{name}.refused"), reason + "\n"),
        };
        let expected = fs::read_to_string(dir.join(&file)).unwrap();
        assert_eq!(actual, expected, "golden {file}");
    }
}
