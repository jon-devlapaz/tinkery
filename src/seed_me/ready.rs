//! A verified seed's exact goal and acceptance section for read-only handoff. Tinkery's ledger
//! verifies confirmation; changing source files refuse the read. This module never writes or runs intake.

use std::{fs, path::Path};

use super::shaping::{self, Shaped};

/// One consistent, ledger-verified seed reading; commands are text, never execution permission.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadySeed {
    /// The confirmed goal exactly as recorded.
    pub goal: String,
    /// The confirmed seed revision.
    pub revision: String,
    /// The acceptance section, including its heading and original whitespace.
    pub acceptance: String,
    /// One combined manual command, or no copy without a safely representable named checkout.
    pub commands: Option<String>,
    /// Confirmation attribution and consumer requirements; never implementation permission.
    pub confirmation_note: Option<String>,
}

/// The existing seed location for a session; resolving and checking it creates nothing.
pub(crate) fn path(session: &Path) -> std::path::PathBuf {
    session.join("seed-contract.md")
}

/// Read the exact saved goal beside an original seed, never a placeholder or a run's copy.
/// The caller must establish the seed's identity separately; this does not verify confirmation.
pub(crate) fn goal(seed: &Path) -> Result<String, String> {
    super::migration::refuse_old_root(seed)?;
    let ledger = seed
        .parent()
        .ok_or("The seed has no session folder.")?
        .join("ledger.json");
    let bytes = fs::read(&ledger).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["schema_version"].as_u64() != Some(1) {
        return Err("The saved goal format isn't supported.".into());
    }
    value["goal"]
        .as_str()
        .filter(|goal| !goal.trim().is_empty())
        .map(str::to_owned)
        .ok_or("The seed has no saved goal.".into())
}

/// Read a confirmed seed without mutation. `None` means not verified ready; errors mean unavailable.
/// Blocks on disk reads, so callers must use a worker, never an input or paint path.
pub fn read(session: &Path, root: Option<&Path>) -> Result<Option<ReadySeed>, String> {
    super::migration::refuse_old_root(session)?;
    read_with(session, root, || shaping::read(session))
}

fn read_with(
    session: &Path,
    root: Option<&Path>,
    inspect: impl FnOnce() -> Result<shaping::Shaping, String>,
) -> Result<Option<ReadySeed>, String> {
    let seed_path = path(session);
    let seed = match fs::read_to_string(&seed_path) {
        Ok(seed) => seed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot read the seed: {e}")),
    };
    if seed.lines().next() != Some("status: confirmed for intake") {
        return Ok(None);
    }
    let ledger_path = session.join("ledger.json");
    let ledger = fs::read(&ledger_path).map_err(|e| format!("Cannot read the ledger: {e}"))?;
    let shaped = inspect()?;
    if fs::read(&seed_path).map_err(|e| e.to_string())? != seed.as_bytes()
        || fs::read(&ledger_path).map_err(|e| e.to_string())? != ledger
    {
        return Err(
            "The session changed while being read. Return to the menu and reopen Approve.".into(),
        );
    }
    let Shaped::Ready { revision, .. } = shaped.stage else {
        return Ok(None);
    };
    Ok(Some(ReadySeed {
        goal: shaped
            .goal
            .filter(|g| !g.trim().is_empty())
            .ok_or("The verified seed has no goal.")?,
        revision: revision.ok_or("The verified seed has no revision.")?,
        acceptance: acceptance(&seed)?,
        commands: root.and_then(|root| {
            let title = seed.lines().find_map(|line| line.strip_prefix("# "))?;
            let run = slug(title.strip_prefix("Seed: ").unwrap_or(title))?;
            shaping::intake(session, &std::env::current_exe().ok()?, &run, Some(root))
        }),
        confirmation_note: shaped.confirmation_note,
    }))
}

fn slug(title: &str) -> Option<String> {
    let words: Vec<_> = title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let mut slug = words.join("-").to_ascii_lowercase();
    slug.truncate(80);
    let slug = slug.trim_end_matches('-');
    (!slug.is_empty()).then(|| slug.to_owned())
}

fn acceptance(seed: &str) -> Result<String, String> {
    let mut section = String::new();
    let mut found = false;
    let mut inside = false;
    for line in seed.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "## Acceptance criteria" {
            if found {
                return Err("The seed repeats its acceptance section.".into());
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
        return Err("The seed has no acceptance section.".into());
    }
    Ok(section)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        (root, session)
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
                    shaping::intake(
                        &session,
                        &std::env::current_exe().unwrap(),
                        "test",
                        Some(&session),
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
        let actor: crate::think_record::GoalConfirmation = serde_json::from_value(serde_json::json!({
            "by":"agent", "agent":"TEST-agent", "authorizing_operator":"TEST-person",
            "source":"TEST explicit permission", "class":{"door":"two_way", "door_reason":"TEST reversible",
            "blast_radius":"small", "blast_radius_reason":"TEST isolated"}
        })).unwrap();
        super::super::ledger::confirm_as(
            &session,
            "r1",
            "TEST seed only",
            "2026-10-21T00:00:00Z",
            Some(&actor),
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
            shaping::intake(
                &session,
                &std::env::current_exe().unwrap(),
                "test",
                Some(root.path())
            )
            .is_some()
        );
        let ready = read(&session, Some(root.path())).unwrap().unwrap();
        assert_eq!(
            ready.commands,
            shaping::intake(
                &session,
                &std::env::current_exe().unwrap(),
                "test",
                Some(root.path())
            )
        );
        assert!(
            ready
                .confirmation_note
                .unwrap()
                .contains("Agent-confirmed seed. Intake requires tink-sdlc 1.38.0")
        );
        let shaped = shaping::read(&session).unwrap();
        assert!(shaped.intake.is_some());
        assert!(shaped.confirmation_note.is_some());
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
            assert!(shaping::read(&session).unwrap().intake.is_none());
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
        let (_root, session) = fixture(true);
        assert!(
            read_with(&session, Some(&session), || {
                let shaped = shaping::read(&session)?;
                fs::write(session.join("seed-contract.md"), "TEST changed").unwrap();
                Ok(shaped)
            })
            .unwrap_err()
            .contains("changed while being read")
        );
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
        assert_eq!(slug(&"a".repeat(200)).unwrap().len(), 80);
        assert_eq!(slug(&format!("{} next", "a".repeat(79))).unwrap().len(), 79);
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
