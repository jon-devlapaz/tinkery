//! A bounded, cancellable GitHub CLI read. No shell, provider, writes or implicit retries.

use std::{path::Path, process::Command, sync::atomic::AtomicBool, time::Duration};

use super::Report;
use crate::process::{self, Failure, Limits, Task};

/// Start the one read. Only the explicitly labelled read action may call this; dropping the
/// task stops and reaps its process group.
pub(super) fn start(root: &Path, since: String) -> Task<Report> {
    let root = root.to_path_buf();
    Task::start(move |stop| {
        run(&root, &since, stop, Duration::from_secs(30)).and_then(|text| super::parse(&text))
    })
}

fn run(
    root: &Path,
    since: &str,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<String, String> {
    let mut command = Command::new("gh");
    command
        .args(["pr", "list", "--state", "all", "--search"])
        .arg(format!("updated:>={since} sort:updated-desc"))
        .args(["--limit", "101", "--json",
            "number,title,state,url,author,createdAt,updatedAt,closedAt,mergedAt,headRefOid,statusCheckRollup"])
        .current_dir(root)
        .env_remove("GH_REPO")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat");
    let output = process::run(command, Limits::text(deadline), cancel).map_err(|failure| {
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
