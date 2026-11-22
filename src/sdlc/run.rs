//! tink-sdlc commands in a named checkout, run through [`crate::process`]. This layer adds only
//! API 1's refusal words; bounds, cancellation and process ownership are the process module's.

use crate::process::{self, Limits, Output, Task};
use std::{path::Path, process::Command, sync::atomic::AtomicBool, time::Duration};

/// The script every read runs, relative to the named checkout.
pub(crate) const SCRIPT: &str = "_system/scripts/sdlc.py";
/// How long one read may take before it is stopped.
pub(crate) const DEADLINE: Duration = Duration::from_secs(30);

/// Start `python3 _system/scripts/sdlc.py <args>` in `root` on its own task, stopped after
/// `deadline`, and hand its stdout to `prepare`.
pub(crate) fn read<T: Send + 'static>(
    root: &Path,
    args: Vec<String>,
    deadline: Duration,
    prepare: fn(&str) -> Result<T, String>,
) -> Task<T> {
    let root = root.to_path_buf();
    Task::start(move |stop| run(&root, &args, stop, deadline).and_then(|out| prepare(&out)))
}

/// One synchronous bounded `sdlc.py <args>` in `root`; cancellation stops its process.
pub(crate) fn run(
    root: &Path,
    args: &[String],
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<String, String> {
    let mut command = Command::new("python3");
    command.arg(SCRIPT).args(args).current_dir(root);
    let name = args.first().map_or_else(
        || "sdlc.py".to_owned(),
        |subcommand| format!("sdlc.py {subcommand}"),
    );
    let output =
        process::run(command, Limits::text(deadline), cancel).map_err(|f| f.words(&name))?;
    let failed = (!output.status.success()).then(|| output.exited(&name, &refusal(&output)));
    let out = String::from_utf8(output.stdout)
        .map_err(|_| format!("{name} printed text that isn't UTF-8."))?;
    failed.map_or(Ok(out), Err)
}

/// Why tink-sdlc refused: API 1's `error.message` on stdout, else its `sdlc.py: error:` line,
/// else the last line it printed on stderr.
pub(crate) fn refusal(output: &Output) -> String {
    serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()
        .and_then(|value| value["error"]["message"].as_str().map(str::to_owned))
        .or_else(|| {
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .find(|line| line.starts_with("sdlc.py: error:"))
                .map(|line| line.trim().to_owned())
        })
        .unwrap_or_else(|| output.said())
}
