//! Explicit repo declarations and one cancellable exact-head preview. No command inference,
//! public URL, operator checkout, agent or durable preview record is supported.

use crate::sdlc::{self, DEADLINE, Read};
use serde::Deserialize;
use std::{
    io::{Read as _, Write as _},
    net::{SocketAddr, TcpStream},
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const DECLARE: &str = "No preview declared. Add tinkery.preview: {\"command\":\"npm run dev\",\"url\":\"http://127.0.0.1:5173/\"}";

/// A declared shell line and loopback-only HTTP address from the exact result commit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Spec {
    command: String,
    url: String,
}

impl Spec {
    fn parse(source: &str) -> Result<Self, String> {
        let spec: Self = serde_json::from_str(source).map_err(|_| {
            "Invalid tinkery.preview; declare one JSON object with command and url.".to_owned()
        })?;
        if spec.command.trim().is_empty()
            || spec.command.len() > 4096
            || spec.command.chars().any(char::is_control)
        {
            return Err("Invalid preview command; use one nonempty shell line.".into());
        }
        spec.address()?;
        Ok(spec)
    }

    fn address(&self) -> Result<(SocketAddr, &str), String> {
        let invalid = || "Preview URL must be http://127.0.0.1:PORT/path (local only).".to_owned();
        let rest = self
            .url
            .strip_prefix("http://127.0.0.1:")
            .ok_or_else(invalid)?;
        if self
            .url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
            || rest.contains(['#', '?', '@'])
        {
            return Err(invalid());
        }
        let (port, path) = rest
            .split_once('/')
            .map_or((rest, "/"), |(port, _)| (port, &rest[port.len()..]));
        let port: u16 = port.parse().map_err(|_| invalid())?;
        if port == 0 {
            return Err(invalid());
        }
        Ok((SocketAddr::from(([127, 0, 0, 1], port)), path))
    }

    /// Exact command and full commit disclosure, shown before v can execute it.
    pub(super) fn disclosure(&self, head: &str) -> String {
        format!(
            "v run {:?} at {head}\nLocal preview: {}",
            self.command, self.url
        )
    }
}

/// Read only the declared file at this snapshot's commit; absence is not inferred from scripts.
pub(super) fn declaration(
    root: &Path,
    url: &str,
    head: &str,
    stop: &AtomicBool,
) -> Result<Spec, String> {
    let repo = repository(url)?;
    let mut command = Command::new("gh");
    command.args([
        "api",
        "-H",
        "Accept: application/vnd.github.raw+json",
        &format!("repos/{repo}/contents/tinkery.preview?ref={head}"),
    ]);
    match sdlc::execute(root, command, stop, DEADLINE) {
        Ok(source) => Spec::parse(&source),
        Err(why) if why.contains("HTTP 404") => Err(DECLARE.into()),
        Err(_) => {
            Err("Couldn't read tinkery.preview at this commit. Check gh access and press r.".into())
        }
    }
}

fn repository(url: &str) -> Result<&str, String> {
    url.strip_prefix("https://github.com/")
        .and_then(|s| s.split_once("/pull/").map(|(repo, _)| repo))
        .ok_or_else(|| "Result repository unknown; press r.".into())
}

/// Own the preview through readiness and browser navigation; dropping stops and reaps it.
pub(super) struct Preview {
    _worker: Read<()>,
    status: Arc<Mutex<String>>,
}

impl Preview {
    /// Start only after the explicit v action, using the already-disclosed declaration.
    pub(super) fn start(url: String, head: String, spec: Spec) -> Self {
        let status = Arc::new(Mutex::new(
            "Preparing temporary exact-commit checkout… Esc stops.".into(),
        ));
        let progress = status.clone();
        let worker = Read::task(move |stop| {
            if let Err(why) = run(&url, &head, &spec, stop, &progress) {
                *progress.lock().unwrap() =
                    format!("Preview stopped: {why} Press Esc, then v to retry.");
            }
            Ok(())
        });
        Self {
            _worker: worker,
            status,
        }
    }

    /// Current lifecycle state only; never claims the app was accepted or inspected.
    pub(super) fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: this child owns a new process group; no operator process is targeted.
        unsafe {
            libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run(
    url: &str,
    head: &str,
    spec: &Spec,
    stop: &AtomicBool,
    progress: &Mutex<String>,
) -> Result<(), String> {
    let temp = tempfile::Builder::new()
        .prefix("tinkery-TEST-preview-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let root = temp.path();
    let git = |args: &[&str]| {
        let mut command = Command::new("git");
        command
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "credential.helper=",
                "-c",
                "credential.helper=!gh auth git-credential",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0");
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_CONFIG_COUNT",
        ] {
            command.env_remove(key);
        }
        sdlc::execute(root, command, stop, DEADLINE)
    };
    git(&["init", "--quiet"])?;
    git(&[
        "fetch",
        "--quiet",
        "--depth=1",
        &format!("https://github.com/{}.git", repository(url)?),
        head,
    ])?;
    git(&["checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
    if git(&["rev-parse", "HEAD"])?.trim() != head {
        return Err("Checkout commit disagrees with the displayed result.".into());
    }
    // Read the blob, not a working-tree symlink or checkout-filter output.
    let checked = Spec::parse(&git(&["show", "HEAD:tinkery.preview"])?)?;
    if &checked != spec {
        return Err("Preview declaration changed; press r before running.".into());
    }
    let (address, path) = spec.address()?;
    if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
        return Err("Preview port is already in use; stop that server before retrying.".into());
    }
    let home = root.join("TEST-home");
    std::fs::create_dir(&home).map_err(|e| e.to_string())?;
    if stop.load(Ordering::SeqCst) {
        return Err("Cancelled.".into());
    }
    let child = Command::new("sh")
        .args(["-c", &spec.command])
        .current_dir(root)
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Couldn't start declared command: {e}"))?;
    let mut process = Process(child);
    *progress.lock().unwrap() = format!("Waiting for {}… Esc stops.", spec.url);
    let until = Instant::now() + DEADLINE;
    while !stop.load(Ordering::SeqCst) {
        if let Some(status) = process.0.try_wait().map_err(|e| e.to_string())? {
            return Err(format!(
                "Declared command exited ({status}); check it externally."
            ));
        }
        if answers(address, path) {
            break;
        }
        if Instant::now() >= until {
            return Err("Local URL did not answer within 30 seconds.".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
    if stop.load(Ordering::SeqCst) {
        return Ok(());
    }
    let mut opener = Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    });
    opener.arg(&spec.url);
    sdlc::execute(root, opener, stop, DEADLINE)
        .map_err(|_| "Couldn't open browser; check the default opener.".to_owned())?;
    *progress.lock().unwrap() = format!(
        "Local URL answered; sent {} to browser. Esc stops and reaps.",
        spec.url
    );
    while !stop.load(Ordering::SeqCst) {
        if let Some(status) = process.0.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("Declared command exited ({status})."));
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn answers(address: SocketAddr, path: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
    if write!(stream, "GET {path} HTTP/1.0\r\nHost: {address}\r\n\r\n").is_err() {
        return false;
    }
    let mut response = [0; 64];
    let n = stream.read(&mut response).unwrap_or(0);
    let line = String::from_utf8_lossy(&response[..n]);
    line.starts_with("HTTP/1.")
        && line
            .split_whitespace()
            .nth(1)
            .is_some_and(|s| s.starts_with('2'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarations_refuse_public_or_ambiguous_urls_and_empty_commands() {
        for url in [
            "https://example.com",
            "http://localhost:5173",
            "http://127.0.0.1:0/",
            "http://127.0.0.1:12@evil/",
            "http://127.0.0.1:80/\r\nx",
        ] {
            assert!(
                Spec::parse(&serde_json::json!({"command":"TEST preview", "url":url}).to_string())
                    .is_err(),
                "{url}"
            );
        }
        assert!(Spec::parse(r#"{"command":" ","url":"http://127.0.0.1:5173/"}"#).is_err());
        let spec =
            Spec::parse(r#"{"command":"TEST preview","url":"http://127.0.0.1:5173/"}"#).unwrap();
        assert!(spec.disclosure(&"a".repeat(40)).contains("TEST preview"));
    }
}
