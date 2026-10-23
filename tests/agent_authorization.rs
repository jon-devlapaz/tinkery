//! Every operator-action boundary refuses invalid agent context before any file write.
use tinkery::{
    seed_me::{self, answering, ledger},
    think_record::{BlastRadius, ConfirmationClass, Door, GoalConfirmation},
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
fn every_write_refuses_invalid_agent_context_before_touching_disk() {
    let mut invalid = named();
    if let GoalConfirmation::Agent { class, .. } = &mut invalid {
        class.door = Door::OneWay;
    }
    for context in [invalid] {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-authorization-")
            .tempdir()
            .unwrap();
        let session = root.path().join("TEST-no-session");
        let config = seed_me::Config::new(Some(session.clone()));
        let error = config
            .confirm_as(
                seed_me::Affirmed {
                    goal: "TEST goal".into(),
                    outcome: "TEST check".into(),
                    options: vec![],
                    originals: vec![],
                    context: "TEST context".into(),
                    source: "TEST confirmation".into(),
                },
                &context,
            )
            .err()
            .unwrap();
        assert!(error.contains("two-way"), "{error}");
        let errors = [
            answering::answer(&session, 0, "choice", "TEST answer", Some(&context))
                .unwrap_err()
                .to_string(),
            answering::respond(
                &session,
                0,
                "choice",
                &ledger::AnswerAction::Delegate,
                Some(&context),
            )
            .unwrap_err()
            .to_string(),
            answering::keep(&session, 0, "goal", "TEST reason", Some(&context))
                .unwrap_err()
                .to_string(),
            answering::recommend(&session, 0, "choice", "TEST suggestion", Some(&context))
                .unwrap_err()
                .to_string(),
            answering::object(&session, 0, "TEST assumption", Some(&context))
                .unwrap_err()
                .to_string(),
            answering::teach_back(
                &session,
                0,
                "r1",
                "TEST seed",
                &ledger::TeachBackRecord::Words("TEST words"),
                Some(&context),
            )
            .unwrap_err()
            .to_string(),
            answering::confirm_seed(&session, "r1", "TEST confirming words", Some(&context))
                .unwrap_err()
                .to_string(),
            answering::end(&session, answering::Ending::Stopped, Some(&context))
                .unwrap_err()
                .to_string(),
        ];
        for error in errors {
            assert!(error.contains("two-way"), "{error}");
        }
        assert!(config.recovery_path().is_none());
        assert!(!session.exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
