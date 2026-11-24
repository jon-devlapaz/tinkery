//! One run's own files under `<checkout>/runs/<slug>/`, read without following a link below
//! the run folder and never past one size limit. Nothing is written, listed beyond a bounded
//! folder or interpreted beyond tink-sdlc's file grammar.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

/// The most bytes read from any one run file; a larger file is refused, never truncated.
const LIMIT: u64 = 8 * 1024 * 1024;
/// The most entries listed in a steps folder; more is refused, never sampled in listing order.
pub(crate) const MAX_ENTRIES: usize = 256;

/// Why a run file has no readable content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileGap {
    /// Missing, or reached through a link.
    Unreachable,
    NotRegular,
    /// Larger than the one size limit.
    TooLarge,
    /// The read failed after opening, with the system's words.
    Io(String),
    /// A JSON-lines file whose last line has no newline.
    Incomplete,
}

/// One run's folder. Links in the checkout's own path are resolved; a link at `runs/`, at the run
/// folder or below it is refused, and the slug names exactly one folder.
#[derive(Clone)]
pub(crate) struct RunFiles {
    folder: PathBuf,
}

/// One Pi launch log named `<step>.launch-<digits>.pi.log` (or `resume-` where allowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Log {
    /// The step name before the last launch or resume marker.
    pub(crate) step: String,
    /// The launcher's time stamp.
    pub(crate) time: u64,
    /// The file's name inside the steps folder.
    pub(crate) name: String,
    /// The entry itself is a regular file; a link or folder is not.
    pub(crate) file: bool,
}

impl RunFiles {
    /// The run folder below `checkout`; `None` when it is missing, linked, or the slug is not
    /// one plain folder name.
    pub(crate) fn open(checkout: &Path, slug: &str) -> Option<Self> {
        let mut parts = Path::new(slug).components();
        if !matches!(
            (parts.next(), parts.next()),
            (Some(Component::Normal(_)), None)
        ) {
            return None;
        }
        let folder = checkout.canonicalize().ok()?.join("runs").join(slug);
        (folder.canonicalize().ok()? == folder).then_some(Self { folder })
    }

    /// The resolved run folder, for identity comparison only.
    pub(crate) fn folder(&self) -> &Path {
        &self.folder
    }

    /// The bytes of a regular file at `relative` below the run folder.
    pub(crate) fn read(&self, relative: &str) -> Result<Vec<u8>, FileGap> {
        let path = self.folder.join(relative);
        if path.canonicalize().ok().as_ref() != Some(&path) {
            return Err(FileGap::Unreachable);
        }
        crate::regular_file::read_bounded(&path, LIMIT, "not a regular file")
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::InvalidInput => FileGap::NotRegular,
                _ => FileGap::Io(error.to_string()),
            })?
            .bytes
            .ok_or(FileGap::TooLarge)
    }

    /// Each line of `labels.jsonl` as JSON, oldest first; `None` is a line that is not one JSON
    /// value. An empty file has none; any other must end with a newline. Counts and kinds stay
    /// with the caller.
    pub(crate) fn labels(&self) -> Result<Vec<Option<serde_json::Value>>, FileGap> {
        let bytes = self.read("labels.jsonl")?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        let body = bytes.strip_suffix(b"\n").ok_or(FileGap::Incomplete)?;
        Ok(body
            .split(|&b| b == b'\n')
            .map(|line| serde_json::from_slice(line).ok())
            .collect())
    }

    /// Pi launch logs in `steps/`, in no promised order. `resume` also admits `resume-` logs.
    /// `None` when the folder is missing or linked. With `bounded`, an unreadable entry or more
    /// than [`MAX_ENTRIES`] entries also gives `None`; without it, unreadable entries are skipped.
    pub(crate) fn logs(&self, resume: bool, bounded: bool) -> Option<Vec<Log>> {
        let steps = self.folder.join("steps");
        if steps.canonicalize().ok()? != steps {
            return None;
        }
        let listing = fs::read_dir(&steps).ok()?;
        let entries: Vec<_> = if bounded {
            let entries: Vec<_> = listing
                .take(MAX_ENTRIES + 1)
                .collect::<Result<_, _>>()
                .ok()?;
            if entries.len() > MAX_ENTRIES {
                return None;
            }
            entries
        } else {
            listing.filter_map(Result::ok).collect()
        };
        Some(
            entries
                .into_iter()
                .filter_map(|entry| {
                    let name = entry.file_name().into_string().ok()?;
                    let rest = name.strip_suffix(".pi.log")?;
                    let marker = [".launch-", ".resume-"]
                        .into_iter()
                        .take(if resume { 2 } else { 1 })
                        .filter_map(|marker| Some((rest.rfind(marker)?, marker)))
                        .max()?;
                    let (step, time) = (&rest[..marker.0], &rest[marker.0 + marker.1.len()..]);
                    if time.is_empty() || !time.bytes().all(|b| b.is_ascii_digit()) {
                        return None;
                    }
                    Some(Log {
                        step: step.into(),
                        time: time.parse().ok()?,
                        file: entry.file_type().is_ok_and(|kind| kind.is_file()),
                        name,
                    })
                })
                .collect(),
        )
    }

    /// A step file's text lines, split as `BufRead::lines` splits them; a line that is not
    /// UTF-8 is `None` so each caller keeps its own refusal.
    pub(crate) fn lines(&self, relative: &str) -> Option<Vec<Option<String>>> {
        let bytes = self.read(relative).ok()?;
        let body = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        if bytes.is_empty() {
            return Some(Vec::new());
        }
        Some(
            body.split(|&b| b == b'\n')
                .map(|line| {
                    let line = line.strip_suffix(b"\r").unwrap_or(line);
                    std::str::from_utf8(line).ok().map(Into::into)
                })
                .collect(),
        )
    }
}
