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
