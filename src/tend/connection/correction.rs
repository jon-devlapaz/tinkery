//! One explicit repository correction. Reads never repair data; replacement refuses stale views,
//! unsafe paths and pending goal handoffs. Stored guidance grants no execution authority.

use super::*;
use std::os::unix::fs::MetadataExt;

const FILE_LIMIT: usize = 16 * 1024;

/// A bounded read and local change token. Invalid regular files can be explicitly cleared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    digest: Option<[u8; 32]>,
    value: Result<Option<Correction>, String>,
}

impl Snapshot {
    pub(super) fn value(&self) -> Result<Option<Correction>, String> {
        self.value.clone()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema: u8,
    repo: PathBuf,
    correction: serde_json::Value,
}

impl Context {
    fn correction_file(&self) -> PathBuf {
        self.file().with_extension("correction.json")
    }

    pub(super) fn correction(&self) -> Result<Snapshot, String> {
        let regular = "The correction is not a regular local file.";
        let read = match crate::regular_file::read_bounded(
            &self.correction_file(),
            FILE_LIMIT as u64,
            regular,
        ) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Snapshot {
                    digest: None,
                    value: Ok(None),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => return Err(regular.into()),
            Err(e) => return Err(format!("The correction cannot be read safely: {e}")),
            Ok(read) => read,
        };
        // Oversized records remain clearable, without reading an unbounded file.
        let (bytes, meta) = (read.bytes, read.metadata);
        let mut digest = Sha256::new();
        digest.update(bytes.as_deref().unwrap_or_default());
        for part in [
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime() as u64,
            meta.mtime_nsec() as u64,
            meta.ctime() as u64,
            meta.ctime_nsec() as u64,
        ] {
            digest.update(part.to_le_bytes());
        }
        let value = (|| {
            let bytes = bytes
                .ok_or("The correction file is oversized. Clear it explicitly or restore it.")?;
            let stored: Stored = serde_json::from_slice(&bytes)
                .map_err(|_| "The correction is malformed. Clear it explicitly or restore it.")?;
            if stored.schema != 1 || stored.repo != self.repo {
                return Err("The correction has another repository or format. Clear it explicitly or restore it.".into());
            }
            if stored.correction.is_null() {
                return Ok(None);
            }
            let correction: Correction = serde_json::from_value(stored.correction).map_err(
                |_| "The correction words are malformed. Clear it explicitly or restore it.",
            )?;
            correction.validate()?;
            Ok(Some(correction))
        })();
        Ok(Snapshot {
            digest: Some(digest.finalize().into()),
            value,
        })
    }

    pub(super) fn correct(
        &self,
        shown: &Snapshot,
        proposal: Option<&Record>,
        words: Option<&str>,
    ) -> Result<(), String> {
        let _lock = self.lock()?;
        if self.correction()?.digest != shown.digest {
            return Err(
                "The correction changed. Reopen it before replacing or clearing it.".into(),
            );
        }
        let current = self.read()?;
        if current
            .as_ref()
            .is_some_and(|r| r.pending.is_some() && r.saved.is_none())
        {
            return Err(
                "Resume or cancel the pending goal save before changing the correction.".into(),
            );
        }
        let correction = if let Some(words) = words {
            // New guidance must come from an explicitly reviewed proposal. Replacement uses its shown revision.
            if shown.value()?.is_none()
                && (proposal.is_none()
                    || current.as_ref() != proposal
                    || proposal.is_none_or(|r| r.proposal.is_none()))
            {
                return Err("Review a connection before saving a correction.".into());
            }
            let mut random = [0; 24];
            crate::regular_file::random(&mut random).map_err(|e| e.to_string())?;
            let correction = Correction {
                revision: random.iter().map(|b| format!("{b:02x}")).collect(),
                text: words.into(),
            };
            correction.validate()?;
            Some(correction)
        } else {
            None
        };
        let stored = Stored {
            schema: 1,
            repo: self.repo.clone(),
            correction: serde_json::to_value(correction).map_err(|e| e.to_string())?,
        };
        let bytes = serde_json::to_vec(&stored).map_err(|e| e.to_string())?;
        match crate::regular_file::publish(
            &self.correction_file(),
            &bytes,
            crate::regular_file::Existing::Replace,
        )
        .map_err(|e| e.to_string())?
        {
            crate::regular_file::Published::Durable => Ok(()),
            crate::regular_file::Published::Uncertain(e) => Err(format!(
                "Correction replaced, but persistence is uncertain. Reopen before another change: {e}"
            )),
        }
    }
}
