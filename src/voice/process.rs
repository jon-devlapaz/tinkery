use super::{Command, Event, Failure, VoiceHost, protocol::MAX_LINE};
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
static NODE: OnceLock<Result<Node, Failure>> = OnceLock::new();

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
            .ok_or_else(|| Failure::new("protocol", "Voice is closed. F6 reloads voice."))?
            .try_send(command)
            .map_err(|_| {
                Failure::new(
                    "protocol",
                    "Voice command queue is unavailable. Restart think.",
                )
            })
    }
    fn poll(&mut self) -> Option<Event> {
        if self.overflow.swap(false, Ordering::SeqCst) {
            return Some(
                Failure::new(
                    "protocol",
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
fn launch(command: &mut Process) -> std::io::Result<Child> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()
}
fn kill_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Process::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}
fn probe(mut command: Process, stop: &AtomicBool) -> Result<String, Failure> {
    let mut child = launch(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()),
    )
    .map_err(|_| {
        Failure::new(
            "no-node",
            "Voice needs Node 22 or later. Set TINKERY_NODE to its executable.",
        )
    })?;
    let stdout = child.stdout.take().unwrap();
    let reader = thread::spawn(move || {
        let mut text = String::new();
        stdout.take(1024).read_to_string(&mut text).map(|_| text)
    });
    let end = Instant::now() + Duration::from_secs(3);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < end && !stop.load(Ordering::SeqCst) => {
                thread::sleep(Duration::from_millis(10))
            }
            _ => {
                kill_group(&mut child);
                let _ = child.wait();
                break None;
            }
        }
    };
    kill_group(&mut child);
    let text = reader.join().ok().and_then(Result::ok);
    if !status.is_some_and(|s| s.success()) {
        return Err(Failure::new(
            "no-node",
            "Voice needs Node 22 or later. Set TINKERY_NODE to its executable.",
        ));
    }
    text.map(|text| text.trim().into()).ok_or_else(|| {
        Failure::new(
            "no-node",
            "Could not read Node discovery result. Set TINKERY_NODE.",
        )
    })
}
fn version(path: &std::ffi::OsStr, stop: &AtomicBool) -> Result<String, Failure> {
    let mut command = Process::new(path);
    command.arg("--version");
    let value = probe(command, stop)?;
    let major = value
        .strip_prefix('v')
        .and_then(|v| v.split('.').next())
        .filter(|v| v.parse::<u32>().is_ok_and(|v| v >= 22))
        .ok_or_else(|| {
            Failure::new(
                "no-node",
                "Invalid Node version. Set TINKERY_NODE to a Node 22+ executable.",
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
    let shell = std::env::var_os("SHELL").ok_or_else(|| {
        Failure::new(
            "no-node",
            "Voice needs Node 22 or later. Set TINKERY_NODE to its executable.",
        )
    })?;
    let mut command = Process::new(shell);
    command.args(["-lc", "command -v node"]);
    let path = PathBuf::from(probe(command, stop)?);
    if !path.is_absolute() {
        return Err(Failure::new(
            "no-node",
            "Login shell did not find Node. Set TINKERY_NODE.",
        ));
    }
    version(path.as_os_str(), stop)?;
    Ok(Node { executable: path })
}
fn discover(stop: &AtomicBool) -> Result<Node, Failure> {
    if let Some(cached) = NODE.get() {
        return cached.clone();
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
    if !stop.load(Ordering::SeqCst) {
        let _ = NODE.set(result.clone());
    }
    result
}
fn reap(child: &mut Child) {
    let end = Instant::now() + Duration::from_millis(200);
    loop {
        if child.try_wait().ok().flatten().is_some() {
            kill_group(child);
            return;
        }
        if Instant::now() >= end {
            kill_group(child);
            let _ = child.wait();
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
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
            "no-node"
        );
        assert!(start.elapsed() < Duration::from_secs(1));
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
            "no-node"
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
    let mut child = match launch(
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
                    "no-node",
                    "Could not start Node. Set TINKERY_NODE to a Node 22+ executable.",
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
                        Failure::new("protocol", "Could not read voice response. Restart think.")
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
                    Failure::new("engine-load", "Voice helper exited. F6 reloads voice.").event(),
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
                        Failure::new("protocol", "Could not send voice command. Restart think.")
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
