//! Confirmation provenance controls: declared identity is not authentication or permission.
use super::readiness_tests::{app, parts};
use super::*;
use crate::think_record::{BlastRadius, ConfirmationClass, Door, GoalConfirmation};

fn class() -> ConfirmationClass {
    ConfirmationClass {
        door: Door::TwoWay,
        door_reason: "TEST throwaway goal; nothing runs".into(),
        blast_radius: BlastRadius::Small,
        blast_radius_reason: "TEST isolated goal only".into(),
    }
}
fn agent() -> GoalConfirmation {
    GoalConfirmation::Agent {
        agent: "TEST-agent".into(),
        authorizing_operator: "TEST-person".into(),
        source: "TEST explicit authorization for this goal only".into(),
        class: class(),
    }
}
fn operator() -> GoalConfirmation {
    GoalConfirmation::Operator {
        operator: "TEST-later-person".into(),
        source: "TEST unrelated later confirmation".into(),
        class: class(),
    }
}
fn declared() -> BrainDump {
    app(serde_json::json!({"parts": parts()}))
        .with_goal_confirmation(agent())
        .unwrap()
}

#[test]
fn unspecified_confirmer_is_not_claimed_as_the_operator() {
    let mut a = app(serde_json::json!({"parts": parts()}));
    a.review_goal();
    let source = a
        .handoff
        .review
        .as_ref()
        .unwrap()
        .affirmation
        .affirmed()
        .source;
    assert!(!source.contains("operator typed confirm"), "{source}");
    assert!(source.contains("unrecorded"), "{source}");
    assert!(source.contains("goal only, not seed or implementation approval"));
}

#[test]
fn declared_agent_source_never_claims_an_operator_keypress_or_implementation_permission() {
    let mut a = declared();
    a.review_goal();
    let affirmation = &a.handoff.review.as_ref().unwrap().affirmation;
    let saved = affirmation.affirmed();
    assert!(saved.source.contains("Agent") && saved.source.contains("TEST-agent"));
    assert!(saved.source.contains("TEST-person") && saved.source.contains("TwoWay"));
    assert!(saved.source.contains("Small") && saved.source.contains("TEST explicit authorization"));
    assert!(!saved.source.contains("operator typed confirm"));
    assert!(saved.source.contains("not seed or implementation approval"));
    assert_eq!(saved.goal, affirmation.goal);
    assert_eq!(saved.originals[0].text, a.conversation.sources()[0].text);
}

#[test]
fn delegated_goal_save_is_held_before_the_user_only_seed_me_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("TEST-records");
    let mut a = declared().with_record_root(root.clone());
    a.screen.help_all = true;
    assert!(!layout::help_text(&a).contains("typing confirm saves"));
    assert!(layout::how_text(&a).contains("Seed Me integration"));
    a.screen.help_all = false;
    a.review_goal();
    for size in [(80, 24), (100, 30)] {
        let screen = snapshot(size.0, size.1, &mut a, true).unwrap();
        let plain = screen.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(plain.contains("Seed Me integration"), "{screen}");
        assert!(!plain.contains("type confirm"), "{screen}");
        assert!(layout::help_text(&a).contains("Seed Me integration"));
    }
    a.screen.help_all = true;
    assert!(!layout::help_text(&a).contains("typing confirm saves"));
    assert!(layout::how_text(&a).contains("Seed Me integration"));
    a.screen.help_all = false;
    let review = a.handoff.review.as_mut().unwrap();
    review.scroll = review.max;
    review.input = "confirm".into();
    a.goal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(a.notice.contains("Nothing saved"), "{}", a.notice);
    assert!(
        a.handoff.job.is_none(),
        "delegation reached the held helper path"
    );
    assert!(a.handoff.config.as_ref().unwrap().recovery_path().is_none());
    assert!(!root.exists());
}

#[test]
fn new_visits_do_not_inherit_goal_authorization_and_review_cannot_be_relabelled() {
    let mut a = declared();
    let fresh = a.fresh_factory()();
    assert_eq!(fresh.handoff.confirmation, GoalConfirmation::default());
    a.review_goal();
    assert!(
        a.with_goal_confirmation(operator())
            .err()
            .unwrap()
            .contains("before review")
    );
}

#[test]
fn attribution_is_not_added_to_the_shaping_request() {
    let a = declared();
    let request = a
        .preview_send("TEST additional goal words")
        .unwrap()
        .request;
    let wire = serde_json::to_string(&request).unwrap();
    for metadata in ["TEST-agent", "TEST-person", "TEST explicit authorization"] {
        assert!(!wire.contains(metadata), "{wire}");
    }
}

#[test]
fn invalid_declared_agent_class_is_refused_by_the_think_api() {
    let mut context = agent();
    if let GoalConfirmation::Agent { class, .. } = &mut context {
        class.blast_radius = BlastRadius::Large;
    }
    let error = app(serde_json::json!({"parts": parts()}))
        .with_goal_confirmation(context)
        .err()
        .unwrap();
    assert!(error.contains("two-way") && error.contains("small"));
}

#[test]
fn a_future_helper_receipt_records_the_frozen_agent_context_not_a_later_identity() {
    // Simulated receipt injection tests record plumbing only. Production agent saves stay held.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("TEST-records");
    let mut a = declared().with_record_root(root.clone());
    a.review_goal();
    let goal = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
    a.handoff.confirmation = operator();
    let path = tmp.path().join("TEST-sessions/one");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("ledger.json"),
        serde_json::json!({
            "schema_version": 1, "session_id": "TEST-existing-id", "status": "active", "goal": goal,
        })
        .to_string(),
    )
    .unwrap();
    let mut receipt = seed_me::Receipt::test(path);
    receipt.goal = goal.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    a.handoff.job = Some(rx);
    tx.send(Ok(receipt)).unwrap();
    a.goal_tick();
    let file = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(
        record["goal_confirmation"],
        serde_json::to_value(agent()).unwrap()
    );
    assert_eq!(record["goal_confirmation"]["by"], "agent");
    assert_eq!(
        record["origin"], "simulated",
        "model origin is not the confirmer"
    );
    assert_eq!(record["goal"], goal);
    assert_eq!(a.conversation.sources()[0].text, "An intact original");
}
