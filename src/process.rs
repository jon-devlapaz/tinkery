//! Bounded child processes and the background task that owns one. A child starts in its own
//! process group with no shell added, runs under a deadline and a cancel flag, and keeps a bounded
//! amount of output. Every finished, failed or cancelled run stops that group and reaps the direct
//! child; descendants that leave the group are outside this ownership. Exit status, stdout and
//! stderr stay apart. Screen words and tool error formats belong to callers. No implicit retries,
//! except waiting up to a second for a busy executable. Names no phase and no tool.

use std::{
    io::{self, Write as _},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[cfg(test)]
mod tests;

/// How much one run may print and how long it may take.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    /// Most stdout bytes; one more stops the run as [`Failure::TooMuch`].
    pub(crate) stdout: usize,
    /// Most stderr bytes kept; later bytes are read and dropped, and the output says so.
    pub(crate) stderr: usize,
    /// How long the run may take before it is stopped.
    pub(crate) deadline: Duration,
    /// Stop the group as soon as the direct child exits, instead of waiting until everything
    /// holding its output closes it.
    pub(crate) stop_at_exit: bool,
}

impl Limits {
    /// 4 MiB of stdout and 64 KiB of stderr, stopped after `deadline`.
    pub(crate) fn text(deadline: Duration) -> Self {
        Self {
            stdout: 4 * 1024 * 1024,
            stderr: 64 * 1024,
            deadline,
            stop_at_exit: false,
        }
    }
}

/// A run that ended on its own, whatever its exit status.
#[derive(Debug)]
pub(crate) struct Output {
    /// How the direct child exited.
    pub(crate) status: ExitStatus,
    /// Everything it printed on stdout, within the limit.
    pub(crate) stdout: Vec<u8>,
    /// The first bytes it printed on stderr, up to the limit.
    pub(crate) stderr: Vec<u8>,
    /// True when stderr passed its limit and later bytes were dropped.
    pub(crate) stderr_cut: bool,
    /// Whether all input reached the child's stdin.
    pub(crate) input: io::Result<()>,
}

impl Output {
    /// The last non-empty stderr line, trimmed, or "no details".
    pub(crate) fn said(&self) -> String {
        String::from_utf8_lossy(&self.stderr)
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map_or_else(|| "no details".into(), |line| line.trim().to_owned())
    }

    /// One line naming `name`, its exit status and `said`.
    pub(crate) fn exited(&self, name: &str, said: &str) -> String {
        format!("{name} exited with {}: {said}", self.status)
    }
}

/// Why a run has no output. The process is already stopped and reaped.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The child could not start.
    Start(io::Error),
    /// The cancel flag was set.
    Cancelled,
    /// The deadline passed.
    Late(Duration),
    /// Stdout passed this many bytes.
    TooMuch(usize),
    /// Reading the child's output failed.
    Read(io::Error),
    /// Waiting for the child failed.
    Wait(io::Error),
    /// The caller's watch stopped it, with the caller's words.
    Halted(String),
}

impl Failure {
    /// Plain words naming `name`, for callers with nothing more specific to say.
    pub(crate) fn words(&self, name: &str) -> String {
        match self {
            Self::Start(_) => {
                format!("Couldn't start {name}. Check that it is installed and on PATH.")
            }
            Self::Cancelled => "Stopped.".into(),
            Self::Late(deadline) => format!(
                "{name} took longer than {} s and was stopped.",
                deadline.as_secs()
            ),
            Self::TooMuch(limit) => format!(
                "{name} printed more than {}; nothing is shown.",
                size(*limit)
            ),
            Self::Read(_) => format!("Couldn't read what {name} printed."),
            Self::Wait(_) => format!("Couldn't finish {name}."),
            Self::Halted(why) => why.clone(),
        }
    }
}

fn size(bytes: usize) -> String {
    match bytes {
        b if b >= 1024 * 1024 && b % (1024 * 1024) == 0 => format!("{} MiB", b / (1024 * 1024)),
        b if b >= 1024 && b % 1024 == 0 => format!("{} KiB", b / 1024),
        b => format!("{b} bytes"),
    }
}

/// Run `command` in `root` with [`Limits::text`]. Success is its UTF-8 stdout; any other end is
/// one line naming `name`, ending with the last line it printed on stderr.
pub(crate) fn text(
    root: &Path,
    mut command: Command,
    name: &str,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<String, String> {
    command.current_dir(root);
    let output = run(command, Limits::text(deadline), cancel).map_err(|f| f.words(name))?;
    let failed = (!output.status.success()).then(|| output.exited(name, &output.said()));
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| format!("{name} printed text that isn't UTF-8."))?;
    failed.map_or(Ok(stdout), Err)
}

/// Run `command` with no input until it ends, fails, is cancelled or passes `limits`.
pub(crate) fn run(
    command: Command,
    limits: Limits,
    cancel: &AtomicBool,
) -> Result<Output, Failure> {
    feed(command, None, limits, cancel, |_| None)
}

/// Like [`run`], with `input` on stdin, and `watch` asked on each check whether to stop. `watch`
/// learns whether the direct child has exited; its words become [`Failure::Halted`].
pub(crate) fn feed(
    mut command: Command,
    input: Option<Vec<u8>>,
    limits: Limits,
    cancel: &AtomicBool,
    mut watch: impl FnMut(bool) -> Option<String>,
) -> Result<Output, Failure> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = spawn(&mut command).map_err(Failure::Start)?;
    let result = drive(
        &mut child,
        input.unwrap_or_default(),
        limits,
        cancel,
        &mut watch,
    );
    stop(&mut child);
    result
}

/// Start `command` as the leader of a new process group, waiting up to a second while its
/// executable is busy: one being replaced cannot start until its writer closes it.
pub(crate) fn spawn(command: &mut Command) -> io::Result<Child> {
    command.process_group(0);
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        match command.spawn() {
            Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) && Instant::now() < until => {
                thread::sleep(Duration::from_millis(10));
            }
            started => return started,
        }
    }
}

/// Kill the child's process group, then reap the child, even if it already exited.
pub(crate) fn stop(child: &mut Child) {
    signal(child);
    let _ = child.kill();
    let _ = child.wait();
}

fn signal(child: &Child) {
    // SAFETY: kill(2) on the group `spawn` created for this child; no memory is shared.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
}

fn drive(
    child: &mut Child,
    input: Vec<u8>,
    limits: Limits,
    cancel: &AtomicBool,
    watch: &mut dyn FnMut(bool) -> Option<String>,
) -> Result<Output, Failure> {
    let mut stdin = child.stdin.take().filter(|_| !input.is_empty());
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    for fd in [
        stdin.as_ref().map(AsRawFd::as_raw_fd),
        stdout.as_ref().map(AsRawFd::as_raw_fd),
        stderr.as_ref().map(AsRawFd::as_raw_fd),
    ]
    .into_iter()
    .flatten()
    {
        nonblocking(fd).map_err(Failure::Read)?;
    }
    let (mut out, mut err, mut cut, mut written) = (Vec::new(), Vec::new(), false, 0);
    let mut sent = Ok(());
    let mut status = None;
    let until = Instant::now() + limits.deadline;
    let mut chunk = [0; 8192];
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(Failure::Cancelled);
        }
        let now = Instant::now();
        if now >= until {
            return Err(Failure::Late(limits.deadline));
        }
        if let Some(why) = watch(status.is_some()) {
            return Err(Failure::Halted(why));
        }
        if status.is_none() {
            status = child.try_wait().map_err(Failure::Wait)?;
            if status.is_some() && limits.stop_at_exit {
                signal(child);
            }
        }
        if let Some(status) = status
            && stdin.is_none()
            && stdout.is_none()
            && stderr.is_none()
        {
            return Ok(Output {
                status,
                stdout: out,
                stderr: err,
                stderr_cut: cut,
                input: sent,
            });
        }
        let mut fds: Vec<libc::pollfd> = [
            stdin.as_ref().map(|p| (p.as_raw_fd(), libc::POLLOUT)),
            stdout.as_ref().map(|p| (p.as_raw_fd(), libc::POLLIN)),
            stderr.as_ref().map(|p| (p.as_raw_fd(), libc::POLLIN)),
        ]
        .into_iter()
        .flatten()
        .map(|(fd, events)| libc::pollfd {
            fd,
            events,
            revents: 0,
        })
        .collect();
        let wait = (until - now).min(Duration::from_millis(10));
        // SAFETY: `fds` is a live, correctly sized array of pollfd for this call only.
        unsafe {
            libc::poll(
                fds.as_mut_ptr(),
                fds.len() as libc::nfds_t,
                wait.as_millis().max(1) as libc::c_int,
            );
        }
        if let Some(pipe) = stdin.as_mut() {
            match pipe.write(&input[written..]) {
                Ok(count) => {
                    written += count;
                    if written == input.len() {
                        stdin = None;
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => {
                    sent = Err(e);
                    stdin = None;
                }
            }
        }
        if let Some(pipe) = stdout.as_mut() {
            if drain(pipe, &mut chunk, |bytes| out.extend_from_slice(bytes))? {
                stdout = None;
            }
            if out.len() > limits.stdout {
                return Err(Failure::TooMuch(limits.stdout));
            }
        }
        if let Some(pipe) = stderr.as_mut()
            && drain(pipe, &mut chunk, |bytes| {
                let room = limits.stderr.saturating_sub(err.len());
                cut |= bytes.len() > room;
                err.extend_from_slice(&bytes[..room.min(bytes.len())]);
            })?
        {
            stderr = None;
        }
    }
}

/// Read one ready chunk; true at end of file.
fn drain(
    pipe: &mut impl io::Read,
    chunk: &mut [u8],
    keep: impl FnOnce(&[u8]),
) -> Result<bool, Failure> {
    match pipe.read(chunk) {
        Ok(0) => Ok(true),
        Ok(count) => {
            keep(&chunk[..count]);
            Ok(false)
        }
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(Failure::Read(e)),
    }
}

fn nonblocking(fd: libc::c_int) -> io::Result<()> {
    // SAFETY: fcntl changes flags only on this run's live, owned pipe descriptor.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Work on its own thread, owned by the caller. Completion unparks the thread that started it
/// after publishing the result; dropping the task sets its cancel flag and waits for the work to
/// return. A result that arrives after cancellation is never delivered.
pub(crate) struct Task<T> {
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<T, String>>,
    worker: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Task<T> {
    /// Start `work` with the task's cancel flag.
    pub(crate) fn start(
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (send, result) = mpsc::channel();
        let stop = cancel.clone();
        let owner = thread::current();
        let worker = thread::spawn(move || {
            let outcome = work(&stop);
            if !stop.load(Ordering::SeqCst) {
                let _ = send.send(outcome);
                owner.unpark();
            }
        });
        Self {
            cancel,
            result,
            worker: Some(worker),
        }
    }

    /// The result or why it failed; `None` while the work is still running.
    pub(crate) fn poll(&self) -> Option<Result<T, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("This read stopped unexpectedly. Try again.".into()))
            }
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
