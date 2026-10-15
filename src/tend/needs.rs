//! Tinkery-owned needs links: strict reads, explicit edits and derived readiness.
//! Never edits Seed Me sessions. Unreadable stores refuse mutation; readiness uses only
//! supplied sessions, including ended ones, and never infers identity from names or paths.

use crate::seed_me::{Session, Stage};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// The links folder under an absolute HOME. Creates nothing; refuses an unsafe default.
pub fn default_root(home: Option<&OsStr>) -> Result<PathBuf, String> {
    match home.map(Path::new) {
        Some(home) if home.is_absolute() => Ok(home.join(".local/share/tinkery/tend")),
        _ => Err(
            "HOME is unset or not absolute, so linking is unavailable; pass --link-root PATH."
                .into(),
        ),
    }
}

/// Capture an explicit links folder against the launch directory, or use the checked HOME.
/// Empty explicit roots are refused; this never reads or creates a links file.
pub fn resolve_root(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    match explicit {
        Some(root) if root.as_os_str().is_empty() => Err("--link-root needs a folder".into()),
        Some(root) if root.is_absolute() => Ok(root),
        Some(root) => std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .map_err(|error| format!("Cannot resolve --link-root PATH: {error}")),
        None => default_root(std::env::var_os("HOME").as_deref()),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Link {
    goal: String,
    needs: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u64,
    needs: Vec<Link>,
}

/// Validated, sorted links. An empty set means no needs, not ready.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Links {
    entries: BTreeSet<Link>,
}

/// One goal's derived needs. No state here is persisted.
#[derive(Debug, PartialEq, Eq)]
pub struct Summary {
    /// First goal lines of uniquely found needs, sorted without rewriting their words.
    pub titles: Vec<String>,
    /// Needs without a uniquely found session id; never guessed from a title or folder.
    pub missing: usize,
    /// At least one need, with every need uniquely found and ended. No needs is false.
    pub ready: bool,
}

impl Links {
    pub(super) fn pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|link| (link.goal.as_str(), link.needs.as_str()))
    }

    /// Whether this exact directed need exists; never infers identity from session titles.
    pub fn contains(&self, goal: &str, needs: &str) -> bool {
        self.entries.contains(&Link {
            goal: goal.into(),
            needs: needs.into(),
        })
    }

    /// Resolve one goal's needs against all readable sessions, including ended ones.
    /// Unknown stages, missing ids and ambiguous duplicate ids can never establish readiness.
    pub fn summary(&self, goal: &str, sessions: &[Session]) -> Summary {
        let mut summary = Summary {
            titles: vec![],
            missing: 0,
            ready: false,
        };
        let mut count = 0;
        let mut all_ended = true;
        for link in self.entries.iter().filter(|link| link.goal == goal) {
            count += 1;
            let mut found = sessions
                .iter()
                .filter(|s| s.id.as_deref() == Some(&link.needs));
            match (found.next(), found.next()) {
                (Some(session), None) => {
                    summary
                        .titles
                        .push(session.goal.lines().next().unwrap_or("").into());
                    all_ended &= session.stage == Stage::Ended;
                }
                _ => {
                    summary.missing += 1;
                    all_ended = false;
                }
            }
        }
        summary.titles.sort();
        summary.ready = count > 0 && all_ended;
        summary
    }
}

/// A missing file is normal and empty; an unreadable file must never be overwritten.
#[derive(Debug, PartialEq, Eq)]
pub enum Read {
    /// No needs.json exists yet. Reading creates nothing.
    Missing,
    /// The complete, validated links file.
    Readable(Links),
    /// The file or configured destination cannot be read safely, with a plain reason.
    Unreadable(String),
}

impl Read {
    /// Treat a missing file as empty, but preserve every unreadable-file refusal.
    pub fn into_links(self) -> Result<Links, String> {
        match self {
            Self::Missing => Ok(Links::default()),
            Self::Readable(links) => Ok(links),
            Self::Unreadable(reason) => Err(reason),
        }
    }
}

/// The configured needs.json destination, or why linking is unavailable. Construction does no IO.
/// Edits serialize across cooperating writers; a missing previously populated store is refused.
pub struct Store {
    root: Result<PathBuf, String>,
    populated: Cell<bool>,
}

impl Store {
    /// Retain a captured absolute root or a default-root refusal. Never creates a directory.
    pub fn new(root: Result<PathBuf, String>) -> Self {
        let root = root.and_then(|root| {
            if root.is_absolute() {
                Ok(root)
            } else {
                Err("Links need an absolute folder; pass --link-root PATH.".into())
            }
        });
        Self {
            root,
            populated: Cell::new(false),
        }
    }

    /// Read strictly: bad JSON, schema, version, entries or IO yield a refusal, never empty links.
    pub fn read(&self) -> Read {
        let root = match &self.root {
            Ok(root) => root,
            Err(reason) => return Read::Unreadable(reason.clone()),
        };
        let bytes = match fs::read(root.join("needs.json")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return if self.populated.get() {
                    Read::Unreadable(
                        "Cannot read links: previously loaded needs.json is missing.".into(),
                    )
                } else {
                    Read::Missing
                };
            }
            Err(error) => return Read::Unreadable(format!("Cannot read needs.json: {error}")),
        };
        if serde_json::from_slice::<serde_json::Value>(&bytes).is_err() {
            return Read::Unreadable("Cannot read links: needs.json is not valid JSON.".into());
        }
        let file: File = match serde_json::from_slice(&bytes) {
            Ok(file) => file,
            Err(_) => {
                return Read::Unreadable(
                    "Cannot read links: needs.json has invalid entries or fields.".into(),
                );
            }
        };
        if file.version != 1 {
            return Read::Unreadable(
                "Cannot read links: needs.json has an unsupported version.".into(),
            );
        }
        let mut entries = BTreeSet::new();
        for link in file.needs {
            if !valid_id(&link.goal) || !valid_id(&link.needs) || link.goal == link.needs {
                return Read::Unreadable(
                    "Cannot read links: needs.json has an invalid needs link.".into(),
                );
            }
            if !entries.insert(link) {
                return Read::Unreadable(
                    "Cannot read links: needs.json has a duplicate needs link.".into(),
                );
            }
        }
        self.populated.set(!entries.is_empty());
        Read::Readable(Links { entries })
    }

    /// Set the requested intent (`linked` adds); return links and whether they changed.
    /// Matching files stay intact. Refuses invalid ids, self-links and unreadable stores unchanged,
    /// including a missing previously populated file. Waits at most two seconds for a sibling lock
    /// across re-reading and atomic replacement. Refuses changes to files we cannot write.
    /// Returns the resulting links; IO failures are explicit and no partial file is published.
    pub fn set(&self, goal: &str, needs: &str, linked: bool) -> Result<(Links, bool), String> {
        if !valid_id(goal) || !valid_id(needs) {
            return Err("Cannot link goals without nonempty, control-free session ids.".into());
        }
        if goal == needs {
            return Err("A goal cannot need itself.".into());
        }
        self.read().into_links()?;
        let root = self.root.as_ref().map_err(Clone::clone)?;
        let lock = (|| -> std::io::Result<std::fs::File> {
            fs::create_dir_all(root)?;
            let lock = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(root.join("needs.lock"))?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match lock.try_lock() {
                    Ok(()) => break,
                    Err(std::fs::TryLockError::WouldBlock) => {
                        if std::time::Instant::now() >= deadline {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "another Tinkery is saving links",
                            ));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(std::fs::TryLockError::Error(error)) => return Err(error),
                }
            }
            Ok(lock)
        })()
        .map_err(|error| format!("Cannot save needs.json: {error}"))?;
        let mut links = self.read().into_links()?;
        let link = Link {
            goal: goal.into(),
            needs: needs.into(),
        };
        if links.contains(goal, needs) == linked {
            return Ok((links, false));
        }
        if linked {
            links.entries.insert(link);
        } else {
            links.entries.remove(&link);
        }
        let file = File {
            version: 1,
            needs: links.entries.iter().cloned().collect(),
        };
        let write = || -> Result<(), String> {
            let bytes = serde_json::to_vec(&file).map_err(|error| error.to_string())?;
            match fs::metadata(root.join("needs.json")) {
                Ok(metadata) => {
                    if metadata.permissions().readonly() {
                        return Err("The existing links file is read-only.".into());
                    }
                    // Opening without truncation checks this process's permissions, including ACLs.
                    fs::OpenOptions::new()
                        .write(true)
                        .open(root.join("needs.json"))
                        .map_err(|error| error.to_string())?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
            let mut temp =
                tempfile::NamedTempFile::new_in(root).map_err(|error| error.to_string())?;
            temp.write_all(&bytes).map_err(|error| error.to_string())?;
            temp.as_file()
                .sync_all()
                .map_err(|error| error.to_string())?;
            temp.persist(root.join("needs.json"))
                .map_err(|error| error.to_string())?;
            Ok(())
        };
        write().map_err(|error| format!("Cannot save needs.json: {error}"))?;
        self.populated.set(!links.entries.is_empty());
        drop(lock);
        Ok((links, true))
    }

    /// Test fixture convenience; production callers must supply the displayed intent.
    #[cfg(test)]
    pub(crate) fn toggle(&self, goal: &str, needs: &str) -> Result<Links, String> {
        let linked = !self.read().into_links()?.contains(goal, needs);
        self.set(goal, needs, linked).map(|(links, _)| links)
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && !id.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, Store, PathBuf) {
        let tmp = tempfile::Builder::new()
            .prefix("tinkery-TEST-needs-")
            .tempdir()
            .unwrap();
        let root = tmp.path().join("TEST-links");
        let store = Store::new(Ok(root.clone()));
        (tmp, store, root)
    }

    fn session(id: Option<&str>, goal: &str, stage: Stage) -> Session {
        Session {
            id: id.map(str::to_owned),
            path: PathBuf::from("/TEST/not-an-id"),
            goal: goal.into(),
            stage,
            updated: String::new(),
        }
    }

    #[test]
    fn missing_is_empty_without_creating_anything() {
        let (_tmp, store, root) = fixture();
        assert_eq!(store.read(), Read::Missing);
        assert_eq!(store.read().into_links().unwrap(), Links::default());
        assert!(!root.exists());
    }

    #[test]
    fn add_remove_add_round_trip_is_repeatable_and_leaves_no_temp_files() {
        let (_tmp, store, root) = fixture();
        let added = store.toggle("goal", "need").unwrap();
        assert_eq!(store.read(), Read::Readable(added.clone()));
        let bytes = fs::read(root.join("needs.json")).unwrap();
        assert_eq!(
            bytes,
            br#"{"version":1,"needs":[{"goal":"goal","needs":"need"}]}"#
        );
        assert_eq!(store.toggle("goal", "need").unwrap(), Links::default());
        assert_eq!(store.toggle("goal", "need").unwrap(), added);
        assert_eq!(fs::read(root.join("needs.json")).unwrap(), bytes);
        let mut names: Vec<_> = fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, ["needs.json", "needs.lock"]);
    }

    #[test]
    fn concurrent_writers_preserve_every_successful_link() {
        use std::sync::{Arc, Barrier};
        for round in 0..32 {
            let (_tmp, store, root) = fixture();
            let barrier = Arc::new(Barrier::new(8));
            std::thread::scope(|scope| {
                for writer in 0..8 {
                    let root = root.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        let store = Store::new(Ok(root));
                        barrier.wait();
                        store.toggle("goal", &format!("need-{writer}")).unwrap();
                    });
                }
            });
            let links = store.read().into_links().unwrap();
            assert_eq!(
                links.entries.len(),
                8,
                "round {round}: lost a successful save"
            );
        }
    }

    #[test]
    fn bytes_are_sorted_independent_of_toggle_order() {
        let (_a, first, a) = fixture();
        let (_b, second, b) = fixture();
        for (goal, need) in [("z", "b"), ("a", "z"), ("a", "b")] {
            first.toggle(goal, need).unwrap();
        }
        for (goal, need) in [("a", "b"), ("z", "b"), ("a", "z")] {
            second.toggle(goal, need).unwrap();
        }
        assert_eq!(
            fs::read(a.join("needs.json")).unwrap(),
            fs::read(b.join("needs.json")).unwrap()
        );
    }

    #[test]
    fn self_and_invalid_ids_refuse_without_creating_a_store() {
        let (_tmp, store, root) = fixture();
        assert_eq!(
            store.toggle("a", "a").unwrap_err(),
            "A goal cannot need itself."
        );
        for bad in ["", "line\nbreak", "null\0", "delete\u{7f}", "next\u{85}"] {
            for (goal, needs) in [(bad, "valid"), ("valid", bad)] {
                assert_eq!(
                    store.toggle(goal, needs).unwrap_err(),
                    "Cannot link goals without nonempty, control-free session ids."
                );
            }
        }
        assert!(!root.exists());
    }

    #[test]
    fn unreadable_files_refuse_toggle_and_keep_exact_bytes() {
        let (_tmp, store, root) = fixture();
        fs::create_dir_all(&root).unwrap();
        for (bytes, reason) in [
            ("not JSON", "not valid JSON"),
            (r#"{"version":2,"needs":[]}"#, "unsupported version"),
            (r#"{"version":1,"needs":[{}]}"#, "invalid entries or fields"),
            (
                r#"{"version":1,"needs":[{"goal":4,"needs":"b"}]}"#,
                "invalid entries or fields",
            ),
            (
                r#"{"version":1,"needs":[{"goal":"","needs":"b"}]}"#,
                "invalid needs link",
            ),
            (
                r#"{"version":1,"needs":[{"goal":"a","needs":"b\n"}]}"#,
                "invalid needs link",
            ),
            (
                r#"{"version":1,"needs":[{"goal":"a","needs":"a"}]}"#,
                "invalid needs link",
            ),
            (
                r#"{"version":1,"needs":[{"goal":"a","needs":"b"},{"goal":"a","needs":"b"}]}"#,
                "duplicate needs link",
            ),
            (
                r#"{"version":1,"needs":[],"future":true}"#,
                "invalid entries or fields",
            ),
            (
                r#"{"version":1,"version":1,"needs":[]}"#,
                "invalid entries or fields",
            ),
            (
                r#"{"version":1,"needs":[{"goal":"a","goal":"c","needs":"b"}]}"#,
                "invalid entries or fields",
            ),
        ] {
            fs::write(root.join("needs.json"), bytes).unwrap();
            let Read::Unreadable(error) = store.read() else {
                panic!("{bytes}")
            };
            assert!(error.contains(reason), "{error}");
            assert_eq!(store.toggle("a", "b").unwrap_err(), error);
            assert_eq!(fs::read(root.join("needs.json")).unwrap(), bytes.as_bytes());
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        }
    }

    #[test]
    fn io_errors_are_unreadable_and_never_replaced() {
        let (_tmp, store, root) = fixture();
        let file = root.join("needs.json");
        fs::create_dir_all(&file).unwrap();
        fs::write(file.join("TEST-sentinel"), "unchanged").unwrap();
        let Read::Unreadable(error) = store.read() else {
            panic!()
        };
        assert!(error.starts_with("Cannot read needs.json:"));
        assert_eq!(store.toggle("a", "b").unwrap_err(), error);
        assert_eq!(
            fs::read_to_string(file.join("TEST-sentinel")).unwrap(),
            "unchanged"
        );
    }

    #[test]
    fn changes_since_a_previous_read_survive_a_toggle() {
        let (_tmp, store, root) = fixture();
        assert_eq!(store.read(), Read::Missing);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("needs.json"),
            json!({
                "version":1, "needs":[{"goal":"other","needs":"existing"}]
            })
            .to_string(),
        )
        .unwrap();
        let links = store.toggle("goal", "need").unwrap();
        assert_eq!(links.entries.len(), 2);
        assert!(links.entries.contains(&Link {
            goal: "other".into(),
            needs: "existing".into()
        }));
        assert_eq!(store.read(), Read::Readable(links));
    }

    #[test]
    fn readiness_needs_at_least_one_found_ended_session() {
        let (_tmp, store, _root) = fixture();
        let mut sessions = vec![session(
            Some("need"),
            "Ended title\nWhy: original",
            Stage::Ended,
        )];
        assert_eq!(
            Links::default().summary("goal", &sessions),
            Summary {
                titles: vec![],
                missing: 0,
                ready: false,
            }
        );
        let links = store.toggle("goal", "need").unwrap();
        assert_eq!(
            links.summary("goal", &sessions),
            Summary {
                titles: vec!["Ended title".into()],
                missing: 0,
                ready: true,
            }
        );
        for stage in [
            Stage::GoalSaved,
            Stage::SeedDrafted,
            Stage::SeedReady,
            Stage::Unknown,
            Stage::Simulated,
        ] {
            sessions[0].stage = stage;
            assert!(!links.summary("goal", &sessions).ready);
        }
        sessions[0].stage = Stage::Ended;
        let links = store.toggle("goal", "unknown-id").unwrap();
        let unknown = session(Some("another-id"), "unknown-id", Stage::Ended);
        let no_id = session(None, "unknown-id", Stage::Ended);
        sessions.extend([unknown, no_id]);
        assert_eq!(
            links.summary("goal", &sessions),
            Summary {
                titles: vec!["Ended title".into()],
                missing: 1,
                ready: false,
            }
        );
        assert_eq!(
            links.summary("other-goal", &sessions).titles,
            Vec::<String>::new()
        );
        assert_eq!(links.summary("goal", &[]).missing, 2);
    }

    #[test]
    fn duplicate_session_ids_are_not_guessed_and_titles_stay_verbatim() {
        let (_tmp, store, _root) = fixture();
        let links = store.toggle("goal", "need").unwrap();
        let session = session(Some("need"), "  界 title  \nWhy: intact", Stage::Ended);
        assert_eq!(
            links.summary("goal", std::slice::from_ref(&session)).titles,
            ["  界 title  "]
        );
        assert_eq!(
            links.summary("goal", &[session.clone(), session]),
            Summary {
                titles: vec![],
                missing: 1,
                ready: false,
            }
        );
    }

    #[test]
    fn roots_are_checked_without_creating_folders() {
        let (tmp, _, _) = fixture();
        assert_eq!(
            default_root(Some(tmp.path().as_os_str())).unwrap(),
            tmp.path().join(".local/share/tinkery/tend")
        );
        assert!(!tmp.path().join(".local").exists());
        for home in [None, Some(""), Some("relative"), Some(".")] {
            let reason = default_root(home.map(OsStr::new)).unwrap_err();
            assert!(reason.contains("linking is unavailable; pass --link-root PATH"));
            let store = Store::new(Err(reason.clone()));
            assert_eq!(store.read(), Read::Unreadable(reason.clone()));
            assert_eq!(store.toggle("a", "b").unwrap_err(), reason);
        }
        assert_eq!(
            resolve_root(Some(PathBuf::new())).unwrap_err(),
            "--link-root needs a folder"
        );
        assert_eq!(
            resolve_root(Some("TEST-relative-links".into())).unwrap(),
            std::env::current_dir().unwrap().join("TEST-relative-links")
        );
        assert_eq!(
            Store::new(Ok("relative".into()))
                .toggle("a", "b")
                .unwrap_err(),
            "Links need an absolute folder; pass --link-root PATH."
        );
    }
}
