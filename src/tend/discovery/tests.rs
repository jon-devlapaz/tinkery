//! TEST archives bound discovery's record IO independently of wall-clock timing.

use super::*;
use crate::think_record::{self, Origin};
use std::time::{Duration, SystemTime};

#[test]
fn repo_discovery_reads_each_of_400_records_once_per_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-sessions");
    let records = dir.path().join("TEST-records");
    let repo = dir.path().join("TEST-repo");
    let origin = Origin::simulated();
    for index in 0..400 {
        let session = root.join(format!("TEST-{index}"));
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("ledger.json"),
            serde_json::json!({
                "schema_version": 1, "session_id": format!("TEST-id-{index}"),
                "status": if index == 0 { "completed" } else { "active" }, "goal": "TEST goal"
            })
            .to_string(),
        )
        .unwrap();
        think_record::write(
            &records,
            &think_record::Session {
                origin: &origin,
                turns: &[],
                checks: vec![],
                goal: "TEST goal",
                seed_me_session: &session,
                repo: Some(&repo),
                confirmed: SystemTime::now(),
            },
        )
        .unwrap();
    }
    let counts = think_record::count_reads(&records);
    let mut discovery = Discovery::new(Arc::new(scan));
    discovery.scope_to_repo(Some(repo), Ok(records.clone()));
    for refresh in 1..=2 {
        discovery.request(&root);
        let deadline = Instant::now() + Duration::from_secs(60);
        while !discovery.poll() {
            assert!(Instant::now() < deadline, "discovery did not settle");
            std::thread::sleep(Duration::from_millis(1));
        }
        let landed = discovery.landed().unwrap();
        assert_eq!(landed.repo_paths.as_ref().unwrap().len(), 401 - refresh);
        assert_eq!(landed.repo_unknown, 0);
        assert_eq!(discovery.exact_active(), Some(399));
        eprintln!(
            "TEST refresh {refresh}: root enumerations / record reads = {:?}",
            counts.totals()
        );
        assert_eq!(
            counts.totals(),
            (refresh, 400 * refresh),
            "one root scan and one read per record per discovery, not sessions × records"
        );
        if refresh == 1 {
            let path = fs::read_dir(&records)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value["repo"] = serde_json::Value::Null;
            fs::write(path, value.to_string()).unwrap();
            fs::write(records.join("TEST-unrelated.txt"), "ignored").unwrap();
        }
    }
}

#[test]
fn scan_never_offers_an_empty_or_control_ledger_id_as_an_identity() {
    let dir = tempfile::tempdir().unwrap();
    let bad = ["", "line\nbreak", "null\0", "delete\u{7f}", "next\u{85}"];
    for (index, id) in bad.iter().chain(["TEST-good"].iter()).enumerate() {
        let session = dir.path().join(format!("TEST-{index}"));
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("ledger.json"),
            serde_json::json!({
                "schema_version": 1, "session_id": id, "status": "active", "goal": "TEST goal"
            })
            .to_string(),
        )
        .unwrap();
    }
    let found = scan(dir.path()).unwrap();
    assert_eq!(found.sessions.len(), 6);
    let ids = found.identities();
    for session in &found.sessions {
        let held = ids.id(session);
        assert_eq!(held, session.id.as_deref());
        assert!(held.is_none_or(|id| id == "TEST-good"), "{held:?}");
    }
    for id in bad {
        assert!(ids.session(id).is_none() && !ids.ambiguous(id), "{id:?}");
    }
    assert!(ids.session("TEST-good").is_some());
}
