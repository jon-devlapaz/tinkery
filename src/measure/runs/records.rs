//! Closed-run postmortem records, read from tink-sdlc status, the run's own files and Git.
//! Nothing is written, inferred or estimated: a record that is missing or doesn't agree with its
//! source reads as unknown, never as zero.

use crate::{process, sdlc};
use serde_json::Value;
use std::{
    path::Path,
    process::Command,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// At most this many recently closed runs are read in one pass.
pub(super) const RUNS: usize = 20;
/// A merge younger than this many days is too recent to call held.
pub(super) const HOLD_DAYS: u64 = 14;

/// One read: recently closed runs, newest first, and what the checkout couldn't report.
pub(super) struct Postmortem {
    /// Closed runs, newest closure first.
    pub(super) runs: Vec<Closed>,
    /// Closed runs beyond [`RUNS`] that this pass left unread.
    pub(super) more: usize,
    /// Runs whose status tink-sdlc reported as unreadable.
    pub(super) unreadable: usize,
}

/// One closed run as its records state it.
pub(super) struct Closed {
    /// The run's folder name.
    pub(super) slug: String,
    /// The merge commit, or `None` when the run was abandoned.
    pub(super) merge: Option<String>,
    /// Closure time in nanoseconds since the Unix epoch.
    pub(super) closed_ns: u128,
    /// The recorded PR URL, if any.
    pub(super) pr: Option<String>,
    /// Every decision receipt, oldest first.
    pub(super) decisions: Vec<Decision>,
    /// Operator labels, oldest first, or why their words can't be shown.
    pub(super) labels: Result<Vec<Label>, String>,
    /// Pi responses per step, in step-name order. `None` per step is unknown.
    pub(super) steps: Vec<(String, Option<usize>)>,
    /// Whether the merge held on this checkout's `HEAD`.
    pub(super) held: Hold,
}

/// One `decide` receipt. Only the operator's count as gates.
pub(super) struct Decision {
    pub(super) stage: u64,
    /// `approved` or `changes-requested`, verbatim.
    pub(super) decision: String,
    /// The recorded reviewer, verbatim.
    pub(super) reviewer: String,
    /// Who recorded it.
    pub(super) by: sdlc::Attribution,
}

/// One operator label, with its words exactly as written.
pub(super) struct Label {
    /// `correction`, `none`, `accepted` or `rejected`.
    pub(super) kind: String,
    /// The operator's words.
    pub(super) words: String,
}

/// Whether a merged result held. Unknown keeps its reason.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Hold {
    /// Abandoned runs have no merge to hold.
    Abandoned,
    /// A commit on `HEAD` reverts the merge.
    Reverted(String),
    /// Not reverted, and closed at least [`HOLD_DAYS`] ago.
    Held,
    /// Not reverted yet, but closed under [`HOLD_DAYS`] ago.
    TooRecent,
    /// Git couldn't say.
    Unknown(String),
}

impl Closed {
    /// Operator gate decisions. Agent and automatic receipts are not the operator's.
    pub(super) fn gates(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.by == sdlc::Attribution::Operator)
            .count()
    }

    /// Correction labels, or `None` when the labels are unreadable.
    pub(super) fn corrections(&self) -> Option<usize> {
        self.labels
            .as_ref()
            .ok()
            .map(|labels| labels.iter().filter(|l| l.kind == "correction").count())
    }

    /// All counted Pi responses, or `None` if any step is unknown or no step was recorded.
    pub(super) fn pi(&self) -> Option<usize> {
        if self.steps.is_empty() {
            return None;
        }
        self.steps.iter().map(|(_, n)| *n).sum()
    }
}

/// Read up to [`RUNS`] closed runs in `root`, all within `deadline`.
pub(super) fn read(
    root: &Path,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<Postmortem, String> {
    let until = Instant::now() + deadline;
    let left = || until.saturating_duration_since(Instant::now());
    let listed = sdlc::list(root, cancel, left())?
        .ok_or("The run list is unreadable. Results unknown; press p to retry.")?;
    let unreadable = listed
        .iter()
        .filter(|r| r.data.get("error").is_some())
        .count();
    let mut closed: Vec<(u128, &str)> = listed
        .iter()
        .filter_map(|run| Some((closure_ns(run)?, run.data["slug"].as_str()?)))
        .collect();
    closed.sort_unstable_by(|a, b| b.cmp(a));
    let more = closed.len().saturating_sub(RUNS);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut runs = Vec::new();
    for (closed_ns, slug) in closed.into_iter().take(RUNS) {
        let parsed = sdlc::status(root, slug, cancel, left())?;
        let run = &parsed.data;
        if run["slug"].as_str() != Some(slug) || closure_ns(&parsed) != Some(closed_ns) {
            return Err(format!(
                "Run {slug} changed while it was read. Press p to read again."
            ));
        }
        runs.push(closed_run(root, slug, &parsed, now, cancel, left())?);
    }
    Ok(Postmortem {
        runs,
        more,
        unreadable,
    })
}

fn closure_ns(run: &sdlc::Run) -> Option<u128> {
    run.closure.as_ref()?.time_ns.map(u128::from)
}

fn closed_run(
    root: &Path,
    slug: &str,
    parsed: &sdlc::Run,
    now: u128,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<Closed, String> {
    let run = &parsed.data;
    let merge = match parsed.closure.as_ref().map(|closure| &closure.outcome) {
        Some(sdlc::Outcome::Merged(commit)) => Some(
            commit
                .as_ref()
                .ok_or(format!(
                    "Run {slug} is merged without a readable merge commit."
                ))?
                .as_str()
                .to_owned(),
        ),
        _ => None,
    };
    // No folder holds a NUL name; a relative one would depend on Tinkery's own folder.
    let files = match &parsed.checkout {
        Ok(path) => sdlc::RunFiles::open(path.as_deref().unwrap_or(root), slug),
        Err(sdlc::CheckoutGap::NotText) => return Err("A build checkout is unreadable.".into()),
        Err(sdlc::CheckoutGap::Nul | sdlc::CheckoutGap::Relative) => None,
    };
    let pr = run["pr"]["url"].as_str().map(str::to_owned);
    let closed_ns = closure_ns(parsed).unwrap_or_default();
    let held = match &merge {
        None => Hold::Abandoned,
        Some(commit) => held(
            root,
            commit,
            pr.as_deref(),
            now.saturating_sub(closed_ns),
            cancel,
            deadline,
        ),
    };
    Ok(Closed {
        slug: slug.into(),
        merge,
        closed_ns,
        pr,
        decisions: parsed
            .decisions
            .iter()
            .flatten()
            .map(|d| Decision {
                stage: d.stage.unwrap_or_default(),
                decision: d.decision.as_deref().unwrap_or("unknown").into(),
                reviewer: d.reviewer.as_deref().unwrap_or("unknown").into(),
                by: d.by,
            })
            .collect(),
        labels: labels(files.as_ref(), parsed.labels.count),
        steps: steps(files.as_ref(), parsed),
        held,
    })
}

// tink-sdlc has validated the chain; this reads only kind and words, and refuses a count mismatch.
fn labels(files: Option<&sdlc::RunFiles>, count: Option<u64>) -> Result<Vec<Label>, String> {
    let records = match files.map(sdlc::RunFiles::labels) {
        Some(Ok(records)) => records,
        Some(Err(sdlc::FileGap::Incomplete)) => return Err("A label line is unreadable.".into()),
        None | Some(Err(_)) => return Err("Labels unreadable in this checkout.".into()),
    };
    let mut labels = Vec::new();
    for record in records {
        let record = record.ok_or("A label line is unreadable.")?;
        match (record["kind"].as_str(), record["words"].as_str()) {
            (Some(kind), Some(words)) => labels.push(Label {
                kind: kind.into(),
                words: words.into(),
            }),
            _ => return Err("A label has no kind or words.".into()),
        }
    }
    if Some(labels.len() as u64) != count {
        return Err("Labels don't match the run's recorded count.".into());
    }
    Ok(labels)
}

fn steps(files: Option<&sdlc::RunFiles>, run: &sdlc::Run) -> Vec<(String, Option<usize>)> {
    let logs = files.and_then(|files| files.logs(true, false));
    let mut steps: Vec<_> = run
        .steps
        .iter()
        .flatten()
        .filter_map(|s| s.data["name"].as_str())
        .map(|name| {
            let responses = files.zip(logs.as_ref()).and_then(|(files, logs)| {
                responses(files, logs.iter().filter(|log| log.step == name), name)
            });
            (name.to_owned(), responses)
        })
        .collect();
    steps.sort();
    steps
}

/// Assistant answers across a step's launch and resume logs, bound to its contract's session.
/// No log, a foreign session or an unreadable event makes the step unknown.
fn responses<'a>(
    files: &sdlc::RunFiles,
    logs: impl Iterator<Item = &'a sdlc::Log>,
    step: &str,
) -> Option<usize> {
    let contract: Value =
        serde_json::from_slice(&files.read(&format!("steps/{step}.json")).ok()?).ok()?;
    let session = contract["session"].as_str().filter(|s| !s.is_empty())?;
    let mut total = None;
    for log in logs {
        let mut lines = files.lines(&format!("steps/{}", log.name))?.into_iter();
        let header: Value = serde_json::from_str(&lines.next()??).ok()?;
        if header["type"] != "session" || header["id"] != session {
            return None;
        }
        let mut count = 0;
        for line in lines {
            let line = line?;
            // The launcher's own notes share the log; only JSON events are Pi's.
            if !line.starts_with('{') {
                continue;
            }
            let event: Value = serde_json::from_str(&line).ok()?;
            if event["type"] == "message_end" && event["message"]["role"] == "assistant" {
                count += 1;
            }
        }
        total = Some(total.unwrap_or(0) + count);
    }
    total
}

fn held(
    root: &Path,
    merge: &str,
    pr: Option<&str>,
    age_ns: u128,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Hold {
    let git = |args: &[&str]| {
        let mut command = Command::new("git");
        command.arg("--no-optional-locks").args(args);
        process::text(root, command, "git", cancel, deadline)
    };
    if git(&["merge-base", "--is-ancestor", merge, "HEAD"]).is_err() {
        return Hold::Unknown("the merge commit isn't in this checkout's HEAD history".into());
    }
    let mut args = vec![
        "log".to_owned(),
        "--format=%H".into(),
        "--extended-regexp".into(),
        format!("--grep=This reverts commit {merge}"),
    ];
    // GitHub's revert button writes "Reverts OWNER/REPO#N"; #1 must not match #12.
    if let Some(url) = pr.and_then(crate::github::PrUrl::parse) {
        let repo = url.repo().replace('.', "[.]");
        args.push(format!("--grep=Reverts {repo}#{}([^0-9]|$)", url.number()));
    }
    args.push(format!("{merge}..HEAD"));
    match git(&args.iter().map(String::as_str).collect::<Vec<_>>()) {
        Err(error) => Hold::Unknown(error),
        Ok(out) => match out.lines().last() {
            Some(commit) => Hold::Reverted(commit.to_owned()),
            None if age_ns >= u128::from(HOLD_DAYS) * 86_400 * 1_000_000_000 => Hold::Held,
            None => Hold::TooRecent,
        },
    }
}
