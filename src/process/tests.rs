use super::*;
use std::fs;

mod descendant {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/command_descendant.rs"
    ));
}

fn temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("tinkery-TEST-process-")
        .tempdir()
        .unwrap()
}

fn shell(root: &Path, script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]).current_dir(root);
    command
}

/// True while `pid` names a live, non-zombie process.
fn running(pid: &str) -> bool {
    let result = Command::new("ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&result.stdout);
    !state.trim().is_empty() && !state.trim().starts_with('Z')
}

fn gone(pid: &str) {
    let until = Instant::now() + Duration::from_secs(1);
    while running(pid) && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!running(pid), "process {pid} is still running");
}

#[test]
fn a_child_that_ignores_the_deadline_is_stopped_with_its_group_and_reaped() {
    let root = temp();
    let command = shell(
        root.path(),
        "trap '' TERM INT HUP; sleep 60 & echo $! > TEST-descendant; echo $$ > TEST-child; wait",
    );
    let limits = Limits {
        deadline: Duration::from_millis(300),
        ..Limits::text(Duration::ZERO)
    };
    let started = Instant::now();
    let failure = run(command, limits, &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(failure, Failure::Late(_)), "{failure:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        failure.words("TEST"),
        "TEST took longer than 0 s and was stopped."
    );
    let child = fs::read_to_string(root.path().join("TEST-child")).unwrap();
    // Reaped: the direct child is neither running nor left as this process's zombie.
    let ps = Command::new("ps")
        .args(["-o", "stat=", "-p", child.trim()])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&ps.stdout).trim().is_empty());
    gone(&fs::read_to_string(root.path().join("TEST-descendant")).unwrap());
}

#[test]
fn cancelling_stops_a_running_child_at_once() {
    let root = temp();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let setter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        flag.store(true, Ordering::SeqCst);
    });
    let started = Instant::now();
    let command = shell(root.path(), "echo $$ > TEST-child; exec sleep 60");
    let failure = run(command, Limits::text(Duration::from_secs(30)), &cancel).unwrap_err();
    setter.join().unwrap();
    assert!(matches!(failure, Failure::Cancelled), "{failure:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    gone(&fs::read_to_string(root.path().join("TEST-child")).unwrap());
}

#[test]
fn stdout_past_its_bound_stops_the_child_and_shows_nothing() {
    let root = temp();
    let limits = Limits {
        stdout: 1000,
        ..Limits::text(Duration::from_secs(10))
    };
    let started = Instant::now();
    let failure = run(
        shell(root.path(), "exec yes"),
        limits,
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(failure, Failure::TooMuch(1000)), "{failure:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        failure.words("TEST"),
        "TEST printed more than 1000 bytes; nothing is shown."
    );
}

#[test]
fn stderr_past_its_bound_is_cut_and_the_run_still_finishes() {
    let root = temp();
    let limits = Limits {
        stderr: 1000,
        ..Limits::text(Duration::from_secs(10))
    };
    let script = "head -c 200000 /dev/zero >&2; echo TEST done; exit 4";
    let output = run(shell(root.path(), script), limits, &AtomicBool::new(false)).unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(output.stdout, b"TEST done\n");
    assert_eq!(output.stderr.len(), 1000);
    assert!(output.stderr_cut);
}

#[test]
fn input_larger_than_a_pipe_reaches_the_child_and_streams_stay_apart() {
    let root = temp();
    let input: Vec<u8> = (0..300_000).map(|i| b'a' + (i % 26) as u8).collect();
    let limits = Limits {
        stdout: input.len(),
        ..Limits::text(Duration::from_secs(10))
    };
    let command = shell(root.path(), "cat; echo TEST note >&2");
    let output = feed(
        command,
        Some(input.clone()),
        limits,
        &AtomicBool::new(false),
        |_| None,
    )
    .unwrap();
    assert!(output.status.success());
    assert!(output.input.is_ok());
    assert_eq!(output.stdout, input);
    assert_eq!(output.said(), "TEST note");
}

#[test]
fn a_watch_stops_the_run_with_its_own_words() {
    let root = temp();
    let command = shell(root.path(), "exec sleep 60");
    let failure = feed(
        command,
        None,
        Limits::text(Duration::from_secs(10)),
        &AtomicBool::new(false),
        |exited| (!exited).then(|| "TEST halted".to_owned()),
    )
    .unwrap_err();
    assert_eq!(failure.words("TEST"), "TEST halted");
}

#[test]
fn successful_commands_stop_descendants_and_preserve_results() {
    completed_command(0);
}

#[test]
fn failed_commands_stop_descendants_and_preserve_results() {
    completed_command(7);
}

fn completed_command(exit: u8) {
    let fixture = descendant::Fixture::new(exit);
    let name = fixture.program().to_string_lossy().to_string();
    let result = text(
        fixture.root(),
        Command::new(fixture.program()),
        &name,
        &AtomicBool::new(false),
        Duration::from_secs(10),
    );
    fixture.assert_stopped();
    assert!(fixture.input().is_empty(), "plain runs receive no input");
    if exit == 0 {
        assert_eq!(result.unwrap(), " TEST reply café  \n");
    } else {
        let error = result.unwrap_err();
        assert!(error.starts_with(&name), "{error}");
        assert!(error.contains("exit status: 7"), "{error}");
        assert!(error.ends_with("TEST command failed"), "{error}");
    }
}

#[test]
fn commands_name_themselves_when_they_fail() {
    let root = temp();
    let why = text(
        root.path(),
        Command::new("tinkery-TEST-missing-gh"),
        "tinkery-TEST-missing-gh",
        &AtomicBool::new(false),
        Duration::from_secs(10),
    )
    .unwrap_err();
    assert_eq!(
        why,
        "Couldn't start tinkery-TEST-missing-gh. Check that it is installed and on PATH."
    );
    let mut failing = Command::new("sh");
    failing.args(["-c", "echo 'fatal: TEST remote ref' >&2; exit 3"]);
    let why = text(
        root.path(),
        failing,
        "sh",
        &AtomicBool::new(false),
        Duration::from_secs(10),
    )
    .unwrap_err();
    assert_eq!(why, "sh exited with exit status: 3: fatal: TEST remote ref");
}

#[test]
fn completed_tasks_wake_their_owner_with_success_or_failure() {
    for outcome in [Ok("TEST result"), Err("TEST refusal".to_owned())] {
        thread::spawn(move || {
            let expected = outcome.clone();
            let task = Task::start(move |_| {
                thread::sleep(Duration::from_millis(30));
                outcome
            });
            let started = Instant::now();
            thread::park_timeout(Duration::from_secs(2));
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!(task.poll(), Some(expected));
        })
        .join()
        .unwrap();
    }
}

#[test]
fn completion_before_parking_retains_the_wakeup() {
    thread::spawn(|| {
        let task = Task::start(|_| Ok("TEST ready"));
        let until = Instant::now() + Duration::from_secs(2);
        while task.poll().is_none() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(1));
        }
        let started = Instant::now();
        thread::park_timeout(Duration::from_secs(2));
        assert!(started.elapsed() < Duration::from_secs(1));
    })
    .join()
    .unwrap();
}
