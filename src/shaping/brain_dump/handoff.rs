use super::{Approach, Question, Settled, Source};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Config {
    skill: PathBuf,
    root: Option<PathBuf>,
    session: Arc<Mutex<Option<PathBuf>>>,
}
#[derive(Clone, Serialize)]
pub struct Affirmation {
    pub goal: String,
    pub outcome: String,
    pub options: Vec<Approach>,
    pub sources: Vec<Source>,
    pub answered: Vec<Settled>,
    pub unresolved: Vec<Question>,
    pub source: String,
}
pub struct Receipt {
    pub session: PathBuf,
    pub ledger: String,
    pub warning: Option<String>,
    viewer: Option<Child>,
}
impl Receipt {
    pub fn text(&self) -> String {
        format!(
            "Goal confirmed. The seed isn't written yet.\nSession: {}\nContinue this session with Seed Me in Claude Code, Codex, or Pi.\nLedger: {}{}",
            self.session.display(),
            self.ledger,
            self.warning
                .as_ref()
                .map_or(String::new(), |w| format!("\n{w}"))
        )
    }
    pub fn stop_viewer(&mut self) -> Result<(), String> {
        if let Some(child) = self.viewer.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                child.kill().map_err(|e| e.to_string())?;
            }
            child.wait().map_err(|e| e.to_string())?;
        }
        self.viewer = None;
        Ok(())
    }
}
impl Drop for Receipt {
    fn drop(&mut self) {
        let _ = self.stop_viewer();
    }
}
fn output(script: &Path, args: &[&std::ffi::OsStr]) -> Result<String, String> {
    let mut child = Command::new("python3")
        .arg(script)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot run {}: {e}", script.display()))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let drain = |pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 4 * 1024 * 1024 {
                return Err("Helper output exceeded 4 MiB".into());
            }
            String::from_utf8(bytes).map_err(|e| e.to_string())
        })
    };
    let out = drain(Box::new(stdout));
    let err = drain(Box::new(stderr));
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break Some(s),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = out.join().map_err(|_| "Helper output reader failed")??;
    let stderr = err.join().map_err(|_| "Helper error reader failed")??;
    match status {
        Some(s) if s.success() => Ok(stdout),
        Some(_) => Err(format!("Seed Me helper refused: {}", stderr.trim())),
        None => Err("Seed Me helper timed out; durable writes may already exist. Inspect the session before retrying.".into()),
    }
}
impl Config {
    pub fn new(skill: PathBuf, root: Option<PathBuf>) -> Self {
        Self {
            skill,
            root,
            session: Arc::new(Mutex::new(None)),
        }
    }
    pub fn recovery_path(&self) -> Option<PathBuf> {
        self.session.lock().unwrap().clone()
    }
    pub fn confirm(&self, a: Affirmation) -> Result<Receipt, String> {
        let skill = self
            .skill
            .canonicalize()
            .map_err(|e| format!("Cannot locate Seed Me: {e}"))?;
        let scripts = skill
            .parent()
            .ok_or("Seed Me skill has no directory")?
            .join("scripts");
        let helper = scripts.join("session.py");
        if !helper.is_file() {
            return Err(format!("Missing official helper: {}", helper.display()));
        }
        let call = |args: &[&std::ffi::OsStr]| -> Result<Value, String> {
            serde_json::from_str(&output(&helper, args)?)
                .map_err(|e| format!("Invalid helper JSON: {e}"))
        };
        let session = if let Some(path) = self.recovery_path() {
            path
        } else {
            let mut args = vec![std::ffi::OsStr::new("init")];
            if let Some(root) = &self.root {
                args.extend([std::ffi::OsStr::new("--root"), root.as_os_str()]);
            }
            let created = call(&args)?;
            let path = PathBuf::from(
                created["session"]
                    .as_str()
                    .ok_or("Helper returned no session path")?,
            );
            if !path.is_absolute() || path.to_string_lossy().chars().any(char::is_control) {
                return Err("Invalid helper session path".into());
            }
            *self.session.lock().unwrap() = Some(path.clone());
            path
        };
        let current = call(&["read".as_ref(), session.as_os_str()])?;
        if current["status"] != "active" || current["operator"] != "human" {
            return Err("Session is not an active human goal session; nothing overwritten.".into());
        }
        if current["origin"].is_null() {
            let at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            // Receipts need an ISO timestamp. Let Python provide it; never execute authored data.
            let stamp = Command::new("python3").args(["-c", "from datetime import datetime,timezone; print(datetime.now(timezone.utc).isoformat())"]).output().map_err(|e| e.to_string())?;
            if !stamp.status.success() {
                return Err("Cannot timestamp goal affirmation".into());
            }
            let stamp = String::from_utf8(stamp.stdout).map_err(|e| e.to_string())?;
            let evidence = a.sources.iter().map(|s| json!({"checked": format!("Tinkery original {} / reply to {:?}",s.id,s.in_reply_to),"at":stamp.trim(),"observed":s.text})).chain(std::iter::once(json!({"checked":"Tinkery answer/question context; questions remain provisional, not settled Seed Me decisions","at":stamp.trim(),"observed":serde_json::to_string(&json!({"answered":a.answered,"unresolved":a.unresolved})).unwrap()}))).collect::<Vec<_>>();
            let payload = json!({"expected_version":current["version"],"reason":format!("Explicit Tinkery goal affirmation at unix {at}: {}",a.source),"state":{"status":"active","draft":{"goal":a.goal,"outcome":a.outcome,"options":a.options.iter().map(|o|format!("{} — benefit: {}; cost: {}; undo cost: {}",o.label,o.benefit,o.cost,o.undo_cost)).collect::<Vec<_>>()},"goal":a.goal,"origin":"goal","current_question":null,"nodes":[{"id":"goal","kind":"decision","status":"settled","prerequisites":[],"evidence":evidence,"owner":"User","gate":"Explicit affirmation of the complete displayed goal; not seed or implementation approval","answer":a.goal,"authority":"user","authority_source":a.source}]}});
            let mut input = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            serde_json::to_writer(input.as_file_mut(), &payload).map_err(|e| e.to_string())?;
            let published = call(&[
                "publish".as_ref(),
                session.as_os_str(),
                input.path().as_os_str(),
            ]);
            // publish can save the ledger then fail saving its view. Always reconcile through read.
            if let Err(error) = published {
                let read = call(&["read".as_ref(), session.as_os_str()])?;
                if read["goal"] != a.goal {
                    return Err(error);
                }
            }
        } else if current["goal"] != a.goal {
            return Err("This session already pins another goal. No origin was replaced.".into());
        }
        let read = call(&["read".as_ref(), session.as_os_str()])?;
        let origin = read["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|n| n["id"] == read["origin"]))
            .ok_or("Missing published origin")?;
        if read["goal"] != a.goal
            || origin["answer"] != a.goal
            || origin["authority"] != "user"
            || origin["status"] != "settled"
        {
            return Err("Helper read-back did not match the affirmed goal".into());
        }
        if session.join("seed-contract.md").exists() {
            return Err(
                "A seed file now exists; goal-only handoff will not claim it is unwritten.".into(),
            );
        }
        for original in &a.sources {
            if !origin["evidence"]
                .as_array()
                .is_some_and(|entries| entries.iter().any(|e| e["observed"] == original.text))
            {
                return Err(
                    "Saved originals differ from this confirmation; no goal was replaced.".into(),
                );
            }
        }
        let viewer_script = scripts.join("viewer.py");
        let (viewer, ledger, warning) = match start_viewer(&viewer_script, &session) {
            Ok((child, url)) => (Some(child), url, None),
            Err(error) => {
                let snapshot = output(
                    &viewer_script,
                    &[session.as_os_str(), "--snapshot".as_ref()],
                )?;
                (
                    None,
                    snapshot.trim().to_owned(),
                    Some(format!(
                        "Viewer unavailable: {error}. Saved view shown instead."
                    )),
                )
            }
        };
        let mut receipt = Receipt {
            session,
            viewer,
            ledger,
            warning,
        };
        let status = call(&["status".as_ref(), receipt.session.as_os_str()])?;
        if status["status"] != "active" || status["snapshot_current"] != true {
            return Err("Goal was saved, but its active session/current view could not be verified. Use the recovery path.".into());
        }
        if receipt.viewer.is_some()
            && status["viewer"]
                .as_str()
                .is_none_or(|s| s != format!("live {}", receipt.ledger))
        {
            receipt.stop_viewer()?;
            return Err(
                "Goal was saved, but its viewer failed verification. Use the recovery path.".into(),
            );
        }
        Ok(receipt)
    }
}
fn start_viewer(script: &Path, session: &Path) -> Result<(Child, String), String> {
    let errors = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut child = Command::new("python3")
        .arg(script)
        .arg(session)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(errors.try_clone().map_err(|e| e.to_string())?))
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(20)) {
        Ok(Ok(line))
            if line.trim().starts_with("http://127.0.0.1:")
                && line.trim().ends_with("/ledger-view.html") =>
        {
            Ok((child, line.trim().to_owned()))
        }
        result => {
            let _ = child.kill();
            let _ = child.wait();
            let mut errors = errors;
            let mut detail = String::new();
            let _ = errors.seek(SeekFrom::Start(0));
            let _ = errors.take(4096).read_to_string(&mut detail);
            Err(format!(
                "Official viewer did not announce a loopback URL within 20s: {result:?}; {}",
                detail.trim()
            ))
        }
    }
}
