//! The session folder's file names and the Tink Contract's header lines. Each name and status
//! line is spelled only here; a contract's revision label has one reading rule.

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
