//! Attribution round trips and legacy controls use synthetic records in temporary TEST roots.
use super::*;
use std::fs;

const GOAL: &str =
    "TEST I keep this goal.\nWhy: exact intent matters.\nDone when: all words survive.";
const WORDS: &str = "  TEST Café 👩‍💻\n\"original\"\ttab  ";

fn class(door: Door, blast_radius: BlastRadius) -> ConfirmationClass {
    ConfirmationClass {
        door,
        door_reason: "TEST throwaway goal; nothing runs".into(),
        blast_radius,
        blast_radius_reason: "TEST isolated fixture only".into(),
    }
}

fn agent() -> GoalConfirmation {
    GoalConfirmation::Agent {
        agent: "TEST-agent".into(),
        authorizing_operator: "TEST-operator".into(),
        source: "TEST authorization reference\nverbatim second line".into(),
        class: class(Door::TwoWay, BlastRadius::Small),
    }
}

fn session(root: &Path) -> PathBuf {
    let path = root.join("TEST-sessions/one");
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("ledger.json"),
        serde_json::json!({
            "schema_version": 1, "session_id": "TEST-id", "status": "active", "goal": GOAL
        })
        .to_string(),
    )
    .unwrap();
    path.canonicalize().unwrap()
}

fn record(root: &Path, path: &Path, confirmation: &GoalConfirmation) -> Result<PathBuf, String> {
    write_with_confirmation(
        root,
        &Session {
            origin: &Origin {
                origin: "live".into(),
                model: Some("TEST/reading-model".into()),
                thinking: None,
                prompt_hash: None,
            },
            turns: &[Turn {
                at: UNIX_EPOCH,
                event: Event::Words {
                    source: 1,
                    text: WORDS.into(),
                },
            }],
            checks: vec![],
            goal: GOAL,
            seed_me_session: path,
            repo: None,
            confirmed: UNIX_EPOCH,
        },
        confirmation,
    )
}

#[test]
fn agent_confirmation_round_trips_without_becoming_operator_or_reading_model() {
    let tmp = tempfile::tempdir().unwrap();
    let path = session(tmp.path());
    let root = tmp.path().join("TEST-records");
    let confirmation = agent();
    let file = record(&root, &path, &confirmation).unwrap();
    let bytes = fs::read(&file).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json["schema"], 3,
        "additive metadata does not invalidate old readers"
    );
    assert_eq!(
        json["goal_confirmation"],
        serde_json::to_value(&confirmation).unwrap()
    );
    assert_eq!(json["goal_confirmation"]["by"], "agent");
    assert_eq!(json["goal_confirmation"]["agent"], "TEST-agent");
    assert_eq!(
        json["goal_confirmation"]["authorizing_operator"],
        "TEST-operator"
    );
    assert_eq!(
        json["goal_confirmation"]["source"],
        "TEST authorization reference\nverbatim second line"
    );
    assert_eq!(json["origin"], "live");
    assert_eq!(json["model"], "TEST/reading-model");
    assert_eq!(json["goal"], GOAL);
    assert_eq!(json["turns"][0]["text"], WORDS);
    let history = read(&root, &path, GOAL).unwrap();
    assert_eq!(history.goal, GOAL);
    assert!(history.details.contains("agent") && history.details.contains("TEST-agent"));
    assert!(history.details.contains("authorized by operator"));
    assert!(
        history
            .details
            .contains("caller-declared, not authenticated")
    );
    assert_eq!(
        read_origins(&root, &[(&path, GOAL)]).unwrap(),
        [MadeIn::NoRepo]
    );
    assert_eq!(
        fs::read(&file).unwrap(),
        bytes,
        "readback never rewrites records"
    );
}

#[test]
fn only_explicit_operator_context_records_operator_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let path = session(tmp.path());
    let operator = GoalConfirmation::Operator {
        operator: "TEST-person".into(),
        source: "TEST explicit self-confirmation".into(),
        class: class(Door::OneWay, BlastRadius::Large),
    };
    let file = record(&tmp.path().join("TEST-records"), &path, &operator).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
    assert_eq!(json["goal_confirmation"]["by"], "operator");
    assert_eq!(json["goal_confirmation"]["operator"], "TEST-person");
}

#[test]
fn old_records_keep_their_existing_readability_and_never_gain_an_operator_identity() {
    for schema in [1, 2, 3] {
        let tmp = tempfile::tempdir().unwrap();
        let path = session(tmp.path());
        let root = tmp.path().join("TEST-records");
        let file = record(&root, &path, &GoalConfirmation::default()).unwrap();
        let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("goal_confirmation");
        old["schema"] = schema.into();
        if schema < 3 {
            old.as_object_mut().unwrap().remove("repo");
        }
        fs::write(&file, serde_json::to_vec(&old).unwrap()).unwrap();
        let original = fs::read(&file).unwrap();
        let history = read(&root, &path, GOAL);
        if schema == 1 {
            assert!(history.unwrap_err().message.contains("older"));
            assert_eq!(
                read_origins(&root, &[(&path, GOAL)]).unwrap(),
                [MadeIn::Unknown]
            );
        } else {
            let history = history.unwrap();
            assert!(history.details.contains("unrecorded"));
            assert!(!history.details.contains("TEST-operator"));
            let expected = if schema == 2 {
                MadeIn::Unknown
            } else {
                MadeIn::NoRepo
            };
            assert_eq!(history.made_in, expected);
            assert_eq!(read_origins(&root, &[(&path, GOAL)]).unwrap(), [expected]);
        }
        assert_eq!(fs::read(&file).unwrap(), original);
    }
}

#[test]
fn ineligible_or_incomplete_agent_context_refuses_before_any_record_filesystem_access() {
    let tmp = tempfile::tempdir().unwrap();
    for door in [Door::OneWay, Door::TwoWay] {
        for radius in [BlastRadius::Small, BlastRadius::Large] {
            if door == Door::TwoWay && radius == BlastRadius::Small {
                continue;
            }
            let mut confirmation = agent();
            if let GoalConfirmation::Agent { class, .. } = &mut confirmation {
                class.door = door;
                class.blast_radius = radius;
            }
            let root = tmp.path().join("TEST-records");
            let error =
                record(&root, Path::new("/TEST-nonexistent-session"), &confirmation).unwrap_err();
            assert!(error.contains("two-way"), "{error}");
            assert!(!root.exists());
        }
    }
    for field in [
        "agent",
        "authorizing_operator",
        "source",
        "door_reason",
        "blast_radius_reason",
    ] {
        let mut json = serde_json::to_value(agent()).unwrap();
        if field.ends_with("reason") {
            json["class"][field] = "  ".into();
        } else {
            json[field] = "  ".into();
        }
        let confirmation: GoalConfirmation = serde_json::from_value(json).unwrap();
        let root = tmp.path().join("TEST-records");
        assert!(record(&root, Path::new("/TEST-nonexistent-session"), &confirmation).is_err());
        assert!(!root.exists());
    }
}

#[test]
fn malformed_present_attribution_refuses_history_and_batch_instead_of_falling_back_to_operator() {
    let mut ineligible = serde_json::to_value(agent()).unwrap();
    ineligible["class"]["door"] = "one_way".into();
    for invalid in [
        ineligible,
        serde_json::json!({"by":"operator"}),
        serde_json::json!({"by":"robot"}),
        serde_json::json!({"by":"unrecorded", "operator":"TEST-invented"}),
        serde_json::Value::Null,
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let path = session(tmp.path());
        let root = tmp.path().join("TEST-records");
        let file = record(&root, &path, &agent()).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        json["goal_confirmation"] = invalid;
        fs::write(&file, json.to_string()).unwrap();
        assert!(read(&root, &path, GOAL).is_err());
        assert!(read_origins(&root, &[(&path, GOAL)]).is_err());
    }
}

#[test]
fn declared_attribution_display_escapes_controls_without_changing_the_source() {
    let confirmation = GoalConfirmation::Agent {
        agent: "TEST-agent\u{202e}\x1b".into(),
        authorizing_operator: "TEST-person".into(),
        source: "TEST exact\tquote\nsecond line".into(),
        class: class(Door::TwoWay, BlastRadius::Small),
    };
    assert!(
        !confirmation
            .details()
            .contains(['\u{202e}', '\x1b', '\t', '\n'])
    );
    let json = serde_json::to_value(&confirmation).unwrap();
    assert_eq!(json["agent"], "TEST-agent\u{202e}\x1b");
    assert_eq!(json["source"], "TEST exact\tquote\nsecond line");
}
