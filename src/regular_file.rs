//! The one door for opening files on disk, and the small-store rules on top of it. Promises: no
//! open here waits, and every file it returns is a regular file, checked on the opened descriptor
//! so a path swapped for a FIFO or device cannot turn a read into a wait for a writer. Refuses
//! anything else with the caller's words. A bounded read never follows a link and never holds
//! more than its limit. A publish is whole or absent, and says when it may not survive a crash.
//! `clippy.toml` forbids `std::fs` opens and reads everywhere else (#489, #585, #764), except
//! two allowed reads that may wait on purpose: the controlling terminal and the command-line goal
//! file.
#![allow(clippy::disallowed_methods)]

use std::{
    fs::{File, Metadata, OpenOptions, Permissions},
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

/// Opens `path` with `options`, the extra open `flags` (such as `O_NOFOLLOW`) and `O_NONBLOCK`.
/// Refuses, with `refusal` as the error text, anything that is not a regular file once opened.
pub(crate) fn open(
    options: &mut OpenOptions,
    flags: libc::c_int,
    path: &Path,
    refusal: &str,
) -> io::Result<File> {
    let file = options.custom_flags(flags | libc::O_NONBLOCK).open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, refusal));
    }
    Ok(file)
}

/// Reads all of a regular file; symlinks keep their usual target semantics.
pub(crate) fn read(path: &Path, refusal: &str) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open(OpenOptions::new().read(true), 0, path, refusal)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Reads a regular file as UTF-8, refusing invalid text without changing the bytes.
pub(crate) fn read_text(path: &Path, refusal: &str) -> io::Result<String> {
    String::from_utf8(read(path, refusal)?).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })
}

/// One bounded read of one opened file.
#[derive(Debug)]
pub(crate) struct Bounded {
    /// The whole file, or `None` when it holds more than the limit.
    pub(crate) bytes: Option<Vec<u8>>,
    /// The metadata of the same opened file the bytes came from.
    pub(crate) metadata: Metadata,
}

/// Reads the regular file at `path` without holding more than `limit + 1` bytes. Never follows a
/// link at `path`: a link, like any other non-regular file, is refused as `InvalidInput` with
/// `refusal`.
pub(crate) fn read_bounded(path: &Path, limit: u64, refusal: &str) -> io::Result<Bounded> {
    let file = match open(
        OpenOptions::new().read(true),
        libc::O_NOFOLLOW,
        path,
        refusal,
    ) {
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, refusal));
        }
        file => file?,
    };
    let metadata = file.metadata()?;
    if metadata.len() > limit {
        return Ok(Bounded {
            bytes: None,
            metadata,
        });
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    Ok(Bounded {
        bytes: (bytes.len() as u64 <= limit).then_some(bytes),
        metadata,
    })
}

/// What [`publish`] does with a file already at its path.
pub(crate) enum Existing {
    /// Keep it and refuse with `AlreadyExists`.
    Refuse,
    /// Replace it; the new file is readable by its owner only.
    Replace,
    /// Replace it; the new file has these permissions.
    ReplaceWith(Permissions),
}

/// How far a [`publish`] got once the new file is in place. Either way the file is saved.
#[derive(Debug)]
pub(crate) enum Published {
    /// The directory entry is on disk too.
    Durable,
    /// The directory sync failed: the entry may not survive a crash.
    Uncertain(io::Error),
}

/// Publishes `bytes` at `path` whole or not at all: readers see the old file or the new one,
/// never part of either. An error means nothing was published.
pub(crate) fn publish(path: &Path, bytes: &[u8], existing: Existing) -> io::Result<Published> {
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "a file needs a folder"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(folder)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    match existing {
        Existing::Refuse => {
            temporary.persist_noclobber(path).map_err(|e| e.error)?;
        }
        Existing::Replace => {
            temporary.persist(path).map_err(|e| e.error)?;
        }
        Existing::ReplaceWith(permissions) => {
            temporary.as_file().set_permissions(permissions)?;
            temporary.persist(path).map_err(|e| e.error)?;
        }
    }
    Ok(sync_directory(folder).map_or_else(Published::Uncertain, |()| Published::Durable))
}

/// Flushes a directory's entries to disk. Refuses anything that is not a directory.
pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NONBLOCK)
        .open(path)?
        .sync_all()
}

/// Fills `bytes` from the system's random device, the one non-regular file this module opens.
pub(crate) fn random(bytes: &mut [u8]) -> io::Result<()> {
    File::open("/dev/urandom")?.read_exact(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_read_refuses_links_and_files_over_the_limit() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("TEST-record.json");
        let link = folder.path().join("TEST-link.json");
        std::fs::write(&file, b"12345").unwrap();
        std::os::unix::fs::symlink(&file, &link).unwrap();

        let read = read_bounded(&file, 5, "TEST refusal").unwrap();
        assert_eq!(read.bytes.unwrap(), b"12345");
        assert_eq!(read.metadata.len(), 5);
        let linked = read_bounded(&link, 5, "TEST refusal").unwrap_err();
        assert_eq!(linked.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(linked.to_string(), "TEST refusal");
        assert!(
            read_bounded(&file, 4, "TEST refusal")
                .unwrap()
                .bytes
                .is_none()
        );
    }

    #[test]
    fn publish_refuses_or_replaces_an_existing_file_whole() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("TEST-store.json");
        publish(&file, b"first", Existing::Refuse).unwrap();
        let kept = publish(&file, b"second", Existing::Refuse).unwrap_err();
        assert_eq!(kept.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(read(&file, "TEST").unwrap(), b"first");
        publish(&file, b"second", Existing::Replace).unwrap();
        assert_eq!(read(&file, "TEST").unwrap(), b"second");
        let names: Vec<_> = std::fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["TEST-store.json"]);
    }
}
