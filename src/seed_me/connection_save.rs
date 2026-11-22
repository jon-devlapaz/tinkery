//! One frozen Tend confirmation, staged privately and published once. Recovery never chooses
//! another destination, never overwrites another operation, and never starts Shape or a model.

use super::{Affirmed, Config, Receipt, connection::Origin, ledger};
use crate::think_record::{Actor, GoalConfirmation};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

/// A receipt persisted by Tend before any staging or session directory is created.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pending {
    pub(crate) id: String,
    pub(crate) root: PathBuf,
    pub(crate) origin: Origin,
    pub(crate) outcome: String,
    pub(crate) actor: GoalConfirmation,
}

impl Pending {
    /// Freeze validated words and one random identity without writing session storage.
    pub(crate) fn prepare(
        root: &Path,
        origin: Origin,
        outcome: String,
        actor: GoalConfirmation,
    ) -> Result<Self, String> {
        origin.validate(&origin.goal)?;
        Actor::authorize(Some(&actor))?;
        let root = root
            .canonicalize()
            .map_err(|_| "The sessions folder is unavailable.")?;
        let mut random = [0u8; 24];
        crate::regular_file::random(&mut random).map_err(|e| e.to_string())?;
        Ok(Self {
            id: format!(
                "session-tend-{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            root,
            origin,
            outcome,
            actor,
        })
    }

    fn paths(&self) -> Result<(PathBuf, PathBuf), String> {
        self.origin.validate(&self.origin.goal)?;
        Actor::authorize(Some(&self.actor))?;
        if !self.id.starts_with("session-tend-")
            || self.id.len() != 61
            || !self.id[13..].bytes().all(|c| c.is_ascii_hexdigit())
            || self.root.canonicalize().ok().as_ref() != Some(&self.root)
        {
            return Err("The pending save has an invalid destination.".into());
        }
        let staging = self
            .root
            .parent()
            .ok_or("The sessions root has no parent.")?
            .join(".tinkery-tend-staging")
            .join(&self.id);
        if let Ok(meta) = fs::symlink_metadata(staging.parent().expect("staging parent"))
            && (meta.is_symlink() || !meta.is_dir())
        {
            return Err("The staging parent is invalid.".into());
        }
        Ok((staging, self.root.join(&self.id)))
    }

    /// Inspect only. A published destination wins over later source membership changes.
    pub(crate) fn inspect(&self) -> Result<Option<Receipt>, String> {
        let (_, destination) = self.paths()?;
        match fs::symlink_metadata(&destination) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("The save outcome is unknown: {e}")),
            Ok(meta) if meta.is_symlink() || !meta.is_dir() => {
                Err("The destination belongs to another operation.".into())
            }
            Ok(_) => {
                self.owned(&destination)?;
                let value = ledger::load(&destination)?;
                let actor = Actor::authorize(Some(&self.actor))?;
                let origin = value["nodes"]
                    .array()
                    .unwrap_or_default()
                    .iter()
                    .find(|node| node["id"] == value["origin"])
                    .ok_or("The saved goal origin is missing.")?;
                let delegation = actor.delegation().map(ledger::Value::from);
                if value["draft"]["outcome"].string() != Some(&self.outcome)
                    || origin["authority"].string() != Some(actor.authority())
                    || origin.get("delegation") != delegation.as_ref()
                    || origin["authority_source"].string()
                        != Some("Tend: explicit save of displayed connection goal")
                {
                    return Err("The destination differs from the frozen confirmation.".into());
                }
                if value["session_id"].string() != Some(&self.id)
                    || value["goal"].string() != Some(&self.origin.goal)
                    || super::connection::read(&destination, &self.origin.goal)?.as_ref()
                        != Some(&self.origin)
                {
                    return Err(
                        "The published destination conflicts with this confirmation.".into(),
                    );
                }
                Ok(Some(Receipt {
                    session: destination,
                    goal: self.origin.goal.clone(),
                }))
            }
        }
    }

    fn owned(&self, path: &Path) -> Result<(), String> {
        let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.is_symlink() {
            return Err("The staging path is not an owned directory.".into());
        }
        let marker = path.join("tend-handoff.json");
        let meta =
            fs::symlink_metadata(&marker).map_err(|_| "The staging ownership is unknown.")?;
        if !meta.is_file() || meta.is_symlink() {
            return Err("The staging ownership is invalid.".into());
        }
        let read: Self = serde_json::from_slice(
            &crate::regular_file::read(&marker, "The staging ownership is invalid.")
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if read != *self {
            return Err("This directory belongs to another confirmation.".into());
        }
        Ok(())
    }

    /// Explicit resume validates current inputs immediately before publishing the fixed destination.
    pub(crate) fn resume(
        &self,
        current: impl Fn() -> Result<(), String>,
    ) -> Result<Receipt, String> {
        let _writer = self.writer()?;
        if let Some(receipt) = self.inspect()? {
            // A readable rename can precede durable parent updates. Retry both before success.
            let (staging, _) = self.paths()?;
            sync(staging.parent().expect("staging has parent"))?;
            sync(&self.root)?;
            return Ok(receipt);
        }
        current()?;
        let (staging, destination) = self.paths()?;
        let parent = staging.parent().expect("staging has parent");
        if parent.exists()
            && fs::symlink_metadata(parent)
                .map_err(|e| e.to_string())?
                .is_symlink()
        {
            return Err("The staging parent must not be a symlink.".into());
        }
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        match fs::DirBuilder::new().mode(0o700).create(&staging) {
            Ok(()) => {
                let marker = staging.join("tend-handoff.json");
                let mut file = crate::regular_file::open(
                    fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600),
                    0,
                    &marker,
                    "The staging ownership is invalid.",
                )
                .map_err(|e| e.to_string())?;
                file.write_all(&serde_json::to_vec(self).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
                sync(&staging)?;
                sync(parent)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => self.owned(&staging)?,
            Err(e) => return Err(e.to_string()),
        }
        // A complete marker binds all rebuilding to this frozen confirmation. No other path is removed.
        for entry in fs::read_dir(&staging).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == "tend-handoff.json" {
                continue;
            }
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                return Err("Unexpected staging content; inspect the pending save.".into());
            }
            fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        }
        ledger::initialize_at(
            &staging,
            &self.id,
            &crate::calendar::now(),
            Actor::authorize(Some(&self.actor))?,
        )?;
        Config::at(self.root.clone(), staging.clone()).confirm_with_origin(
            Affirmed {
                goal: self.origin.goal.clone(),
                outcome: self.outcome.clone(),
                options: vec![],
                originals: vec![],
                context: self.origin.interpretation.clone(),
                source: "Tend: explicit save of displayed connection goal".into(),
            },
            &self.actor,
            Some(&self.origin),
        )?;
        sync(&staging)?;
        current()?;
        publish(&staging, &destination)?;
        sync(parent)?;
        sync(&self.root)?;
        self.inspect()?
            .ok_or_else(|| "The published session could not be verified.".into())
    }

    fn writer(&self) -> Result<fs::File, String> {
        use std::os::fd::AsRawFd;
        let (staging, _) = self.paths()?;
        let parent = staging.parent().expect("staging parent");
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        sync(parent.parent().expect("staging ancestor"))?;
        let file = crate::regular_file::open(
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false),
            libc::O_NOFOLLOW,
            &parent.join(format!("{}.lock", self.id)),
            "The pending save's lock is not a regular file.",
        )
        .map_err(|e| e.to_string())?;
        // SAFETY: the file descriptor stays owned by this File until the operation ends.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Another writer owns this pending save.".into());
        }
        Ok(file)
    }

    /// Cancel only a proved unpublished save, removing only the staging directory it owns.
    pub(crate) fn cancel(&self) -> Result<(), String> {
        let _writer = self.writer()?;
        if self.inspect()?.is_some() {
            return Err("This goal is already saved; it cannot be cancelled.".into());
        }
        let (staging, _) = self.paths()?;
        match fs::symlink_metadata(&staging) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
            Ok(_) => {
                self.owned(&staging)?;
                fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
                sync(staging.parent().expect("staging has parent"))
            }
        }
    }
}

fn sync(path: &Path) -> Result<(), String> {
    crate::regular_file::sync_directory(path).map_err(|e| e.to_string())
}

fn publish(source: &Path, destination: &Path) -> Result<(), String> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(source.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    let destination =
        CString::new(destination.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: both C strings live through the syscall. Exclusive rename is atomic and refuses replacement.
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let result = -1;
    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "The session was not published: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_me::connection::{Source, Support};

    fn setup() -> (tempfile::TempDir, Pending) {
        let temp = tempfile::Builder::new()
            .prefix("tinkery-TEST-tend-save-")
            .tempdir()
            .unwrap();
        let root = temp.path().join("sessions");
        fs::create_dir(&root).unwrap();
        let repo = temp.path().canonicalize().unwrap();
        let sources = ["First exact goal", "Second exact goal"]
            .into_iter()
            .enumerate()
            .map(|(i, goal)| Source {
                id: format!("source-{i}"),
                goal: goal.into(),
                repo: repo.clone(),
                status: "active".into(),
            })
            .collect::<Vec<_>>();
        let origin = Origin {
            schema: 1,
            entry: "tend".into(),
            repo,
            goal: "TEST exact confirmed goal\nWith a second line.".into(),
            support: sources
                .iter()
                .map(|s| Support {
                    id: s.id.clone(),
                    quote: s.goal.clone(),
                })
                .collect(),
            sources,
            interpretation: "TEST connection, not a fact.".into(),
            correction: None,
        };
        let pending = Pending::prepare(
            &root,
            origin,
            "TEST inspectable outcome".into(),
            GoalConfirmation::default(),
        )
        .unwrap();
        (temp, pending)
    }

    #[test]
    fn correction_origin_survives_staging_restart_and_history() {
        let (temp, mut pending) = setup();
        pending.origin.schema = 2;
        pending.origin.correction = Some(super::super::connection::Correction {
            revision: "a".repeat(48),
            text: "Keep exact operator words.\nIncluding café.".into(),
        });
        let frozen = serde_json::to_vec(&pending).unwrap();
        let stage = pending
            .root
            .parent()
            .unwrap()
            .join(".tinkery-tend-staging")
            .join(&pending.id);
        fs::create_dir_all(&stage).unwrap();
        fs::write(stage.join("tend-handoff.json"), &frozen).unwrap();
        fs::write(stage.join("ledger.json"), "interrupted").unwrap();
        let restarted: Pending = serde_json::from_slice(&frozen).unwrap();
        let receipt = restarted.resume(|| Ok(())).unwrap();
        assert_eq!(
            fs::read(receipt.session.join("tend-handoff.json")).unwrap(),
            frozen
        );
        assert_eq!(
            super::super::connection::read(&receipt.session, &receipt.goal).unwrap(),
            Some(pending.origin.clone())
        );
        let records = temp.path().join("records");
        fs::create_dir(&records).unwrap();
        let history = crate::think_record::read(&records, &receipt.session, &receipt.goal).unwrap();
        assert!(
            history
                .sequence
                .contains("Operator guidance supplied during discovery:")
        );
        assert!(
            history
                .sequence
                .contains(&pending.origin.correction.as_ref().unwrap().text)
        );
        assert_eq!(
            restarted
                .resume(|| Err("Changed current correction".into()))
                .unwrap()
                .session,
            receipt.session
        );
        let mut conflicting = restarted.clone();
        conflicting.origin.correction.as_mut().unwrap().text = "Different words".into();
        assert!(conflicting.inspect().is_err());
        assert!(conflicting.resume(|| Ok(())).is_err());
        assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 1);
    }

    #[test]
    fn versioned_origin_refuses_missing_null_unknown_and_conflicting_evidence() {
        for case in [
            "missing",
            "null",
            "extra",
            "wrong-schema",
            "wrong-tag",
            "duplicate",
            "invalid-correction",
        ] {
            let (_temp, mut pending) = setup();
            pending.origin.schema = 2;
            pending.origin.correction = Some(super::super::connection::Correction {
                revision: "b".repeat(48),
                text: "Exact correction".into(),
            });
            let receipt = pending.resume(|| Ok(())).unwrap();
            let path = receipt.session.join("ledger.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let entries = value["nodes"][0]["evidence"].as_array_mut().unwrap();
            let index = entries
                .iter()
                .position(|e| e["artifact"] == "tinkery:origin/tend-v2")
                .unwrap();
            let mut origin: serde_json::Value =
                serde_json::from_str(entries[index]["observed"].as_str().unwrap()).unwrap();
            match case {
                "missing" => {
                    origin.as_object_mut().unwrap().remove("correction");
                }
                "null" => origin["correction"] = serde_json::Value::Null,
                "extra" => origin["other"] = true.into(),
                "wrong-schema" => origin["schema"] = 1.into(),
                "wrong-tag" => {
                    entries[index]["artifact"] = super::super::connection::ARTIFACT.into()
                }
                "duplicate" => entries.push(entries[index].clone()),
                "invalid-correction" => origin["correction"]["revision"] = "bad".into(),
                _ => unreachable!(),
            }
            entries[index]["observed"] = serde_json::to_string(&origin).unwrap().into();
            fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(
                super::super::connection::read(&receipt.session, &receipt.goal).is_err(),
                "{case}"
            );
            assert!(pending.inspect().is_err(), "{case}");
        }
    }

    #[test]
    fn legacy_receipt_keeps_its_wire_fields_and_refuses_added_correction() {
        let (_temp, pending) = setup();
        let bytes = serde_json::to_vec(&pending).unwrap();
        let restarted: Pending = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&restarted).unwrap(), bytes);
        let mut wire = serde_json::to_value(&pending).unwrap();
        assert!(wire["origin"].get("correction").is_none());
        wire["origin"]["correction"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Pending>(wire).is_err());
        let receipt = restarted.resume(|| Ok(())).unwrap();
        assert_eq!(
            fs::read(receipt.session.join("tend-handoff.json")).unwrap(),
            bytes
        );
    }

    #[test]
    fn published_goal_and_receipt_remain_private() {
        use std::os::unix::fs::PermissionsExt;
        let (_temp, pending) = setup();
        let receipt = pending.resume(|| Ok(())).unwrap();
        assert_eq!(
            fs::metadata(&receipt.session).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(receipt.session.join("tend-handoff.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn restart_reuses_published_goal_before_rechecking_changed_membership() {
        let (_temp, pending) = setup();
        assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 0);
        assert!(pending.inspect().unwrap().is_none());
        let receipt = pending.resume(|| Ok(())).unwrap();
        let restarted: Pending =
            serde_json::from_slice(&serde_json::to_vec(&pending).unwrap()).unwrap();
        let resumed = restarted
            .resume(|| Err("membership now includes saved goal".into()))
            .unwrap();
        assert_eq!(receipt.session, resumed.session);
        assert_eq!(receipt.goal, pending.origin.goal);
        assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 1);
        assert!(restarted.cancel().is_err());
    }

    #[test]
    fn published_resume_retries_both_parent_syncs_before_reporting_success() {
        use std::os::unix::fs::PermissionsExt;
        // Root bypasses this OS permission refusal. The ordinary local and CI users exercise it.
        if unsafe { libc::geteuid() } == 0 {
            eprintln!("Directory-open refusal requires an unprivileged test process.");
            return;
        }
        for source_parent in [true, false] {
            let (_temp, pending) = setup();
            let receipt = pending.resume(|| Ok(())).unwrap();
            let parent = if source_parent {
                pending.root.parent().unwrap().join(".tinkery-tend-staging")
            } else {
                pending.root.clone()
            };
            let original = fs::metadata(&parent).unwrap().permissions();
            // Traversal still permits inspecting the named published goal and opening its lock.
            // Opening the parent directory for fsync must fail without read permission.
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o111)).unwrap();
            let inspected = pending.inspect();
            let refusal = pending.resume(|| panic!("published goals bypass changed membership"));
            fs::set_permissions(&parent, original).unwrap();
            assert_eq!(inspected.unwrap().unwrap().session, receipt.session);
            assert!(
                refusal.is_err(),
                "resume reported success without syncing {parent:?}"
            );
            let retried = pending
                .resume(|| panic!("published goals bypass changed membership"))
                .unwrap();
            assert_eq!(retried.session, receipt.session);
            assert_eq!(retried.goal, pending.origin.goal);
            assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 1);
        }
    }

    #[test]
    fn restart_rebuilds_owned_partial_staging_and_refuses_unowned_content() {
        let (_temp, pending) = setup();
        let staging = pending
            .root
            .parent()
            .unwrap()
            .join(".tinkery-tend-staging")
            .join(&pending.id);
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("ledger.json"), "partial").unwrap();
        assert!(pending.resume(|| Ok(())).is_err());
        assert!(pending.cancel().is_err());
        fs::write(
            staging.join("tend-handoff.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        let result = pending.resume(|| Ok(())).unwrap();
        assert_eq!(result.goal, pending.origin.goal);
        assert!(!staging.exists());
    }

    #[test]
    fn changed_source_before_publication_keeps_receipt_and_can_cancel_owned_staging() {
        let (_temp, pending) = setup();
        let calls = std::cell::Cell::new(0);
        let result = pending.resume(|| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Ok(())
            } else {
                Err("changed input".into())
            }
        });
        assert!(result.is_err());
        assert!(pending.inspect().unwrap().is_none());
        pending.cancel().unwrap();
        assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 0);
    }

    #[test]
    fn concurrent_resume_refuses_without_creating_another_goal() {
        let (_temp, pending) = setup();
        let attempts = std::cell::Cell::new(0);
        pending
            .resume(|| {
                attempts.set(attempts.get() + 1);
                assert!(
                    pending
                        .resume(|| Ok(()))
                        .err()
                        .unwrap()
                        .contains("Another writer")
                );
                Ok(())
            })
            .unwrap();
        assert_eq!(attempts.get(), 2);
        assert_eq!(fs::read_dir(&pending.root).unwrap().count(), 1);
    }

    #[test]
    fn published_receipt_refuses_changed_outcome_or_confirmation() {
        for field in ["outcome", "authority"] {
            let (_temp, pending) = setup();
            let receipt = pending.resume(|| Ok(())).unwrap();
            let path = receipt.session.join("ledger.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            if field == "outcome" {
                value["draft"]["outcome"] = "TEST different outcome".into();
            } else {
                value["nodes"][0]["authority_source"] = "TEST different confirmation".into();
            }
            fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            let before = fs::read(&path).unwrap();
            assert!(pending.inspect().is_err());
            assert!(pending.resume(|| Ok(())).is_err());
            assert!(pending.cancel().is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn conflicting_destination_is_never_replaced_or_cancelled() {
        let (_temp, pending) = setup();
        let destination = pending.root.join(&pending.id);
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("keep"), "other operation").unwrap();
        assert!(pending.resume(|| Ok(())).is_err());
        assert!(pending.cancel().is_err());
        assert_eq!(
            fs::read_to_string(destination.join("keep")).unwrap(),
            "other operation"
        );
    }

    #[test]
    fn symlinked_staging_parent_refuses_resume_inspection_and_cancel() {
        let (temp, pending) = setup();
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(
            &outside,
            pending.root.parent().unwrap().join(".tinkery-tend-staging"),
        )
        .unwrap();
        assert!(pending.inspect().is_err());
        assert!(pending.resume(|| Ok(())).is_err());
        assert!(pending.cancel().is_err());
        assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    }

    #[test]
    fn tend_origin_has_honest_history_and_invalid_evidence_is_distinct() {
        let (temp, pending) = setup();
        let records = temp.path().join("records");
        fs::create_dir(&records).unwrap();
        let receipt = pending.resume(|| Ok(())).unwrap();
        let origins = crate::think_record::read_origins(
            &records,
            &[(receipt.session.as_path(), receipt.goal.as_str())],
        )
        .unwrap();
        assert_eq!(
            origins,
            vec![crate::think_record::MadeIn::Repo(
                pending.origin.repo.clone()
            )]
        );
        let history = crate::think_record::read(&records, &receipt.session, &receipt.goal).unwrap();
        assert!(history.turns.is_empty());
        assert_eq!(history.sequence, "Created from a Tend connection");
        let path = receipt.session.join("ledger.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["nodes"][0]["evidence"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["artifact"] == super::super::connection::ARTIFACT)
            .unwrap()["observed"] = "malformed".into();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        let origins = crate::think_record::read_origins(
            &records,
            &[(receipt.session.as_path(), receipt.goal.as_str())],
        )
        .unwrap();
        assert!(matches!(
            &origins[0],
            crate::think_record::MadeIn::Invalid(_)
        ));
        assert!(pending.inspect().is_err());
        let budget = crate::pi::CallBudget::new(1);
        let agent = crate::shape::Agent::new(
            crate::pi::Harness::Pi,
            temp.path().join("must-not-launch"),
            "TEST/model".into(),
            String::new(),
            Some(temp.path().to_path_buf()),
            budget.clone(),
            Some(records),
        )
        .unwrap();
        let result = crate::shape::command::step(
            &receipt.session,
            Some(&agent),
            &GoalConfirmation::default(),
            crate::shape::command::Action::Look,
        );
        assert!(result.is_err());
        assert_eq!(budget.remaining(), 1);
        value["nodes"][0]["evidence"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["artifact"] != super::super::connection::ARTIFACT);
        fs::write(
            receipt.session.join("ledger.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert!(super::super::connection::read(&receipt.session, &receipt.goal).is_err());
        assert!(
            crate::shape::command::step(
                &receipt.session,
                Some(&agent),
                &GoalConfirmation::default(),
                crate::shape::command::Action::Look
            )
            .is_err()
        );
        assert_eq!(budget.remaining(), 1);
    }
}
