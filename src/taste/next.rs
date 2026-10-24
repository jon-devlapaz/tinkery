//! One deterministic pause after an explicit report of external acceptance. Evidence gaps
//! outrank new features; no feedback, structural obstacle or permission is invented.

use super::{read::ResultRow, safe};
use crate::think_record::DraftSuggestion;

pub(super) struct Pause {
    pub(super) lines: Vec<String>,
    pub(super) draft: DraftSuggestion,
}

pub(super) fn suggest(row: &ResultRow) -> Pause {
    let failed = row
        .items
        .iter()
        .find(|i| i.state == "fail" || i.state == "stale");
    // Acceptance is external; API 1's permanently unavailable independent-review receipt
    // is an evidence limit, not an invented remaining completion condition.
    let unmet = row
        .items
        .iter()
        .find(|i| i.state != "pass" && i.kind != "review, not acceptance");
    let (decision, target, check) = if let Some(item) = failed {
        (
            "stop and repair",
            format!("Resolve {}: {}", safe(&item.label), safe(&item.state)),
            format!(
                "Rerun its check against the current PR head: {}",
                safe(&item.evidence)
            ),
        )
    } else if let Some(gap) = row.gaps.first() {
        (
            "stop and repair",
            format!("Resolve the source gap: {}", safe(gap)),
            "Read the repaired source and verify its exact result identity.".into(),
        )
    } else if let Some(gate) = row.gates.first() {
        (
            "stop and repair",
            format!("Resolve the owed action: {}", safe(gate)),
            "Obtain the required external decision; a draft cannot grant permission.".into(),
        )
    } else if let Some(item) = unmet {
        (
            "stop and repair",
            format!(
                "Establish matching evidence for {}: {}",
                safe(&item.label),
                safe(&item.state)
            ),
            format!(
                "Inspect the exact result externally: {}",
                safe(&item.evidence)
            ),
        )
    } else {
        (
            "proceed",
            "Observe the accepted behavior before choosing another feature".into(),
            "Ask the operator to try the accepted behavior and report a counterexample.".into(),
        )
    };
    let mut text = format!(
        "{target}.\nUncertainty: does the accepted result satisfy the unchanged goal?\n\
         Keep the confirmed goal and its constraints unchanged; do not start another feature or bypass gates.\n\
         Disprove this slice: {check}"
    );
    if let crate::sdlc::Goal::Matched(goal) = &row.goal {
        text.push_str("\n\nConfirmed goal (unchanged source):\n");
        text.push_str(goal);
    }
    let provenance = format!(
        "Deterministic Taste guess after a report of external acceptance; not a verified verdict. Run {} ({}); seed {}; {} @{}. Decision: {decision}. Basis: {}. Gates owed: {}. Gaps: {}.",
        row.id,
        row.slug,
        row.seed,
        row.url.as_deref().unwrap_or("unknown"),
        row.head.as_deref().unwrap_or("unknown"),
        row.items
            .iter()
            .map(|i| format!("{}: {}", i.label, i.state))
            .collect::<Vec<_>>()
            .join("; "),
        row.gates.join("; "),
        row.gaps.join("; ")
    );
    Pause {
        lines: vec![
            "Tinkery's guess · accepted externally (your report, not verified)".into(),
            "Outcome: unobserved; acceptance alone is not user feedback.".into(),
            "Evidence: snapshot only; independent review and user outcomes remain unverified."
                .into(),
            format!("Learning: {target}."),
            "Futures: unknown; these sources show no code-level changeability evidence.".into(),
            format!("Decision: {decision}; resolve uncertainty before stacking features."),
            format!("Next: {target}. Keep the goal unchanged. Check: {check}"),
            "t use and edit in Think · nothing sent until F2".into(),
            String::new(),
        ],
        draft: DraftSuggestion { text, provenance },
    }
}
