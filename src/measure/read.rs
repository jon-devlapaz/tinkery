//! A bounded, cancellable GitHub CLI read. No shell, provider, writes or implicit retries.

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

use super::Report;

const LIMIT: u64 = 4 * 1024 * 1024;

/// Owns one read; leaving the view stops and reaps its process group.
pub(super) struct Read {
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<Report, String>>,
    worker: Option<JoinHandle<()>>,
}

impl Read {
    /// Only the explicitly labelled read action may call this constructor.
    pub(super) fn start(root: &Path, since: String) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (send, result) = mpsc::channel();
        let (root, stop) = (root.to_path_buf(), cancel.clone());
        let worker = thread::spawn(move || {
            let result = run(&root, &since, &stop, Duration::from_secs(30))
                .and_then(|text| super::parse(&text));
            if !stop.load(Ordering::SeqCst) {
                let _ = send.send(result);
            }
        });
        Self {
            cancel,
            result,
            worker: Some(worker),
        }
    }

    /// Poll without blocking painting or navigation.
    pub(super) fn poll(&self) -> Option<Result<Report, String>> {
        match self.result.try_recv() {
            Ok(value) => Some(value),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "GitHub read stopped unexpectedly. Press r to retry.".into(),
            )),
        }
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
    since: &str,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<String, String> {
    let mut child = Command::new("gh")
        .args(["pr", "list", "--state", "all", "--search"])
        .arg(format!("updated:>={since} sort:updated-desc"))
        .args(["--limit", "101", "--json",
            "number,title,state,url,author,createdAt,updatedAt,closedAt,mergedAt,headRefOid,statusCheckRollup"])
        .current_dir(root)
        .env_remove("GH_REPO")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .process_group(0).spawn()
        .map_err(|_| "Couldn't start gh. Install GitHub CLI and check repo access, then press r.".to_owned())?;
    let drain = |pipe: Box<dyn std::io::Read + Send>, limit: u64| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let ok = pipe.take(limit + 1).read_to_end(&mut bytes).is_ok();
            (ok && bytes.len() as u64 <= limit, bytes)
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()), LIMIT);
    let err = drain(Box::new(child.stderr.take().unwrap()), 64 * 1024);
    let until = Instant::now() + deadline;
    let result = loop {
        if cancel.load(Ordering::SeqCst) {
            break Err("Stopped.".to_owned());
        }
        if Instant::now() >= until {
            break Err("GitHub read exceeded 30 seconds. Press r to retry.".to_owned());
        }
        match child.try_wait() {
            Ok(Some(status)) if out.is_finished() && err.is_finished() => break Ok(status),
            Ok(_) => thread::sleep(Duration::from_millis(10)),
            Err(_) => break Err("Couldn't finish reading GitHub. Press r to retry.".to_owned()),
        }
    };
    stop(&mut child);
    let (ok, bytes) = out.join().unwrap_or_default();
    let _ = err.join();
    let status = result?;
    if !ok {
        return Err(
            "GitHub output was incomplete or exceeded 4 MiB. Results unknown; press r to retry."
                .into(),
        );
    }
    if !status.success() {
        // Never display CLI stderr: it can contain credentials or terminal escape sequences.
        return Err(
            "GitHub read failed. Check gh auth status and this repo's remote/access, then press r."
                .into(),
        );
    }
    String::from_utf8(bytes)
        .map_err(|_| "GitHub returned invalid UTF-8. Results unknown; press r to retry.".into())
}

fn stop(child: &mut Child) {
    // SAFETY: signal only the process group created for this read.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}
