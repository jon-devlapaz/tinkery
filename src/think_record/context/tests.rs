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
        &Value::Null,
    )
    .details
}
#[test]
fn context_order_survives_equal_or_backward_wall_clocks_and_preserves_frozen_text() {
    let mut context = ReadingContext::default();
    context.submitted(1);
    context.presented(board(), UNIX_EPOCH + Duration::from_secs(100));
    context.submitted(2);
    context.presented(board(), UNIX_EPOCH + Duration::from_secs(50));
    let value = serde_json::to_value(&context).unwrap();
    let result = history(
        &value,
        words()
            .iter()
            .map(|event| (event, "2026-10-24T23:16:53Z".into())),
        &Value::Null,
    );
    let sequence = result.sequence;
    let text = result.details;
    assert_eq!(result.readings.len(), 2);
    assert_eq!(result.readings[0].main, ["TEST exact frozen wording"]);
    assert_eq!(result.readings[1].main, result.readings[0].main);
    assert_eq!(result.readings[0].at, "1970-01-01T00:01:40.000Z");
    assert_eq!(result.readings[1].at, "1970-01-01T00:00:50.000Z");
    assert_eq!(
        sequence.lines().collect::<Vec<_>>(),
        vec![
            "Recorded sequence · 24 Oct 2026, UTC",
            "23:16:53  Original words 1",
            "1 Jan 1970",
            "00:01:40  Reading 1 shown",
            "24 Oct 2026",
            "23:16:53  Answer 2 · after reading 1",
            "1 Jan 1970",
            "00:00:50  Reading 2 shown",
            "Shown means it reached the screen, not that it was read.",
        ]
    );
    assert!(!sequence.contains("TEST exact frozen wording"));
    assert_eq!(sequence.matches("Shown means").count(), 1);
    assert!(text.contains("Original words 1 at 2026-10-24T23:16:53Z."));
    assert!(text.contains("Answer 2 · after reading 1 at 2026-10-24T23:16:53Z."));
    assert!(text.contains("TEST exact frozen wording"));
    assert!(text.contains("Presented reading 2 at 1970-01-01T00:00:50.000Z"));
    assert_eq!(
        check(Value::Null, &words()),
        "Reading context: not recorded."
    );
}
#[test]
fn source_labels_follow_event_kinds_and_only_presentations_add_relations() {
    let mut events = words();
    events.push(Event::ScopeAnswered {
        id: "scope".into(),
        question: "Same goal?".into(),
        source: 4,
        answer: "TEST yes".into(),
    });
    let value = serde_json::json!({"events":[
        {"kind":"submitted","source":1},
        {"kind":"submitted","source":2},
        {"kind":"presented","at":"2026-10-27T23:59:59.999Z","board":board()},
        {"kind":"submitted","source":4},
        {"kind":"presented","at":"2026-10-28T00:00:00Z","board":board()}
    ]});
    let result = history(
        &value,
        events.iter().enumerate().map(|(index, event)| {
            (
                event,
                if index == 2 {
                    "2026-10-28T02:00:00+02:00"
                } else {
                    "2026-10-27T23:59:58Z"
                }
                .into(),
            )
        }),
        &Value::Null,
    );
    assert_eq!(
        result.sequence.lines().collect::<Vec<_>>(),
        [
            "Recorded sequence · 27 Oct 2026, UTC",
            "23:59:58  Original words 1",
            "23:59:58  Answer 2",
            "23:59:59  Reading 1 shown",
            "28 Oct 2026",
            "00:00:00  Answer 4 · after reading 1",
            "00:00:00  Reading 2 shown",
            "Shown means it reached the screen, not that it was read.",
        ]
    );
    for (event, label) in events
        .iter()
        .zip(["Original words 1", "Answer 2", "Answer 4"])
    {
        assert_eq!(event.source_label().as_deref(), Some(label));
        assert!(result.sequence.contains(label));
        assert!(result.details.contains(label));
    }
    assert_eq!(result.readings[0].heading, "Reading 1 · shown 23:59:59");
    assert_eq!(
        result.readings[1].heading,
        "Reading 2 · shown 00:00:00, 28 Oct 2026"
    );
    assert_eq!(result.sequence.matches("after reading").count(), 1);
    assert!(!result.sequence.contains("no earlier") && !result.sequence.contains("Submission"));
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
        serde_json::json!([[{"text":"TEST positional board"}],[],[]]),
        serde_json::json!({"framings":[["TEST positional framing"]],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[{"text":5}],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[{"text":"  "}],"other_goals":[],"deferred":[]}),
        serde_json::json!({"framings":[],"other_goals":[5]}),
        serde_json::json!({"framings":[{"text":"TEST valid"}],"other_goals":[]}),
        serde_json::json!({"framings":[{"text":"TEST valid"}],"deferred":[]}),
        serde_json::json!({"other_goals":["TEST valid"],"deferred":[]}),
        serde_json::json!({"framings":[{"text":"\t"}],"other_goals":[" "],"deferred":["\n"]}),
    ] {
        let mut bad = good.clone();
        bad["events"][1]["board"] = board;
        cases.push(bad);
    }
    for bad in cases {
        let result = history(
            &bad,
            words()
                .iter()
                .map(|event| (event, "2026-10-24T23:16:53Z".into())),
            &Value::Null,
        );
        assert_eq!(
            result.sequence,
            "Reading sequence unavailable; saved words remain below."
        );
        assert!(!result.sequence.contains("after reading"));
        assert!(result.details.contains("unavailable"));
        assert!(!result.details.contains("after reading"));
        assert!(result.readings.is_empty());
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
fn sequence_converts_offsets_to_utc_and_retains_exact_times_in_details() {
    let events = words();
    let value = serde_json::json!({"events":[{"kind":"submitted","source":1},
        {"kind":"presented","at":"2026-10-25T01:00:00+02:00","board":board()},
        {"kind":"submitted","source":2}]});
    let result = history(
        &value,
        events
            .iter()
            .map(|event| (event, "2026-10-24T23:16:53-05:00".into())),
        &Value::Null,
    );
    let sequence = result.sequence;
    let details = result.details;
    assert_eq!(
        sequence.lines().collect::<Vec<_>>(),
        [
            "Recorded sequence · 25 Oct 2026, UTC",
            "04:16:53  Original words 1",
            "24 Oct 2026",
            "23:00:00  Reading 1 shown",
            "25 Oct 2026",
            "04:16:53  Answer 2 · after reading 1",
            "Shown means it reached the screen, not that it was read.",
        ]
    );
    assert_eq!(
        result.readings[0].heading,
        "Reading 1 · shown 23:00:00, 24 Oct 2026"
    );
    assert!(details.contains("2026-10-25T01:00:00+02:00"));
    assert!(details.contains("2026-10-24T23:16:53-05:00"));
    assert!(!sequence.contains("+02:00") && !sequence.contains("-05:00"));
    assert!(details.contains("TEST exact frozen wording"));
    let missing = history(
        &Value::Null,
        events.iter().map(|event| (event, String::new())),
        &Value::Null,
    );
    assert_eq!(missing.sequence, "Reading sequence was not recorded.");
    assert!(missing.readings.is_empty());
}

#[test]
fn reading_text_preserves_lists_whitespace_and_partial_presentations_without_recomposition() {
    let events = words();
    let frozen = serde_json::json!({
        "framings":[{"text":"  TEST frozen Café\n\nsecond  "},{"text":"TEST second framing\r\n"}],
        "other_goals":["  TEST other; preserve semicolon  ","TEST other second"],
        "deferred":["TEST deferred first","  TEST deferred second\n  "],
        "parts":{"who":"TEST invented","outcome":"MUST NEVER RECOMPOSE"}
    });
    let partial =
        serde_json::json!({"framings":[],"other_goals":[],"deferred":["  TEST partial only\n  "]});
    let value = serde_json::json!({"events":[{"kind":"submitted","source":1},
        {"kind":"presented","at":"2026-10-27T01:00:00Z","board":frozen},
        {"kind":"submitted","source":2},
        {"kind":"presented","at":"2026-10-27T00:00:00Z","board":partial}]});
    let result = history(
        &value,
        events
            .iter()
            .map(|event| (event, "2026-10-26T23:00:00Z".into())),
        &Value::Null,
    );
    assert_eq!(result.readings.len(), 2);
    assert_eq!(
        result.readings[0].main,
        ["  TEST frozen Café\n\nsecond  ", "TEST second framing\r\n"]
    );
    assert_eq!(
        result.readings[0].other_goals,
        ["  TEST other; preserve semicolon  ", "TEST other second"]
    );
    assert_eq!(
        result.readings[0].deferred,
        ["TEST deferred first", "  TEST deferred second\n  "]
    );
    assert!(result.readings[1].main.is_empty());
    assert_eq!(result.readings[1].deferred, ["  TEST partial only\n  "]);
    assert!(
        !result.readings[0]
            .main
            .iter()
            .any(|s| s.contains("MUST NEVER RECOMPOSE"))
    );
    assert!(result.sequence.contains("00:00:00  Reading 2 shown"));
}
