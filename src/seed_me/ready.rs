//! A verified seed's exact goal and acceptance section for read-only handoff. Tinkery's ledger
//! verifies confirmation; changing source files refuse the read. This module never writes or runs intake.

use std::path::{Path, PathBuf};

use super::retry::Base;
use serde_json::Value;

/// The exact verified seed selected for a read-only visit; not copy or execution permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedTarget {
    pub(crate) session: PathBuf,
    pub(crate) session_id: String,
    pub(crate) retry: Option<Base>,
    pub(crate) version: u64,
    pub(crate) sha256: String,
}

/// One consistent, ledger-verified seed reading; commands are text, never execution permission.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadySeed {
    /// Identity of the verified reading, when the source supplies it completely.
    pub target: Option<SeedTarget>,
    /// The confirmed goal exactly as recorded.
    pub goal: String,
    /// The confirmed seed revision.
    pub revision: String,
    /// The acceptance section, including its heading and original whitespace.
    pub acceptance: String,
    /// One combined manual command, or no copy without a safely representable named checkout.
    pub commands: Option<String>,
    /// True for an exact detached retry intake; uncertain bindings prevent a second start.
    pub(crate) retry_started: Result<bool, String>,
    /// Confirmation attribution and consumer requirements; never implementation permission.
    pub confirmation_note: Option<String>,
}

/// The existing seed location for a session; resolving and checking it creates nothing.
pub(crate) fn path(session: &Path) -> std::path::PathBuf {
    session.join(super::files::SEED)
}

/// Read the confirmed seed from its recorded path, refusing changed or unconfirmed session files.
/// The caller must separately establish that these seed bytes belong to its run.
pub(crate) fn confirmed(seed: &Path) -> Result<ReadySeed, String> {
    let session = seed
        .parent()
        .ok_or("The Tink Contract has no session folder.")?;
    if path(session) != seed {
        return Err("The source is not this session's Tink Contract.".into());
    }
    read(session, None)?.ok_or("The original Tink's confirmation cannot be verified.".into())
}

/// Read a confirmed seed without mutation. `None` means not verified ready; errors mean unavailable.
/// Blocks on disk reads, so callers must use a worker, never an input or paint path.
pub fn read(session: &Path, root: Option<&Path>) -> Result<Option<ReadySeed>, String> {
    super::migration::refuse_old_root(session)?;
    read_with(session, root, || standing(session))
}

/// Attribution and intake requirement of an agent-confirmed seed; never operator confirmation.
const AGENT_NOTE: &str = "Agent-confirmed Tink. Intake requires tink-sdlc 1.38.0 or newer; this is not operator confirmation or implementation approval.";

/// Where one session's seed stands, read once through the ledger and its verifier. Reads only;
/// each error is Tinkery's refusal in the operator's words.
pub(crate) struct Standing {
    /// The ledger as stored.
    pub(crate) ledger: super::ledger::Value,
    /// The same ledger as JSON.
    pub(crate) json: Value,
    /// The verified status, with each node's review flags, revisability and what it waits on.
    pub(crate) status: Value,
    /// Where the seed stands.
    pub(crate) stand: Stand,
    /// The retry checkout pinned in the goal's evidence.
    pub(crate) retry: Option<Base>,
    /// The exact verified seed, only while it stands ready.
    pub(crate) target: Option<SeedTarget>,
    /// Attribution and intake requirement of an agent-confirmed ready seed; not permission.
    pub(crate) note: Option<&'static str>,
    /// The single eligible deferred prerequisite, only while waiting.
    pub(crate) deferred: Option<super::ledger::DeferredPrerequisite>,
}

/// Where a session's seed stands. Each says only what the ledger's files show; nothing is guessed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Stand {
    /// A settled node the ledger flags for review, by id; shown before any question or seed.
    Review(String),
    /// The current question's id; its node is in the ledger and has wording.
    Question(String),
    /// Active with no current question and no seed.
    Waiting,
    /// A seed draft waiting for confirmation, with its revision label.
    Drafted(Option<String>),
    /// A confirmed seed the verifier accepts; it stays ready after the session is completed.
    Ready {
        /// The confirmed revision label.
        revision: Option<String>,
        /// Whether the session is completed.
        ended: bool,
    },
    /// A seed that says it is confirmed but fails verification, with Tinkery's reason.
    Unverified(String),
    /// A practice session's seed, never a real one.
    Practice,
    /// Completed or stopped, with the ledger's status word.
    Ended(String),
    /// What the files say that Tinkery does not recognise.
    Unknown(&'static str),
}

/// Read where `session`'s seed stands. Blocks on disk reads, so callers must use a worker.
pub(crate) fn standing(session: &Path) -> Result<Standing, String> {
    let read = || -> Result<Standing, String> {
        let ledger = super::ledger::load(session)?;
        let flags = super::ledger::review_flags(&ledger["nodes"]);
        let status = super::ledger::status_from_ledger(session, &ledger)?;
        let revisable: Vec<_> = ledger["nodes"]
            .array()
            .unwrap_or(&[])
            .iter()
            .filter_map(|n| n["id"].string())
            .filter(|id| super::ledger::can_revise(&ledger, id))
            .map(str::to_owned)
            .collect();
        let waits: serde_json::Map<String, Value> = ledger["nodes"]
            .array()
            .unwrap_or(&[])
            .iter()
            .filter_map(|node| {
                let id = node["id"].string()?;
                Some((
                    id.to_owned(),
                    serde_json::json!(super::ledger::waiting_on(&ledger["nodes"], id)),
                ))
            })
            .collect();
        let json = ledger.to_json()?;
        let mut status = status.to_json()?;
        status["review_flags"] = flags.to_json()?;
        status["revisable"] = serde_json::json!(revisable);
        status["waiting_on"] = Value::Object(waits);
        let stand = stand(&json, &status);
        let retry = super::retry::base(&ledger)?;
        let ready = matches!(stand, Stand::Ready { .. });
        let target = ready
            .then(|| {
                Some(SeedTarget {
                    session: session.canonicalize().ok()?,
                    session_id: json["session_id"]
                        .as_str()
                        .filter(|id| !id.is_empty() && !id.chars().any(char::is_control))?
                        .to_owned(),
                    retry: retry.clone(),
                    version: json.get("version").and_then(Value::as_u64)?,
                    sha256: status["seed"]["sha256"].as_str()?.to_owned(),
                })
            })
            .flatten();
        if ready && retry.is_some() && target.is_none() {
            return Err("The retry has no verified session identity.".into());
        }
        let deferred = match stand {
            Stand::Waiting => super::ledger::deferred_prerequisite(&ledger)?,
            _ => None,
        };
        let note = (ready && status["handoff"]["operator"] == "agent").then_some(AGENT_NOTE);
        Ok(Standing {
            ledger,
            json,
            status,
            stand,
            retry,
            target,
            note,
            deferred,
        })
    };
    read().map_err(|reason| format!("Tinkery refused: {reason}"))
}

/// Where the seed stands, from the ledger and its verified `status`. An open question comes
/// before a seed, so a reopened decision is never hidden behind an older draft.
pub(crate) fn stand(ledger: &Value, status: &Value) -> Stand {
    let text = |value: &Value, key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
    };
    let nodes = ledger
        .get("nodes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let Some(state) = text(status, "status") else {
        return Stand::Unknown("Tinkery read no session status.");
    };
    let handoff = status.get("handoff");
    let verified = handoff
        .and_then(|h| h.get("verified"))
        .and_then(Value::as_bool)
        == Some(true);
    let seed = status.get("seed").filter(|s| !s.is_null());
    let line = seed.and_then(|s| s.get("line_1")).and_then(Value::as_str);
    let revision = seed.and_then(|s| text(s, "revision"));
    if state == "completed" && line == Some(super::files::CONFIRMED) && verified {
        return Stand::Ready {
            revision,
            ended: true,
        };
    }
    if state != "active" {
        return Stand::Ended(state);
    }
    if ledger
        .get("goal")
        .and_then(Value::as_str)
        .is_none_or(|g| g.trim().is_empty())
    {
        return Stand::Unknown("This session has no confirmed goal.");
    }
    if let Some(id) = nodes
        .iter()
        .filter_map(|n| n["id"].as_str())
        .find(|id| status["review_flags"].get(id).is_some())
    {
        return Stand::Review(id.to_owned());
    }
    if let Some(id) = text(status, "current_question") {
        let Some(node) = nodes.iter().find(|n| text(n, "id").as_deref() == Some(&id)) else {
            return Stand::Unknown("The current question isn't in its ledger.");
        };
        if text(node, "question")
            .or_else(|| text(node, "gate"))
            .is_none()
        {
            return Stand::Unknown("The current question has no wording.");
        }
        return Stand::Question(id);
    }
    if seed.is_none() {
        return Stand::Waiting;
    }
    match line {
        Some(super::files::DRAFT) => Stand::Drafted(revision),
        Some(super::files::SIMULATED) => Stand::Practice,
        Some(super::files::CONFIRMED) if verified => Stand::Ready {
            revision,
            ended: false,
        },
        Some(super::files::CONFIRMED) => Stand::Unverified(
            handoff
                .and_then(|h| text(h, "reason"))
                .unwrap_or_else(|| "Tinkery gave no reason.".into()),
        ),
        _ => Stand::Unknown("Tinkery cannot recognize the Tink Contract's status line."),
    }
}

fn read_with(
    session: &Path,
    root: Option<&Path>,
    inspect: impl FnOnce() -> Result<Standing, String>,
) -> Result<Option<ReadySeed>, String> {
    let seed_path = path(session);
    let seed = match super::read_seed_text(&seed_path) {
        Ok(seed) => seed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot read the Tink Contract: {e}")),
    };
    if seed.lines().next() != Some(super::files::CONFIRMED) {
        return Ok(None);
    }
    let ledger =
        super::ledger::read_bytes(session).map_err(|e| format!("Cannot read the ledger: {e}"))?;
    let shaped = inspect()?;
    if super::read_seed(&seed_path).map_err(|e| e.to_string())? != seed.as_bytes()
        || super::ledger::read_bytes(session).map_err(|e| e.to_string())? != ledger
    {
        return Err(
            "The session changed while being read. Return to the menu and reopen Approve.".into(),
        );
    }
    let Stand::Ready { revision, .. } = shaped.stand else {
        return Ok(None);
    };
    let retry = shaped
        .target
        .as_ref()
        .and_then(|target| target.retry.as_ref());
    let retry_started = match (shaped.target.as_ref(), retry, root) {
        (Some(target), Some(base), Some(named)) => {
            base.started(named, &target.session, &retry_name(target))
        }
        (_, Some(base), None) => Err(format!(
            "Name this retry’s checkout with --sdlc-root {}.",
            base.checkout.display()
        )),
        _ => Ok(false),
    };
    let commands = if matches!(retry_started, Ok(false)) {
        root.and_then(|named| {
            let title = seed.lines().find_map(|line| line.strip_prefix("# "))?;
            let run = run_name(title, shaped.target.as_ref())?;
            let checkout = retry.map_or(named, |base| base.checkout.as_path());
            crate::sdlc::installed(checkout).then_some(())?;
            intake(
                session,
                &std::env::current_exe().ok()?,
                &run,
                checkout,
                retry.map(|base| (base, named)),
            )
        })
    } else {
        None
    };
    Ok(Some(ReadySeed {
        goal: shaped.json["goal"]
            .as_str()
            .filter(|g| !g.trim().is_empty())
            .ok_or("The verified Tink has no goal.")?
            .to_owned(),
        target: shaped.target,
        revision: revision.ok_or("The verified Tink has no revision.")?,
        acceptance: acceptance(&seed)?,
        commands,
        retry_started,
        confirmation_note: shaped.note.map(Into::into),
    }))
}

/// The one shell line that starts a build: change to `root`, create run `run` and import the seed
/// through `executable`. Paths are canonical and shell-quoted; unsafe paths offer no command.
fn intake(
    session: &Path,
    executable: &Path,
    run: &str,
    root: &Path,
    retry: Option<(&Base, &Path)>,
) -> Option<String> {
    let intake = Intake::new(session, executable, run, root)?;
    let quote = |path: &Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
    let guard = if let Some((base, named)) = retry {
        let named = named.canonicalize().ok()?;
        if named.to_str()?.chars().any(char::is_control) {
            return None;
        }
        format!(
            "test \"$(pwd -P)\" = {} && \
             test \"$(git rev-parse --abbrev-ref HEAD)\" = HEAD && \
             test \"$(git rev-parse HEAD)\" = '{}' && \
             test \"$(git rev-parse --path-format=absolute --git-common-dir)\" = \"$(git -C {} rev-parse --path-format=absolute --git-common-dir)\" && ",
            quote(&intake.root),
            base.head,
            quote(&named)
        )
    } else {
        String::new()
    };
    Some(format!(
        "cd {} && {guard}python3 _system/scripts/sdlc.py new {run} && python3 _system/scripts/sdlc.py stage {run} 1 --seed-contract {} --tinkery {}",
        quote(&intake.root),
        quote(&intake.seed),
        quote(&intake.executable),
    ))
}

/// Canonical intake arguments shared by copying and explicitly requested execution.
/// These paths and the selected seed are rechecked before execution; this is not permission.
#[derive(Clone, Debug)]
pub(crate) struct Intake {
    /// The named checkout, without a shell or unresolved aliases.
    pub(crate) root: PathBuf,
    /// The same title-derived run name used by the copied command.
    pub(crate) slug: String,
    /// The selected session's verified seed file.
    pub(crate) seed: PathBuf,
    /// This Tinkery executable, used by the consumer to verify confirmation.
    pub(crate) executable: PathBuf,
}

impl Intake {
    fn new(session: &Path, executable: &Path, run: &str, root: &Path) -> Option<Self> {
        let canonical = |path: &Path| {
            let path = path.canonicalize().ok()?;
            (!path.to_str()?.chars().any(char::is_control)).then_some(path)
        };
        Some(Self {
            root: canonical(root)?,
            slug: run.into(),
            seed: canonical(&path(session))?,
            executable: canonical(executable)?,
        })
    }
}

/// Revalidate the exact selected seed and return argv data, never parsed shell text.
/// Disk reads belong on an explicitly requested worker, not the input or paint path.
pub(crate) fn execution(target: &SeedTarget, root: &Path) -> Result<(Intake, String), String> {
    use sha2::{Digest, Sha256};
    let ready = read(&target.session, None)?
        .filter(|ready| ready.target.as_ref() == Some(target))
        .ok_or("The selected Tink changed or is no longer confirmed. Reopen Approve.")?;
    let bytes = super::read_seed(&path(&target.session)).map_err(|error| error.to_string())?;
    if format!("{:x}", Sha256::digest(&bytes)) != target.sha256 {
        return Err("The selected Tink changed before intake. Reopen Approve.".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let title = text.lines().find_map(|line| line.strip_prefix("# "));
    let run = title
        .and_then(|title| run_name(title, Some(target)))
        .ok_or("The selected Tink has no usable run name.")?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let checkout = target
        .retry
        .as_ref()
        .map_or(root, |base| base.checkout.as_path());
    let intake = Intake::new(&target.session, &executable, &run, checkout)
        .ok_or("The checkout, Tink Contract or Tinkery executable cannot be resolved safely.")?;
    Ok((intake, ready.goal))
}

fn run_name(title: &str, target: Option<&SeedTarget>) -> Option<String> {
    if let Some(target) = target.filter(|target| target.retry.is_some()) {
        return Some(retry_name(target));
    }
    slug(title.strip_prefix("Seed: ").unwrap_or(title))
}

fn retry_name(target: &SeedTarget) -> String {
    use sha2::{Digest, Sha256};
    let id = format!("{:x}", Sha256::digest(target.session_id.as_bytes()));
    format!("retry-{}", &id[..16])
}

/// Run names stay scannable: whole lowercase ASCII words joined by hyphens, at most
/// `SLUG_MAX` bytes. Only a first word longer than the bound is cut.
const SLUG_MAX: usize = 40;

fn slug(title: &str) -> Option<String> {
    let mut slug = String::new();
    for word in title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if slug.is_empty() {
            slug = word[..word.len().min(SLUG_MAX)].to_ascii_lowercase();
        } else if slug.len() + 1 + word.len() <= SLUG_MAX {
            slug.push('-');
            slug.push_str(&word.to_ascii_lowercase());
        } else {
            break;
        }
    }
    (!slug.is_empty()).then_some(slug)
}

fn acceptance(seed: &str) -> Result<String, String> {
    let mut section = String::new();
    let mut found = false;
    let mut inside = false;
    for line in seed.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "## Acceptance criteria" {
            if found {
                return Err("The Tink Contract repeats its acceptance section.".into());
            }
            found = true;
            inside = true;
        } else if line.starts_with("## ") {
            inside = false;
        }
        if inside {
            section.push_str(line);
        }
    }
    if !found {
        return Err("The Tink Contract has no acceptance section.".into());
    }
    Ok(section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(verified: bool) -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-ready-'")
            .tempdir()
            .unwrap();
        let session = root.path().join("session");
        fs::create_dir(&session).unwrap();
        super::super::ledger::fixtures::confirmed(&session, " TEST goal\nverbatim  ", "r1");
        if !verified {
            fs::remove_file(session.join("seed-confirmation.json")).unwrap();
        }
        // Both named checkouts have tink-sdlc's script, so a command can be offered.
        for checkout in [root.path(), session.as_path()] {
            fs::create_dir_all(checkout.join("_system/scripts")).unwrap();
            fs::write(checkout.join(crate::sdlc::SCRIPT), "TEST script").unwrap();
        }
        (root, session)
    }

    #[test]
    fn copy_paths_are_canonical_and_shell_quoted() {
        let temp = tempfile::Builder::new()
            .prefix("TEST-intake-'quoted-")
            .tempdir()
            .unwrap();
        let session = temp.path().join("session");
        let scripts = temp.path().join("TEST executable's folder");
        fs::create_dir(&session).unwrap();
        fs::create_dir(&scripts).unwrap();
        let seed = session.join("seed-contract.md");
        let helper = scripts.join("tinkery");
        fs::write(&seed, "TEST seed").unwrap();
        fs::write(&helper, "TEST helper").unwrap();
        let quote = |path: &Path| {
            format!(
                "'{}'",
                path.canonicalize()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\'', "'\\''")
            )
        };
        assert_eq!(
            intake(&session, &helper, "test-run", temp.path(), None),
            Some(format!(
                "cd {} && python3 _system/scripts/sdlc.py new test-run && python3 _system/scripts/sdlc.py stage test-run 1 --seed-contract {} --tinkery {}",
                quote(temp.path()),
                quote(&seed),
                quote(&helper)
            ))
        );
        let alias = temp.path().join("TEST-linked-executable");
        std::os::unix::fs::symlink(&helper, &alias).unwrap();
        assert_eq!(
            intake(&session, &alias, "test-run", temp.path(), None),
            intake(&session, &helper, "test-run", temp.path(), None)
        );
        assert_eq!(
            intake(
                &session,
                &helper,
                "test-run",
                &temp.path().join("TEST-missing"),
                None
            ),
            None
        );
    }

    #[test]
    fn only_verified_seeds_supply_exact_words_and_shared_intake_commands() {
        for verified in [false, true] {
            let (_root, session) = fixture(verified);
            let before = fs::read(session.join("seed-contract.md")).unwrap();
            let seed = read(&session, Some(&session)).unwrap();
            assert_eq!(seed.is_some(), verified);
            if let Some(seed) = seed {
                assert_eq!(seed.goal, " TEST goal\nverbatim  ");
                assert_eq!(
                    seed.acceptance,
                    "## Acceptance criteria\n\nTEST exact café  \nyou judge: TEST exact café  \n\n"
                );
                assert_eq!(
                    seed.commands.unwrap(),
                    intake(
                        &session,
                        &std::env::current_exe().unwrap(),
                        "test",
                        &session,
                        None,
                    )
                    .unwrap()
                );
            }
            assert_eq!(fs::read(session.join("seed-contract.md")).unwrap(), before);
            fs::write(session.join("seed-contract.md"), "status: draft\n").unwrap();
            assert!(read(&session, Some(&session)).unwrap().is_none());
        }
    }

    #[test]
    fn agent_receipts_name_the_actual_actor_and_offer_manual_external_intake() {
        let (root, session) = fixture(true);
        let seed_path = path(&session);
        let draft = fs::read_to_string(&seed_path)
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("Confirmed by:"))
            .map(|line| {
                if line == "status: confirmed for intake" {
                    "status: draft"
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&seed_path, draft).unwrap();
        let actor: crate::confirmation::GoalConfirmation = serde_json::from_value(serde_json::json!({
            "by":"agent", "agent":"TEST-agent", "authorizing_operator":"TEST-person",
            "source":"TEST explicit permission", "class":{"door":"two_way", "door_reason":"TEST reversible",
            "blast_radius":"small", "blast_radius_reason":"TEST isolated"}
        })).unwrap();
        super::super::ledger::confirm_as(
            &session,
            "r1",
            "TEST seed only",
            "2026-10-21T00:00:00Z",
            actor.actor().unwrap(),
        )
        .unwrap();
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(session.join("seed-confirmation.json")).unwrap())
                .unwrap();
        assert_eq!(
            receipt["operator"], "agent",
            "An agent's receipt must never say human"
        );
        assert_eq!(receipt["actor"], serde_json::to_value(&actor).unwrap());
        // Agent and human receipts use the same formatter and safely representable paths.
        assert!(
            intake(
                &session,
                &std::env::current_exe().unwrap(),
                "test",
                root.path(),
                None
            )
            .is_some()
        );
        let ready = read(&session, Some(root.path())).unwrap().unwrap();
        assert_eq!(
            ready.commands,
            intake(
                &session,
                &std::env::current_exe().unwrap(),
                "test",
                root.path(),
                None
            )
        );
        assert!(
            ready
                .confirmation_note
                .unwrap()
                .contains("Agent-confirmed Tink. Intake requires tink-sdlc 1.38.0")
        );
        assert!(standing(&session).unwrap().note.is_some());
        let before = fs::read(&seed_path).unwrap();
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!({"by": "agent"}),
            serde_json::json!({ "by": "agent", "agent": "TEST-agent",
                "authorizing_operator": "TEST-person", "source": "TEST explicit permission",
                "class": {"door": "one_way", "door_reason": "TEST",
                    "blast_radius": "small", "blast_radius_reason": "TEST"}}),
            serde_json::json!({ "by": "agent", "agent": "TEST-agent",
                "authorizing_operator": "TEST-person", "source": "TEST explicit permission",
                "class": {"door": "two_way", "door_reason": "TEST",
                    "blast_radius": "large", "blast_radius_reason": "TEST"}}),
        ] {
            let mut changed = receipt.clone();
            if invalid.is_null() {
                changed.as_object_mut().unwrap().remove("actor");
            } else {
                changed["actor"] = invalid;
            }
            fs::write(session.join("seed-confirmation.json"), changed.to_string()).unwrap();
            assert!(read(&session, Some(root.path())).unwrap().is_none());
            assert!(!matches!(
                standing(&session).unwrap().stand,
                Stand::Ready { .. }
            ));
        }
        fs::write(session.join("seed-confirmation.json"), receipt.to_string()).unwrap();
        assert!(
            read(&session, Some(root.path()))
                .unwrap()
                .unwrap()
                .commands
                .is_some()
        );
        assert_eq!(fs::read(&seed_path).unwrap(), before);
    }

    #[test]
    fn a_changed_source_is_refused_instead_of_displaying_unverified_bytes() {
        for file in ["seed-contract.md", "ledger.json"] {
            let (_root, session) = fixture(true);
            assert!(
                read_with(&session, Some(&session), || {
                    let shaped = standing(&session)?;
                    fs::write(session.join(file), "TEST changed").unwrap();
                    Ok(shaped)
                })
                .unwrap_err()
                .contains("changed while being read")
            );
        }
    }

    #[test]
    fn a_seed_or_ledger_replaced_during_verification_is_refused_without_waiting() {
        use std::{
            ffi::CString,
            os::unix::fs::OpenOptionsExt,
            sync::mpsc,
            time::{Duration, Instant},
        };

        for file in ["seed-contract.md", "ledger.json"] {
            let (_root, session) = fixture(true);
            let seed = session.join(file);
            let (cancel, released) = mpsc::channel();
            let fifo = seed.clone();
            // A reverted blocking read is released safely; successful refusal cancels the release.
            let release = std::thread::spawn(move || {
                if released.recv_timeout(Duration::from_secs(3)).is_err() {
                    drop(
                        fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .custom_flags(libc::O_NONBLOCK)
                            .open(fifo)
                            .unwrap(),
                    );
                }
            });
            let started = Instant::now();
            let result = read_with(&session, None, || {
                let shaped = standing(&session)?;
                fs::remove_file(&seed).unwrap();
                let name = CString::new(seed.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                Ok(shaped)
            });
            let elapsed = started.elapsed();
            let _ = cancel.send(());
            release.join().unwrap();
            assert!(result.is_err());
            assert!(
                elapsed < Duration::from_secs(2),
                "replacement waited for a FIFO writer: {elapsed:?}"
            );
        }
    }

    #[test]
    fn unsafe_copy_paths_keep_the_seed_readable_but_offer_no_command() {
        let (root, session) = fixture(true);
        let moved = root.path().join("session\nunsafe");
        fs::rename(session, &moved).unwrap();
        let seed = read(&moved, Some(root.path())).unwrap().unwrap();
        assert!(seed.commands.is_none());
        assert_eq!(seed.revision, "r1");
    }

    #[test]
    fn title_slugs_are_bounded_shell_words_and_missing_checkouts_offer_no_copy() {
        assert_eq!(
            slug("TEST café; $(touch nope)"),
            Some("test-caf-touch-nope".into())
        );
        assert_eq!(slug("你好"), None);
        assert_eq!(slug(&"a".repeat(200)).unwrap().len(), 40);
        assert_eq!(slug(&format!("{} next", "a".repeat(39))).unwrap().len(), 39);
        assert_eq!(
            slug(
                "Encounter an explicit unknown route state on unsupported paths rather than the app"
            ),
            Some("encounter-an-explicit-unknown-route".into())
        );
        assert_eq!(
            slug("Fix the Safari export"),
            Some("fix-the-safari-export".into())
        );
        let (_root, session) = fixture(true);
        assert!(read(&session, None).unwrap().unwrap().commands.is_none());
    }

    #[test]
    fn acceptance_bytes_are_never_trimmed_or_rewritten() {
        let section = "## Acceptance criteria\n\n- TEST café  \n  continuation\n\n";
        assert_eq!(
            acceptance(&format!("# TEST\n{section}## Evidence\nTEST\n")).unwrap(),
            section
        );
        assert!(acceptance("# TEST no section").is_err());
        assert!(acceptance(&format!("{section}{section}")).is_err());
    }
}
