//! Explicit, bounded worker actions through the named checkout's SDLC runtime. SDLC owns gates,
//! sessions, caps and detached workers. Recent launch evidence can inform the operator’s cap choice.
//! It records relayed start attribution, never SDLC approvals, and never retries or calls a provider.

mod authorization;
pub(crate) use authorization::{Gate, Relay};
mod orientation;
use orientation::cap_advice;
mod panel;
pub(crate) use panel::{Destination, Panel};
#[cfg(test)]
mod tests;

use crate::sdlc::git;
use crate::{
    repo::Repo,
    sdlc,
    seed_me::{ready, runs},
    think_record,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

/// Tinkery's worker steps are `build`, then `build-2`, `build-3`… Other step names stay SDLC's.
const STEP: &str = "build";
// Official resolution may take 15 s, followed by admission taking 60 s.
const ADMISSION: Duration = Duration::from_secs(90);
const STAGE: Duration = Duration::from_secs(180);

/// A separate worker allowance; neither drafting settings nor seed confirmation supply it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Config {
    /// Exact Pi provider/model:thinking pin, never a saved default or alias.
    pub(crate) model: String,
    /// Maximum recorded responses across launch and resume; zero grants no allowance.
    pub(crate) responses: usize,
}

impl Config {
    /// Refuse missing allowance before any command or durable record is created.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let valid = regex::Regex::new(
            r"^[a-zA-Z0-9][a-zA-Z0-9._-]*/[a-zA-Z0-9][a-zA-Z0-9._:/-]*:(off|minimal|low|medium|high|xhigh|max)$",
        )
        .expect("worker pin pattern");
        if self.responses == 0 || !valid.is_match(&self.model) {
            return Err("Starting a worker needs an exact provider/model:thinking and a positive response allowance.".into());
        }
        Ok(())
    }
}

/// A verified seed and its canonical checkout, handed over for a start review. It grants nothing.
pub(crate) struct Review {
    /// Only the verified seed selected in this visit.
    pub(crate) seed: ready::ReadySeed,
    /// The canonical checkout verified for this seed.
    pub(crate) root: PathBuf,
}

/// An exact seed and canonical checkout selected for an explicit Start worker action.
#[derive(Clone, Debug)]
struct Start {
    /// Verified identity selected in Approve; execution rechecks its bytes and confirmation.
    target: ready::SeedTarget,
    /// Canonical checkout shown before the action; a substituted path is refused.
    root: PathBuf,
    /// Where Think records live; a record naming a different repository origin refuses the start.
    record_root: Result<PathBuf, String>,
    /// The separately disclosed worker allowance.
    config: Config,
    /// The reviewed start authority; an absent or mismatched agent relay refuses before creation.
    authorization: Gate,
}

impl Start {
    /// What F2 does, in the order `start` does it, with the cap guidance shown beside the cap.
    fn disclosure(&self, cap_advice: &str) -> String {
        let target = &self.target;
        let checkout = target
            .retry
            .as_ref()
            .map_or(self.root.as_path(), |base| base.checkout.as_path());
        let retry = target.retry.as_ref().map(|base| format!("Required starting commit: {}. Use this checkout with --sdlc-root to find the new run.\n", base.head)).unwrap_or_default();
        format!(
            "Tink Contract SHA-256: {}\nPi model: {}\nResponse cap: {} (one response may already be in flight).\n{cap_advice}\n\
             Checkout: {}\n\
             {retry}F2 uses committed source, creates the run, imports this Tink Contract, prepares an isolated build checkout, \
             and launches its bounded worker through tink-sdlc.\n\
             Your goal, confirmed Tink Contract and relevant source files go to that provider. \
             The worker may edit, run commands and commit inside its build checkout.\n\
             Commit project changes first. Definition gates still apply; passing checks is not your acceptance.",
            target.sha256,
            self.config.model,
            self.config.responses,
            checkout.display(),
        ) + &self.authorization.disclosure(Some((target, &self.config)))
    }
}

/// Source-owned run identity pinned across later actions, not a new workflow store.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    /// The exact canonical checkout where the build lives.
    root: PathBuf,
    /// SDLC run name, never interpreted as shell text.
    slug: String,
    /// SDLC's stable run identity; a reused name cannot substitute a run.
    id: String,
}

/// The current turn as reported by SDLC's offline source read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Turn {
    /// The adapter observed the latest turn end normally; this proves no checks.
    Ended,
    /// The latest turn has not ended normally, including an interrupted turn.
    Running,
    /// The adapter could not establish the turn state.
    Unknown,
    /// The launcher reached the response cap; the turn has not ended and only a stop is owed.
    AtCap,
    /// The turn has not ended and its worker process has exited; resume or a stop is owed.
    Gone,
}

/// One step's state, without converting stopped or uncertain work into completion. Watch and
/// the worker screen read and word it the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// Intake exists but no worker step has been prepared.
    NotStarted,
    /// The immutable step exists and has not been admitted.
    Prepared,
    /// A session is recorded; its current turn is independently observed.
    Active { turn: Turn },
    /// Admission was interrupted or failed; no automatic retry is safe.
    Uncertain,
    /// SDLC recorded a finished turn and a report, not passing verification.
    Completed,
    /// The worker was stopped explicitly or at the recorded response cap.
    Stopped { cap: bool },
    /// SDLC could not read the step record; no action may rely on it.
    Unreadable,
}

impl State {
    /// The one mapping from SDLC's step words. Without a step there is `NotStarted`. The state
    /// and stop reason come from the caller, the turn and cap flag from the step's status. A
    /// word, turn or stop reason this reader does not know is refused with the field that
    /// failed, and so is a cap or gone flag on a step that is not active, running and unstopped.
    /// A reached cap wins over a gone worker.
    pub(crate) fn read(
        state: &sdlc::Word<sdlc::StepState>,
        stopped_by: &sdlc::Word<sdlc::StopReason>,
        step: &sdlc::Step,
    ) -> Result<Self, &'static str> {
        use sdlc::{StepState as S, StopReason as R, TurnStatus as T};
        let turn = &step.turn;
        if step.cap_reached? {
            if !state.is(S::Active) || !turn.is(T::Running) || stopped_by.present() {
                return Err("The worker reports its response cap in an unsupported state.");
            }
            return Ok(Self::Active { turn: Turn::AtCap });
        }
        if step.gone? {
            if !state.is(S::Active) || !turn.is(T::Running) || stopped_by.present() {
                return Err("The worker reports its process gone in an unsupported state.");
            }
            return Ok(Self::Active { turn: Turn::Gone });
        }
        Ok(match state.known() {
            Some(S::Prepared) => Self::Prepared,
            Some(S::Active) => Self::Active {
                turn: match turn.known() {
                    Some(T::Ended) => Turn::Ended,
                    Some(T::Running) => Turn::Running,
                    Some(T::Unknown) => Turn::Unknown,
                    None => return Err("The worker turn is not reported in a supported form."),
                },
            },
            Some(S::Uncertain) => Self::Uncertain,
            Some(S::Completed) => Self::Completed,
            Some(S::Stopped) => Self::Stopped {
                cap: match stopped_by.known() {
                    Some(R::Cap) => true,
                    Some(R::Operator) => false,
                    None => return Err("The stopped worker has no known stop reason."),
                },
            },
            Some(S::Unreadable) => Self::Unreadable,
            None => return Err(UNSUPPORTED),
        })
    }

    /// States where a new step is a way forward: a started run with no step, a prepared step
    /// that may never launch (a new step abandons it), an uncertain launch, a completed or a
    /// stopped step.
    pub(crate) fn needs_new_step(self) -> bool {
        matches!(
            self,
            Self::NotStarted
                | Self::Prepared
                | Self::Uncertain
                | Self::Completed
                | Self::Stopped { .. }
        )
    }

    /// The one wording for a step state, never a completion or acceptance claim.
    pub(crate) fn words(self) -> &'static str {
        match self {
            Self::NotStarted => "not started",
            Self::Prepared => "prepared, not launched",
            Self::Active { turn: Turn::Ended } => "turn ended; completion still unrecorded",
            Self::Active {
                turn: Turn::Running,
            } => "running",
            Self::Active {
                turn: Turn::Unknown,
            } => "turn state unknown; inspect before resuming",
            Self::Active { turn: Turn::AtCap } => "reached its response cap",
            Self::Active { turn: Turn::Gone } => "not running; its turn did not end",
            Self::Uncertain => "launch uncertain; inspect before starting another",
            Self::Completed => "turn completed; verification is separate",
            Self::Stopped { cap: true } => "stopped by response cap",
            Self::Stopped { cap: false } => "stopped by operator",
            Self::Unreadable => "unreadable",
        }
    }
}

const UNSUPPORTED: &str = "The worker state is not supported; inspect SDLC before continuing.";

/// The position of a Tinkery step name: `build` is 1, `build-N` is N; other names are not ours.
fn step_number(name: &str) -> Option<u32> {
    if name == STEP {
        return Some(1);
    }
    let n = name.strip_prefix("build-")?;
    (!n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit()))
        .then(|| n.parse().ok())
        .flatten()
        .filter(|n| *n > 1)
}

/// The newest Tinkery step SDLC reports, the one every worker action addresses.
fn latest_step(steps: &[sdlc::Step]) -> Option<(u32, &sdlc::Step)> {
    steps
        .iter()
        .filter_map(|step| Some((step_number(step.data["name"].as_str()?)?, step)))
        .max_by_key(|(n, _)| *n)
}

/// The name a new step takes after `number`, or `build` when there is none.
fn step_name(number: u32) -> String {
    if number == 1 {
        STEP.into()
    } else {
        format!("{STEP}-{number}")
    }
}

/// One current SDLC observation for presentation and explicitly requested follow-up actions.
#[derive(Clone, Debug)]
struct Snapshot {
    /// Stable source identity used to recheck the next action.
    run: Run,
    /// The step every action addresses: the newest Tinkery step, or `build` before any exists.
    step: String,
    /// The name a new step would take.
    next_step: String,
    /// Effective immutable step pin, empty when there is no step.
    model: String,
    /// Effective cumulative response cap, zero when there is no step.
    responses: usize,
    /// The observed immutable step identity; resume must present this same observation.
    pin: Option<Pin>,
    /// Current source-owned worker state.
    state: State,
    /// SDLC verification state; a completed worker does not imply current verification.
    verification: sdlc::Word<sdlc::Verification>,
    /// Validated source-reported judgment IDs, not a completion or acceptance claim.
    pending_attestations: Vec<String>,
    /// SDLC's next action in its own words.
    next: String,
}

/// Opaque fingerprint of the source contract's fixed run, session, model, allowance and paths.
/// It permits comparison with what was displayed, never supplies authority on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pin(String);

/// Each variant needs its own explicit interface action; none runs after a refresh.
#[derive(Clone, Debug)]
enum Action {
    /// Read current source records without admitting work or writing anything.
    Refresh,
    /// Explicitly retry a prepared step after inspecting its unchanged contract.
    Launch(Pin),
    /// Resume the recorded session under its original cumulative cap.
    Resume(Pin),
    /// Record a finished turn only after current passing checks and SDLC's report/session check.
    Finish,
    /// Run the checkout's verification commands, separately from worker completion.
    Verify,
    /// Prepare and launch a new uniquely named step under this launch's separate allowance,
    /// first opening the build checkout if a refused start left none. Only for a state where
    /// no other action moves the work on, and only while the observed step is unchanged.
    Next { pin: Option<Pin>, config: Config },
    /// Stop the known step, never as normal completion. The reason is cap only if the step's
    /// Pi logs show the launcher's cap marker after at least cap responses; otherwise operator.
    Stop,
}

/// The safe follow-up established by the transaction, never inferred from a missing identity.
#[derive(Clone, Debug)]
enum Recovery {
    /// No creation was attempted; the operator may fix the cause and explicitly retry.
    Retry,
    /// Creation may have happened; read SDLC in Watch before any further start.
    Inspect,
    /// A source-owned identity is available for explicit follow-up actions.
    Run(Run),
}

/// A refusal with its transaction-owned explanation and safe follow-up.
#[derive(Clone, Debug)]
struct Failure {
    /// The actual failed boundary and any next step, without an invented success.
    message: String,
    /// Whether retry is safe, inspection is needed, or a pinned run is available.
    recovery: Recovery,
}

/// One finite action owned by the menu across visits. Dropping waits for its bounded command
/// sequence rather than killing admission between its uncertain and active receipts.
struct Job {
    result: mpsc::Receiver<Result<Snapshot, Failure>>,
    worker: Option<JoinHandle<()>>,
}

impl Job {
    /// Start only after the interface explicitly grants the displayed worker action.
    fn start(request: Start) -> Self {
        Self::spawn(move || start(request))
    }

    /// Perform exactly one requested action, rechecking the pinned run before any write.
    fn act(gate: Gate, run: Run, action: Action) -> Self {
        Self::spawn(move || {
            act(&gate, &run, action).map_err(|message| Failure {
                message,
                recovery: Recovery::Run(run),
            })
        })
    }

    fn spawn(work: impl FnOnce() -> Result<Snapshot, Failure> + Send + 'static) -> Self {
        let (send, result) = mpsc::channel();
        let owner = thread::current();
        let worker = thread::spawn(move || {
            let _ = send.send(work());
            owner.unpark();
        });
        Self {
            result,
            worker: Some(worker),
        }
    }

    /// Deliver the action result once; a pending job performs no additional actions.
    fn poll(&mut self) -> Option<Result<Snapshot, Failure>> {
        self.worker.as_ref()?;
        let result = match self.result.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(Failure {
                message: "The worker request stopped unexpectedly. Inspect SDLC before retrying."
                    .into(),
                recovery: Recovery::Inspect,
            }),
        };
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(result)
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn command(root: &Path, args: &[String], deadline: Duration) -> Result<String, String> {
    sdlc::command(root, args, &AtomicBool::new(false), deadline)
}

/// A checkout that cannot be the exact repository root it claims; never a guessed folder.
const MOVED: &str = "The selected checkout moved or is not a canonical repository root. Reopen it.";

fn canonical_root(root: &Path) -> Result<(), String> {
    if !root.is_absolute()
        || root.canonicalize().ok().as_deref() != Some(root)
        || Repo::enclosing(root).as_ref().map(Repo::root) != Some(root)
    {
        return Err(MOVED.into());
    }
    if !sdlc::installed(root) {
        return Err("The named checkout has no SDLC runtime.".into());
    }
    Ok(())
}

fn status(root: &Path, slug: &str) -> Result<sdlc::Run, String> {
    read_status(root, slug, &AtomicBool::new(false), sdlc::DEADLINE)
}

fn read_status(
    root: &Path,
    slug: &str,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<sdlc::Run, String> {
    canonical_root(root)?;
    if slug.is_empty()
        || slug.len() > 80
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || slug.starts_with('-')
    {
        return Err("The run name is not supported; no worker action was taken.".into());
    }
    let run = sdlc::status(root, slug, cancel, deadline)?;
    if run.data["slug"].as_str() != Some(slug) {
        return Err("The checkout reported a different run; no worker action was taken.".into());
    }
    Ok(run)
}

fn identity(root: &Path, slug: &str, report: &sdlc::Run) -> Result<Run, String> {
    let value = &report.data;
    let id = value["run_id"]
        .as_str()
        .filter(|id| sdlc::canonical_run_id(id))
        .ok_or("The run has no stable identity.")?;
    let root = match &report.checkout {
        Ok(path) => path.clone().unwrap_or_else(|| root.to_path_buf()),
        Err(sdlc::CheckoutGap::NotText) => return Err("The build checkout is unreadable.".into()),
        Err(sdlc::CheckoutGap::Nul | sdlc::CheckoutGap::Relative) => {
            return Err(MOVED.into());
        }
    };
    canonical_root(&root)?;
    Ok(Run {
        root,
        slug: slug.into(),
        id: id.into(),
    })
}

fn current(run: &Run) -> Result<sdlc::Run, String> {
    let value = status(&run.root, &run.slug)?;
    if identity(&run.root, &run.slug, &value)? != *run {
        return Err("This run or its build checkout changed. No worker action was taken.".into());
    }
    Ok(value)
}

fn require_seed(value: &sdlc::Run) -> Result<ready::ReadySeed, String> {
    runs::confirmed(value).map_err(|_| "The run's original confirmed Tink Contract is unavailable or changed. Recheck it before running work.".into())
}

// Check both the recorded ancestry and the files a newly launched worker will actually read.
fn check_retry_source(run: &Run, target: Option<&ready::SeedTarget>) -> Result<(), String> {
    if let Some(base) = target.and_then(|t| t.retry.as_ref()) {
        base.check_build(&run.root, &run.slug, &source_head(&run.root)?)
            .map_err(|_| "The new build did not start at the confirmed candidate. No worker was launched.")?;
        sdlc::source::check(&run.root, &base.head, |args| git(&run.root, args)).map_err(|why| {
            format!("{why} Inspect the build checkout before launching a worker.")
        })?;
    }
    Ok(())
}

fn source_head(root: &Path) -> Result<String, String> {
    Ok(git(root, &["rev-parse", "--verify", "HEAD"])?.trim().into())
}

fn committed_source(root: &Path) -> Result<String, String> {
    let head = source_head(root)?;
    let changes = git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
            ":(exclude)runs",
        ],
    )?;
    if !changes.is_empty() {
        return Err("Commit source changes first. The worker receives the committed checkout; uncommitted source outside runs/ would be omitted.".into());
    }
    Ok(head)
}

/// SDLC's stage 3 commits the run's records, so it refuses without a Git identity. Check the
/// same names it reads (environment, then repository config) before anything is created.
fn committer_identity(root: &Path) -> Result<(), String> {
    for who in ["COMMITTER", "AUTHOR"] {
        for (field, key) in [("NAME", "user.name"), ("EMAIL", "user.email")] {
            let set = std::env::var(format!("GIT_{who}_{field}")).is_ok_and(|v| !v.is_empty())
                || git(root, &["config", key]).is_ok_and(|v| !v.trim().is_empty());
            if !set {
                return Err("Git has no user.name and user.email here, and stage 3 must commit the run's records. Set them (git config user.name …; git config user.email …).".into());
            }
        }
    }
    Ok(())
}

fn start(request: Start) -> Result<Snapshot, Failure> {
    let (intake, source_commit) = (|| {
        request
            .authorization
            .check(Some(&request.target), &request.config, STEP)?;
        request.config.validate()?;
        canonical_root(&request.root)?;
        let (intake, goal) = ready::execution(&request.target, &request.root)?;
        if let Some(base) = &request.target.retry {
            base.check_checkout(&request.root)?;
            canonical_root(&intake.root)?;
        }
        let origins = request.record_root.as_ref().ok().and_then(|root| {
            crate::seed_me::connection::origins(root, &[(&request.target.session, goal.as_str())])
                .ok()
        });
        if let Some(think_record::MadeIn::Repo(origin)) = origins.as_ref().and_then(|o| o.first())
            && origin != &intake.root
        {
            return Err(
                "The selected Tink belongs to a different repository. No worker was started."
                    .into(),
            );
        }
        let source_commit = committed_source(&intake.root)?;
        committer_identity(&intake.root)?;
        if request
            .target
            .retry
            .as_ref()
            .is_some_and(|base| source_commit != base.head)
        {
            return Err("The retry checkout moved off its confirmed starting commit.".into());
        }
        let seeds = runs::seeds(&intake.root, &AtomicBool::new(false))?;
        if runs::started(&seeds, &intake.seed)? {
            return Err(
                "This Tink already has a run. Inspect that run instead of starting another worker."
                    .into(),
            );
        }
        ready::execution(&request.target, &intake.root)?;
        Ok((intake, source_commit))
    })()
    .map_err(|message: String| Failure {
        message: format!("{message} Nothing was started; fix that, then F2 tries again."),
        recovery: Recovery::Retry,
    })?;
    let mut known = None;
    let result = (|| {
        command(
            &intake.root,
            &["new".into(), intake.slug.clone()],
            sdlc::DEADLINE,
        )?;
        let value = status(&intake.root, &intake.slug)?;
        let run = identity(&intake.root, &intake.slug, &value)?;
        known = Some(run.clone());
        ready::execution(&request.target, &intake.root)?;
        command(
            &intake.root,
            &[
                "stage".into(),
                intake.slug.clone(),
                "1".into(),
                "--seed-contract".into(),
                intake.seed.to_string_lossy().into_owned(),
                "--tinkery".into(),
                intake.executable.to_string_lossy().into_owned(),
            ],
            STAGE,
        )?;
        let value = current(&run)?;
        let seed = value
            .seed?
            .ok_or("Intake did not record the selected Tink.")?;
        if !runs::matches(&seed, &intake.seed)? {
            return Err("Intake recorded a different Tink. No worker started.".into());
        }
        ready::execution(&request.target, &intake.root)?;
        if let Some(base) = &request.target.retry {
            base.check_checkout(&request.root)?;
        }
        if source_head(&intake.root)? != source_commit {
            return Err("The selected source commit changed during intake. Inspect the run before starting a worker.".into());
        }
        command(
            &intake.root,
            &["stage".into(), intake.slug.clone(), "3".into()],
            STAGE,
        )?;
        let value = status(&intake.root, &intake.slug)?;
        let build = identity(&intake.root, &intake.slug, &value)?;
        if build.id != run.id || build.root == run.root {
            return Err(
                "Stage 3 did not report this run's separate build checkout; no worker was started."
                    .into(),
            );
        }
        known = Some(build.clone());
        check_retry_source(&build, Some(&request.target))?;
        let value = current(&build)?;
        require_seed(&value)?;
        ready::execution(&request.target, &intake.root)?;
        request.authorization.record(
            &build,
            Some(&request.target),
            &request.config,
            "start",
            STEP,
        )?;
        prepare(&build, STEP, &request.config)?;
        let value = current(&build)?;
        require_seed(&value)?;
        ready::execution(&request.target, &intake.root)?;
        check_retry_source(&build, Some(&request.target))?;
        command(&build.root, &step_args(&build, STEP, "launch"), ADMISSION)?;
        snapshot(&build)
    })();
    result.map_err(|message| match known {
        Some(run) => Failure {
            message,
            recovery: Recovery::Run(run),
        },
        None => Failure {
            message: format!(
                "{message} Run {} may have been created. Read it in Watch with --sdlc-root {} before starting more work.",
                intake.slug,
                intake.root.display()
            ),
            recovery: Recovery::Inspect,
        },
    })
}

fn step_args(run: &Run, step: &str, action: &str) -> Vec<String> {
    vec!["step".into(), run.slug.clone(), step.into(), action.into()]
}

/// Prepare one pinned step with a fresh report path, and confirm SDLC recorded the displayed
/// model and cap before anything can launch it.
fn prepare(run: &Run, step: &str, config: &Config) -> Result<(), String> {
    let report = if step == STEP {
        "worker-report.md".to_string()
    } else {
        format!("worker-report-{}.md", &step[STEP.len() + 1..])
    };
    let mut args = step_args(run, step, "prepare");
    args.extend([
        "--report".into(),
        run.root
            .join("runs")
            .join(&run.slug)
            .join(report)
            .to_string_lossy()
            .into_owned(),
        "--model".into(),
        config.model.clone(),
        "--launcher".into(),
        run.root
            .join("_system/scripts/pi-durable-launcher")
            .to_string_lossy()
            .into_owned(),
        "--max-responses".into(),
        config.responses.to_string(),
    ]);
    command(&run.root, &args, sdlc::DEADLINE)?;
    let prepared = snapshot(run)?;
    if prepared.step != step
        || prepared.state != State::Prepared
        || prepared.model != config.model
        || prepared.responses != config.responses
    {
        return Err("The prepared worker differs from the displayed model or allowance. No worker was launched.".into());
    }
    Ok(())
}

/// A new step for a run whose current step can do nothing more. The build checkout is the one
/// on the run's own branch; a start refused before stage 3 left only the intake checkout.
fn next_step(
    gate: &Gate,
    run: &Run,
    pin: Option<Pin>,
    config: &Config,
) -> Result<Snapshot, String> {
    let observed = snapshot(run)?;
    if observed.pin != pin {
        return Err(
            "The worker changed since it was read. Nothing was prepared; read it again.".into(),
        );
    }
    if !observed.state.needs_new_step() {
        return Err(format!(
            "The {} step is {}; use its own keys. n is only for a step that can do nothing more.",
            observed.step,
            observed.state.words()
        ));
    }
    if observed.state == State::Uncertain && gate.agent() {
        return Err("The launch is uncertain, so a worker may still be running. Only the operator can reconcile it before a new step.".into());
    }
    config.validate()?;
    let value = current(run)?;
    // An active or unreadable step, whatever its name, may still write to this checkout. A
    // prepared step has no session, and an uncertain one was accounted for by the `n` that
    // replaced it; SDLC cannot complete either, so they never block.
    let busy: Vec<&str> = value
        .steps
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|step| {
            step.state.is(sdlc::StepState::Active) || step.state.is(sdlc::StepState::Unreadable)
        })
        .filter_map(|step| step.data["name"].as_str())
        .filter(|name| *name != observed.step)
        .collect();
    if !busy.is_empty() {
        return Err(format!(
            "Step {} is active or unreadable and may still write here. Finish or stop it in SDLC first.",
            busy.join(", ")
        ));
    }
    let seed = require_seed(&value)?;
    let target = seed.target.as_ref();
    gate.check(target, config, &observed.next_step)?;
    let mut run = run.clone();
    let branch =
        git(&run.root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).unwrap_or_default();
    if observed.state == State::NotStarted && branch.trim() != run.slug {
        if target.is_some_and(|t| t.retry.is_some()) {
            return Err("This retry run has no build checkout. Inspect it in Watch; a retry is restarted from Taste.".into());
        }
        let target = target.ok_or("The run's confirmed Tink Contract is unavailable.")?;
        ready::execution(target, &run.root)?;
        committed_source(&run.root)?;
        committer_identity(&run.root)?;
        command(
            &run.root,
            &["stage".into(), run.slug.clone(), "3".into()],
            STAGE,
        )?;
        let build = identity(&run.root, &run.slug, &status(&run.root, &run.slug)?)?;
        if build.id != run.id || build.root == run.root {
            return Err(
                "Stage 3 did not report this run's separate build checkout; no worker was started."
                    .into(),
            );
        }
        run = build;
        require_seed(&current(&run)?)?;
    }
    gate.record(&run, target, config, "new step", &observed.next_step)?;
    prepare(&run, &observed.next_step, config)?;
    command(
        &run.root,
        &step_args(&run, &observed.next_step, "launch"),
        ADMISSION,
    )?;
    snapshot(&run)
}

fn act(gate: &Gate, run: &Run, action: Action) -> Result<Snapshot, String> {
    if matches!(action, Action::Refresh) {
        let value = status(&run.root, &run.slug)?;
        let observed = identity(&run.root, &run.slug, &value)?;
        if observed.id != run.id {
            return Err("This run changed. No worker action was taken.".into());
        }
        return snapshot(&observed);
    }
    let value = current(run)?;
    if matches!(action, Action::Resume(_) | Action::Verify) {
        require_seed(&value)?;
    }
    if let Action::Next { pin, config } = action {
        return next_step(gate, run, pin, &config);
    }
    let name = value
        .steps
        .as_deref()
        .and_then(latest_step)
        .and_then(|(_, step)| step.data["name"].as_str())
        .unwrap_or(STEP)
        .to_owned();
    let step = |action: &str| step_args(run, &name, action);
    match action {
        Action::Refresh | Action::Next { .. } => {}
        Action::Launch(pin) => {
            let seed = require_seed(&value)?;
            admit(gate, run, &name, pin, true, seed.target.as_ref())?;
        }
        Action::Resume(pin) => {
            let seed = require_seed(&value)?;
            admit(gate, run, &name, pin, false, seed.target.as_ref())?;
        }
        Action::Finish => {
            // Only a machine-checked pass finishes; pending judgments are not passing checks.
            if !(value.verification.is(sdlc::Verification::Current) && value.receipt_current) {
                return Err("Verify first. Finish needs current passing checks; keep this worker available to repair failures.".into());
            }
            command(&run.root, &step("complete"), sdlc::DEADLINE)?;
        }
        Action::Stop => {
            let mut args = step("complete");
            args.extend([
                "--stopped-by".into(),
                if orientation::stopped_at_cap(run, &name) {
                    "cap"
                } else {
                    "operator"
                }
                .into(),
            ]);
            command(&run.root, &args, sdlc::DEADLINE)?;
        }
        Action::Verify => {
            let verify = &value.data["actions"]["verify"];
            if verify["allowed"].as_bool() != Some(true) {
                return Err(verify["reason"]
                    .as_str()
                    .unwrap_or("This runtime does not permit verification.")
                    .into());
            }
            let timeout = verify["timeout_seconds"]
                .as_u64()
                .filter(|seconds| *seconds > 0)
                .ok_or("The runtime supplied no bounded verification timeout.")?;
            command(
                &run.root,
                &["verify".into(), run.slug.clone()],
                Duration::from_secs(timeout),
            )?;
        }
    }
    snapshot(run)
}

fn admit(
    gate: &Gate,
    run: &Run,
    step: &str,
    pin: Pin,
    launch: bool,
    target: Option<&ready::SeedTarget>,
) -> Result<(), String> {
    let observed = snapshot(run)?;
    if observed.pin.as_ref() != Some(&pin) {
        return Err("The worker's model, allowance, session or contract changed. Admission refused; inspect it again.".into());
    }
    if launch && observed.state != State::Prepared {
        return Err(
            "Launch needs a prepared step. An uncertain or active step cannot be launched again."
                .into(),
        );
    }
    if !launch
        && !matches!(
            observed.state,
            State::Active {
                turn: Turn::Ended | Turn::Gone
            }
        )
    {
        return Err("Resume needs an ended turn or a gone worker. A running or unknown turn may still own this session, and a reached cap allows no more responses.".into());
    }
    if launch {
        check_retry_source(run, target)?;
    }
    gate.record(
        run,
        target,
        &Config {
            model: observed.model,
            responses: observed.responses,
        },
        if launch { "launch" } else { "resume" },
        step,
    )?;
    command(
        &run.root,
        &step_args(run, step, if launch { "launch" } else { "resume" }),
        ADMISSION,
    )?;
    Ok(())
}

fn snapshot(run: &Run) -> Result<Snapshot, String> {
    let value = current(run)?;
    let steps = value
        .steps
        .as_ref()
        .ok_or("This runtime does not report worker steps.")?;
    let latest = latest_step(steps);
    let name = latest
        .and_then(|(_, step)| step.data["name"].as_str())
        .unwrap_or(STEP)
        .to_owned();
    let next_step = step_name(latest.map_or(1, |(n, _)| n + 1));
    let (state, model, responses, pin) = if let Some((_, step)) = latest {
        let output = command(&run.root, &step_args(run, &name, "show"), sdlc::DEADLINE)?;
        let contract = sdlc::StepContract::read(
            serde_json::from_str(&output).map_err(|error| error.to_string())?,
            &run.id,
            &run.root,
        )
        .ok_or("The worker contract does not belong to this exact run and checkout.")?;
        let record = &contract.record;
        let model = record["model"]
            .as_str()
            .ok_or("The worker model pin is unavailable.")?
            .to_owned();
        let responses = contract
            .max_responses
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("The worker has no response cap; this interface cannot resume it.")?;
        Config {
            model: model.clone(),
            responses,
        }
        .validate()?;
        for field in ["report", "launcher"] {
            if record[field].as_str().is_none_or(|value| {
                !Path::new(value).is_absolute() || value.chars().any(char::is_control)
            }) {
                return Err(format!(
                    "The worker's {field} path is unavailable or malformed."
                ));
            }
        }
        if record["launcher_sha256"].as_str().is_none_or(|value| {
            value.len() != 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Err("The worker launcher fingerprint is unavailable or malformed.".into());
        }
        if matches!(
            contract.state.known(),
            Some(sdlc::StepState::Active | sdlc::StepState::Completed | sdlc::StepState::Stopped)
        ) && contract
            .session
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err("The worker has no recorded session identity.".into());
        }
        let fixed = serde_json::json!({
            "schema": record["schema"], "run_id": record["run_id"], "checkout": record["checkout"],
            "model": record["model"], "max_responses": record["max_responses"], "session": record["session"],
            "report": record["report"], "launcher": record["launcher"], "launcher_sha256": record["launcher_sha256"],
            "prepared_ns": record["prepared_ns"],
        });
        let pin = Pin(format!(
            "{:x}",
            Sha256::digest(fixed.to_string().as_bytes())
        ));
        let state = match State::read(&contract.state, &contract.stopped_by, step)? {
            State::Unreadable => return Err(UNSUPPORTED.into()),
            state => state,
        };
        (state, model, responses, Some(pin))
    } else {
        (State::NotStarted, String::new(), 0, None)
    };
    Ok(Snapshot {
        run: run.clone(),
        step: name,
        next_step,
        model,
        responses,
        pin,
        state,
        pending_attestations: value.pending_attestations?,
        verification: {
            value
                .verification
                .known()
                .ok_or("Verification state is unknown or missing.")?;
            value.verification
        },
        next: value.data["next_action"]
            .as_str()
            .ok_or("The run's next action is missing.")?
            .into(),
    })
}
