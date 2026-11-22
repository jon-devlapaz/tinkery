//! Where Tinkery keeps saved sessions, Think records and Tend's stores. Each default comes from
//! one absolute HOME check; an unusable HOME refuses each root with the flag that replaces it,
//! never a folder under the launch directory. Creates and reads nothing.

use std::{ffi::OsStr, path::PathBuf};

/// The three data folders, or why each has none. An explicit flag replaces one root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataRoots {
    /// Saved sessions.
    pub sessions: Result<PathBuf, String>,
    /// Think records.
    pub records: Result<PathBuf, String>,
    /// Tend's needs links, saved thoughts and connection files.
    pub tend: Result<PathBuf, String>,
}

impl DataRoots {
    /// The defaults under `home` (the `HOME` variable); refuses each when `home` is unset,
    /// empty or relative.
    pub fn from_home(home: Option<&OsStr>) -> Self {
        let home = home.map(PathBuf::from).filter(|home| home.is_absolute());
        let under = |folder: &str, refusal: &str| {
            home.as_ref()
                .map(|home| home.join(".local/share/tinkery").join(folder))
                .ok_or_else(|| refusal.to_owned())
        };
        Self {
            sessions: under(
                "sessions",
                "HOME is unset or not absolute; pass --seed-session-root PATH for sessions.",
            ),
            records: under(
                "think",
                "HOME is unset or not an absolute path, so think records have no default folder; pass --record-root PATH",
            ),
            tend: under(
                "tend",
                "HOME is unset or not absolute, so linking is unavailable; pass --link-root PATH.",
            ),
        }
    }

    /// [`DataRoots::from_home`] for this process's `HOME`.
    pub fn from_env() -> Self {
        Self::from_home(std::env::var_os("HOME").as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_sit_under_an_absolute_home_and_refuse_any_other_without_creating_folders() {
        let home = tempfile::tempdir().unwrap();
        let share = home.path().join(".local/share/tinkery");
        assert_eq!(
            DataRoots::from_home(Some(home.path().as_os_str())),
            DataRoots {
                sessions: Ok(share.join("sessions")),
                records: Ok(share.join("think")),
                tend: Ok(share.join("tend")),
            }
        );
        assert!(!home.path().join(".local").exists());
        for home in [None, Some(""), Some("TEST-relative"), Some(".")] {
            let roots = DataRoots::from_home(home.map(OsStr::new));
            for (refusal, remedy) in [
                (
                    roots.sessions,
                    "pass --seed-session-root PATH for sessions.",
                ),
                (
                    roots.records,
                    "think records have no default folder; pass --record-root PATH",
                ),
                (roots.tend, "linking is unavailable; pass --link-root PATH."),
            ] {
                assert!(refusal.unwrap_err().ends_with(remedy), "{home:?}");
            }
        }
    }
}
