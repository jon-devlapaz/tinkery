//! Running the voice helper: finding Node, starting the embedded helper script, and moving
//! commands and events across its pipes on a worker thread, with bounded queues and line
//! lengths. Think sees none of this; it talks to [`VoiceHost`].

use super::{Command, Event, Failure, Kind, VoiceHost, key_label, protocol::MAX_LINE};
use crate::process;
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, Command as Process, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const HELPER: &str = include_str!("../../voice/helper.mjs");
const SETUP: &str = include_str!("../../voice/setup.mjs");
#[derive(Clone)]
struct Node {
    executable: PathBuf,
}
static NODE: OnceLock<Node> = OnceLock::new();
/// The oldest Node major the helper runs on; every Node message names it from here.
const NODE_MAJOR: u32 = 22;
fn needs_node() -> Failure {
    Failure::new(
        Kind::NoNode,
        &format!("Voice needs Node {NODE_MAJOR} or later. Set TINKERY_NODE to its executable."),
    )
}
/// `failure` and how to bring voice back, naming the voice key as the key bar on `platform` does.
fn reload(platform: &str, kind: Kind, failure: &str) -> Failure {
    let key = key_label(platform);
    Failure::new(kind, &format!("{failure} {key} reloads voice."))
}

/// The real [`VoiceHost`]: the embedded Node helper, run and read on its own thread so think
/// never waits on it. Without a recent enough Node, voice says how to point it at one instead.
pub struct ProcessHost {
    commands: Option<SyncSender<Command>>,
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Default for ProcessHost {
    fn default() -> Self {
        Self::new()
    }
}
impl ProcessHost {
    /// Starts the worker thread; the helper itself starts on the worker.
    pub fn new() -> Self {
        let (commands, input) = mpsc::sync_channel(32);
        let (output, events) = mpsc::sync_channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let overflow = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_overflow = overflow.clone();
        let worker = thread::spawn(move || run(input, output, worker_stop, worker_overflow));
        Self {
            commands: Some(commands),
            events,
            stop,
            overflow,
            worker: Some(worker),
        }
    }
}
impl VoiceHost for ProcessHost {
    fn setup_session(&self) -> Arc<std::sync::Mutex<super::Consent>> {
        super::setup::session()
    }
    fn send(&mut self, command: Command) -> Result<(), Failure> {
        self.commands
            .as_ref()
            .ok_or_else(|| reload(std::env::consts::OS, Kind::Protocol, "Voice is closed."))?
            .try_send(command)
            .map_err(|_| {
                Failure::new(
                    Kind::Protocol,
                    "Voice command queue is unavailable. Restart think.",
                )
            })
    }
    fn poll(&mut self) -> Option<Event> {
        if self.overflow.swap(false, Ordering::SeqCst) {
            return Some(
                Failure::new(
                    Kind::Protocol,
                    "Voice event queue exceeded its limit. Restart think.",
                )
                .event(),
            );
        }
        self.events.try_recv().ok()
    }
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.commands.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
    fn restart(&mut self) {
        self.shutdown();
        *self = Self::new();
    }
}
impl Drop for ProcessHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn push(output: &SyncSender<Event>, overflow: &AtomicBool, event: Event) {
    if output.try_send(event).is_err() {
        overflow.store(true, Ordering::SeqCst);
    }
}
fn probe(command: Process, stop: &AtomicBool) -> Result<String, Failure> {
    let unread = || {
        Failure::new(
            Kind::NoNode,
            "Could not read Node discovery result. Set TINKERY_NODE.",
        )
    };
    let limits = process::Limits {
        stdout: 1024,
        stderr: 0,
        deadline: Duration::from_secs(3),
        stop_at_exit: true,
    };
    let output = process::run(command, limits, stop).map_err(|failure| match failure {
        process::Failure::TooMuch(_) | process::Failure::Read(_) => unread(),
        _ => needs_node(),
    })?;
    if !output.status.success() {
        return Err(needs_node());
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().into())
        .map_err(|_| unread())
}
fn version(path: &std::ffi::OsStr, stop: &AtomicBool) -> Result<String, Failure> {
    let mut command = Process::new(path);
    command.arg("--version");
    let value = probe(command, stop)?;
    let major = value
        .strip_prefix('v')
        .and_then(|v| v.split('.').next())
        .filter(|v| v.parse::<u32>().is_ok_and(|v| v >= NODE_MAJOR))
        .ok_or_else(|| {
            Failure::new(
                Kind::NoNode,
                &format!(
                    "Invalid Node version. Set TINKERY_NODE to a Node {NODE_MAJOR}+ executable."
                ),
            )
        })?;
    Ok(major.into())
}
fn default_node(stop: &AtomicBool) -> Result<Node, Failure> {
    if version(std::ffi::OsStr::new("node"), stop).is_ok() {
        return Ok(Node {
            executable: "node".into(),
        });
    }
    let shell = std::env::var_os("SHELL").ok_or_else(needs_node)?;
    let mut command = Process::new(shell);
    command.args(["-lc", "command -v node"]);
    let path = PathBuf::from(probe(command, stop)?);
    if !path.is_absolute() {
        return Err(Failure::new(
            Kind::NoNode,
            "Login shell did not find Node. Set TINKERY_NODE.",
        ));
    }
    version(path.as_os_str(), stop)?;
    Ok(Node { executable: path })
}
fn discover(stop: &AtomicBool) -> Result<Node, Failure> {
    if let Some(cached) = NODE.get() {
        return Ok(cached.clone());
    }
    let result = (|| {
        if let Some(path) = std::env::var_os("TINKERY_NODE") {
            version(&path, stop)?;
            Ok(Node {
                executable: path.into(),
            })
        } else {
            default_node(stop)
        }
    })();
    // Only a found Node is kept: a slow login shell or a Node installed later can still be found
    // when the voice key reloads voice.
    if let Ok(node) = &result
        && !stop.load(Ordering::SeqCst)
    {
        let _ = NODE.set(node.clone());
    }
    result
}
/// Give the helper a moment to exit after its stdin closes, then stop its group.
fn reap(child: &mut Child) {
    let end = Instant::now() + Duration::from_millis(200);
    while child.try_wait().ok().flatten().is_none() && Instant::now() < end {
        thread::sleep(Duration::from_millis(10));
    }
    process::stop(child);
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn discovery_releases_stdout_inherited_by_shell_children() {
        let mut command = Process::new("/bin/sh");
        command.args(["-c", "sleep 60 & printf 'v26.1.0\\n'"]);
        let start = Instant::now();
        assert_eq!(probe(command, &AtomicBool::new(false)).unwrap(), "v26.1.0");
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn cancelled_discovery_kills_and_reaps_without_waiting_for_shell() {
        let mut command = Process::new("/bin/sh");
        command.args(["-c", "sleep 60"]);
        let start = Instant::now();
        assert_eq!(
            probe(command, &AtomicBool::new(true)).unwrap_err().kind,
            Kind::NoNode
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn reload_names_the_voice_key_the_key_bar_shows() {
        let closed = |platform| reload(platform, Kind::Protocol, "Voice is closed.").message;
        assert_eq!(closed("macos"), "Voice is closed. ⌃⌥Z reloads voice.");
        assert_eq!(closed("linux"), "Voice is closed. F6 reloads voice.");
    }
    #[test]
    fn absent_explicit_executable_is_no_node() {
        assert_eq!(
            version(
                std::ffi::OsStr::new("/tinkery-TEST-missing-node"),
                &AtomicBool::new(false)
            )
            .unwrap_err()
            .kind,
            Kind::NoNode
        );
    }
}

fn run(
    input: Receiver<Command>,
    output: SyncSender<Event>,
    stop: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
) {
    let node = match discover(&stop) {
        Ok(node) if !stop.load(Ordering::SeqCst) => node,
        Ok(_) => return,
        Err(error) => {
            push(&output, &overflow, error.event());
            return;
        }
    };
    let mut child = match process::spawn(
        Process::new(node.executable)
            .args([
                "--input-type=module",
                "--eval",
                &format!("{SETUP}\n{HELPER}"),
            ])
            .env_remove("NODE_PATH")
            .env_remove("NODE_OPTIONS")
            .env_remove("TRANSCRIBE_LIBRARY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    ) {
        Ok(child) => child,
        Err(_) => {
            push(
                &output,
                &overflow,
                Failure::new(
                    Kind::NoNode,
                    &format!(
                        "Could not start Node. Set TINKERY_NODE to a Node {NODE_MAJOR}+ executable."
                    ),
                )
                .event(),
            );
            return;
        }
    };
    let stdout = child.stdout.take().unwrap();
    let reader_output = output.clone();
    let reader_overflow = overflow.clone();
    let reader = thread::spawn(move || {
        let mut source = BufReader::new(stdout);
        loop {
            let mut line = Vec::new();
            match source
                .by_ref()
                .take((MAX_LINE + 1) as u64)
                .read_until(b'\n', &mut line)
            {
                Ok(0) => break,
                Ok(_) => match Event::parse(&line) {
                    Ok(event) => push(&reader_output, &reader_overflow, event),
                    Err(error) => {
                        push(&reader_output, &reader_overflow, error.event());
                        break;
                    }
                },
                Err(_) => {
                    push(
                        &reader_output,
                        &reader_overflow,
                        Failure::new(
                            Kind::Protocol,
                            "Could not read voice response. Restart think.",
                        )
                        .event(),
                    );
                    break;
                }
            }
        }
    });
    let mut stdin = child.stdin.take().unwrap();
    while !stop.load(Ordering::SeqCst) && !overflow.load(Ordering::SeqCst) {
        match child.try_wait() {
            Ok(None) => {}
            _ => {
                push(
                    &output,
                    &overflow,
                    reload(
                        std::env::consts::OS,
                        Kind::EngineLoad,
                        "Voice helper exited.",
                    )
                    .event(),
                );
                break;
            }
        }
        match input.recv_timeout(Duration::from_millis(10)) {
            Ok(command) => {
                let line = serde_json::to_vec(&command).unwrap();
                if stdin
                    .write_all(&line)
                    .and_then(|()| stdin.write_all(b"\n"))
                    .is_err()
                {
                    push(
                        &output,
                        &overflow,
                        Failure::new(
                            Kind::Protocol,
                            "Could not send voice command. Restart think.",
                        )
                        .event(),
                    );
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    drop(stdin);
    reap(&mut child);
    let _ = reader.join();
}
