//! Decode model review transport; the ledger owns check pairing and strict saved-review coverage.

use crate::seed_me::ledger::{CompletionReview, FailureCase};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    contradictions: Vec<String>,
    decided: Vec<String>,
    done: Vec<String>,
    failure_cases: Cases,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Cases {
    Named(Vec<FailureCase>),
    Ordered(Vec<String>),
}

/// Ordered strings use check positions; historical named cases keep their strict exact-word contract.
pub(super) fn decode(value: Value, checks: &Value) -> Result<Box<CompletionReview>, String> {
    let wire: Review = serde_json::from_value(value).map_err(|_| {
        "The seed reply needs a typed completion review: contradictions, decided, done and failure_cases. Retry the step.".to_owned()
    })?;
    let failure_cases = match wire.failure_cases {
        Cases::Named(cases) => cases,
        Cases::Ordered(cases) => CompletionReview::pair_cases(cases, &checks.clone().into())?,
    };
    Ok(Box::new(CompletionReview {
        contradictions: wire.contradictions,
        decided: wire.decided,
        done: wire.done,
        failure_cases,
    }))
}
