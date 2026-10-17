//! Repo scope changes the garden, never discovery identity or global needs.

use super::*;
use crate::think_record::{self, Origin};
use std::{fs, sync::Arc};

fn screen(tend: &mut Tend) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| render(frame, tend, Rect::new(2, 3, 96, 26), Palette::new(true)))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn attributed_repo_scope_keeps_unknowns_global_and_readiness_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-sessions");
    let records = dir.path().join("TEST-records");
    let repo = dir.path().join("TEST-repo-a");
    let other = dir.path().join("TEST-repo-b");
    let origin = Origin::simulated();
    for (id, made_in, schema) in [
        ("a", Some(&repo), 3),
        ("b", Some(&other), 3),
        ("outside", None, 3),
        ("legacy", Some(&repo), 2),
        ("missing", None, 0),
    ] {
        let session = root.join(id);
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("ledger.json"),
            serde_json::json!({
                "schema_version": 1, "session_id": id, "goal": format!("TEST {id}"), "status": "active"
            })
            .to_string(),
        )
        .unwrap();
        if schema == 0 {
            continue;
        }
        let path = think_record::write(
            &records,
            &think_record::Session {
                origin: &origin,
                turns: &[],
                checks: vec![],
                goal: &format!("TEST {id}"),
                seed_me_session: &session,
                repo: made_in.map(|p| p.as_path()),
                confirmed: SystemTime::now(),
            },
        )
        .unwrap();
        if schema == 2 {
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value["schema"] = 2.into();
            value.as_object_mut().unwrap().remove("repo");
            fs::write(path, value.to_string()).unwrap();
        }
    }
    let mut discovery = Discovery::new(Arc::new(scan));
    discovery.scope_to_repo(Some(repo), Ok(records.clone()));
    discovery.request(&root);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !discovery.poll() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let landed = discovery.landed().unwrap();
    let mut tend = Tend::new(Ok(dir.path().join("TEST-links")));
    tend.repo_paths = landed.repo_paths.clone();
    tend.repo_unknown = landed.repo_unknown;
    assert_eq!(
        tend.repo_unknown, 2,
        "missing and legacy are unknown; explicit no-repo is known"
    );
    tend.current = true;
    tend.apply(&landed.outcome, true, 1);
    assert_eq!(tend.sessions.len(), 1);
    assert_eq!(tend.current().unwrap().id.as_deref(), Some("a"));
    assert_eq!(tend.all_sessions.len(), 5);
    tend.toggle_scope();
    assert_eq!(
        tend.sessions.len(),
        5,
        "all must include other repos and unknowns"
    );
    tend.selected = tend
        .sessions
        .iter()
        .position(|s| s.id.as_deref() == Some("a"))
        .unwrap();
    tend.start_link();
    tend.selected = tend
        .sessions
        .iter()
        .position(|s| s.id.as_deref() == Some("b"))
        .unwrap();
    tend.toggle_scope(); // Never hide an in-progress link's source or target.
    assert_eq!(tend.sessions.len(), 5);
    tend.commit_link();
    let needs = tend.needs_text();
    assert!(needs.contains("TEST b"), "{needs}");
    tend.toggle_scope();
    assert_eq!(tend.sessions.len(), 1);
    assert_eq!(tend.needs_text(), needs);
    assert!(needs.contains("not ready"), "{needs}");

    // Unsupported or conflicting records never guess a repo; all remains recoverable.
    fs::write(records.join("TEST-broken.json"), "not JSON").unwrap();
    discovery.request(&root);
    while !discovery.poll() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let landed = discovery.landed().unwrap();
    tend.repo_paths = landed.repo_paths.clone();
    tend.repo_unknown = landed.repo_unknown;
    tend.apply(&landed.outcome, true, 2);
    assert!(tend.sessions.is_empty());
    let rendered = screen(&mut tend);
    assert!(
        rendered.contains("5 goals have no readable repo record. Press a for all goals."),
        "{rendered}"
    );
    assert!(
        !rendered.contains("No active goals"),
        "unknown attribution is not absence: {rendered}"
    );
    assert!(tend.can_toggle_scope(), "empty repo must still offer all");
    tend.toggle_scope();
    assert_eq!(tend.sessions.len(), 5);
    // An explicit handoff of an unknown goal still opens exactly that goal.
    tend.toggle_scope();
    tend.want = Some((
        root.join("missing"),
        root.join("missing").canonicalize().ok(),
    ));
    tend.apply(&landed.outcome, true, 3);
    assert!(tend.detail);
    assert_eq!(tend.current().unwrap().id.as_deref(), Some("missing"));
    tend.toggle_scope();
    assert!(tend.detail && tend.show_all, "detail ignores a");
}

#[test]
fn empty_repo_with_known_non_matches_names_only_recorded_absence_and_retains_ended_note() {
    let mut tend = Tend::new(Err("TEST unavailable links".into()));
    tend.repo_paths = Some([PathBuf::from("/TEST/ended")].into_iter().collect());
    tend.current = true;
    let outcome = Found {
        sessions: vec![
            Session {
                id: Some("TEST-other".into()),
                path: "/TEST/other".into(),
                goal: "TEST other repo".into(),
                stage: Stage::GoalSaved,
                updated: String::new(),
            },
            Session {
                id: Some("TEST-ended".into()),
                path: "/TEST/ended".into(),
                goal: "TEST ended".into(),
                stage: Stage::Ended,
                updated: String::new(),
            },
        ],
        unreadable: 0,
    };
    tend.apply(&Ok(outcome.clone()), true, 1);
    let rendered = screen(&mut tend);
    assert!(
        rendered.contains("No active goals are recorded for this repo. Press a for all goals."),
        "{rendered}"
    );
    assert!(rendered.contains("1 ended not shown"), "{rendered}");
    assert!(
        !rendered.contains("no readable repo record"),
        "known non-matches are not unknown: {rendered}"
    );
    let mut incomplete = outcome.clone();
    incomplete.unreadable = 1;
    tend.apply(&Ok(incomplete), true, 2);
    let rendered = screen(&mut tend);
    assert!(!rendered.contains("No active goals"), "{rendered}");
    assert!(
        rendered.contains("1 saved record couldn't be read"),
        "{rendered}"
    );
    assert_eq!(rendered.matches("saved record").count(), 1, "{rendered}");
    assert!(rendered.contains("Links couldn't be read"), "{rendered}");
    assert!(rendered.contains("Press a for all goals."), "{rendered}");
    assert!(rendered.contains("1 ended not shown"), "{rendered}");
    tend.toggle_scope();
    assert_eq!(tend.current().unwrap().id.as_deref(), Some("TEST-other"));
    tend.toggle_scope();
    tend.repo_unknown = 1;
    let rendered = screen(&mut tend);
    assert!(rendered.contains("1 goal has no readable repo record."));
    assert_eq!(rendered.matches("saved record").count(), 1, "{rendered}");
    tend.repo_unknown = 0;
    let mut incomplete = outcome;
    incomplete.sessions.retain(|s| s.stage != Stage::Ended);
    incomplete.unreadable = 1;
    tend.apply(&Ok(incomplete), true, 3);
    let rendered = screen(&mut tend);
    assert_eq!(rendered.matches("saved record").count(), 1, "{rendered}");
    assert!(rendered.contains("Links couldn't be read"), "{rendered}");
}

#[test]
fn outside_repo_a_does_nothing_and_keeps_every_active_goal() {
    let mut tend = Tend::new(Err("TEST unavailable links".into()));
    let outcome = Ok(Found {
        sessions: vec![Session {
            id: Some("TEST-unknown".into()),
            path: "/TEST/unknown".into(),
            goal: "TEST unknown repo".into(),
            stage: Stage::GoalSaved,
            updated: String::new(),
        }],
        unreadable: 0,
    });
    tend.apply(&outcome, true, 1);
    tend.toggle_scope();
    assert_eq!(tend.sessions.len(), 1);
    assert!(!tend.show_all && !tend.can_toggle_scope());
}
