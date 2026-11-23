//! The retry hand-off: the reviewed checkout and commit pinned in a new attempt's goal evidence,
//! and the Git checks that hold a build to it. A fresh session keeps the verified goal and its
//! original attribution; earlier decisions become assumptions. Only the operator's explicit save
//! creates a session. Git checks read metadata only and run no script from the checkout.

use super::ledger;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const PIN: &str = "tinkery:retry/base";

/// The isolated checkout and exact commit reviewed for this attempt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Base {
    pub(crate) checkout: PathBuf,
    pub(crate) head: String,
}

/// The pin belongs to the goal's evidence, which the seed confirmation binds by digest.
pub(crate) fn base(ledger: &ledger::Value) -> Result<Option<Base>, String> {
    let origin = ledger["nodes"]
        .array()
        .unwrap_or_default()
        .iter()
        .find(|node| node["id"] == ledger["origin"]);
    let mut entries = origin
        .and_then(|n| n["evidence"].array())
        .unwrap_or_default()
        .iter()
        .filter(|entry| entry["artifact"].string() == Some(PIN));
    let Some(entry) = entries.next() else {
        return Ok(None);
    };
    let parsed: Option<Base> = entry["observed"]
        .string()
        .and_then(|s| serde_json::from_str(s).ok());
    parsed
        .filter(|pin| {
            entries.next().is_none()
                && pin.checkout.is_absolute()
                && pin
                    .checkout
                    .to_str()
                    .is_some_and(|s| !s.chars().any(char::is_control))
                && pin.head.len() == 40
                && pin
                    .head
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .map(Some)
        .ok_or_else(|| {
            "The rejected result's checkout or commit is unreadable. Inspect the original attempt."
                .into()
        })
}

impl Base {
    /// Require the reviewed candidate to remain a detached checkout of the named project.
    /// Only Git metadata is read. No script from the candidate is executed.
    pub(crate) fn check_checkout(&self, named: &Path) -> Result<(), String> {
        if self.detached_head(named)? == self.head {
            Ok(())
        } else {
            Err(self.moved())
        }
    }

    fn moved(&self) -> String {
        format!(
            "The retry needs its detached checkout at {} on commit {}. Open the matching project and restore that checkout before starting.",
            self.checkout.display(),
            self.head
        )
    }

    fn detached_head(&self, named: &Path) -> Result<String, String> {
        if self.checkout.canonicalize().ok().as_deref() != Some(&self.checkout) {
            return Err(self.moved());
        }
        let git = |root: &Path, args: &[&str]| {
            crate::sdlc::git(
                root,
                &[&["rev-parse", "--path-format=absolute"][..], args].concat(),
            )
            .map_err(|_| self.moved())
        };
        let shared = git(named, &["--git-common-dir"])?;
        let metadata = git(
            &self.checkout,
            &[
                "--show-toplevel",
                "--git-common-dir",
                "HEAD",
                "--abbrev-ref",
                "HEAD",
            ],
        )?;
        metadata
            .strip_prefix(&format!("{}\n{}", self.checkout.display(), shared))
            .and_then(|tail| tail.strip_suffix("\nHEAD\n"))
            .map(str::to_owned)
            .ok_or_else(|| self.moved())
    }

    /// A stage-3 build may include one direct commit containing only its own run evidence.
    pub(crate) fn check_build(&self, root: &Path, slug: &str, head: &str) -> Result<(), String> {
        use crate::sdlc::git;
        if head == self.head {
            return Ok(());
        }
        if git(root, &["show", "-s", "--format=%P", head])?.trim() != self.head {
            return Err(self.moved());
        }
        let exclude = format!(":(exclude)runs/{slug}");
        git(
            root,
            &[
                "diff",
                "--quiet",
                "--no-ext-diff",
                "--no-textconv",
                &self.head,
                head,
                "--",
                ".",
                &exclude,
            ],
        )
        .map(|_| ())
        .map_err(|_| self.moved())
    }

    pub(super) fn started(&self, named: &Path, session: &Path, slug: &str) -> Result<bool, String> {
        let head = self.detached_head(named)?;
        let unknown = || {
            format!(
                "Cannot identify the retry intake. Open Watch with --sdlc-root {} and inspect run {slug}.",
                self.checkout.display()
            )
        };
        let mut directory = self.checkout.clone();
        for component in ["runs", slug] {
            directory.push(component);
            match std::fs::symlink_metadata(&directory) {
                Ok(meta) if meta.is_dir() => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return if head == self.head {
                        Ok(false)
                    } else {
                        Err(self.moved())
                    };
                }
                _ => return Err(unknown()),
            }
        }
        self.check_build(&self.checkout, slug, &head)?;
        // Only the expected detached retry's intake record; never its runtime or steps.
        let recorded = (|| {
            let bytes = crate::regular_file::read_bounded(
                &directory.join("run.json"),
                4 * 1024 * 1024,
                "run.json is not a regular file",
            )
            .ok()?
            .bytes?;
            let run: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            let bound = &run["seed_contract"];
            if run["schema"] != 1
                || !crate::sdlc::canonical_run_id(run["run_id"].as_str()?)
                || bound["path"].as_str()? != format!("runs/{slug}/seed-contract.md")
                || bound.get("definitions_sha256").is_none()
            {
                return None;
            }
            crate::sdlc::Seed::parse(&run).ok().flatten()
        })()
        .ok_or_else(unknown)?;
        if !super::runs::matches(&recorded, &super::ready::path(session))? {
            return Err(unknown());
        }
        Ok(true)
    }
}

/// Goal and source decisions as reviewed on the new-attempt screen.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Prior {
    pub(crate) goal: String,
    pub(crate) decisions: Vec<String>,
    origin: Value,
    draft: Value,
}

/// Refuse an unverified source or a goal that differs from the displayed one.
pub(crate) fn prior(seed: &Path, goal: &str) -> Result<Prior, String> {
    let session = seed
        .parent()
        .ok_or("The prior Tink Contract has no session.")?;
    let ledger = ledger::load(session)?;
    if ledger["goal"].string() != Some(goal) || !ledger::verify_handoff(session, &ledger)?.verified
    {
        return Err("The prior Tink Contract changed. Reread Taste.".into());
    }
    let state = ledger::editable(&ledger)?.to_json()?;
    let nodes = state["nodes"]
        .as_array()
        .ok_or("The prior decisions are missing.")?;
    let mut origin = nodes
        .iter()
        .find(|n| n["id"] == state["origin"])
        .ok_or("The prior goal has no origin.")?
        .clone();
    let evidence = origin["evidence"]
        .as_array_mut()
        .ok_or("The prior goal has no evidence.")?;
    // Draft reviews and checkout pins belong only to their own attempt.
    evidence.retain(|entry| {
        entry["artifact"] != PIN
            && !entry["artifact"].as_str().is_some_and(|s| {
                s.starts_with("tinkery:teach-back/") || s == "tinkery:review/completed"
            })
    });
    let decisions = nodes
        .iter()
        .filter(|n| {
            n["id"] != state["origin"] && n["kind"] == "decision" && n["status"] == "settled"
        })
        .map(|n| {
            format!(
                "{}: {}",
                n["label"]
                    .as_str()
                    .or(n["id"].as_str())
                    .unwrap_or("Decision"),
                n["answer"].as_str().unwrap_or_default()
            )
        })
        .collect();
    Ok(Prior {
        goal: goal.into(),
        decisions,
        origin,
        draft: state["draft"].clone(),
    })
}

impl Prior {
    /// Save the displayed words and reference in one publication to an empty session.
    pub(crate) fn save(
        &self,
        session: &Path,
        base: &Base,
        words: &str,
        run: &str,
    ) -> Result<(), String> {
        let stamp = crate::calendar::now();
        let mut origin = self.origin.clone();
        let evidence = origin["evidence"]
            .as_array_mut()
            .ok_or("The prior goal has no evidence.")?;
        evidence
            .push(json!({"checked":"Taste rejection","at":stamp,"observed":words,"artifact":run}));
        evidence.push(
            json!({"artifact":PIN, "checked":"Rejected result checkout","at":stamp,
            "check":"The operator chose this starting commit for the next attempt.",
            "observed":serde_json::to_string(base).map_err(|e| e.to_string())?}),
        );
        let assumed: Vec<_> = self.decisions.iter().map(|s| json!({"text":s,"why":"Carried from the prior seed for review in this attempt."})).collect();
        let state = ledger::Value::from(
            json!({"status":"active","draft":self.draft,"goal":self.goal,
            "origin":origin["id"],"nodes":[origin],"current_question":null,"operator":"human","assumed":assumed}),
        );
        ledger::publish(
            session,
            &state,
            &ledger::Value::Int(0),
            &"Saved a new attempt from the reviewed Taste rejection.".into(),
            None,
            &stamp,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}
