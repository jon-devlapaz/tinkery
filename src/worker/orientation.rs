//! Estimate room for a first edit and a first commit from recent, run-bound Pi launches, read
//! from each step's session file. A missing file, a resume, a different session or a failed tool
//! call establishes neither.

use super::{Run, identity, read_status};
use crate::{process, sdlc};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
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

#[derive(Clone, Copy)]
enum Observation {
    /// The first successful edit/write, or commit, came with this response.
    At(usize),
    /// The launch reached its cap at this response without one.
    NoneThrough(usize),
}

impl Observation {
    fn minimum(self) -> usize {
        match self {
            Self::At(n) => n + 1,
            Self::NoneThrough(n) => n + 2,
        }
    }
}

/// What one launch log shows; at least one of the two is known.
struct Seen {
    edit: Option<Observation>,
    commit: Option<Observation>,
}

/// Advisory only. Sample at most five recent Pi launches in the named repository and warn
/// when `cap` is below what they needed. Reads only the session file each step names; never
/// sends its contents or alters the allowance.
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
            && let Some(seen) = observe(launch, run, value, cancel)
        {
            samples.push((launch, seen));
        }
    }
    if samples.is_empty() {
        return NO_SAMPLE.into();
    }
    let mut minimum = 0;
    let mut lines = Vec::new();
    let edits = samples.iter().filter_map(|(l, seen)| Some((l, seen.edit?)));
    let commits = samples
        .iter()
        .filter_map(|(l, seen)| Some((l, seen.commit?)));
    for (what, verb, observed) in [
        ("edit/write", "edit", edits.collect::<Vec<_>>()),
        ("commit", "commit", commits.collect()),
    ] {
        let Some((source, observed)) = observed.into_iter().max_by_key(|(_, o)| o.minimum()) else {
            lines.push(format!("No {what} in these logs."));
            continue;
        };
        minimum = minimum.max(observed.minimum());
        let reason = match observed {
            Observation::At(n) => format!("first {what} at response {n}. Allow one follow-up"),
            Observation::NoneThrough(n) => {
                format!("no {what} through response {n}. Allow one {verb} and one follow-up")
            }
        };
        lines.push(format!("{}/{}: {reason}.", source.slug, source.log.step));
    }
    let count = samples.len();
    let noun = if count == 1 { "log" } else { "logs" };
    let mut text = format!(
        "Suggested minimum: {minimum} responses. Based on {count} recent Pi launch {noun}.\n{}",
        lines.join("\n")
    );
    if cap < minimum {
        text.push_str(&format!(
            "\n! Your cap of {cap} is below this minimum. Starting keeps your cap."
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
#[serde(tag = "role")]
enum Message {
    #[serde(rename = "assistant")]
    Assistant {
        #[serde(default)]
        content: Vec<Content>,
    },
    #[serde(rename = "toolResult", rename_all = "camelCase")]
    Tool {
        tool_name: String,
        is_error: bool,
        #[serde(default)]
        tool_call_id: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Content {
    #[serde(rename = "toolCall")]
    Call {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        arguments: Value,
    },
    #[serde(other)]
    Other,
}

/// Whether a launch log holds the launcher's own note that it stopped the worker at `cap`.
fn cap_marked(files: &sdlc::RunFiles, log: &sdlc::Log, cap: Option<u64>) -> bool {
    let marker = cap.map(|n| format!("pi-launcher: response cap {n} reached;"));
    marker.is_some_and(|marker| {
        files
            .lines(&format!("steps/{}", log.name))
            .is_some_and(|lines| lines.iter().flatten().any(|line| line.starts_with(&marker)))
    })
}

// The one place a step transcript is checked: contract identity and session header.
fn open_transcript(
    files: &sdlc::RunFiles,
    step: &str,
    run: &Run,
) -> Option<(Option<u64>, sdlc::Transcript)> {
    let name = format!("steps/{step}.json");
    let contract = sdlc::StepContract::read(
        serde_json::from_slice(&files.read(&name).ok()?).ok()?,
        &run.id,
        &run.root,
    )?;
    let session = contract.session.as_deref().filter(|s| !s.is_empty())?;
    let transcript = sdlc::Transcript::read(&files.folder().join(name), session)?;
    let header = &transcript.header;
    if header["type"] != "session"
        || header["version"] != 3
        || header["id"] != session
        || header["cwd"] != contract.record["checkout"]
    {
        return None;
    }
    let cap = contract.max_responses.filter(|n| *n > 0);
    Some((cap, transcript))
}

/// True only if the step's session file passes the same checks as cap advice, holds at least
/// the contract's cap of answers, and its newest launch or resume log ends with the launcher's
/// cap marker. Any doubt gives false. Never writes.
pub(crate) fn stopped_at_cap(run: &Run, step: &str) -> bool {
    let Some(files) = sdlc::RunFiles::open(&run.root, &run.slug) else {
        return false;
    };
    let mut logs = files.logs(true, true).unwrap_or_default();
    logs.retain(|log| log.step == step);
    let Some(newest) = logs
        .iter()
        .max_by(|a, b| (a.time, &a.name).cmp(&(b.time, &b.name)))
    else {
        return false;
    };
    let Some((cap, transcript)) = open_transcript(&files, step, run) else {
        return false;
    };
    // The launcher's cap is cumulative per session, so answers count across launch and resume.
    transcript
        .responses()
        .is_some_and(|n| cap.is_some_and(|c| n as u64 >= c))
        && cap_marked(&files, newest, cap)
}

fn observe(launch: &Launch, run: &Run, value: &sdlc::Run, cancel: &AtomicBool) -> Option<Seen> {
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
    let (cap, transcript) = open_transcript(&launch.files, &launch.log.step, run)?;
    // This launch's messages: from its start to the step's next resume, if any.
    let next = launch
        .files
        .logs(true, true)?
        .into_iter()
        .filter(|log| log.step == launch.log.step && log.time > launch.log.time)
        .map(|log| log.time)
        .min();
    let start = i64::try_from(launch.log.time).ok()?;
    let end = next.and_then(|t| i64::try_from(t).ok());
    // A shell `git commit` that starts a command and whose tool result succeeded is the first
    // commit. Global options such as `-C dir` or `-c user.name=x` may come before `commit`.
    // Unclear cases (`sudo git commit`, a message naming --dry-run) count as no commit, which
    // only leaves the suggestion to the other evidence.
    let commit_command = regex::Regex::new(
        r"(?:^|[;&|(\n])\s*(?:\S*/)?git(?:\s+-[Cc]\s+\S+|\s+--\S+)*\s+commit(?:\s|$)([^;&|\n]*)",
    )
    .expect("commit pattern");
    let dry_run = regex::Regex::new(r"(?:^|\s)--dry-run(?:\s|$)").expect("dry-run pattern");
    let mut commits = HashSet::new();
    let (mut edit, mut commit) = (None, None);
    let mut responses = 0;
    for said in &transcript.said {
        if cancel.load(Ordering::SeqCst) {
            return None;
        }
        // An unreadable or undated line ends the reading; what came before it still counts.
        let Some(at) = said.as_ref().and_then(|said| said.at) else {
            break;
        };
        if at < start {
            continue;
        }
        if end.is_some_and(|end| at >= end) {
            break;
        }
        let Some(said) = said else { break };
        let Ok(message) = Message::deserialize(&said.message) else {
            break;
        };
        match message {
            Message::Assistant { content } => {
                responses += 1;
                for part in content {
                    if let Content::Call {
                        id,
                        name,
                        arguments,
                    } = part
                        && name == "bash"
                        && !id.is_empty()
                        && arguments["command"].as_str().is_some_and(|c| {
                            commit_command
                                .captures_iter(c)
                                .any(|m| !dry_run.is_match(&m[1]))
                        })
                    {
                        commits.insert(id);
                    }
                }
            }
            Message::Tool {
                tool_name,
                is_error: false,
                tool_call_id,
            } if responses > 0 => {
                if edit.is_none() && matches!(tool_name.as_str(), "edit" | "write") {
                    edit = Some(Observation::At(responses));
                }
                if commit.is_none() && commits.contains(&tool_call_id) {
                    commit = Some(Observation::At(responses));
                }
            }
            _ => {}
        }
        if edit.is_some() && commit.is_some() {
            break;
        }
    }
    if stopped_at_cap
        && (edit.is_none() || commit.is_none())
        && cap_marked(&launch.files, &launch.log, cap)
    {
        if !cap.is_some_and(|n| responses as u64 >= n) {
            return None;
        }
        let through = Some(Observation::NoneThrough(responses));
        return Some(Seen {
            edit: edit.or(through),
            commit: commit.or(through),
        });
    }
    (edit.is_some() || commit.is_some()).then_some(Seen { edit, commit })
}
