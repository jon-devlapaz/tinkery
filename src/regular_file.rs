//! The one door for opening files on disk, and the small-store rules on top of it. Promises: no
//! open here waits, and every file it returns is a regular file, checked on the opened descriptor
//! so a path swapped for a FIFO or device cannot turn a read into a wait for a writer. Refuses
//! anything else with the caller's words. A bounded read never follows a link and never holds more
//! than its limit; a publish is whole or absent and says when it may not survive a crash. `clippy.toml` forbids `std::fs` opens and reads everywhere else (#489, #585, #764),
//! except two allowed reads that may wait on purpose: the controlling terminal and the
//! command-line goal file.
#![allow(clippy::disallowed_methods)]

use std::{
    fs::{File, OpenOptions, Permissions},
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

/// Reads all of the regular file at `path` if it holds at most `limit` bytes. Never follows a link
/// at `path`: a link, like any other non-regular file, is refused as `InvalidInput` with `refusal`.
/// Refuses a longer file as `FileTooLarge` without holding more than `limit + 1` bytes.
pub(crate) fn read_bounded(path: &Path, limit: u64, refusal: &str) -> io::Result<Vec<u8>> {
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
    let oversized = || io::Error::new(io::ErrorKind::FileTooLarge, "the file exceeds its limit");
    if file.metadata()?.len() > limit {
        return Err(oversized());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(oversized());
    }
    Ok(bytes)
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

/// Publishes `bytes` at `path` whole or not at all: readers see the old file or the new one,
/// never part of either. The outer error means nothing was published. The inner error means the
/// new file is in place, but its directory entry may not survive a crash.
pub(crate) fn publish(path: &Path, bytes: &[u8], existing: Existing) -> io::Result<io::Result<()>> {
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
    Ok(sync_directory(folder))
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

        assert_eq!(read_bounded(&file, 5, "TEST refusal").unwrap(), b"12345");
        let linked = read_bounded(&link, 5, "TEST refusal").unwrap_err();
        assert_eq!(linked.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(linked.to_string(), "TEST refusal");
        let long = read_bounded(&file, 4, "TEST refusal").unwrap_err();
        assert_eq!(long.kind(), io::ErrorKind::FileTooLarge);
    }

    #[test]
    fn publish_refuses_or_replaces_an_existing_file_whole() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("TEST-store.json");
        publish(&file, b"first", Existing::Refuse).unwrap().unwrap();
        let kept = publish(&file, b"second", Existing::Refuse).unwrap_err();
        assert_eq!(kept.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(read(&file, "TEST").unwrap(), b"first");
        publish(&file, b"second", Existing::Replace)
            .unwrap()
            .unwrap();
        assert_eq!(read(&file, "TEST").unwrap(), b"second");
        let names: Vec<_> = std::fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["TEST-store.json"]);
    }
}
