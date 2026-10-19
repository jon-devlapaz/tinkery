//! A verified seed's exact goal and acceptance section for read-only handoff. Seed Me alone
//! verifies confirmation; changing source files refuse the read. This module never writes or runs intake.

use std::{fs, path::Path};

use super::shaping::{self, Shaped};

/// One consistent, helper-verified seed reading; commands are text, never execution permission.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadySeed {
    /// The confirmed goal exactly as recorded by Seed Me.
    pub goal: String,
    /// The confirmed seed revision.
    pub revision: String,
    /// The acceptance section, including its heading and original whitespace.
    pub acceptance: String,
    /// Both manual commands, or no copy when a canonical path cannot be safely represented.
    pub commands: Option<String>,
}

/// Read a confirmed seed without mutation. `None` means not verified ready; errors mean unavailable.
/// Blocks on bounded helper calls, so callers must use a worker, never an input or paint path.
pub fn read(skill: &Path, session: &Path) -> Result<Option<ReadySeed>, String> {
    let seed_path = session.join("seed-contract.md");
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
    let shaped = shaping::read(skill, session)?;
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
        commands: shaped
            .intake
            .map(|intake| format!("python3 _system/scripts/sdlc.py new <run>\n{intake}")),
    }))
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

    fn fixture(verified: bool) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let root = tempfile::Builder::new()
            .prefix("tinkery-TEST-ready-'")
            .tempdir()
            .unwrap();
        let skill = root.path().join("skill/SKILL.md");
        let session = root.path().join("session");
        fs::create_dir_all(skill.parent().unwrap().join("scripts")).unwrap();
        fs::create_dir(&session).unwrap();
        fs::write(&skill, "TEST skill").unwrap();
        fs::write(session.join("ledger.json"), "TEST unchanged ledger").unwrap();
        fs::write(session.join("seed-contract.md"), "status: confirmed for intake\nrevision: r1\n## Acceptance criteria\n\n- TEST exact café  \n\n## Evidence\nTEST\n").unwrap();
        let ledger = serde_json::json!({"goal":" TEST goal\nverbatim  ","nodes":[]}).to_string();
        let status = serde_json::json!({"status":"active","seed":{"line_1":"status: confirmed for intake","revision":"r1"},"handoff":{"verified":verified}}).to_string();
        fs::write(
            skill.parent().unwrap().join("scripts/session.py"),
            format!("import sys\nprint({ledger:?} if sys.argv[1] == 'read' else {status:?})\n"),
        )
        .unwrap();
        (root, skill, session)
    }

    #[test]
    fn only_verified_seeds_supply_exact_words_and_shared_intake_commands() {
        for verified in [false, true] {
            let (_root, skill, session) = fixture(verified);
            let before = fs::read(session.join("seed-contract.md")).unwrap();
            let seed = read(&skill, &session).unwrap();
            assert_eq!(seed.is_some(), verified);
            if let Some(seed) = seed {
                assert_eq!(seed.goal, " TEST goal\nverbatim  ");
                assert_eq!(
                    seed.acceptance,
                    "## Acceptance criteria\n\n- TEST exact café  \n\n"
                );
                assert_eq!(
                    seed.commands.unwrap(),
                    format!(
                        "python3 _system/scripts/sdlc.py new <run>\n{}",
                        shaping::read(&skill, &session).unwrap().intake.unwrap()
                    )
                );
            }
            assert_eq!(fs::read(session.join("seed-contract.md")).unwrap(), before);
            fs::write(session.join("seed-contract.md"), "status: draft\n").unwrap();
            assert!(read(&skill, &session).unwrap().is_none());
        }
    }

    #[test]
    fn a_changed_source_is_refused_instead_of_displaying_unverified_bytes() {
        let (_root, skill, session) = fixture(true);
        let helper = skill.parent().unwrap().join("scripts/session.py");
        let mut code = fs::read_to_string(&helper).unwrap();
        code.push_str("\nif sys.argv[1] == 'status':\n from pathlib import Path\n Path(sys.argv[2], 'seed-contract.md').write_text('TEST changed')\n");
        fs::write(helper, code).unwrap();
        assert!(
            read(&skill, &session)
                .unwrap_err()
                .contains("changed while being read")
        );
    }

    #[test]
    fn unsafe_copy_paths_keep_the_seed_readable_but_offer_no_command() {
        let (root, skill, session) = fixture(true);
        let moved = root.path().join("session\nunsafe");
        fs::rename(session, &moved).unwrap();
        let seed = read(&skill, &moved).unwrap().unwrap();
        assert!(seed.commands.is_none());
        assert_eq!(seed.revision, "r1");
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
