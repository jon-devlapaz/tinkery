use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

/// A fake command whose child signals readiness, closes all pipes and outlives its parent.
/// Owns cleanup even when a regression assertion fails; it never starts a provider.
pub(super) struct Fixture(tempfile::TempDir);

impl Fixture {
    pub(super) fn new(exit: u8) -> Self {
        let fixture = Self(
            tempfile::Builder::new()
                .prefix("tinkery-TEST-command-descendant-")
                .tempdir()
                .unwrap(),
        );
        fs::write(fixture.root().join("TEST-exit"), exit.to_string()).unwrap();
        fs::write(
            fixture.program(),
            r#"#!/usr/bin/env python3
import pathlib, subprocess, sys, time
if sys.argv[1:] == ['--TEST-ready']:
    sys.exit(0)
pathlib.Path('TEST-input').write_bytes(sys.stdin.buffer.read())
child = subprocess.Popen([sys.executable, '-c', '''import os, pathlib, time
ready = pathlib.Path('TEST-child-pid')
ready.with_suffix('.tmp').write_text(str(os.getpid()))
ready.with_suffix('.tmp').replace(ready)
time.sleep(60)
'''], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
until = time.monotonic() + 5
while not pathlib.Path('TEST-child-pid').exists():
    if child.poll() is not None or time.monotonic() >= until:
        child.kill()
        child.wait()
        raise RuntimeError('TEST child never became ready')
    time.sleep(.01)
print(' TEST reply café  ')
print('TEST command failed', file=sys.stderr)
sys.exit(int(pathlib.Path('TEST-exit').read_text()))
"#,
        )
        .unwrap();
        fs::set_permissions(fixture.program(), fs::Permissions::from_mode(0o755)).unwrap();
        // Another test thread's fork can briefly hold the new file open for writing, which makes
        // starting it fail; once it has started once, it always can.
        let until = Instant::now() + Duration::from_secs(5);
        while Command::new(fixture.program())
            .arg("--TEST-ready")
            .status()
            .is_err_and(|e| e.raw_os_error() == Some(26) && Instant::now() < until)
        {
            thread::sleep(Duration::from_millis(10));
        }
        fixture
    }

    pub(super) fn root(&self) -> &Path {
        self.0.path()
    }

    pub(super) fn program(&self) -> PathBuf {
        self.root().join("TEST-command")
    }

    pub(super) fn input(&self) -> Vec<u8> {
        fs::read(self.root().join("TEST-input")).unwrap()
    }

    fn pid(&self) -> Option<i32> {
        fs::read_to_string(self.root().join("TEST-child-pid"))
            .ok()?
            .parse()
            .ok()
            .filter(|pid| *pid > 0)
    }

    pub(super) fn assert_stopped(&self) {
        let pid = self.pid().expect("the fake descendant signalled readiness");
        let running = || {
            let result = Command::new("ps")
                .args(["-o", "stat=", "-p", &pid.to_string()])
                .output()
                .unwrap();
            let state = String::from_utf8_lossy(&result.stdout);
            // Init can briefly retain a stopped orphan as a zombie.
            !state.trim().is_empty() && !state.trim().starts_with('Z')
        };
        let until = Instant::now() + Duration::from_secs(1);
        while running() && Instant::now() < until {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !running(),
            "the command returned with descendant {pid} still running"
        );
        fs::remove_file(self.root().join("TEST-child-pid")).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(pid) = self.pid() {
            // SAFETY: this PID came from this fixture's private readiness file.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
    }
}
