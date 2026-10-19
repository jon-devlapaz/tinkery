use super::*;
use std::{fs, path::Path};

fn envelope(runs: &str) -> String {
    format!(r#"{{"protocol":"tink-sdlc","api_version":1,"workspace":"/TEST","runs":[{runs}]}}"#)
}

fn run(slug: &str, state: &str) -> String {
    format!(
        r#"{{"slug":"{slug}","run_id":"x","profile":"light","kind":"feature","verification_status":"{state}","next_action":"TEST next for {slug}","has_lock":false,"checklist_summary":{{"total":3,"passed":1}}}}"#
    )
}

/// A TEST checkout whose fake `sdlc.py` prints `body` (and sleeps `delay` seconds first).
fn checkout(body: &str, delay: f64) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-watch-")
        .tempdir()
        .unwrap();
    fs::create_dir_all(dir.path().join("_system/scripts")).unwrap();
    fs::write(
        dir.path().join(SCRIPT),
        format!(
            "import sys, time\nfrom pathlib import Path\nPath('{pid}').write_text(str(__import__('os').getpid()))\ntime.sleep({delay})\nsys.stdout.write({body:?})\n",
            pid = dir.path().join("TEST-pid").display()
        ),
    )
    .unwrap();
    dir
}

fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n != "TEST-pid") {
                files.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn settle(watch: &mut Watch) {
    let until = Instant::now() + Duration::from_secs(10);
    while matches!(watch.list, Some(Load::Reading(_)))
        || matches!(&watch.detail, Some((_, Load::Reading(_))))
    {
        assert!(Instant::now() < until, "read never landed");
        watch.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn runs_waiting_on_a_person_lead_and_the_rest_keep_tink_sdlcs_order() {
    let runs = parse_list(&envelope(
        &[
            run("quiet-a", "current"),
            run("waits", "blocked"),
            run("quiet-b", "not-run"),
            run("broke", "failed"),
            run("cut", "interrupted"),
            run("busy", "running"),
            run("old", "stale"),
        ]
        .join(","),
    ))
    .unwrap();
    let order: Vec<_> = ordered(&runs).iter().map(|r| r.slug.as_str()).collect();
    assert_eq!(
        order,
        ["waits", "broke", "cut", "quiet-a", "quiet-b", "busy", "old"]
    );
}

#[test]
fn an_invalid_run_needs_you_and_shows_its_reason() {
    let invalid = r#"{"slug":"bad","verification_status":"invalid","error":"TEST reason","checklist_summary":{"total":0,"passed":0}}"#;
    let runs = parse_list(&envelope(&format!("{},{invalid}", run("fine", "current")))).unwrap();
    let first = ordered(&runs)[0];
    assert_eq!(
        (first.slug.as_str(), first.state, first.next.as_str()),
        ("bad", State::Invalid, "TEST reason")
    );
}

#[test]
fn another_interface_or_an_unknown_state_is_refused_not_shown() {
    let body = run("a", "current");
    for (text, needle) in [
        (
            envelope(&body).replace("tink-sdlc", "other"),
            "protocol other",
        ),
        (
            envelope(&body).replace(r#""api_version":1"#, r#""api_version":2"#),
            "API 2",
        ),
        (
            envelope(&body).replace(r#""api_version":1"#, r#""api_version":"1""#),
            "API \"1\"",
        ),
        (
            envelope(&body).replace(r#""api_version":1,"#, ""),
            "API null",
        ),
        (
            envelope(&run("a", "merged")),
            "state Watch doesn't know (merged)",
        ),
        ("not json".to_string(), "didn't print JSON"),
    ] {
        let error = parse_list(&text).unwrap_err();
        assert!(error.contains(needle), "{needle}: {error}");
    }
}

#[test]
fn detail_counts_proven_and_attested_items_and_refuses_unknown_gates() {
    let detail = r#"{"protocol":"tink-sdlc","api_version":1,"workspace":"/T","run":{"verification_status":"blocked","next_action":"TEST review","gates":[{"stage":3,"status":"pending","blocked":false}],"checklist":[{"status":"passed","proof":"automated"},{"status":"pending","proof":"attested"}],"errors":["TEST error"]}}"#;
    let parsed = parse_detail(detail).unwrap();
    assert_eq!((parsed.passed, parsed.proven, parsed.attested), (1, 1, 1));
    assert_eq!(parsed.gates, [(3, "pending".to_string())]);
    assert_eq!(parsed.errors, ["TEST error"]);
    assert!(
        parse_detail(&detail.replace("pending\",\"blocked", "skipped\",\"blocked"))
            .unwrap_err()
            .contains("skipped")
    );
}

#[test]
fn opening_and_reading_again_change_nothing_in_the_checkout() {
    let dir = checkout(&envelope(&run("a", "blocked")), 0.0);
    let before = tree(dir.path());
    let mut watch = Watch::open(Some(dir.path().to_path_buf()));
    settle(&mut watch);
    assert_eq!(watch.runs().unwrap().len(), 1);
    watch.key(KeyEvent::from(KeyCode::Char('r')));
    settle(&mut watch);
    watch.key(KeyEvent::from(KeyCode::Enter));
    settle(&mut watch);
    assert_eq!(tree(dir.path()), before);
}

#[test]
fn a_missing_script_is_said_plainly_and_nothing_runs() {
    let dir = tempfile::Builder::new()
        .prefix("tinkery-TEST-watch-")
        .tempdir()
        .unwrap();
    let watch = Watch::open(Some(dir.path().to_path_buf()));
    match &watch.list {
        Some(Load::Done(Err(why), _)) => assert!(why.contains("is missing"), "{why}"),
        _ => panic!("expected a refusal without running anything"),
    }
}

#[test]
fn a_slow_read_is_stopped_at_its_deadline() {
    let dir = checkout(&envelope(""), 30.0);
    let mut watch =
        Watch::with_deadline(Some(dir.path().to_path_buf()), Duration::from_millis(300));
    settle(&mut watch);
    match &watch.list {
        Some(Load::Done(Err(why), _)) => assert!(why.contains("was stopped"), "{why}"),
        _ => panic!("expected a timeout"),
    }
    assert!(!alive(dir.path()), "the timed-out process still runs");
}

#[test]
fn leaving_mid_read_stops_the_process_and_delivers_nothing() {
    let dir = checkout(&envelope(""), 30.0);
    let mut watch = Watch::open(Some(dir.path().to_path_buf()));
    let until = Instant::now() + Duration::from_secs(5);
    while !dir.path().join("TEST-pid").exists() {
        assert!(Instant::now() < until, "read never started");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(watch.key(KeyEvent::from(KeyCode::Esc)), "Esc leaves Watch");
    drop(watch);
    assert!(!alive(dir.path()), "leaving left the read running");
}

#[test]
fn a_failing_status_shows_its_own_message() {
    let dir = checkout("", 0.0);
    fs::write(
        dir.path().join(SCRIPT),
        "import sys\nprint('{\"protocol\":\"tink-sdlc\",\"api_version\":1,\"error\":{\"code\":\"not-found\",\"message\":\"TEST no such run\"}}')\nsys.exit(1)\n",
    )
    .unwrap();
    let mut watch = Watch::open(Some(dir.path().to_path_buf()));
    settle(&mut watch);
    match &watch.list {
        Some(Load::Done(Err(why), _)) => assert!(why.contains("TEST no such run"), "{why}"),
        _ => panic!("expected the refusal"),
    }
}

fn alive(dir: &Path) -> bool {
    let pid: i32 = fs::read_to_string(dir.join("TEST-pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // SAFETY: signal 0 only checks whether the process exists; it sends nothing.
    unsafe { libc::kill(pid, 0) == 0 }
}
