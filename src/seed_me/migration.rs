//! One-time session-root rename; session bytes and Think records are never rewritten.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

/// Rename the default sessions folder once without reading or rewriting its contents.
/// Existing destinations and linked roots refuse; Think records are never opened or changed.
/// When `allow_move` is false, a pending move refuses before any filesystem writes.
pub fn migrate(home: Option<&OsStr>, allow_move: bool) -> Result<Option<String>, String> {
    let Some(home) = home.map(Path::new).filter(|p| p.is_absolute()) else {
        return Ok(None);
    };
    let old = home.join(".local/share/seed-me/sessions");
    let new = home.join(".local/share/tinkery/sessions");
    let metadata = match fs::symlink_metadata(&old) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot inspect the old sessions folder. Check its permissions before reopening Tinkery.".into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("The old sessions root is not an ordinary folder. Move it aside yourself before reopening Tinkery.".into());
    }
    match fs::symlink_metadata(&new) {
        Ok(_) => return Err("Both old and new sessions roots exist. Tinkery will not merge or read the old root. Move the leftover old root aside before reopening.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(_) => return Err("Cannot inspect the new sessions folder. Check its permissions before reopening Tinkery.".into()),
    }
    if !allow_move {
        return Err("Snapshot mode does not move saved sessions. Open Tinkery normally to migrate them, then retry --snapshot.".into());
    }
    fs::create_dir_all(new.parent().unwrap())
        .map_err(|_| "Cannot create Tinkery's data folder. Check its permissions.")?;
    let old = old
        .canonicalize()
        .map_err(|_| "Cannot locate the old sessions folder.")?;
    let new = new
        .parent()
        .unwrap()
        .canonicalize()
        .map_err(|_| "Cannot locate Tinkery's data folder.")?
        .join("sessions");
    fs::rename(&old, &new).map_err(|_| "Cannot move saved sessions. Keep both locations unchanged and check permissions before retrying.")?;
    Ok(Some(
        "Moved saved sessions to ~/.local/share/tinkery/sessions.".into(),
    ))
}

/// Refuse explicit attempts to read the retired default location, without inspecting its files.
pub(crate) fn refuse_old_root(path: &Path) -> Result<(), String> {
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|_| "Cannot resolve the session path.")?
                .join(path)
        };
        let mut clean = PathBuf::new();
        for component in absolute.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    clean.pop();
                }
                component => clean.push(component),
            }
        }
        let canonical_old = home
            .canonicalize()
            .unwrap_or_else(|_| home.clone())
            .join(".local/share/seed-me/sessions");
        if clean.starts_with(home.join(".local/share/seed-me/sessions"))
            || clean
                .canonicalize()
                .is_ok_and(|path| path.starts_with(canonical_old))
        {
            return Err("The old sessions root is retired. Open the session under ~/.local/share/tinkery/sessions instead.".into());
        }
    }
    Ok(())
}
