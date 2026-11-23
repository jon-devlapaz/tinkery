//! Tinkery's only GitHub reads, all through the `gh` CLI and the operator's login: one PR, one
//! PR list, one file at a commit, and a PR sent to the browser. Every read is bounded at 30
//! seconds and cancellable. It accepts only canonical PR URLs, reads each reported check one
//! way, never shows `gh` stderr, never writes to GitHub and never retries on its own.

use crate::process::{self, Failure, Limits};
use serde_json::Value;
use std::{
    fmt,
    ops::Deref,
    path::Path,
    process::Command,
    sync::{LazyLock, atomic::AtomicBool},
    time::Duration,
};

const DEADLINE: Duration = Duration::from_secs(30);

/// A checked `https://github.com/OWNER/REPO/pull/N`; holding one means the URL is canonical.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrUrl(String);

impl PrUrl {
    /// Only an exact `https://github.com/OWNER/REPO/pull/N` with N of 1 or more. No suffix,
    /// query, fragment, other host or `.`/`..` repository is accepted.
    pub(crate) fn parse(url: &str) -> Option<Self> {
        static PR: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(
                r"^https://github\.com/[A-Za-z0-9-]+/([A-Za-z0-9_.-]+)/pull/[1-9][0-9]*$",
            )
            .unwrap()
        });
        let repo = PR.captures(url)?.get(1)?.as_str();
        (!matches!(repo, "." | "..")).then(|| Self(url.into()))
    }

    /// `OWNER/REPO`.
    pub(crate) fn repo(&self) -> &str {
        self.parts().0
    }

    /// The PR number, as digits.
    pub(crate) fn number(&self) -> &str {
        self.parts().1
    }

    /// Whether a URL GitHub reported names this PR. GitHub owner and repo names are
    /// case-insensitive, so ASCII case is ignored everywhere.
    pub(crate) fn names(&self, reported: &str) -> bool {
        self.0.eq_ignore_ascii_case(reported)
    }

    fn parts(&self) -> (&str, &str) {
        self.0["https://github.com/".len()..]
            .split_once("/pull/")
            .unwrap()
    }
}

impl Deref for PrUrl {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PrUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Synthetic rows name their PR as a literal; a non-canonical literal fails the test.
#[cfg(test)]
impl From<&str> for PrUrl {
    fn from(url: &str) -> Self {
        Self::parse(url).expect("canonical TEST PR URL")
    }
}

/// What one reported PR-head check establishes. It is never merge-commit evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Pass,
    Fail,
    /// Finished without success or failure, such as neutral or skipped.
    NotSuccessful,
    Pending,
}

/// Why one reported check cannot give a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unread {
    /// GitHub omitted the field this kind of check needs.
    Missing(&'static str),
    /// Neither a check run nor a commit status.
    Kind,
}

/// One entry of GitHub's `statusCheckRollup`, read once for every screen.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(from = "Value")]
pub(crate) struct Check {
    /// The check run's name or the status's context, when GitHub reported it.
    pub(crate) name: Option<String>,
    /// GitHub's own word: a nonempty conclusion, else its status, else its state.
    pub(crate) reported: Option<String>,
    /// What the check establishes; any missing field or unknown kind refuses a verdict.
    pub(crate) verdict: Result<Verdict, Unread>,
}

impl From<Value> for Check {
    fn from(check: Value) -> Self {
        let field = |key: &'static str| check[key].as_str().ok_or(Unread::Missing(key));
        let kind = field("__typename");
        let name = match kind {
            Ok("CheckRun") => check["name"].as_str(),
            Ok("StatusContext") => check["context"].as_str(),
            _ => check["name"].as_str().or(check["context"].as_str()),
        };
        let verdict = kind.and_then(|kind| match kind {
            "CheckRun" => {
                field("name")?;
                if check["status"] != "COMPLETED" {
                    return Ok(Verdict::Pending);
                }
                Ok(match field("conclusion")? {
                    "SUCCESS" => Verdict::Pass,
                    "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
                    | "STARTUP_FAILURE" => Verdict::Fail,
                    _ => Verdict::NotSuccessful,
                })
            }
            "StatusContext" => {
                field("context")?;
                Ok(match field("state")? {
                    "SUCCESS" => Verdict::Pass,
                    "FAILURE" | "ERROR" => Verdict::Fail,
                    _ => Verdict::Pending,
                })
            }
            _ => Err(Unread::Kind),
        });
        let reported = check["conclusion"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or(check["status"].as_str())
            .or(check["state"].as_str());
        Self {
            name: name.map(Into::into),
            reported: reported.map(Into::into),
            verdict,
        }
    }
}

/// The reported checks, or None when the rollup is not a list.
pub(crate) fn checks(rollup: &Value) -> Option<Vec<Check>> {
    let checks = rollup.as_array()?;
    Some(checks.iter().cloned().map(Check::from).collect())
}

fn gh(root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("gh");
    command
        .args(args)
        .current_dir(root)
        .env_remove("GH_REPO")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat");
    command
}

/// The PR's url, title, head, state and checks as GitHub reports them. None when `gh` fails
/// or prints no JSON; the caller checks every field it uses.
pub(crate) fn view(root: &Path, url: &PrUrl, stop: &AtomicBool) -> Option<Value> {
    let command = gh(
        root,
        &[
            "pr",
            "view",
            url,
            "--json",
            "url,title,headRefOid,state,statusCheckRollup",
        ],
    );
    let out = process::text(root, command, "gh", stop, DEADLINE).ok()?;
    serde_json::from_str(&out).ok()
}

/// Ask `gh` to open the PR in the browser. It reads nothing back.
pub(crate) fn open(root: &Path, url: &PrUrl, stop: &AtomicBool) -> Result<(), String> {
    let command = gh(root, &["pr", "view", url, "--web"]);
    process::text(root, command, "gh", stop, DEADLINE).map(|_| ())
}

/// The file at `path` in the PR's repository at commit `head`. Ok(None) only when GitHub
/// answers 404; any other failure is Err and establishes nothing.
pub(crate) fn file(
    root: &Path,
    url: &PrUrl,
    path: &str,
    head: &str,
    stop: &AtomicBool,
) -> Result<Option<String>, ()> {
    let address = format!("repos/{}/contents/{path}?ref={head}", url.repo());
    let command = gh(
        root,
        &[
            "api",
            "-H",
            "Accept: application/vnd.github.raw+json",
            &address,
        ],
    );
    let output = process::run(command, Limits::text(DEADLINE), stop).map_err(|_| ())?;
    if output.status.success() {
        return String::from_utf8(output.stdout).map(Some).map_err(|_| ());
    }
    if output.said().contains("HTTP 404") {
        return Ok(None);
    }
    Err(())
}

/// This checkout's PRs of every state updated on or after `since`, newest first, at most 101
/// so the caller can tell a partial read. Err is one line for the operator.
pub(crate) fn list(root: &Path, since: &str, stop: &AtomicBool) -> Result<String, String> {
    let search = format!("updated:>={since} sort:updated-desc");
    let command = gh(
        root,
        &[
            "pr",
            "list",
            "--state",
            "all",
            "--search",
            &search,
            "--limit",
            "101",
            "--json",
            "number,title,state,url,author,createdAt,updatedAt,closedAt,mergedAt,headRefOid,statusCheckRollup",
        ],
    );
    let output = process::run(command, Limits::text(DEADLINE), stop).map_err(|failure| {
        match failure {
            Failure::Start(_) => {
                "Couldn't start gh. Install GitHub CLI and check repo access, then press r."
            }
            Failure::Cancelled => "Stopped.",
            Failure::Late(_) => "GitHub read exceeded 30 seconds. Press r to retry.",
            Failure::Wait(_) | Failure::Halted(_) => {
                "Couldn't finish reading GitHub. Press r to retry."
            }
            Failure::TooMuch(_) | Failure::Read(_) => {
                "GitHub output was incomplete or exceeded 4 MiB. Results unknown; press r to retry."
            }
        }
        .to_owned()
    })?;
    if !output.status.success() {
        // Never display CLI stderr: it can contain credentials or terminal escape sequences.
        return Err(
            "GitHub read failed. Check gh auth status and this repo's remote/access, then press r."
                .into(),
        );
    }
    String::from_utf8(output.stdout)
        .map_err(|_| "GitHub returned invalid UTF-8. Results unknown; press r to retry.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_canonical_pr_urls_parse_and_name_their_repo_and_number() {
        let url = PrUrl::parse("https://github.com/TEST/repo.x/pull/51").unwrap();
        assert_eq!((url.repo(), url.number()), ("TEST/repo.x", "51"));
        assert!(url.names("https://github.com/test/REPO.x/pull/51"));
        assert!(!url.names("https://github.com/TEST/repo.x/pull/5"));
        for bad in [
            "https://github.com/TEST/repo/pull/1/files",
            "https://github.com/TEST/../pull/1",
            "https://github.com/TEST/./pull/1",
            "https://github.com/TEST/repo/pull/01",
            "https://github.com/TEST/repo/pull/1 ",
        ] {
            assert!(PrUrl::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn each_reported_check_gets_one_reading() {
        let read = |v: Value| Check::from(v);
        let neutral = read(
            json!({"__typename":"CheckRun","name":"TEST CI","status":"COMPLETED","conclusion":"NEUTRAL"}),
        );
        assert_eq!(neutral.verdict, Ok(Verdict::NotSuccessful));
        assert_eq!(neutral.reported.as_deref(), Some("NEUTRAL"));
        let running = read(
            json!({"__typename":"CheckRun","name":"TEST CI","status":"IN_PROGRESS","conclusion":""}),
        );
        assert_eq!(
            (running.verdict, running.reported.as_deref()),
            (Ok(Verdict::Pending), Some("IN_PROGRESS"))
        );
        let status =
            read(json!({"__typename":"StatusContext","context":"TEST ctx","state":"ERROR"}));
        assert_eq!(
            (status.name.as_deref(), status.verdict),
            (Some("TEST ctx"), Ok(Verdict::Fail))
        );
        let untyped = read(json!({"name":"TEST","state":"PENDING"}));
        assert_eq!(untyped.verdict, Err(Unread::Missing("__typename")));
        assert_eq!(untyped.reported.as_deref(), Some("PENDING"));
        assert_eq!(
            read(json!({"__typename":"Other","name":"TEST"})).verdict,
            Err(Unread::Kind)
        );
        assert_eq!(
            read(json!({"__typename":"CheckRun","name":"TEST","status":"COMPLETED"})).verdict,
            Err(Unread::Missing("conclusion"))
        );
        assert!(checks(&Value::Null).is_none());
    }
}
