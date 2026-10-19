//! One bounded `sdlc.py status` read off the paint path. It promises a deadline, bounded output and
//! that dropping the read stops and reaps the whole process group; a late result is never delivered.

use std::{
    io::Read as _,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// The script every read runs, relative to the named checkout.
pub(super) const SCRIPT: &str = "_system/scripts/sdlc.py";
/// How long one read may take before it is stopped.
pub(super) const DEADLINE: Duration = Duration::from_secs(30);
const STDOUT_LIMIT: u64 = 4 * 1024 * 1024;
const STDERR_LIMIT: u64 = 64 * 1024;

/// A read in flight. Dropping it cancels, kills and reaps the process before returning.
pub(super) struct Read {
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<String, String>>,
    worker: Option<JoinHandle<()>>,
}

impl Read {
    /// Start `python3 _system/scripts/sdlc.py <args>` in `root`, stopped after `deadline`.
    pub(super) fn start(root: &Path, args: Vec<String>, deadline: Duration) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (send, result) = mpsc::channel();
        let (root, stop) = (root.to_path_buf(), cancel.clone());
        let worker = thread::spawn(move || {
            let outcome = run(&root, &args, &stop, deadline);
            if !stop.load(Ordering::SeqCst) {
                let _ = send.send(outcome);
            }
        });
        Self {
            cancel,
            result,
            worker: Some(worker),
        }
    }

    /// The finished read's stdout, or why it failed; `None` while it is still running.
    pub(super) fn poll(&self) -> Option<Result<String, String>> {
        self.result.try_recv().ok()
    }
}

impl Drop for Read {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(
    root: &Path,
    args: &[String],
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<String, String> {
    let mut child = Command::new("python3")
        .arg(SCRIPT)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| format!("Couldn't start python3: {error}"))?;
    let drain = |pipe: Box<dyn std::io::Read + Send>, limit: u64| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let read = pipe.take(limit + 1).read_to_end(&mut bytes);
            (read.is_ok() && bytes.len() as u64 <= limit, bytes)
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()), STDOUT_LIMIT);
    let err = drain(Box::new(child.stderr.take().unwrap()), STDERR_LIMIT);
    let until = Instant::now() + deadline;
    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            stop(&mut child);
            return Err("Stopped.".into());
        }
        if Instant::now() >= until {
            stop(&mut child);
            return Err(format!(
                "status took longer than {} s and was stopped.",
                deadline.as_secs()
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) if out.is_finished() && err.is_finished() => break status,
            Ok(_) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                stop(&mut child);
                return Err(format!("Couldn't wait for status: {error}"));
            }
        }
    };
    let (out_ok, out) = out.join().unwrap_or((false, Vec::new()));
    let (_, err) = err.join().unwrap_or((false, Vec::new()));
    if !out_ok {
        return Err("status printed more than 4 MiB; nothing is shown.".into());
    }
    let out =
        String::from_utf8(out).map_err(|_| "status printed text that isn't UTF-8.".to_string())?;
    if status.success() {
        return Ok(out);
    }
    // A refused read still prints the API 1 envelope with error.message on stdout.
    let message = serde_json::from_str::<serde_json::Value>(&out)
        .ok()
        .and_then(|value| value["error"]["message"].as_str().map(str::to_owned))
        .or_else(|| {
            String::from_utf8_lossy(&err)
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_owned())
        })
        .unwrap_or_else(|| "no details".into());
    Err(format!("status exited with {status}: {message}"))
}

/// Kill the whole process group, then reap the child, so no read outlives Watch.
fn stop(child: &mut Child) {
    // SAFETY: kill(2) on the group this process created with process_group(0); no memory is shared.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}
