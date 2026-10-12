//! Session discovery for the menu: one owned, read-only worker that never runs in paint.
//!
//! The menu and shape read only what this has already landed. Four states are kept apart:
//! not known (nothing landed, or a newer request is still running), complete (every
//! record was read), incomplete (some records could not be read) and failed (the root
//! could not be read at all). A missing root is complete and empty: nothing exists yet.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    time::Instant,
};

use super::{self as shape, Session, Stage};
use crate::seed_me::{self, Record};

/// What one scan found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Found {
    /// Every readable session, newest first.
    pub sessions: Vec<Session>,
    /// Records that exist but could not be read; they are never guessed or counted.
    pub unreadable: usize,
}
impl Found {
    /// Every record was read, so the list is the whole truth.
    pub fn complete(&self) -> bool {
        self.unreadable == 0
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
        let Ok(entry) = entry else {
            found.unreadable += 1;
            continue;
        };
        match seed_me::read(&entry.path()) {
            Record::Session(session) => found.sessions.push(session),
            Record::NotASession => {}
            Record::Unreadable => found.unreadable += 1,
        }
    }
    found.sessions.sort_by(|a, b| {
        shape::timestamp(&b.updated)
            .cmp(&shape::timestamp(&a.updated))
            .then_with(|| b.updated.cmp(&a.updated))
    });
    Ok(found)
}

/// The last result that landed, with how many requests had been made when it started.
pub struct Landed {
    /// What the read found, or why the root couldn't be read.
    pub outcome: Outcome,
    epoch: u64,
    active: usize,
}

/// One owned worker at a time. Requests made while it runs coalesce into one follow-up.
pub struct Discovery {
    source: Source,
    root: PathBuf,
    requested: u64,
    running: Option<(u64, PathBuf, Receiver<Outcome>, Instant)>,
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
            landed: None,
            version: 0,
        }
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
            Err(TryRecvError::Disconnected) => Err("Reading sessions stopped unexpectedly.".into()),
        };
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
        let spawned = std::thread::Builder::new()
            .name("tinkery-discovery".into())
            .spawn(move || {
                let _ = tx.send(source(&root));
            });
        match spawned {
            Ok(_) => self.running = Some((self.requested, self.root.clone(), rx, Instant::now())),
            Err(error) => {
                self.landed = Some(Landed {
                    outcome: Err(format!("Cannot read sessions: {error}")),
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
