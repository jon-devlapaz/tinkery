//! Estimate room for a first edit from recent, run-bound Pi launch logs. A missing log,
//! a resume, a different session or a failed tool call cannot establish a first edit.

use super::{Run, identity, read_status};
use crate::{process, sdlc};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const NO_SAMPLE: &str = "Suggested minimum unknown: no usable recent Pi launch evidence.";
struct Launch {
    files: sdlc::RunFiles,
    log: sdlc::Log,
    slug: String,
}

enum Observation {
    FirstEdit(usize),
    NoEditThrough(usize),
}

impl Observation {
    fn minimum(&self) -> usize {
        match self {
            Self::FirstEdit(n) => n + 1,
            Self::NoEditThrough(n) => n + 2,
        }
    }
}

/// Advisory only. Sample at most five recent Pi launches in the named repository.
/// Never inspect Pi's session store, send log contents or alter the allowance.
pub(crate) fn cap_advice(root: &Path, cap: usize, cancel: &AtomicBool) -> String {
    let folders = entries(&root.join("runs")).unwrap_or_default();
    if folders.is_empty() {
        return NO_SAMPLE.into();
    }
    let mut logs = Vec::new();
    let worktrees = build_worktrees(root, cancel);
    for entry in folders {
        if cancel.load(Ordering::SeqCst) {
            return NO_SAMPLE.into();
        }
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let Ok(slug) = entry.file_name().into_string() else {
            continue;
        };
        collect_logs(root, &slug, &mut logs);
        if let Some(build) = worktrees.get(&slug)
            && build != root
        {
            collect_logs(build, &slug, &mut logs);
        }
    }
    let path = |launch: &Launch| launch.files.folder().join("steps").join(&launch.log.name);
    logs.sort_unstable_by(|a, b| {
        b.log
            .time
            .cmp(&a.log.time)
            .then_with(|| path(a).cmp(&path(b)))
    });
    let mut readings = HashMap::new();
    let mut samples = Vec::new();
    for launch in logs.iter().take(5) {
        if cancel.load(Ordering::SeqCst) {
            return NO_SAMPLE.into();
        }
        let source = readings.entry(&launch.slug).or_insert_with(|| {
            let value = read_status(root, &launch.slug, cancel, Duration::from_secs(1)).ok()?;
            Some((identity(root, &launch.slug, &value).ok()?, value))
        });
        if let Some((run, value)) = source
            && let Some(observation) = observe(launch, run, value, cancel)
        {
            samples.push((launch, observation));
        }
    }
    let Some((source, observed)) = samples.iter().max_by_key(|(_, o)| o.minimum()) else {
        return NO_SAMPLE.into();
    };
    let minimum = observed.minimum();
    let reason = match observed {
        Observation::FirstEdit(n) => {
            format!("first edit/write at response {n}. Allow one follow-up")
        }
        Observation::NoEditThrough(n) => {
            format!("no edit/write through response {n}. Allow one edit and one follow-up")
        }
    };
    let count = samples.len();
    let noun = if count == 1 { "log" } else { "logs" };
    let mut text = format!(
        "Suggested minimum: {minimum} responses. Based on {count} recent Pi launch {noun}.\n{}/{}: {reason}.",
        source.slug, source.log.step
    );
    if cap < minimum {
        text.push_str(&format!(
            "\n! Your cap of {cap} is below this minimum. F2 keeps your cap."
        ));
    }
    text
}

// Refuse an oversized or linked directory instead of sampling filesystem enumeration order.
fn entries(path: &Path) -> Option<Vec<fs::DirEntry>> {
    if path.canonicalize().ok()? != path {
        return None;
    }
    let files: Vec<_> = fs::read_dir(path)
        .ok()?
        .take(sdlc::MAX_ENTRIES + 1)
        .collect::<Result<_, _>>()
        .ok()?;
    (files.len() <= sdlc::MAX_ENTRIES).then_some(files)
}

// Git locates possible step directories. Only SDLC can identify which checkout owns a run.
fn build_worktrees(root: &Path, cancel: &AtomicBool) -> HashMap<String, PathBuf> {
    let mut git = Command::new("git");
    git.args([
        "--no-optional-locks",
        "worktree",
        "list",
        "--porcelain",
        "-z",
    ]);
    let listing =
        process::text(root, git, "git", cancel, Duration::from_secs(1)).unwrap_or_default();
    listing
        .split("\0\0")
        .filter_map(|block| {
            let mut fields = block.split('\0');
            let path = fields.next()?.strip_prefix("worktree ")?;
            let branch = fields.find_map(|f| f.strip_prefix("branch refs/heads/"))?;
            Some((branch.to_owned(), PathBuf::from(path)))
        })
        .collect()
}

// Cap advice samples first launches only; a resume continues an earlier session.
fn collect_logs(root: &Path, slug: &str, logs: &mut Vec<Launch>) {
    let Some(files) = sdlc::RunFiles::open(root, slug) else {
        return;
    };
    for log in files.logs(false, true).unwrap_or_default() {
        if log.file {
            logs.push(Launch {
                files: files.clone(),
                log,
                slug: slug.into(),
            });
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Event {
    #[serde(rename = "message_end")]
    End { message: Message },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "role")]
enum Message {
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "toolResult", rename_all = "camelCase")]
    Tool { tool_name: String, is_error: bool },
    #[serde(other)]
    Other,
}

fn cap_marker(cap: Option<u64>, line: &str) -> bool {
    cap.is_some_and(|n| line.starts_with(&format!("pi-launcher: response cap {n} reached;")))
}

// The one place a Pi log is checked: contract identity, session header and size caps.
fn open_log(
    files: &sdlc::RunFiles,
    log: &sdlc::Log,
    run: &Run,
) -> Option<(Option<u64>, impl Iterator<Item = Option<String>>)> {
    let contract: Value =
        serde_json::from_slice(&files.read(&format!("steps/{}.json", log.step)).ok()?).ok()?;
    if contract["schema"] != 1
        || contract["run_id"] != run.id
        || contract["checkout"].as_str() != run.root.to_str()
    {
        return None;
    }
    let session = contract["session"].as_str().filter(|s| !s.is_empty())?;
    let mut lines = files.lines(&format!("steps/{}", log.name))?.into_iter();
    // Pi may print warnings before the header; the header is the first JSON line.
    let header: Value = lines.find_map(|line| serde_json::from_str(&line?).ok())?;
    if header["type"] != "session"
        || header["version"] != 3
        || header["id"] != session
        || header["cwd"] != contract["checkout"]
    {
        return None;
    }
    let cap = contract["max_responses"].as_u64().filter(|n| *n > 0);
    Some((cap, lines))
}

/// True only if the step's launch and resume logs all pass the same checks as cap advice and
/// the newest ends with the launcher's cap marker after at least the contract's cap of answers.
/// Any doubt gives false. Never writes.
pub(crate) fn stopped_at_cap(run: &Run, step: &str) -> bool {
    let Some(files) = sdlc::RunFiles::open(&run.root, &run.slug) else {
        return false;
    };
    let mut logs = files.logs(true, true).unwrap_or_default();
    logs.retain(|log| log.step == step);
    logs.sort_by(|a, b| (a.time, &a.name).cmp(&(b.time, &b.name)));
    // The launcher's cap is cumulative per session, so answers count across launch and resume logs.
    let mut responses = 0;
    for (n, log) in logs.iter().enumerate() {
        let Some((cap, lines)) = open_log(&files, log, run) else {
            return false;
        };
        for line in lines {
            let Some(line) = line else { return false };
            if n + 1 == logs.len()
                && cap_marker(cap, &line)
                && cap.is_some_and(|c| responses as u64 >= c)
            {
                return true;
            }
            let Ok(Event::End { message }) = serde_json::from_str(&line) else {
                continue;
            };
            if matches!(message, Message::Assistant) {
                responses += 1;
            }
        }
    }
    false
}

fn observe(
    launch: &Launch,
    run: &Run,
    value: &sdlc::Run,
    cancel: &AtomicBool,
) -> Option<Observation> {
    if sdlc::RunFiles::open(&run.root, &run.slug)?.folder() != launch.files.folder() {
        return None;
    }
    let step = value
        .steps
        .as_ref()?
        .iter()
        .find(|s| s.data["name"] == launch.log.step)?;
    if !matches!(
        step.state.known(),
        Some(sdlc::StepState::Active | sdlc::StepState::Completed | sdlc::StepState::Stopped)
    ) {
        return None;
    }
    let stopped_at_cap =
        step.state.is(sdlc::StepState::Stopped) && step.stopped_by.is(sdlc::StopReason::Cap);
    let (cap, lines) = open_log(&launch.files, &launch.log, run)?;
    let mut responses = 0;
    for line in lines {
        if cancel.load(Ordering::SeqCst) {
            return None;
        }
        let line = line?;
        if stopped_at_cap && cap_marker(cap, &line) {
            return cap
                .is_some_and(|n| responses as u64 >= n)
                .then_some(Observation::NoEditThrough(responses));
        }
        if let Event::End { message } = serde_json::from_str(&line).ok()? {
            match message {
                Message::Assistant => responses += 1,
                Message::Tool {
                    tool_name,
                    is_error: false,
                } if matches!(tool_name.as_str(), "edit" | "write") => {
                    return (responses > 0).then_some(Observation::FirstEdit(responses));
                }
                _ => {}
            }
        }
    }
    None
}
