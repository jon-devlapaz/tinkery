//! Session discovery for the menu: one owned, read-only worker that never runs in paint.
//!
//! The menu and tend read only what this has already landed. Four states are kept apart:
//! not known (nothing landed, or a newer request is still running), complete (every
//! record was read), incomplete (some records could not be read) and failed (the root
//! could not be read at all). A missing root is complete and empty: nothing exists yet.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    time::Instant,
};

use super::{self as tend, Session, Stage};
use crate::seed_me::{self, Record};

/// What one scan found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Found {
    /// Every readable session, newest first.
    pub sessions: Vec<Session>,
    /// Records that exist but could not be read; they are never guessed or counted.
    pub unreadable: usize,
    /// At most `seed_me::MAX_READ_FAILURES` path/reason samples; the full count is `unreadable`.
    pub failures: Vec<seed_me::ReadFailure>,
}
impl Found {
    /// Every record was read, so the list is the whole truth.
    pub fn complete(&self) -> bool {
        self.unreadable == 0
    }

    fn failed(&mut self, failure: seed_me::ReadFailure) {
        self.unreadable += 1;
        if self.failures.len() < seed_me::MAX_READ_FAILURES {
            self.failures.push(failure);
        }
    }
}

/// `Err` means the root itself could not be read.
pub type Outcome = Result<Found, String>;
/// The blocking read. Injectable so tests can hold it open.
pub type Source = Arc<dyn Fn(&Path) -> Outcome + Send + Sync>;

/// Seed Me sessions, newest first. Each folder is read by [`seed_me::read`].
pub fn scan(root: &Path) -> Outcome {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Found::default());
        }
        Err(error) => return Err(format!("Cannot read {}: {error}", root.display())),
    };
    let mut found = Found::default();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                found.failed(seed_me::ReadFailure::new(
                    root.to_path_buf(),
                    format!("Cannot read a directory entry: {error}"),
                ));
                continue;
            }
        };
        match seed_me::read(&entry.path()) {
            Record::Session(session) => found.sessions.push(session),
            Record::NotASession => {}
            Record::Unreadable(failure) => found.failed(failure),
        }
    }
    found.sessions.sort_by(|a, b| {
        tend::timestamp(&b.updated)
            .cmp(&tend::timestamp(&a.updated))
            .then_with(|| b.updated.cmp(&a.updated))
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(found)
}

/// The last result that landed, with how many requests had been made when it started.
pub struct Landed {
    /// What the read found, or why the root couldn't be read.
    pub outcome: Outcome,
    /// Exact attributed session paths for the launch repo; absent outside a repo.
    pub(crate) repo_paths: Option<HashSet<PathBuf>>,
    /// Active goals whose record cannot establish a repository or an explicit no-repo origin.
    pub(crate) repo_unknown: usize,
    epoch: u64,
    active: usize,
}

struct Scanned {
    outcome: Outcome,
    repo_paths: Option<HashSet<PathBuf>>,
    repo_unknown: usize,
}

/// One owned worker at a time. Requests made while it runs coalesce into one follow-up.
pub struct Discovery {
    source: Source,
    root: PathBuf,
    requested: u64,
    running: Option<(u64, PathBuf, Receiver<Scanned>, Instant)>,
    scope: Option<(PathBuf, Result<PathBuf, String>)>,
    landed: Option<Landed>,
    version: u64,
}

impl Discovery {
    /// No read yet; nothing runs until the first [`Discovery::request`].
    pub fn new(source: Source) -> Self {
        Self {
            source,
            root: PathBuf::new(),
            requested: 0,
            running: None,
            scope: None,
            landed: None,
            version: 0,
        }
    }

    /// A configured refusal, already available to the views; starts no worker or filesystem read.
    pub fn unavailable(reason: String) -> Self {
        let mut discovery = Self::new(Arc::new(scan));
        discovery.requested = 1;
        discovery.version = 1;
        discovery.landed = Some(Landed {
            outcome: Err(reason),
            repo_paths: None,
            repo_unknown: 0,
            epoch: 1,
            active: 0,
        });
        discovery
    }

    /// Attribute the next read using existing Think records only; configuration reads nothing.
    pub(crate) fn scope_to_repo(
        &mut self,
        repo: Option<PathBuf>,
        records: Result<PathBuf, String>,
    ) {
        self.scope = repo.map(|repo| (repo, records));
    }

    /// Ask for a fresh read of `root`. Never blocks; at most one worker exists.
    pub fn request(&mut self, root: &Path) {
        self.requested += 1;
        if self.root != root {
            // A read of another place says nothing about this one.
            self.landed = None;
            self.root = root.to_path_buf();
        }
        self.start_if_idle();
    }

    /// Whether any read has been asked for.
    pub fn requested(&self) -> bool {
        self.requested > 0
    }

    /// Land a finished worker, if any. True when something new can be shown.
    pub fn poll(&mut self) -> bool {
        let Some((epoch, root, rx, _)) = &self.running else {
            return false;
        };
        let (epoch, stale_root) = (*epoch, *root != self.root);
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Scanned {
                outcome: Err("Reading sessions stopped unexpectedly.".into()),
                repo_paths: None,
                repo_unknown: 0,
            },
        };
        let Scanned {
            outcome,
            repo_paths,
            repo_unknown,
        } = outcome;
        self.running = None;
        if stale_root {
            self.start_if_idle();
            return false;
        }
        let active = outcome.as_ref().map_or(0, |found| {
            found
                .sessions
                .iter()
                .filter(|s| s.stage != Stage::Ended)
                .count()
        });
        self.landed = Some(Landed {
            outcome,
            repo_paths,
            repo_unknown,
            epoch,
            active,
        });
        self.version += 1;
        self.start_if_idle();
        true
    }

    fn start_if_idle(&mut self) {
        let behind = self
            .landed
            .as_ref()
            .is_none_or(|l| l.epoch < self.requested);
        if self.running.is_some() || self.requested == 0 || !behind {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let (source, root) = (self.source.clone(), self.root.clone());
        let scope = self.scope.clone();
        let spawned = std::thread::Builder::new()
            .name("tinkery-discovery".into())
            .spawn(move || {
                let outcome = source(&root);
                let mut repo_unknown = 0;
                let repo_paths = scope.map(|(repo, records)| {
                    let sessions = outcome
                        .as_ref()
                        .map_or(&[][..], |found| found.sessions.as_slice());
                    let origins = records
                        .as_ref()
                        .ok()
                        .and_then(|root| {
                            if sessions.is_empty() {
                                return Some(Vec::new());
                            }
                            let goals: Vec<_> = sessions
                                .iter()
                                .map(|session| (session.path.as_path(), session.goal.as_str()))
                                .collect();
                            crate::think_record::read_origins(root, &goals).ok()
                        })
                        .unwrap_or_else(|| {
                            vec![crate::think_record::MadeIn::Unknown; sessions.len()]
                        });
                    sessions
                        .iter()
                        .zip(origins)
                        .filter_map(|(session, made_in)| {
                            if made_in == crate::think_record::MadeIn::Unknown
                                && session.stage != Stage::Ended
                            {
                                repo_unknown += 1;
                            }
                            (made_in == crate::think_record::MadeIn::Repo(repo.clone()))
                                .then(|| session.path.clone())
                        })
                        .collect()
                });
                let _ = tx.send(Scanned {
                    outcome,
                    repo_paths,
                    repo_unknown,
                });
            });
        match spawned {
            Ok(_) => self.running = Some((self.requested, self.root.clone(), rx, Instant::now())),
            Err(error) => {
                self.landed = Some(Landed {
                    outcome: Err(format!("Cannot read sessions: {error}")),
                    repo_paths: None,
                    repo_unknown: 0,
                    epoch: self.requested,
                    active: 0,
                });
                self.version += 1;
            }
        }
    }

    /// Goes up each time a result lands, so a view can tell whether it has applied the latest.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The last result that landed, even if a newer read is running.
    pub fn landed(&self) -> Option<&Landed> {
        self.landed.as_ref()
    }

    /// True when the landed result reflects every request made so far.
    pub fn fresh(&self) -> bool {
        self.landed
            .as_ref()
            .is_some_and(|l| l.epoch == self.requested)
    }

    /// Active sessions, only when the answer is current and every record was read.
    pub fn exact_active(&self) -> Option<usize> {
        let landed = self.landed.as_ref().filter(|_| self.fresh())?;
        landed
            .outcome
            .as_ref()
            .ok()
            .filter(|found| found.complete())
            .map(|_| landed.active)
    }

    /// When the running worker started: how long a pending read has been going.
    pub fn running_since(&self) -> Option<Instant> {
        self.running.as_ref().map(|(.., since)| *since)
    }

    /// Whether a worker is running (tests assert it stays bounded).
    #[cfg(test)]
    pub fn running(&self) -> bool {
        self.running.is_some()
    }
}

#[cfg(test)]
mod tests;
