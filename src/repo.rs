//! The Git repository Tinkery was started in, found by looking for `.git` from the launch folder
//! upward. It runs no `git` command and opens nothing inside the repository: finding one only
//! names where the operator is, never what a goal is about, and grants nothing.

use std::path::{Path, PathBuf};

/// A repository root: the nearest folder at or above the launch folder that holds `.git`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    root: PathBuf,
}

impl Repo {
    /// The repository enclosing `start`, or `None` outside one. A `.git` folder or file (as in
    /// a worktree) counts; its contents are never read. `start` should be absolute.
    pub fn enclosing(start: &Path) -> Option<Self> {
        start
            .ancestors()
            .find(|dir| dir.join(".git").symlink_metadata().is_ok())
            .map(|root| Self {
                root: root.to_path_buf(),
            })
    }

    /// The root's absolute path, as found.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The root folder's name, for display (`made in tinkery`). Unprintable names show lossily.
    pub fn name(&self) -> String {
        self.root.file_name().map_or_else(
            || self.root.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_nearest_root_from_a_nested_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("TEST-repo");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        let repo = Repo::enclosing(&root.join("a/b")).unwrap();
        assert_eq!(repo.root(), root);
        assert_eq!(repo.name(), "TEST-repo");
    }

    #[test]
    fn a_git_file_counts_as_a_worktree_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("TEST-worktree");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".git"), "gitdir: /TEST/elsewhere\n").unwrap();
        assert_eq!(Repo::enclosing(&root).unwrap().root(), root);
    }

    #[test]
    fn outside_a_repository_there_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("TEST-plain");
        std::fs::create_dir_all(&plain).unwrap();
        // The temp folder itself must not sit in a repository for this check to mean anything.
        if Repo::enclosing(dir.path()).is_none() {
            assert_eq!(Repo::enclosing(&plain), None);
        }
    }
}
