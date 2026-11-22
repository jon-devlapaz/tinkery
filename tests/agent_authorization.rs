//! Invalid agent context refuses where it comes in, before any file write. Every session write
//! takes a resolved `Actor`, so no write can receive unchecked context.
// Test code reads the files it made; only Tinkery's own code must use `regular_file`.
#![allow(clippy::disallowed_methods)]
use tinkery::{
    confirmation::{Actor, BlastRadius, ConfirmationClass, Door, GoalConfirmation},
    shape::command,
};

fn named() -> GoalConfirmation {
    GoalConfirmation::Agent {
        agent: "TEST-agent".into(),
        authorizing_operator: "TEST-operator".into(),
        source: "TEST scoped authorization for disposable session".into(),
        class: ConfirmationClass {
            door: Door::TwoWay,
            door_reason: "TEST reversible".into(),
            blast_radius: BlastRadius::Small,
            blast_radius_reason: "TEST isolated".into(),
        },
    }
}

#[test]
fn invalid_agent_context_refuses_at_entry_before_touching_disk() {
    assert!(named().actor().is_ok());
    let mut invalid = named();
    if let GoalConfirmation::Agent { class, .. } = &mut invalid {
        class.door = Door::OneWay;
    }
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-authorization-")
        .tempdir()
        .unwrap();
    let sessions = root.path().join("TEST-no-session");
    let errors = [
        invalid.actor().err().unwrap(),
        Actor::authorize(Some(&invalid)).err().unwrap(),
        command::new_goal(
            sessions.clone(),
            "TEST goal\nDone when: TEST check",
            "TEST origin",
            &invalid,
        )
        .unwrap_err(),
    ];
    for error in errors {
        assert!(error.contains("two-way"), "{error}");
    }
    assert!(!sessions.exists());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
