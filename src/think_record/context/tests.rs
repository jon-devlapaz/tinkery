use super::*;
use std::time::{Duration, UNIX_EPOCH};

fn words() -> Vec<Event> {
    vec![
        Event::Words {
            source: 1,
            text: "  TEST original\r\n".into(),
        },
        Event::Answered {
            source: 2,
            id: "q".into(),
            question: "Change?".into(),
            answer: "TEST correction".into(),
        },
    ]
}
fn board() -> Value {
    serde_json::json!({"framings":[{"text":"TEST exact frozen wording","supports":[]}],"other_goals":[],"deferred":[]})
}
fn check(value: Value, events: &[Event]) -> String {
    history(
        &value,
        events
            .iter()
            .map(|event| (event, "2026-10-24T23:16:53Z".into())),
    )
    .1
}
#[test]
fn context_order_survives_equal_or_backward_wall_clocks_and_preserves_frozen_text() {
    let mut context = ReadingContext::default();
    context.submitted(1);
    context.presented(board(), UNIX_EPOCH + Duration::from_secs(100));
    context.submitted(2);
    context.presented(board(), UNIX_EPOCH + Duration::from_secs(50));
    let value = serde_json::to_value(&context).unwrap();
    let (sequence, text) = history(
        &value,
        words()
            .iter()
            .map(|event| (event, "2026-10-24T23:16:53Z".into())),
    );
    assert_eq!(
        sequence.lines().skip(2).collect::<Vec<_>>(),
        vec![
            "2026-10-24T23:16:53Z  Submission 1 · no earlier reading recorded",
            "1970-01-01T00:01:40.000Z  Reading 1 presented",
            "2026-10-24T23:16:53Z  Submission 2 · after reading 1",
            "1970-01-01T00:00:50.000Z  Reading 2 presented",
        ]
    );
    assert!(!sequence.contains("TEST exact frozen wording"));
    assert_eq!(
        sequence.matches("presentation does not establish").count(),
        1
    );
    assert!(
        text.contains(
            "Submitted source 1 at 2026-10-24T23:16:53Z, before any recorded presentation"
        )
    );
    assert!(text.contains("Submitted source 2 at 2026-10-24T23:16:53Z, after reading 1"));
    assert!(text.contains("TEST exact frozen wording"));
    assert!(text.contains("Presented reading 2 at 1970-01-01T00:00:50.000Z"));
    assert_eq!(
        check(Value::Null, &words()),
        "Reading context: not recorded."
    );
}
#[test]
fn malformed_context_never_supplies_a_relationship() {
    let good = serde_json::json!({"events":[{"kind":"submitted","source":1},
        {"kind":"presented","at":"2026-10-24T23:13:31Z","board":board()},
        {"kind":"submitted","source":2}]});
    let mut cases = vec![serde_json::json!({}), serde_json::json!({"events":[]})];
    for source in [0, 1, 3] {
        let mut bad = good.clone();
        bad["events"][2]["source"] = source.into();
        cases.push(bad);
    }
    let mut bad = good.clone();
    bad["events"][1]["at"] = "yesterday".into();
    cases.push(bad);
    let mut bad = good.clone();
    bad["events"][1]["board"] = serde_json::json!({});
    cases.push(bad);
    let mut bad = good.clone();
    bad["events"].as_array_mut().unwrap().remove(0);
    cases.push(bad);
    for board in [
        serde_json::json!({"framings":[],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[{"text":5}],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[{"text":"  "}],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[],"other_goals":[5]}),
    ] {
        let mut bad = good.clone();
        bad["events"][1]["board"] = board;
        cases.push(bad);
    }
    for bad in cases {
        let (sequence, result) = history(
            &bad,
            words()
                .iter()
                .map(|event| (event, "2026-10-24T23:16:53Z".into())),
        );
        assert_eq!(
            sequence,
            "Reading sequence unavailable; saved words remain below."
        );
        assert!(!sequence.contains("after reading"));
        assert!(result.contains("unavailable"), "{result}");
        assert!(!result.contains("after reading"));
    }
}
#[test]
fn build_stamp_requires_full_commit_and_explicit_modified_state() {
    let commit = "1234567890abcdef1234567890abcdef12345678";
    for dirty in ["true", "false"] {
        let value = build_source(Some(commit), Some(dirty)).unwrap();
        assert_eq!(value["commit"], commit);
        assert_eq!(value["dirty"], dirty == "true");
        assert!(source_details(&value).contains("supplied at build time"));
    }
    for (commit, dirty) in [
        (None, None),
        (Some(commit), None),
        (Some("short"), Some("false")),
        (
            Some("0000000000000000000000000000000000000000"),
            Some("false"),
        ),
        (Some(commit), Some("unknown")),
    ] {
        assert_eq!(build_source(commit, dirty), None);
    }
    assert!(source_details(&Value::Null).contains("unknown"));
    assert!(source_details(&serde_json::json!({"commit":commit})).contains("unavailable"));
}

#[test]
fn sequence_preserves_recorded_timezone_and_distinguishes_missing_from_invalid() {
    let events = words();
    let value = serde_json::json!({"events":[{"kind":"submitted","source":1},
        {"kind":"presented","at":"2026-10-25T01:00:00+02:00","board":board()},
        {"kind":"submitted","source":2}]});
    let (sequence, details) = history(
        &value,
        events
            .iter()
            .map(|event| (event, "2026-10-24T23:16:53-05:00".into())),
    );
    assert!(sequence.contains("2026-10-25T01:00:00+02:00  Reading 1 presented"));
    assert!(sequence.contains("2026-10-24T23:16:53-05:00  Submission 2 · after reading 1"));
    assert!(details.contains("TEST exact frozen wording"));
    let missing = history(
        &Value::Null,
        events.iter().map(|event| (event, String::new())),
    );
    assert_eq!(missing.0, "Reading sequence was not recorded.");
}
