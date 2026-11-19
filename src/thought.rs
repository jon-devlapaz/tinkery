//! Exact repository-bound thought records. This store grants no goal or model authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

const LIMIT: usize = 128 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Thought {
    schema: u8,
    repo: PathBuf,
    words: String,
}

/// The persistence result distinguishes a durable record from an uncertain write.
pub(crate) enum Saved {
    New,
    Duplicate,
    Uncertain(String),
}

/// A captured repository and checked storage root. Existing originals are never replaced.
pub(crate) struct Store {
    root: Result<PathBuf, String>,
    repo: PathBuf,
}
impl Store {
    /// Bind storage to the explicit launch repository without writing anything.
    pub(crate) fn new(root: Result<PathBuf, String>, repo: PathBuf) -> Self {
        Self { root, repo }
    }
    fn prefix(&self) -> String {
        format!(
            "thought-{:x}-",
            Sha256::digest(self.repo.as_os_str().as_encoded_bytes())
        )
    }

    fn file(&self, words: &str) -> Result<PathBuf, String> {
        Ok(self.root.as_ref().map_err(Clone::clone)?.join(format!(
            "{}{:x}.json",
            self.prefix(),
            Sha256::digest(words.as_bytes())
        )))
    }

    fn read_one(&self, path: &PathBuf) -> Result<String, String> {
        let file = crate::regular_file::open(
            fs::OpenOptions::new().read(true),
            libc::O_NOFOLLOW,
            path,
            "A thought is not a regular local file.",
        )
        .map_err(|e| e.to_string())?;
        let mut bytes = vec![];
        file.take((LIMIT * 6 + 4096) as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > LIMIT * 6 + 4096 {
            return Err("A saved thought is oversized.".into());
        }
        let thought: Thought = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if thought.schema != 1
            || thought.repo != self.repo
            || thought.words.is_empty()
            || thought.words.len() > LIMIT
            || self.file(&thought.words)? != *path
        {
            return Err("A thought has invalid words, repository or format.".into());
        }
        Ok(thought.words)
    }

    pub(crate) fn read(&self) -> Result<(Vec<String>, usize), String> {
        let mut unreadable = 0;
        let root = self.root.as_ref().map_err(Clone::clone)?;
        let files = match fs::read_dir(root) {
            Ok(files) => files,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((vec![], 0)),
            Err(e) => return Err(e.to_string()),
        };
        let mut entries = vec![];
        for file in files {
            let path = file.map_err(|e| e.to_string())?.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&self.prefix()))
                && path.extension().is_some_and(|e| e == "json")
            {
                match self.read_one(&path) {
                    Ok(words) => entries.push(words),
                    Err(_) => unreadable += 1,
                }
            }
        }
        entries.sort();
        Ok((entries, unreadable))
    }

    pub(crate) fn save(&self, words: &str) -> Result<Saved, String> {
        if words.is_empty() || words.len() > LIMIT {
            return Err("Write a thought of 1 to 131072 bytes. Nothing was saved.".into());
        }
        let root = self.root.as_ref().map_err(Clone::clone)?;
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let path = self.file(words)?;
        let thought = Thought {
            schema: 1,
            repo: self.repo.clone(),
            words: words.into(),
        };
        let bytes = serde_json::to_vec(&thought).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        match temp.persist_noclobber(&path) {
            Ok(_) => {
                if let Err(e) = crate::regular_file::sync_directory(root) {
                    return Ok(Saved::Uncertain(format!(
                        "Thought persistence is uncertain. Keep these words: {e}"
                    )));
                }
                Ok(Saved::New)
            }
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                if self.read_one(&path)? != words {
                    return Err("The existing thought differs. Nothing was replaced.".into());
                }
                if let Err(e) = crate::regular_file::sync_directory(root) {
                    return Ok(Saved::Uncertain(format!(
                        "Thought persistence is uncertain. Keep these words: {e}"
                    )));
                }
                Ok(Saved::Duplicate)
            }
            Err(e) => Err(e.to_string()),
        }
    }
}
