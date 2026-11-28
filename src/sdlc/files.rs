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
/// The most bytes read from one Pi session file; a larger one is refused, never truncated.
const TRANSCRIPT_LIMIT: u64 = 64 * 1024 * 1024;

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

/// The Pi session file a step's contract names: in the folder `<contract>.session-dir` records,
/// the `.jsonl` whose header is `session`. The one place a worker transcript is found.
pub(crate) fn transcript(contract: &Path, session: &str) -> Result<PathBuf, String> {
    let recorded = contract.with_extension("session-dir");
    let folder = crate::regular_file::read_text(&recorded, "not a regular file")
        .map_err(|e| format!("{}: {e}", recorded.display()))?;
    let folder = PathBuf::from(folder.trim_end_matches('\n'));
    let mut found: Vec<_> = fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    found.sort();
    found
        .into_iter()
        .find(|path| header(path).is_some_and(|h| h["type"] == "session" && h["id"] == session))
        .ok_or_else(|| format!("No Pi session {session} in {}.", folder.display()))
}

/// The first line of a Pi session file, read alone: a session file can be very large.
pub(crate) fn header(path: &Path) -> Option<serde_json::Value> {
    use std::io::{BufRead, Read};
    let file = crate::regular_file::open(
        fs::OpenOptions::new().read(true),
        libc::O_NOFOLLOW,
        path,
        "not a regular file",
    )
    .ok()?;
    let mut line = String::new();
    std::io::BufReader::new(file.take(64 * 1024))
        .read_line(&mut line)
        .ok()?;
    serde_json::from_str(&line).ok()
}

/// One message in a step's Pi session file.
pub(crate) struct Said {
    /// When the file says it was written, in whole seconds since 1970.
    pub(crate) at: Option<i64>,
    /// Pi's message object, as stored.
    pub(crate) message: serde_json::Value,
}

/// A step's worker transcript: its session header and each message in file order. `None` for
/// a line that is not one JSON value; other entries, such as model changes, are left out.
pub(crate) struct Transcript {
    /// The header line.
    pub(crate) header: serde_json::Value,
    /// The messages, oldest first.
    pub(crate) said: Vec<Option<Said>>,
}

impl Transcript {
    /// Read the session file the step contract at `contract` names; `None` when it is missing,
    /// unreadable, over the size limit, or not that session.
    pub(crate) fn read(contract: &Path, session: &str) -> Option<Self> {
        let path = transcript(contract, session).ok()?;
        let bytes =
            crate::regular_file::read_bounded(&path, TRANSCRIPT_LIMIT, "not a regular file")
                .ok()?
                .bytes?;
        let mut lines = bytes.split(|&b| b == b'\n').filter(|line| !line.is_empty());
        let header: serde_json::Value = serde_json::from_slice(lines.next()?).ok()?;
        let said = lines
            .filter_map(
                |line| match serde_json::from_slice::<serde_json::Value>(line) {
                    Err(_) => Some(None),
                    Ok(entry) if entry["type"] == "message" => Some(Some(Said {
                        at: entry["timestamp"]
                            .as_str()
                            .and_then(crate::calendar::parse)
                            .map(|at| at.unix_timestamp()),
                        message: entry["message"].clone(),
                    })),
                    Ok(_) => None,
                },
            )
            .collect();
        Some(Self { header, said })
    }

    /// Assistant answers; `None` if any line is not JSON.
    pub(crate) fn responses(&self) -> Option<usize> {
        self.said.iter().try_fold(0, |n, said| {
            let said = said.as_ref()?;
            Some(n + usize::from(said.message["role"] == "assistant"))
        })
    }
}
