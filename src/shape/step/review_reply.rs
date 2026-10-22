//! Decode model review transport; the ledger owns check pairing and strict saved-review coverage.

use crate::seed_me::ledger::CompletionReview;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    contradictions: Vec<String>,
    decided: Vec<String>,
    done: Vec<String>,
    failure_cases: Vec<String>,
}

/// Only ordered case strings cross the model boundary; the ledger pairs and validates saved reviews.
pub(super) fn decode(value: Value, checks: &Value) -> Result<Box<CompletionReview>, String> {
    let wire: Review = serde_json::from_value(value).map_err(|_| {
        concat!(
            "The seed reply needs a typed completion review: contradictions, decided, done and failure_cases. ",
            "failure_cases must be a list of strings in check order, not statement/case objects. Retry the step."
        ).to_owned()
    })?;
    let failure_cases = CompletionReview::pair_cases(wire.failure_cases, &checks.clone().into())?;
    Ok(Box::new(CompletionReview {
        contradictions: wire.contradictions,
        decided: wire.decided,
        done: wire.done,
        failure_cases,
    }))
}
