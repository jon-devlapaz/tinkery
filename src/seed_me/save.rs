//! Saving a confirmed goal: helper calls, the origin payload, read-back checks and the viewer.

use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Where and how to save. Holds the session once one exists, so a retry after a failure
/// continues that session instead of starting another.
#[derive(Clone)]
pub struct Config {
    skill: PathBuf,
    root: Result<PathBuf, String>,
    session: Arc<Mutex<Option<PathBuf>>>,
}
/// A goal the operator explicitly affirmed, as Seed Me will record it.
#[derive(Clone)]
pub struct Affirmed {
    /// The composed goal block exactly as displayed and confirmed. Saved unchanged.
    pub goal: String,
    /// Seed Me's draft outcome: the goal's done-when check.
    pub outcome: String,
    /// Approaches the reading proposed, recorded as options, not choices.
    pub options: Vec<Choice>,
    /// The operator's own words, recorded verbatim as evidence.
    pub originals: Vec<Original>,
    /// The caller's answered and open questions, recorded verbatim as evidence. Seed Me treats
    /// it as provisional context, never as settled decisions.
    pub context: String,
    /// Who affirmed it and how.
    pub source: String,
}
/// One approach the reading proposed, kept as a proposal.
#[derive(Clone)]
pub struct Choice {
    /// The approach in a few words.
    pub label: String,
    /// What it gains.
    pub benefit: String,
    /// What it costs to do.
    pub cost: String,
    /// What it costs to back out of later.
    pub undo_cost: String,
}
/// One original dump or answer.
#[derive(Clone)]
pub struct Original {
    /// The original's number in the conversation, from 1.
    pub id: usize,
    /// The question it answered, if it was an answer.
    pub in_reply_to: Option<String>,
    /// The words exactly as typed or dictated.
    pub text: String,
}
impl Original {
    /// How the evidence names this original. Never changes its text.
    pub fn label(&self) -> String {
        format!(
            "Tinkery original {}{}",
            self.id,
            self.in_reply_to
                .as_ref()
                .map_or(String::new(), |id| format!(" / reply to {id}"))
        )
    }
}
/// Proof of a saved goal: read back, with its view running or a saved snapshot shown instead.
pub struct Receipt {
    /// The session folder the goal was saved in.
    pub session: PathBuf,
    /// The goal as read back from Seed Me, which matched what was confirmed.
    pub goal: String,
    /// The Seed Me skill that saved it, for the continuation prompt.
    pub skill: PathBuf,
    /// Where to see the ledger: the live viewer's address, or the saved view when the viewer
    /// couldn't start.
    pub ledger: String,
    /// Why the live viewer isn't running, when it isn't. The goal is saved either way.
    pub warning: Option<String>,
    viewer: Option<Child>,
}
impl Receipt {
    #[cfg(test)]
    pub(crate) fn test(session: PathBuf) -> Self {
        Self {
            session,
            goal: "TEST confirmed goal".into(),
            skill: "/TEST/seed-me/SKILL.md".into(),
            ledger: "TEST saved view".into(),
            warning: None,
            viewer: None,
        }
    }
    /// The prompt to continue this session; see [`super::continuation_prompt`].
    pub fn continuation_prompt(&self) -> String {
        super::continuation_prompt(&self.skill, &self.session)
    }
    /// Seed Me's saved view of the ledger, which outlives the viewer.
    pub fn saved_view(&self) -> PathBuf {
        self.session.join("ledger-view.html")
    }
    /// The receipt printed after the terminal is restored.
    pub fn text(&self) -> String {
        format!(
            "Saved.\n{}\nNext: shape it into a seed with Seed Me.\nSession: {}\nLedger: {}{}",
            self.goal,
            self.session.display(),
            self.ledger,
            self.warning
                .as_ref()
                .map_or(String::new(), |w| format!("\n{w}"))
        )
    }
    /// Stop the viewer this receipt started, if it still runs. The session stays active.
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
/// Run one helper command with a 10-second limit and capped output; stdout on success.
pub(super) fn output(script: &Path, args: &[&std::ffi::OsStr]) -> Result<String, String> {
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
    /// Save through `skill` in the explicit root or Tinkery's checked HOME default.
    /// Resolves the destination once; no helper or file access occurs until [`Config::confirm`].
    pub fn new(skill: PathBuf, root: Option<PathBuf>) -> Self {
        Self {
            skill,
            root: super::resolve_root(root),
            session: Arc::new(Mutex::new(None)),
        }
    }
    /// The session a failed save left behind, for the operator to inspect.
    pub fn recovery_path(&self) -> Option<PathBuf> {
        self.session.lock().unwrap().clone()
    }
    /// The same settings with no session yet, for a new dump.
    pub fn fresh(&self) -> Self {
        Self {
            skill: self.skill.clone(),
            root: self.root.clone(),
            session: Arc::new(Mutex::new(None)),
        }
    }
    /// Save `a` as the origin of an active human Seed Me session through the official helper,
    /// then read it back. Promises `Ok` only when the read-back shows this exact goal, settled
    /// by the user, with every original intact, no seed written yet, and a current view.
    /// Refuses to replace another goal or touch a session that isn't active and human. Blocks
    /// on helper calls; run it off the paint path. Writes that already happened stay.
    pub fn confirm(&self, a: Affirmed) -> Result<Receipt, String> {
        let root = self.root.as_ref().map_err(Clone::clone)?;
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
            let created = call(&[
                std::ffi::OsStr::new("init"),
                std::ffi::OsStr::new("--root"),
                root.as_os_str(),
            ])?;
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
            let evidence = a.originals.iter().map(|s| json!({"checked": s.label(),"at":stamp.trim(),"observed":s.text})).chain(std::iter::once(json!({"checked":"Tinkery answer/question context; questions remain provisional, not settled Seed Me decisions","at":stamp.trim(),"observed":a.context}))).collect::<Vec<_>>();
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
        for original in &a.originals {
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
            goal: a.goal,
            skill,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_root_survives_fresh_dumps_and_refuses_before_helper_access() {
        let config = Config {
            skill: "/TEST/missing-seed-me/SKILL.md".into(),
            root: crate::seed_me::default_root(None),
            session: Arc::new(Mutex::new(None)),
        };
        for config in [config.clone(), config.fresh()] {
            let error = config
                .confirm(Affirmed {
                    goal: "TEST goal".into(),
                    outcome: "TEST outcome".into(),
                    options: vec![],
                    originals: vec![],
                    context: "TEST context".into(),
                    source: "TEST".into(),
                })
                .err()
                .expect("An unavailable destination cannot save");
            assert!(error.contains("--seed-session-root PATH"), "{error}");
            assert!(config.recovery_path().is_none());
        }
    }

    #[test]
    fn fresh_dump_keeps_the_selected_root() {
        let root = PathBuf::from("/TEST/captured-sessions");
        let config = Config::new("/TEST/SKILL.md".into(), Some(root.clone()));
        assert_eq!(config.fresh().root, Ok(root));
    }
}
