//! Test-only, root-isolated accounting of record enumeration and file reads.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

static ROOTS: Mutex<Option<HashMap<PathBuf, Arc<Counts>>>> = Mutex::new(None);

#[derive(Default)]
struct Counts {
    scans: AtomicUsize,
    files: AtomicUsize,
}

/// Counts only the registered TEST root; dropping unregisters it.
pub(crate) struct ReadCounts {
    root: PathBuf,
    counts: Arc<Counts>,
}

impl ReadCounts {
    /// Root enumerations and JSON file read attempts, including refused files.
    pub(crate) fn totals(&self) -> (usize, usize) {
        (
            self.counts.scans.load(Ordering::Relaxed),
            self.counts.files.load(Ordering::Relaxed),
        )
    }
}

impl Drop for ReadCounts {
    fn drop(&mut self) {
        ROOTS.lock().unwrap().as_mut().unwrap().remove(&self.root);
    }
}

/// Observe reads of one isolated root without counting other parallel tests.
pub(crate) fn count_reads(root: &Path) -> ReadCounts {
    let counts = Arc::new(Counts::default());
    assert!(
        ROOTS
            .lock()
            .unwrap()
            .get_or_insert_default()
            .insert(root.to_path_buf(), counts.clone())
            .is_none()
    );
    ReadCounts {
        root: root.into(),
        counts,
    }
}

pub(super) fn scan(root: &Path) {
    if let Some(counts) = ROOTS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|roots| roots.get(root))
    {
        counts.scans.fetch_add(1, Ordering::Relaxed);
    }
}

pub(super) fn file(path: &Path) {
    if let Some(counts) = ROOTS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|roots| path.parent().and_then(|root| roots.get(root)))
    {
        counts.files.fetch_add(1, Ordering::Relaxed);
    }
}
