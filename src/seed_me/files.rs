//! The session folder's file names and the Tink Contract's header and section headings. Each name,
//! status line and Full heading is spelled only here, and each header part has one reading rule.

use std::path::Path;

/// The session's ledger.
pub(crate) const LEDGER: &str = "ledger.json";
/// The Tink Contract.
pub(crate) const SEED: &str = "seed-contract.md";
/// The Tink Contract's acceptance records.
pub(crate) const CHECKS: &str = "seed-checks.json";
/// The receipt that binds a confirmed Tink Contract.
pub(crate) const CONFIRMATION: &str = "seed-confirmation.json";
/// Shape's kept model sessions, one folder per request. Local only, never part of the handoff.
pub(crate) const TRANSCRIPTS: &str = "transcripts";

/// The folder where a model request for `session` keeps its Pi session, after
/// [`prune_transcripts`] has run over every session beside it.
pub(crate) fn keep_transcripts(session: &Path) -> std::path::PathBuf {
    if let Some(sessions) = session.parent() {
        prune_transcripts(sessions);
    }
    session.join(TRANSCRIPTS)
}

/// Run [`crate::pi::prune_kept`] over `<session>/transcripts` of every real session folder
/// under `sessions`. Never follows a link.
pub(crate) fn prune_transcripts(sessions: &Path) {
    for session in std::fs::read_dir(sessions).into_iter().flatten().flatten() {
        if session.file_type().is_ok_and(|kind| kind.is_dir()) {
            crate::pi::prune_kept(&session.path().join(TRANSCRIPTS));
        }
    }
}

/// The first line of an unconfirmed Tink Contract.
pub(crate) const DRAFT: &str = "status: draft";
/// The first line of a Tink Contract confirmed for intake.
pub(crate) const CONFIRMED: &str = "status: confirmed for intake";
/// The first line of a practice Tink Contract that intake refuses.
pub(crate) const SIMULATED: &str = "status: simulated";

/// The label on one `revision:` line, or None when the line is another line or has no label.
pub(crate) fn revision_line(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("revision:")?
        .trim_start_matches([' ', '\t']);
    rest.split(super::ledger::whitespace)
        .next()
        .filter(|s| !s.is_empty())
}

/// The label of the first `revision:` line after the status line, as confirmation reads it.
pub(crate) fn revision(seed: &str) -> Option<&str> {
    seed.lines().skip(1).find_map(revision_line)
}

/// What a Tink Contract's first line says it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// [`DRAFT`].
    Draft,
    /// [`CONFIRMED`].
    Confirmed,
    /// [`SIMULATED`].
    Simulated,
}

/// The status that line 1 of `seed` states exactly, or None for any other first line.
pub(crate) fn status(seed: &str) -> Option<Status> {
    match seed.lines().next()? {
        DRAFT => Some(Status::Draft),
        CONFIRMED => Some(Status::Confirmed),
        SIMULATED => Some(Status::Simulated),
        _ => None,
    }
}

/// The text after the first `# ` heading line of `seed`, or None when it has none.
pub(crate) fn title(seed: &str) -> Option<&str> {
    seed.lines().find_map(|line| line.strip_prefix("# "))
}

/// A section of the Full Tink Contract. Tinkery renders four of them; the model writes the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    /// What the operator confirms.
    Confirming,
    /// The problem.
    Problem,
    /// The proposed outcome.
    Outcome,
    /// Acceptance checks that Tinkery renders from the check records.
    Acceptance,
    /// Who and what the change affects.
    Affected,
    /// Constraints and boundaries.
    Constraints,
    /// Decisions that Tinkery copies from the ledger.
    Decisions,
    /// Assumptions that Tinkery copies from the ledger.
    Assumed,
    /// The knowledge map.
    KnowledgeMap,
    /// Evidence and uncertainty that Tinkery records from grounding.
    Evidence,
    /// The suggested first slice.
    FirstSlice,
    /// Risks and verification.
    Risks,
    /// Open questions and deferrals.
    OpenQuestions,
    /// Owners and next steps.
    Owners,
    /// The downstream handoff.
    Handoff,
}

impl Section {
    /// Every Full section in canonical order; hosts derive their prose fields from it.
    pub(crate) const FULL: [Self; 15] = [
        Self::Confirming,
        Self::Problem,
        Self::Outcome,
        Self::Acceptance,
        Self::Affected,
        Self::Constraints,
        Self::Decisions,
        Self::Assumed,
        Self::KnowledgeMap,
        Self::Evidence,
        Self::FirstSlice,
        Self::Risks,
        Self::OpenQuestions,
        Self::Owners,
        Self::Handoff,
    ];

    /// The section's exact heading line.
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Self::Confirming => "## You are confirming",
            Self::Problem => "## Problem statement",
            Self::Outcome => "## Proposed outcome",
            Self::Acceptance => "## Acceptance criteria",
            Self::Affected => "## Affected users and systems",
            Self::Constraints => "## Constraints and boundaries",
            Self::Decisions => "## Decisions (from the ledger)",
            Self::Assumed => "## Assumed (not confirmed by you)",
            Self::KnowledgeMap => "## Knowledge map",
            Self::Evidence => "## Evidence and uncertainty",
            Self::FirstSlice => "## Suggested first slice",
            Self::Risks => "## Risks and verification",
            Self::OpenQuestions => "## Open questions and deferrals",
            Self::Owners => "## Owners and next steps",
            Self::Handoff => "## Downstream handoff",
        }
    }

    /// The prose field name the model writes, or None for a section that Tinkery renders.
    pub(crate) fn prose(self) -> Option<&'static str> {
        match self {
            Self::Acceptance | Self::Decisions | Self::Assumed | Self::Evidence => None,
            _ => self.heading().strip_prefix("## "),
        }
    }
}

/// Read only an opened regular seed file; symlinks keep their existing target semantics.
/// Nonblocking open prevents a replaced path or FIFO from waiting for a writer.
pub(crate) fn read_seed(path: &Path) -> std::io::Result<Vec<u8>> {
    super::read_regular(path, "original Tink Contract")
}

/// Read regular seed bytes as UTF-8, refusing invalid text without changing the bytes.
pub(crate) fn read_seed_text(path: &Path) -> std::io::Result<String> {
    super::read_regular_text(path, "original Tink Contract")
}

#[cfg(test)]
mod prune_tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn age(path: &Path, when: SystemTime) {
        let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        let secs = when
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as libc::time_t;
        let times = [libc::timespec {
            tv_sec: secs,
            tv_nsec: 0,
        }; 2];
        // SAFETY: a valid C path and two valid times; this sets only that entry's own times.
        let set = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path.as_ptr(),
                times.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        assert_eq!(set, 0);
    }

    #[test]
    fn prune_deletes_only_old_kept_folders_in_every_sessions_transcripts() {
        let dir = tempfile::tempdir().unwrap();
        let (sessions, outside) = (
            dir.path().join("TEST-sessions"),
            dir.path().join("TEST-outside"),
        );
        let old =
            SystemTime::now() - Duration::from_secs((crate::pi::KEEP_DAYS + 1) * 24 * 60 * 60);
        let (mine, sibling) = (
            sessions.join("session-TEST-a"),
            sessions.join("session-TEST-b"),
        );
        let made = [
            mine.join("transcripts/pi-TEST-old"),
            mine.join("transcripts/pi-TEST-recent"),
            mine.join("transcripts/TEST-old-unnamed"),
            mine.join("pi-TEST-old-not-in-transcripts"),
            sibling.join("transcripts/pi-TEST-old-sibling"),
            outside.join("pi-TEST-old-outside"),
            outside.join("transcripts/pi-TEST-old-behind-link"),
        ];
        for folder in &made {
            std::fs::create_dir_all(folder).unwrap();
        }
        std::fs::write(mine.join("transcripts/pi-TEST-old-file"), "TEST").unwrap();
        std::os::unix::fs::symlink(&outside, mine.join("transcripts/pi-TEST-old-link")).unwrap();
        std::os::unix::fs::symlink(&outside, sessions.join("session-TEST-link")).unwrap();
        for path in made.iter().filter(|p| !p.ends_with("pi-TEST-recent")) {
            age(path, old);
        }
        for path in [
            "transcripts/pi-TEST-old-file",
            "transcripts/pi-TEST-old-link",
        ] {
            age(&mine.join(path), old);
        }
        let kept = keep_transcripts(&mine);
        assert_eq!(kept, mine.join(TRANSCRIPTS));
        assert!(!made[0].exists(), "this session's old kept folder must go");
        assert!(
            !made[4].exists(),
            "a sibling session's old kept folder must go"
        );
        for stays in [&made[1], &made[2], &made[3], &made[5], &made[6]] {
            assert!(stays.is_dir(), "{} must stay", stays.display());
        }
        assert!(mine.join("transcripts/pi-TEST-old-file").is_file());
        assert!(std::fs::symlink_metadata(mine.join("transcripts/pi-TEST-old-link")).is_ok());
    }
}
