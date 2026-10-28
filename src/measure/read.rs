//! A bounded, cancellable GitHub CLI read. No shell, provider, writes or implicit retries.

use std::{
    io,
    os::{fd::AsRawFd, unix::process::CommandExt},
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
    let stop_drains = Arc::new(AtomicBool::new(false));
    let out = drain(child.stdout.take().unwrap(), LIMIT, stop_drains.clone());
    let err = drain(child.stderr.take().unwrap(), 64 * 1024, stop_drains.clone());
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
    stop_drains.store(true, Ordering::SeqCst);
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

// Each owned reader closes its pipe on cancellation, even if an unowned writer stays alive.
fn drain(
    mut pipe: impl io::Read + AsRawFd + Send + 'static,
    limit: u64,
    stop: Arc<AtomicBool>,
) -> JoinHandle<(bool, Vec<u8>)> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        // SAFETY: fcntl changes flags only on this reader's live, owned pipe descriptor.
        let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
        if flags == -1
            || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                == -1
        {
            return (false, bytes);
        }
        let mut chunk = [0; 8192];
        while !stop.load(Ordering::SeqCst) && bytes.len() as u64 <= limit {
            let remaining = chunk.len().min((limit + 1 - bytes.len() as u64) as usize);
            match pipe.read(&mut chunk[..remaining]) {
                Ok(0) => return (true, bytes),
                Ok(count) => bytes.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        (false, bytes)
    })
}

fn stop(child: &mut Child) {
    // SAFETY: signal only the process group created for this read.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}
