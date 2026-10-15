//! One owned ledger read across visits; cancellation retains capacity until collection and reaping finish.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::{Cause, Reading};

pub(super) const DEADLINE: Duration = Duration::from_secs(30);
pub(super) const LIMIT: usize = 16 * 1024 * 1024;
const GRACE: Duration = Duration::from_secs(1);
const CANCEL: u8 = 1;
const TIMEOUT: u8 = 2;
const OVERFLOW: u8 = 3;
const COLLECT: u8 = 4;

#[derive(Clone)]
pub(super) struct Control {
    reason: Arc<AtomicU8>,
    deadline: Instant,
}

impl Control {
    fn stop(&self, reason: u8) {
        let _ = self
            .reason
            .compare_exchange(0, reason, Ordering::SeqCst, Ordering::SeqCst);
    }

    fn reason_at(&self, now: Instant) -> u8 {
        if now >= self.deadline {
            self.stop(TIMEOUT);
        }
        self.reason.load(Ordering::SeqCst)
    }
}

type Source = Arc<dyn Fn(PathBuf, Control) -> Reading + Send + Sync>;

pub(crate) struct Reader {
    job: Option<Job>,
    next: u64,
    source: Source,
    closed: bool,
    cleanup_error: Option<String>,
}

struct Job {
    generation: u64,
    control: Control,
    task: JoinHandle<Reading>,
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            job: None,
            next: 0,
            source: Arc::new(read),
            closed: false,
            cleanup_error: None,
        }
    }
}

impl Reader {
    #[cfg(test)]
    pub(super) fn with_source(source: Source) -> Self {
        let mut reader = Self::default();
        reader.source = source;
        reader
    }

    #[cfg(test)]
    pub(crate) fn blocked() -> (
        Self,
        std::sync::mpsc::Receiver<()>,
        std::sync::mpsc::Sender<()>,
    ) {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        let source = Arc::new(move |_, _| {
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            Reading::unavailable(Cause::Exit, "completed old request".into())
        });
        (Self::with_source(source), entered_rx, release_tx)
    }

    pub(crate) fn open(&mut self, directory: &Path, now: Instant) -> Reading {
        if self.closed {
            let mut reading = Reading::unavailable(
                Cause::Collection,
                "The ledger reader is closed; further reads are refused in this session.".into(),
            );
            if let Some(error) = &self.cleanup_error {
                reading.add_cleanup_error(error);
            }
            return reading;
        }
        if self.job.is_some() {
            return Reading::waiting(None, "Finishing the previous read…");
        }
        self.next += 1;
        let control = Control {
            reason: Arc::new(AtomicU8::new(0)),
            deadline: now + DEADLINE,
        };
        let source = self.source.clone();
        let directory = directory.to_path_buf();
        let worker_control = control.clone();
        match thread::Builder::new()
            .name("measure-read".into())
            .spawn(move || source(directory, worker_control))
        {
            Ok(task) => {
                self.job = Some(Job {
                    generation: self.next,
                    control,
                    task,
                });
                Reading::waiting(Some(self.next), "Reading ledger…")
            }
            Err(error) => Reading::unavailable(
                Cause::Launch,
                format!("Reader task could not start: {error}"),
            ),
        }
    }

    pub(crate) fn cancel(&mut self) {
        if let Some(job) = &self.job {
            job.control.stop(CANCEL);
        }
    }

    pub(crate) fn poll(&mut self, mut visible: Option<&mut Reading>, now: Instant) {
        let Some(job) = &self.job else { return };
        let reason = job.control.reason_at(now);
        let eligible = visible
            .as_ref()
            .is_some_and(|r| r.request == Some(job.generation));
        if eligible && reason == TIMEOUT {
            let r = visible.as_deref_mut().unwrap();
            if !r.failed() {
                let request = r.request;
                *r = Reading::unavailable(
                    Cause::Timeout,
                    "The request deadline elapsed. Cleanup may still be pending.".into(),
                );
                r.request = request;
            }
        }
        if !job.task.is_finished() {
            return;
        }
        let job = self.job.take().unwrap();
        let mut result = job.task.join().unwrap_or_else(|_| {
            self.closed = true;
            let mut failure = Reading::unavailable(Cause::Collection,
                "The ledger reader task failed unexpectedly. Further reads are refused in this session.".into());
            failure.add_cleanup_error("Reader task panicked; cleanup could not be confirmed.");
            failure
        });
        if let Some(error) = result.cleanup_error.take() {
            self.closed = true;
            self.cleanup_error = Some(error);
        }
        if eligible && reason != CANCEL {
            if reason == TIMEOUT && !matches!(result.report_cause(), Some(Cause::Timeout)) {
                result = Reading::unavailable(
                    Cause::Timeout,
                    "The request deadline elapsed before this report could be accepted.".into(),
                );
                if let Some(error) = &self.cleanup_error {
                    result.add_cleanup_error(error);
                }
            }
            *visible.unwrap() = result;
        } else if let Some(r) = visible
            && r.request.is_none()
            && r.waiting_state()
        {
            *r = if let Some(error) = &self.cleanup_error {
                Reading::unavailable(
                    Cause::Collection,
                    format!(
                        "Cleanup could not be confirmed; further reads are refused in this session.\n{}",
                        super::display_bytes(error.as_bytes())
                    ),
                )
            } else {
                Reading::waiting(
                    None,
                    "Previous read stopped. Return to the menu and open Measure to read again.",
                )
            };
        }
    }

    /// Restore the terminal before calling: cleanup-first shutdown has no hard OS exit bound.
    pub(crate) fn shutdown(&mut self, mut notice: impl FnMut(&str)) {
        let started = Instant::now();
        self.drain(
            &mut notice,
            || started.elapsed(),
            || thread::sleep(Duration::from_millis(10)),
        );
    }

    fn drain(
        &mut self,
        mut notice: impl FnMut(&str),
        mut elapsed: impl FnMut() -> Duration,
        mut pause: impl FnMut(),
    ) {
        self.closed = true;
        self.cancel();
        let mut warned = false;
        while self.job.is_some() {
            self.poll(None, Instant::now());
            if self.job.is_some() && !warned && elapsed() >= Duration::from_secs(3) {
                notice("Ledger reader cleanup is still pending; exit time is unknown.");
                warned = true;
            }
            pause();
        }
        if let Some(error) = self.cleanup_error.take() {
            notice(&format!(
                "Ledger reader cleanup could not be confirmed: {}",
                super::display_bytes(error.as_bytes())
            ));
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.shutdown(|notice| eprintln!("{notice}"));
    }
}

#[cfg(unix)]
fn read(directory: PathBuf, control: Control) -> Reading {
    use std::process::Command;
    let mut command = Command::new("python3");
    command.args(["-B", "-m", "tink_substrate", "ledger", "report"]);
    collect(directory, control, command)
}

#[cfg(unix)]
fn collect(directory: PathBuf, control: Control, mut command: std::process::Command) -> Reading {
    use std::{
        fs::File,
        io::{self, Read},
        os::{
            fd::{AsRawFd, OwnedFd},
            unix::process::CommandExt,
        },
        process::{Output, Stdio},
    };
    command
        .current_dir(&directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Reading::from_result(&directory, Err(error)),
    };
    let pid = child.id() as libc::pid_t;
    let mut pipes = [
        child.stdout.take().map(|p| File::from(OwnedFd::from(p))),
        child.stderr.take().map(|p| File::from(OwnedFd::from(p))),
    ];
    let mut errors = Vec::new();
    for pipe in pipes.iter().flatten() {
        // SAFETY: fcntl acts on the live owned descriptor; no pointer or pre-exec callback.
        let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            errors.push(format!("Pipe setup: {}", io::Error::last_os_error()));
            control.stop(COLLECT);
        }
    }
    let mut captured = [Vec::new(), Vec::new()];
    let mut retained = 0;
    let mut stopping = None;
    let mut exited = false;
    loop {
        let now = Instant::now();
        let reason = control.reason_at(now);
        if reason != 0 || exited {
            let since = *stopping.get_or_insert_with(|| {
                signal(pid, libc::SIGTERM, &mut errors);
                // No blocked readers survive cancellation, even when writers escape the group.
                pipes = [None, None];
                now
            });
            if now.duration_since(since) >= GRACE {
                // Keep the leader unreaped until the last group signal to prevent PID reuse.
                signal(pid, libc::SIGKILL, &mut errors);
                break;
            }
        } else {
            for (i, pipe) in pipes.iter_mut().enumerate() {
                let mut buffer = [0; 16 * 1024];
                // A fixed quota per stream keeps deadlines/cancellation fair under floods.
                for _ in 0..4 {
                    let Some(file) = pipe else { break };
                    match file.read(&mut buffer) {
                        Ok(0) => {
                            *pipe = None;
                            break;
                        }
                        Ok(n) => {
                            let keep = n.min(LIMIT - retained);
                            captured[i].extend_from_slice(&buffer[..keep]);
                            retained += keep;
                            if keep < n {
                                control.stop(OVERFLOW);
                                break;
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            errors.push(format!("Pipe collection: {e}"));
                            control.stop(COLLECT);
                            break;
                        }
                    }
                }
                if control.reason_at(Instant::now()) != 0 {
                    break;
                }
            }
            if pipes.iter().all(Option::is_none) {
                // WNOWAIT detects exit without releasing our PID before a possible group stop.
                let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
                // SAFETY: waitid initializes the provided siginfo; this is our owned child PID.
                let rc = unsafe {
                    libc::waitid(
                        libc::P_PID,
                        pid as libc::id_t,
                        info.as_mut_ptr(),
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                };
                if rc == 0 {
                    // SAFETY: successful waitid initialized info (zero pid means still running).
                    if unsafe { info.assume_init().si_pid() } != 0 {
                        // EOF and leader exit do not establish that same-group helpers stopped.
                        exited = true;
                    }
                } else {
                    let error = io::Error::last_os_error();
                    if error.kind() != io::ErrorKind::Interrupted {
                        errors.push(format!("Exit observation: {error}"));
                        control.stop(COLLECT);
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let status = child.wait();
    if let Err(error) = &status {
        errors.push(format!("Reaping: {error}"));
    }
    let reason = control.reason_at(Instant::now());
    let mut reading = if reason == 0 && errors.is_empty() {
        Reading::from_result(
            &directory,
            status.map(|status| Output {
                status,
                stdout: std::mem::take(&mut captured[0]),
                stderr: std::mem::take(&mut captured[1]),
            }),
        )
    } else {
        let cause = match reason {
            TIMEOUT => Cause::Timeout,
            OVERFLOW => Cause::OutputLimit,
            CANCEL => Cause::Cancelled,
            _ => Cause::Collection,
        };
        let details = format!(
            "Command: {}\nWorking directory: {}\n{}\n\nRetained error output (may be incomplete):\n{}",
            super::COMMAND,
            super::display_bytes(directory.as_os_str().as_encoded_bytes()),
            cause.explanation(),
            super::display_bytes(&captured[1])
        );
        Reading::unavailable(cause, details)
    };
    if !errors.is_empty() {
        let errors = errors.join("\n");
        reading.add_cleanup_error(&errors);
    }
    reading
}

#[cfg(unix)]
fn signal(pid: libc::pid_t, signal: libc::c_int, errors: &mut Vec<String>) {
    // SAFETY: negative PID targets the group created for this still-unreaped child only.
    if unsafe { libc::kill(-pid, signal) } != 0 {
        let error = std::io::Error::last_os_error();
        record_signal_error(pid, signal, error, errors, zombie_only_group);
    }
}

#[cfg(unix)]
fn record_signal_error(
    pid: libc::pid_t,
    signal: libc::c_int,
    error: std::io::Error,
    errors: &mut Vec<String>,
    inspect: impl FnOnce(libc::pid_t) -> bool,
) {
    if error.raw_os_error() != Some(libc::ESRCH)
        && !(error.raw_os_error() == Some(libc::EPERM) && inspect(pid))
    {
        errors.push(format!("Group signal {signal}: {error}"));
    }
}

// XNU killpg1 skips zombies and returns EPERM when no signalable member remains.
// Do not suppress a real permission error: require a complete, bounded group snapshot
// in which every remaining member is positively observed as a zombie.
#[cfg(target_os = "macos")]
fn zombie_only_group(pid: libc::pid_t) -> bool {
    const PROC_PGRP_ONLY: u32 = 2;
    let mut members = [0_i32; 64];
    let size = std::mem::size_of_val(&members);
    // SAFETY: libproc writes at most size bytes into this initialized PID array.
    let bytes = unsafe {
        libc::proc_listpids(
            PROC_PGRP_ONLY,
            pid as u32,
            members.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if bytes <= 0
        || bytes as usize >= size
        || !(bytes as usize).is_multiple_of(std::mem::size_of::<i32>())
    {
        return false;
    }
    members[..bytes as usize / std::mem::size_of::<i32>()]
        .iter()
        .filter(|p| **p > 0)
        .all(|member| {
            let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
            let size = std::mem::size_of::<libc::proc_bsdinfo>();
            // SAFETY: proc_pidinfo writes into the correctly sized proc_bsdinfo allocation.
            let bytes = unsafe {
                libc::proc_pidinfo(
                    *member,
                    libc::PROC_PIDTBSDINFO,
                    1, // XNU requires nonzero arg to include zombies.
                    info.as_mut_ptr().cast(),
                    size as i32,
                )
            };
            bytes as usize == size && unsafe { info.assume_init().pbi_status } == libc::SZOMB
        })
}

#[cfg(not(target_os = "macos"))]
fn zombie_only_group(_: libc::pid_t) -> bool {
    false
}

#[cfg(not(unix))]
fn read(_: PathBuf, _: Control) -> Reading {
    Reading::unavailable(
        Cause::Launch,
        "Ledger process ownership is supported on Linux and macOS only.".into(),
    )
}

#[cfg(all(test, unix))]
mod process_tests;
#[cfg(test)]
mod tests;
