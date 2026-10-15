//! Synthetic Unix process/pipe boundaries; all scripts and markers live in temporary TEST roots.

use super::*;
use std::{fs, process::Command};

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}

fn control() -> Control {
    Control {
        reason: Arc::new(AtomicU8::new(0)),
        deadline: Instant::now() + DEADLINE,
    }
}

fn assert_reaped(path: &Path) {
    let pid: i32 = fs::read_to_string(path).unwrap().trim().parse().unwrap();
    let mut status = 0;
    // SAFETY: this test only observes its recorded child; production must have already reaped it.
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn simultaneous_streams_and_exact_combined_cap_are_not_partial_successes() {
    for amount in [LIMIT - 1, LIMIT, LIMIT + 1] {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-pipes-")
            .tempdir()
            .unwrap();
        let half = amount / 2;
        let script = format!(
            "echo $$ > leader; (head -c {half} /dev/zero | tr '\\000' x) & head -c {} /dev/zero | tr '\\000' y >&2; wait",
            amount - half
        );
        let reading = collect(root.path().into(), control(), shell(&script));
        if amount <= LIMIT {
            let super::super::Report::Complete(body) = &reading.report else {
                panic!("expected complete")
            };
            assert_eq!(body.len(), half);
            assert!(body.bytes().all(|b| b == b'x'));
        } else {
            assert_eq!(reading.report_cause(), Some(&Cause::OutputLimit));
            let super::super::Report::Failed(f) = &reading.report else {
                panic!()
            };
            assert!(f.details.contains("may be incomplete"));
            assert!(f.details.len() < LIMIT + 1024);
        }
        assert_reaped(&root.path().join("leader"));
    }
}

#[test]
fn cancelled_late_spawn_is_terminated_and_reaped_without_publication() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-late-")
        .tempdir()
        .unwrap();
    let ctl = control();
    ctl.stop(CANCEL);
    let reading = collect(root.path().into(), ctl, shell("sleep 60"));
    assert_eq!(reading.report_cause(), Some(&Cause::Cancelled));
    assert!(
        reading.cleanup_error.is_none(),
        "{:?}",
        reading.cleanup_error
    );
}

#[test]
fn inherited_writers_and_term_ignoring_group_are_stopped_without_eof_wait() {
    for exited_leader in [false, true] {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-group-")
            .tempdir()
            .unwrap();
        let script = format!(
            "echo $$ > leader; trap '' TERM; sleep 60 & echo $! > helper; echo ready > ready; {}",
            if exited_leader { "exit 0" } else { "wait" }
        );
        let path = root.path().to_path_buf();
        let ctl = control();
        let worker_ctl = ctl.clone();
        let task = thread::spawn(move || collect(path, worker_ctl, shell(&script)));
        let watchdog = Instant::now() + Duration::from_secs(10);
        while !root.path().join("ready").exists() {
            assert!(Instant::now() < watchdog);
            thread::yield_now();
        }
        ctl.stop(CANCEL);
        while !task.is_finished() {
            assert!(
                Instant::now() < watchdog,
                "cleanup waited for inherited writer"
            );
            thread::yield_now();
        }
        let reading = task.join().unwrap();
        assert_eq!(reading.report_cause(), Some(&Cause::Cancelled));
        assert!(reading.cleanup_error.is_none());
        assert_reaped(&root.path().join("leader"));
        let helper = fs::read_to_string(root.path().join("helper")).unwrap();
        let state = Command::new("ps")
            .args(["-o", "stat=", "-p", helper.trim()])
            .output()
            .unwrap();
        // An orphan may await the system reaper; it must not still be executing.
        let state = String::from_utf8_lossy(&state.stdout);
        assert!(
            state.trim().is_empty() || state.trim().starts_with('Z'),
            "helper still running: {state}"
        );
    }
}

// On a deliberate red probe, clean only the synthetic helpers recorded by the test.
struct Helpers(PathBuf);

impl Drop for Helpers {
    fn drop(&mut self) {
        if let Ok(pids) = fs::read_to_string(&self.0) {
            for pid in pids.lines().map(|p| p.parse::<i32>().unwrap()) {
                // SAFETY: recorded synthetic helpers only; never an operator process.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
            }
        }
    }
}

#[test]
fn normal_cleanup_retains_capacity_and_can_still_be_cancelled_or_timed_out() {
    for reason in [CANCEL, TIMEOUT] {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-normal-stop-")
            .tempdir()
            .unwrap();
        let helpers = Helpers(root.path().join("helpers"));
        let source: Source = Arc::new(|directory, ctl| {
            collect(
                directory,
                ctl,
                shell(
                    "echo $$ > leader; /bin/sh -c 'trap \"echo term > term\" TERM; echo ready > ready; while :; do :; done' </dev/null >/dev/null 2>&1 & echo $! >> helpers; while [ ! -f ready ]; do :; done; printf 'original report'; exit 0",
                ),
            )
        });
        let mut reader = Reader::with_source(source);
        let now = Instant::now();
        let mut visible = reader.open(root.path(), now);
        let watchdog = now + Duration::from_secs(5);
        // The helper acknowledges TERM: ordinary EOF/exit has entered group cleanup.
        while !root.path().join("term").exists() {
            assert!(
                Instant::now() < watchdog,
                "normal completion skipped group cleanup"
            );
            thread::yield_now();
        }
        reader.poll(Some(&mut visible), now);
        assert!(visible.waiting_state());
        assert!(reader.job.is_some());
        assert!(reader.open(root.path(), now).request.is_none());
        assert_eq!(reader.next, 1);
        if reason == CANCEL {
            reader.cancel();
        } else {
            reader.poll(Some(&mut visible), now + DEADLINE);
            assert_eq!(visible.report_cause(), Some(&Cause::Timeout));
        }
        while reader.job.is_some() {
            assert!(Instant::now() < watchdog);
            reader.poll(Some(&mut visible), Instant::now());
            thread::yield_now();
        }
        if reason == CANCEL {
            assert!(visible.waiting_state(), "cancelled success was accepted");
        } else {
            assert_eq!(visible.report_cause(), Some(&Cause::Timeout));
        }
        assert_reaped(&root.path().join("leader"));
        assert_helpers_stopped(&helpers.0);
        reader.shutdown(|notice| panic!("unexpected cleanup notice: {notice}"));
        assert_helpers_stopped(&helpers.0);
    }
}

#[test]
fn normal_completion_stops_redirected_helpers_before_repeated_results_and_shutdown() {
    for exit in [0, 7] {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-normal-group-")
            .tempdir()
            .unwrap();
        let helpers = Helpers(root.path().join("helpers"));
        let source: Source = Arc::new(move |directory, ctl| {
            collect(
                directory,
                ctl,
                shell(&format!(
                    "echo $$ > leader; trap '' TERM; sleep 60 </dev/null >/dev/null 2>&1 & echo $! >> helpers; printf 'Report café\\n'; printf 'original stderr\\n' >&2; exit {exit}"
                )),
            )
        });
        let mut reader = Reader::with_source(source);
        for round in 1..=2 {
            let mut visible = reader.open(root.path(), Instant::now());
            let watchdog = Instant::now() + Duration::from_secs(5);
            while reader.job.is_some() {
                assert!(Instant::now() < watchdog, "normal cleanup did not finish");
                reader.poll(Some(&mut visible), Instant::now());
                thread::yield_now();
            }
            assert!(reader.cleanup_error.is_none());
            if exit == 0 {
                assert!(
                    matches!(&visible.report, super::super::Report::Complete(body) if body == "Report café\n")
                );
            } else {
                assert_eq!(visible.report_cause(), Some(&Cause::Exit));
                let super::super::Report::Failed(failure) = &visible.report else {
                    panic!()
                };
                assert!(failure.details.contains("Exit status: 7"));
                assert!(failure.details.ends_with("original stderr\n"));
            }
            assert_eq!(
                fs::read_to_string(&helpers.0).unwrap().lines().count(),
                round
            );
            assert_reaped(&root.path().join("leader"));
            assert_helpers_stopped(&helpers.0);
        }
        reader.shutdown(|notice| panic!("unexpected cleanup notice: {notice}"));
        assert_helpers_stopped(&helpers.0);
        assert_eq!(reader.next, 2);
    }
}

fn assert_helpers_stopped(path: &Path) {
    for pid in fs::read_to_string(path).unwrap().lines() {
        let state = Command::new("ps")
            .args(["-o", "stat=", "-p", pid])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&state.stdout);
        assert!(
            state.trim().is_empty() || state.trim().starts_with('Z'),
            "helper {pid} still executing: {state}"
        );
    }
}

#[test]
fn permission_errors_require_positive_zombie_evidence_and_other_errors_never_coerce() {
    for evidence in [false, true] {
        let mut errors = vec![];
        record_signal_error(
            123,
            libc::SIGKILL,
            std::io::Error::from_raw_os_error(libc::EPERM),
            &mut errors,
            |_| evidence,
        );
        assert_eq!(errors.is_empty(), evidence);
    }
    let mut errors = vec![];
    record_signal_error(
        123,
        libc::SIGKILL,
        std::io::Error::from_raw_os_error(libc::EINVAL),
        &mut errors,
        |_| true,
    );
    assert_eq!(errors.len(), 1);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_snapshot_distinguishes_owned_live_child_and_unreaped_zombie() {
    use std::os::unix::process::CommandExt;
    let mut child = shell("exec sleep 60").process_group(0).spawn().unwrap();
    let pid = child.id() as i32;
    assert!(!zombie_only_group(pid));
    child.kill().unwrap();
    let watchdog = Instant::now() + Duration::from_secs(5);
    while !zombie_only_group(pid) {
        assert!(Instant::now() < watchdog);
        thread::yield_now();
    }
    let mut errors = vec![];
    signal(pid, libc::SIGKILL, &mut errors);
    assert!(errors.is_empty());
    child.wait().unwrap();
    assert!(
        !zombie_only_group(pid),
        "missing group is not positive zombie evidence"
    );
}

#[test]
fn output_prefix_split_utf8_stays_explicit_and_collection_errors_are_not_installation_claims() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-utf8-")
        .tempdir()
        .unwrap();
    let script = format!(
        "head -c {} /dev/zero | tr '\\000' x >&2; printf '\\303\\251' >&2",
        LIMIT - 1
    );
    let reading = collect(root.path().into(), control(), shell(&script));
    assert_eq!(reading.report_cause(), Some(&Cause::OutputLimit));
    let super::super::Report::Failed(failure) = reading.report else {
        panic!()
    };
    assert!(
        failure.details.ends_with("\\xc3"),
        "tail: {:?}",
        &failure.details[failure.details.len().saturating_sub(256)..]
    );
    let mut errors = vec![];
    signal(i32::MAX, libc::SIGTERM, &mut errors);
    assert!(
        errors.is_empty(),
        "an already absent group is not a signal failure"
    );
}
