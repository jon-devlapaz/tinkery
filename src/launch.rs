//! One launch of `tinkery`, resolved once from the command line before any screen opens: who
//! declared they are typing, the data roots, the drafting setup, Shape's agent, and the worker
//! and Watch inputs. Think, the home menu and the `seed` command all read this same value, so no
//! place guesses attribution or resolves a root again. One option table drives parsing and
//! `--help`. Resolving reads the saved model setup and nothing else; it writes nothing, starts no
//! model request and never makes an agent the operator. Agent operation is `--operator agent`
//! alone.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    confirmation::GoalConfirmation,
    data_roots::DataRoots,
    model_setup::{self, Controls, Setup},
    pi::{CallBudget, Drafting, Harness},
    repo::Repo,
    shape::Agent,
    think::brain_dump::{BrainDump, ModelHost},
    watch::CodexReport,
    worker,
};

/// What an option takes after its flag.
#[derive(Clone, Copy)]
enum Takes {
    Nothing,
    /// A value; the words say what is missing.
    Value(&'static str),
    /// A non-empty value; the words say what is missing.
    Path(&'static str),
}

/// One command-line option: its flag, its exact `--help` line, what it takes, and the refusal
/// when it is given twice (`None` lets the last one win).
struct Opt {
    flag: &'static str,
    help: &'static str,
    takes: Takes,
    once: Option<&'static str>,
}

const fn opt(flag: &'static str, help: &'static str, takes: Takes) -> Opt {
    Opt {
        flag,
        help,
        takes,
        once: None,
    }
}

/// Every option, in `--help` order.
const OPTIONS: [Opt; 19] = [
    opt(
        "--home",
        "--home       Open the home menu (the tinkery launcher passes it)",
        Takes::Nothing,
    ),
    Opt {
        once: Some("--operator may be declared only once"),
        ..opt(
            "--operator",
            "--operator human|agent  Who is typing, not verified identity (default: not declared); agent saves are the agent's, except an answer relayed with --relayed",
            Takes::Value("--operator needs human or agent"),
        )
    },
    opt(
        "--shape-pi",
        "--shape-pi   Draft with a real model for this launch",
        Takes::Nothing,
    ),
    opt(
        "--harness",
        "--harness    pi|claude-code: who drafts (default: saved, else pi); Claude Code uses its own login",
        Takes::Value("--harness needs pi or claude-code"),
    ),
    opt(
        "--model",
        "--model      Pi: provider/model-id; Claude Code: a claude- model id (required with --shape-pi unless one is saved)",
        Takes::Value("--model needs a model id"),
    ),
    opt(
        "--thinking",
        "--thinking   Reasoning level: Pi off|minimal|low|medium|high|xhigh|max, Claude Code low..max (default: saved, else low)",
        Takes::Value("--thinking needs a level"),
    ),
    opt(
        "--seed-session-root",
        "--seed-session-root  Saved goals to list/save (default needs absolute HOME: ~/.local/share/tinkery/sessions)",
        Takes::Path("--seed-session-root needs a location outside a repository"),
    ),
    opt(
        "--record-root",
        "--record-root  Where Think history is saved/read (default: ~/.local/share/tinkery/think)",
        Takes::Path("--record-root needs a folder"),
    ),
    opt(
        "--link-root",
        "--link-root PATH  Where tend's links go (default: ~/.local/share/tinkery/tend)",
        Takes::Path("--link-root needs a folder"),
    ),
    opt(
        "--pi-command",
        "--pi-command Pi executable (default: pi; Claude Code is always claude on PATH)",
        Takes::Value("--pi-command needs an executable path"),
    ),
    opt(
        "--shape-budget",
        "--shape-budget RESPONSES  Drafting model responses shared by Think and Shape this launch (default: saved, else 0)",
        Takes::Value("--shape-budget needs a number of model responses"),
    ),
    opt(
        "--sdlc-root",
        "--sdlc-root PATH  Choose a tink-sdlc checkout. Approve and Watch default to the launch project",
        Takes::Path("--sdlc-root needs a tink-sdlc checkout path"),
    ),
    opt(
        "--worker-model",
        "--worker-model PIN  Exact Pi provider/model:thinking for an explicit worker start",
        Takes::Value("--worker-model needs provider/model:thinking"),
    ),
    opt(
        "--worker-responses",
        "--worker-responses N  Worker response cap, separate from drafting (default 0); one response may be in flight",
        Takes::Value("--worker-responses needs a positive response cap"),
    ),
    Opt {
        once: Some("--worker-authorization may be supplied only once"),
        ..opt(
            "--worker-authorization",
            "--worker-authorization FILE  Operator's relayed words bound to seed/model/cap, or a standing grant for two-way, small Tinks with no gray item; required for agent F2, origin unverified",
            Takes::Path("--worker-authorization needs a relayed authorization file"),
        )
    },
    Opt {
        once: Some("--codex-report may be declared only once"),
        ..opt(
            "--codex-report",
            "--codex-report RUN_UUID:CHAT_ID:PATH  One saved Codex observation for Watch; requires --home and --sdlc-root",
            Takes::Value("--codex-report needs RUN_UUID:CHAT_ID:PATH"),
        )
    },
    opt(
        "--full-redraw",
        "--full-redraw  Repaint every frame, for terminals that draw badly",
        Takes::Nothing,
    ),
    opt(
        "--no-color",
        "--no-color   Turn off colors (also respects NO_COLOR)",
        Takes::Nothing,
    ),
    opt(
        "--snapshot",
        "--snapshot   Print a 100 x 30 view and exit; no terminal mode, no model requests",
        Takes::Nothing,
    ),
];

/// How `seed` is used, for every refusal of its arguments.
pub const SEED_USAGE: &str = "usage: tinkery seed status --json DIR | tinkery seed checks DIR | tinkery seed show DIR | \
    tinkery [options] seed new GOAL_FILE [ORIGIN] | \
    tinkery [options] seed step DIR look|delegate|answer WORDS|answer --relayed FILE|confirm DRAFT_SHA256 | \
    tinkery [options] seed start DIR";

/// The `--help` text, with one line per option from the same table that parses them.
pub fn help() -> String {
    let options = OPTIONS
        .iter()
        .map(|o| o.help)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Tinkery: a calm terminal front door for agentic work.\n\nUsage: tinkery [--home] [--no-color] [--snapshot]\n       tinkery --shape-pi [--harness pi|claude-code] --model MODEL\n       tinkery seed status --json DIR\n       tinkery seed checks DIR\n       tinkery [options] seed new GOAL_FILE [ORIGIN]\n       tinkery seed show DIR\n       tinkery [options] seed step DIR look|delegate|answer WORDS|answer --relayed FILE|confirm DRAFT_SHA256\n       tinkery --operator human|agent --sdlc-root ROOT --worker-model PIN --worker-responses N seed start DIR\n\nWithout a model, think is practice: replies are simulated and nothing leaves your machine.\nWith one, your words go to the model you chose and may cost money. Choose it with m on the menu\n(saved in ~/.config/tinkery/model.json for later launches) or with --shape-pi for one launch.\nNothing runs on its own. Only typing confirm or `seed new` saves a goal, and saving starts no work.\nInside, F2 runs the action the key bar names, F1 lists the rest by number, and Esc goes back.\n\n{options}\nTINKERY_NODE Node for local voice (otherwise found on PATH). Voice stays on your machine; installing it asks first."
    )
}

/// What the command line asks for.
pub enum Args {
    /// `--help` or `-h`: print [`help`] and stop.
    Help,
    /// `seed show`, `seed status` or `seed checks`: a read that needs no launch.
    SeedRead(Vec<String>),
    /// Open Tinkery, or run `seed new`, `seed step` or `seed start`, with these options.
    Run(Options),
}

/// The options as typed, before [`Launch::resolve`] checks how they combine.
#[derive(Default)]
pub struct Options {
    /// `--home`: open the home menu.
    pub home: bool,
    /// `--snapshot`: print one view and exit.
    pub snapshot: bool,
    /// `--no-color`, or `NO_COLOR` set.
    pub no_color: bool,
    /// `--full-redraw`.
    pub full_redraw: bool,
    /// `seed new`, `seed step` or `seed start` and its words, when given.
    pub seed: Option<Vec<String>>,
    switches: Vec<&'static str>,
    values: Vec<(&'static str, String)>,
}

impl Options {
    fn value(&self, flag: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(f, _)| *f == flag)
            .map(|(_, v)| v.as_str())
    }

    fn count(&self, flag: &'static str) -> Result<Option<usize>, String> {
        self.value(flag)
            .map(|n| n.parse::<usize>().map_err(|_| missing(flag)))
            .transpose()
    }

    fn path(&self, flag: &str) -> Option<PathBuf> {
        self.value(flag).map(PathBuf::from)
    }
}

fn missing(flag: &str) -> String {
    match OPTIONS.iter().find(|o| o.flag == flag).map(|o| o.takes) {
        Some(Takes::Value(words) | Takes::Path(words)) => words.into(),
        _ => unreachable!("only value options are read"),
    }
}

/// Read the command line (without the program name). Refuses an unknown argument, a missing
/// value and a repeated declaration; nothing is read from disk.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut options = Options {
        no_color: std::env::var_os("NO_COLOR").is_some(),
        ..Options::default()
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "seed" {
            let rest = args.by_ref().collect::<Vec<_>>();
            // Reads return here, before any launch setup can touch stored state.
            if !matches!(
                rest.first().map(String::as_str),
                Some("new" | "step" | "start")
            ) {
                return Ok(Args::SeedRead(rest));
            }
            options.seed = Some(rest);
            continue;
        }
        if arg == "--help" || arg == "-h" {
            return Ok(Args::Help);
        }
        let opt = OPTIONS
            .iter()
            .find(|o| o.flag == arg)
            .ok_or_else(|| format!("unknown argument: {arg}; use --help"))?;
        if let Some(refusal) = opt.once
            && options.values.iter().any(|(f, _)| *f == opt.flag)
        {
            return Err(refusal.into());
        }
        let value = match opt.takes {
            Takes::Nothing => None,
            Takes::Value(words) => Some(args.next().ok_or(words)?),
            Takes::Path(words) => Some(args.next().filter(|v| !v.is_empty()).ok_or(words)?),
        };
        // Values are checked as they arrive, so the first bad argument is the one refused.
        match (opt.flag, &value) {
            ("--operator", Some(v)) => drop(GoalConfirmation::terminal(Some(v))?),
            ("--codex-report", Some(v)) => drop(CodexReport::parse(v)?),
            ("--harness", Some(v)) => drop(Harness::parse(v)?),
            ("--worker-responses" | "--shape-budget", Some(v)) if v.parse::<usize>().is_err() => {
                return Err(missing(opt.flag));
            }
            _ => {}
        }
        match (opt.flag, value) {
            ("--home", _) => options.home = true,
            ("--snapshot", _) => options.snapshot = true,
            ("--no-color", _) => options.no_color = true,
            ("--full-redraw", _) => options.full_redraw = true,
            (flag, None) => options.switches.push(flag),
            (flag, Some(value)) => options.values.push((flag, value)),
        }
    }
    Ok(Args::Run(options))
}

/// Shape starts Pi in another folder, and a relative program path follows the child's folder.
/// Bare names stay bare so PATH still finds them.
fn launch_relative(program: PathBuf) -> Result<PathBuf, String> {
    if program.is_relative() && program.components().count() > 1 {
        return Ok(std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(program));
    }
    Ok(program)
}

/// A relative root means one under the launch folder.
fn absolute(root: PathBuf) -> Result<PathBuf, String> {
    Ok(match root.is_relative() {
        true => std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(root),
        false => root,
    })
}

/// Everything a fresh Think visit starts from in this launch. Each visit gets a new
/// conversation; the host, folders and launch declaration are the same every time.
#[derive(Clone)]
pub struct ThinkConfig {
    host: Option<Arc<ModelHost>>,
    sessions: Option<PathBuf>,
    records: Option<PathBuf>,
    repo: Option<Repo>,
    confirmation: GoalConfirmation,
    shift_enter: bool,
    voice: bool,
}

impl ThinkConfig {
    /// A fresh Think visit. Practice saves nothing and names no repository.
    pub fn dump(&self) -> BrainDump {
        let mut dump = match &self.host {
            Some(host) => {
                let dump = BrainDump::with_host(host.clone()).with_repo(self.repo.clone());
                let dump = match &self.records {
                    Some(records) => dump.with_record_root(records.clone()),
                    None => dump,
                };
                match &self.sessions {
                    Some(root) => dump.with_sessions(Some(root.clone())),
                    None => dump,
                }
            }
            None => BrainDump::default(),
        };
        dump.set_goal_confirmation(self.confirmation.clone())
            .expect("the launch declaration was checked once at parse");
        let dump = dump.with_shift_enter(self.shift_enter);
        match self.voice {
            true => dump.with_voice(Box::new(crate::voice::ProcessHost::new())),
            false => dump,
        }
    }
}

/// One resolved launch. Only [`Launch::resolve`] and [`Launch::practice`] make one, so holding
/// it means the options were checked together.
#[derive(Clone)]
pub struct Launch {
    /// The `--operator` declaration every write is attributed with; never verified identity.
    pub(crate) confirmation: GoalConfirmation,
    /// `--operator agent`.
    pub(crate) agent: bool,
    /// Whether `--operator` was given at all.
    pub(crate) declared: bool,
    pub(crate) roots: DataRoots,
    /// No `--seed-session-root`: the default sessions folder, which may need migrating.
    pub(crate) default_sessions: bool,
    /// The repository Tinkery was started in, found by looking for `.git`.
    pub(crate) repo: Option<Repo>,
    /// The launch's model transport and allowance, which Shape and Tend's connections share.
    pub(crate) drafting: Option<Drafting>,
    /// The model Shape answers and drafts through; without one, Shape only reads.
    pub(crate) shape_agent: Option<Arc<Agent>>,
    pub(crate) think: ThinkConfig,
    pub(crate) sdlc_root: Option<PathBuf>,
    pub(crate) codex_report: Option<CodexReport>,
    pub(crate) worker: Option<worker::Config>,
    pub(crate) worker_authorization: Result<Option<worker::Relay>, String>,
}

impl Launch {
    /// A practice launch over `roots`: no model, no declaration, no repository and no worker.
    /// It reads nothing.
    pub fn practice(roots: DataRoots) -> Self {
        let confirmation = GoalConfirmation::terminal(None).expect("no declaration is valid");
        Self {
            think: ThinkConfig {
                host: None,
                sessions: None,
                records: None,
                repo: None,
                confirmation: confirmation.clone(),
                shift_enter: false,
                voice: false,
            },
            confirmation,
            agent: false,
            declared: false,
            roots,
            default_sessions: false,
            repo: None,
            drafting: None,
            shape_agent: None,
            sdlc_root: None,
            codex_report: None,
            worker: None,
            worker_authorization: Ok(None),
        }
    }

    /// Check how the options combine and resolve them once, with the header's model controls.
    /// Reads the saved model setup under an absolute HOME; writes nothing and starts nothing.
    pub fn resolve(options: &Options) -> Result<(Self, Controls), String> {
        let shape_pi = options.switches.contains(&"--shape-pi");
        let shape_budget = options.count("--shape-budget")?;
        let pi_command = options
            .value("--pi-command")
            .map(|p| launch_relative(p.into()))
            .transpose()?;
        if !shape_pi
            && (["--harness", "--model", "--thinking", "--pi-command"]
                .iter()
                .any(|flag| options.value(flag).is_some())
                || shape_budget.is_some_and(|calls| calls > 0))
        {
            return Err(
                "--harness, --model, --thinking, --pi-command and --shape-budget require --shape-pi"
                    .into(),
            );
        }
        let sdlc_root = options.path("--sdlc-root");
        let codex_report = options
            .value("--codex-report")
            .map(CodexReport::parse)
            .transpose()?;
        let worker_model = options.value("--worker-model").map(str::to_owned);
        let worker_responses = options.count("--worker-responses")?.unwrap_or(0);
        let worker_authorization = options.path("--worker-authorization");
        // Watch, Approve and Taste live in the home menu; Think alone would ignore their inputs.
        // `seed start` is the one command that uses them without a screen.
        let starts = options
            .seed
            .as_ref()
            .is_some_and(|args| args.first().map(String::as_str) == Some("start"));
        let home = options.home || starts;
        if starts && options.home {
            return Err("seed start runs without a screen; drop --home".into());
        }
        if !home && sdlc_root.is_some() {
            return Err("--sdlc-root requires --home or seed start".into());
        }
        if codex_report.is_some() && (!options.home || sdlc_root.is_none()) {
            return Err("--codex-report requires --home and --sdlc-root".into());
        }
        if (worker_model.is_some() || worker_responses > 0 || worker_authorization.is_some())
            && (!home || sdlc_root.is_none())
        {
            return Err("worker options require --sdlc-root with --home or seed start".into());
        }
        if starts && sdlc_root.is_none() {
            return Err("seed start needs --sdlc-root naming the project checkout".into());
        }
        let sdlc_root = sdlc_root.map(absolute).transpose()?;
        let operator = options.value("--operator");
        let confirmation = GoalConfirmation::terminal(operator)?;
        let agent = operator == Some("agent");
        let home = std::env::var_os("HOME");
        let mut roots = DataRoots::from_home(home.as_deref());
        if let Some(root) = options.path("--record-root") {
            roots.records = Ok(absolute(root)?);
        }
        let setup_path = model_setup::path(home.as_deref());
        let saved = setup_path.as_deref().map_or(Ok(None), model_setup::load);
        let usable = saved.as_ref().ok().and_then(Option::as_ref).map(|setup| {
            if agent {
                setup.clone().for_agent()
            } else {
                setup.clone()
            }
        });
        let launch = Setup::launch(
            shape_pi,
            options.value("--harness").map(Harness::parse).transpose()?,
            options.value("--model").map(str::to_owned),
            options.value("--thinking").map(str::to_owned),
            shape_budget,
            usable.as_ref(),
        )?;
        let live = launch.is_some();
        let harness = launch.as_ref().map(Setup::harness).unwrap_or_default();
        let program = match pi_command.clone() {
            Some(_) if harness == Harness::ClaudeCode => {
                return Err("--pi-command applies to Pi, not Claude Code".into());
            }
            Some(command) => command,
            None => PathBuf::from(harness.program()),
        };
        if live {
            roots.records.clone()?;
        }
        let call_budget = CallBudget::new(launch.as_ref().map_or(0, Setup::calls));
        let controls = Controls::new(
            setup_path,
            saved,
            launch.clone().map(|setup| (setup, call_budget.clone())),
        )
        .listing_pi(pi_command.unwrap_or_else(|| Harness::Pi.program().into()));
        let controls = if agent {
            controls.for_agent()
        } else {
            controls
        };
        let host = launch
            .as_ref()
            .map(|launch| {
                ModelHost::new(
                    launch.harness(),
                    program.clone(),
                    launch.model().into(),
                    call_budget.clone(),
                )?
                .with_thinking(launch.thinking())
                .map(Arc::new)
            })
            .transpose()?;
        let default_sessions = options.value("--seed-session-root").is_none();
        if let Some(root) = options.path("--link-root") {
            roots.tend = crate::tend::needs::resolve_root(root);
        }
        if let Some(root) = options.path("--seed-session-root") {
            roots.sessions = Ok(crate::seed_me::resolve_root(Some(root))?);
        }
        if live && !options.snapshot {
            roots.sessions.clone()?;
        }
        let repo = std::env::current_dir()
            .ok()
            .and_then(|dir| Repo::enclosing(&dir));
        // Shape's agent step reads files where the goal was made, else in the named SDLC
        // checkout's repository, else where Tinkery started.
        let drafting = launch
            .as_ref()
            .map(|launch| {
                Drafting::new(
                    launch.harness(),
                    program,
                    launch.model().into(),
                    launch.thinking().into(),
                    call_budget,
                )
            })
            .transpose()?;
        let shape_agent = drafting.clone().map(|drafting| {
            Arc::new(Agent::new(
                drafting,
                sdlc_root
                    .as_deref()
                    .and_then(Repo::enclosing)
                    .or_else(|| repo.clone())
                    .map(|r| r.root().to_path_buf())
                    .or_else(|| std::env::current_dir().ok()),
                roots.records.clone().ok(),
            ))
        });
        let worker = match (worker_model, worker_responses) {
            (None, 0) => None,
            (model, responses) => {
                let config = worker::Config {
                    model: model.unwrap_or_default(),
                    responses,
                };
                config.validate()?;
                Some(config)
            }
        };
        let think = ThinkConfig {
            host,
            sessions: roots.sessions.clone().ok(),
            records: roots.records.clone().ok(),
            repo: repo.clone(),
            confirmation: confirmation.clone(),
            shift_enter: false,
            voice: false,
        };
        let launch = Self {
            confirmation,
            agent,
            declared: operator.is_some(),
            roots,
            default_sessions,
            repo,
            drafting,
            shape_agent,
            think,
            sdlc_root,
            codex_report,
            worker,
            worker_authorization: worker::Relay::read(worker_authorization.as_deref()),
        };
        Ok((launch, controls))
    }

    /// The same launch at a real terminal: Think offers voice, and names Shift+Enter when the
    /// terminal reports enhanced keys. `--snapshot` never calls this.
    pub fn at_terminal(mut self, shift_enter: bool) -> Self {
        self.think.shift_enter = shift_enter;
        self.think.voice = true;
        self
    }

    /// A fresh Think visit for this launch.
    pub fn think(&self) -> BrainDump {
        self.think.dump()
    }

    /// Move sessions from the old Seed Me folder when this launch uses the default sessions
    /// folder; `write` false only reports. The notice to show, if any.
    pub fn migrate(&self, write: bool) -> Result<Option<String>, String> {
        if !self.default_sessions {
            return Ok(None);
        }
        crate::seed_me::migration::migrate(std::env::var_os("HOME").as_deref(), write)
    }

    /// `seed new`, `seed step` and `seed start`: one explicit write each, attributed to this
    /// launch's declaration. Prints the new session, the session after the step, or the started
    /// worker. `seed start` is F2 without the screen: the same authorization and checks.
    pub fn seed_write(&self, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        use crate::shape::command::{self, Action};
        // No screen shows who is acting, so a write never falls back to the operator.
        if !self.declared {
            return Err(
                "seed new, seed step and seed start need --operator human|agent before seed; nothing was changed"
                    .into(),
            );
        }
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let session = match args.as_slice() {
            ["new", file, origin @ ..]
                if origin.len() <= 1 && !origin.iter().any(|o| o.starts_with("--")) =>
            {
                // The operator names this file on the command line, and a pipe such as `<(…)`
                // is valid here: waiting for its writer blocks only this command, never a view.
                #[allow(clippy::disallowed_methods)]
                let goal = std::fs::read_to_string(file)
                    .map_err(|why| format!("Cannot read the goal file {file}: {why}"))?;
                let origin = origin.first().copied().unwrap_or(file);
                let session = command::new_goal(
                    self.roots.sessions.clone()?,
                    &goal,
                    origin,
                    &self.confirmation,
                )?;
                println!("{}", session.display());
                return Ok(());
            }
            ["start", directory] => {
                let root = self
                    .sdlc_root
                    .clone()
                    .ok_or("seed start needs --sdlc-root")?;
                let review =
                    crate::approve::review(Path::new(directory), root, self.roots.records.clone())
                        .map_err(|why| format!("{why} Nothing was started."))?;
                let started = crate::worker::start_command(
                    review,
                    self.roots.records.clone(),
                    self.worker.clone(),
                    crate::worker::Gate::new(self.agent, self.worker_authorization.clone()),
                )?;
                println!("{}", serde_json::to_string_pretty(&started)?);
                return Ok(());
            }
            ["step", directory, action @ ..] => {
                let action = match action {
                    ["look"] => Action::Look,
                    ["delegate"] => Action::Delegate,
                    // A missing FILE must not fall through and save "--relayed" as the answer.
                    ["answer", word] if word.starts_with("--") => return Err(SEED_USAGE.into()),
                    ["answer", "--relayed", file] => {
                        Action::Relayed(crate::shape::answering::Relay::read(Path::new(file))?)
                    }
                    ["answer", words] => Action::Answer((*words).to_owned()),
                    ["confirm", sha256] => Action::Confirm((*sha256).to_owned()),
                    _ => return Err(SEED_USAGE.into()),
                };
                let session = Path::new(directory);
                for note in command::step(
                    session,
                    self.shape_agent.as_deref(),
                    &self.confirmation,
                    action,
                )? {
                    eprintln!("{note}");
                }
                session
            }
            _ => return Err(SEED_USAGE.into()),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&command::show(session)?)?
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_pi_paths_keep_meaning_when_shape_starts_pi_elsewhere() {
        let launch = std::env::current_dir().unwrap();
        assert_eq!(
            launch_relative("./TEST-bin/pi".into()).unwrap(),
            launch.join("./TEST-bin/pi")
        );
        assert_eq!(
            launch_relative("TEST-bin/pi".into()).unwrap(),
            launch.join("TEST-bin/pi")
        );
        assert_eq!(launch_relative("pi".into()).unwrap(), PathBuf::from("pi"));
        assert_eq!(
            launch_relative("/TEST/pi".into()).unwrap(),
            PathBuf::from("/TEST/pi")
        );
    }
}
