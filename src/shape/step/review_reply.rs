//! Pair model failure cases with their checks; saved checks and reviews keep the ledger's formats.

use crate::seed_me::ledger::CompletionReview;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    contradictions: Vec<String>,
    decided: Vec<String>,
    done: Vec<String>,
    failure_cases: Option<Vec<String>>,
}

/// Consume each check's model-only case; strictly accept older ordered cases, never both formats.
pub(super) fn decode(value: Value, checks: &mut [Value]) -> Result<Box<CompletionReview>, String> {
    let cases: Vec<_> = checks
        .iter_mut()
        .map(|check| check.as_object_mut().and_then(|c| c.remove("failure_case")))
        .collect();
    let attached = cases.iter().any(Option::is_some);
    if attached && value.get("failure_cases").is_some() {
        return Err("Put failure_case on each check, not also in review.failure_cases. No cases were combined or discarded.".into());
    }
    let wire: Review = serde_json::from_value(value).map_err(|_| {
        concat!(
            "The seed reply needs a typed completion review: contradictions, decided and done. ",
            "Put failure_case on each check. Legacy failure_cases must be a list of strings in check order, not statement/case objects. Retry the step."
        ).to_owned()
    })?;
    let cases = match wire.failure_cases {
        Some(cases) => cases,
        None => cases
            .into_iter()
            .enumerate()
            .map(|(index, case)| {
                case.and_then(|c| c.as_str().map(str::to_owned))
                    .filter(|c| !c.trim().is_empty())
                    .ok_or_else(|| format!("Check {} needs a non-empty failure_case string describing what must not pass it.", index + 1))
            })
            .collect::<Result<_, _>>()?,
    };
    let failure_cases = CompletionReview::pair_cases(cases, &Value::Array(checks.to_vec()).into())?;
    Ok(Box::new(CompletionReview {
        contradictions: wire.contradictions,
        decided: wire.decided,
        done: wire.done,
        failure_cases,
    }))
}
