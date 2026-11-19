//! The one door for opening files on disk. Promises: no open here waits, and every file it
//! returns is a regular file, checked on the opened descriptor so a path swapped for a FIFO or
//! device cannot turn a read into a wait for a writer. Refuses anything else with the caller's
//! words. `clippy.toml` forbids `std::fs` opens and reads everywhere else (#489, #585, #764).
#![allow(clippy::disallowed_methods)]

use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
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
    String::from_utf8(read(path, refusal)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Flushes a directory's entries to disk. Refuses anything that is not a directory.
pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NONBLOCK)
        .open(path)?
        .sync_all()
}

/// Fills `bytes` from the system's random device, the only non-regular file Tinkery opens.
pub(crate) fn random(bytes: &mut [u8]) -> io::Result<()> {
    File::open("/dev/urandom")?.read_exact(bytes)
}
