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
