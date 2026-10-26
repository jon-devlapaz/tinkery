//! Closing review and confirmation risks belong to the ledger, not to a phase or model formatter.
//! Review records establish what was considered, never that work or checks actually ran.

use super::{Value, require};

/// A drafting step's review; contradictions must return as a question, never accompany a seed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionReview {
    /// Conflicts still blocking the draft; a seed requires none.
    pub contradictions: Vec<String>,
    /// Settled choices, not claims of completed implementation.
    pub decided: Vec<String>,
    /// Work actually observed complete; an empty list is honest when none was verified.
    pub done: Vec<String>,
    /// One counter-case for every acceptance statement, with its exact wording.
    pub failure_cases: Vec<FailureCase>,
}

/// A case that must fail an acceptance check, not a report that it was executed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FailureCase {
    /// The check's statement, unchanged.
    pub statement: String,
    /// What must not pass this check.
    pub case: String,
}

fn case_count(cases: usize, checks: usize) -> Result<(), String> {
    require(
        cases == checks,
        format!(
            "Completion review needs exactly one failure case per check: received {cases} for {checks} check{}. Return the cases in check order{}",
            if checks == 1 { "" } else { "s" },
            // #432: replies listed one case per failure mode, not per check, even after feedback.
            if cases > checks {
                "; put every failure mode of one check in its one string."
            } else {
                "."
            }
        ),
    )
}

impl CompletionReview {
    /// Attach exact check statements to ordered cases, refusing incomplete coverage or missing statements.
    /// This creates review records, never evidence that a case ran.
    pub fn pair_cases(cases: Vec<String>, checks: &Value) -> Result<Vec<FailureCase>, String> {
        let checks = checks.items();
        case_count(cases.len(), checks.len())?;
        cases
            .into_iter()
            .zip(checks)
            .enumerate()
            .map(|(index, (case, check))| {
                let statement = check["statement"].string().ok_or_else(|| {
                    format!(
                        "Check {} needs a statement before its failure case can be paired.",
                        index + 1
                    )
                })?;
                Ok(FailureCase {
                    statement: statement.to_owned(),
                    case,
                })
            })
            .collect()
    }

    /// Refuse blockers, blank review entries or missing, repeated and foreign failure cases.
    /// This checks the review's structure and coverage, not the truth of model judgments.
    pub fn validate(&self, checks: &Value) -> Result<(), String> {
        require(
            self.contradictions.is_empty(),
            "Completion review found a blocker. Return it as a question, not a seed draft.",
        )?;
        require(
            self.decided
                .iter()
                .chain(&self.done)
                .all(|s| !s.trim().is_empty()),
            "Completion review decided and done entries must be non-empty words.",
        )?;
        let statements: Vec<_> = checks
            .items()
            .iter()
            .map(|c| c["statement"].string())
            .collect();
        case_count(self.failure_cases.len(), statements.len())?;
        for (index, failure) in self.failure_cases.iter().enumerate() {
            require(
                !failure.case.trim().is_empty(),
                format!(
                    "Completion review failure case {} is blank; describe what must not pass the check.",
                    index + 1
                ),
            )?;
        }
        for (index, statement) in statements.iter().enumerate() {
            require(
                statements.iter().filter(|s| *s == statement).count() == 1
                    && self
                        .failure_cases
                        .iter()
                        .filter(|f| Some(f.statement.as_str()) == *statement)
                        .count()
                        == 1,
                format!(
                    "Completion review needs exactly one failure case for check {}. Its statement was changed or repeated; return case strings in check order instead of copying statements.",
                    index + 1
                ),
            )?;
        }
        Ok(())
    }
}

fn accepted(node: &Value) -> bool {
    node["recommendation"].string().is_some() && node["answer"] == node["recommendation"]
}

/// Plain confirmation risks, in risk order, with exact-match suggestion counts. The confirmed
/// goal and facts are not choices. Changed answers are named without guessing semantic intent.
/// A choice appears once even when several risk labels apply.
pub fn confirmation_summary(ledger: &Value) -> Vec<String> {
    let decisions: Vec<_> = ledger["nodes"]
        .items()
        .iter()
        .filter(|n| {
            n["kind"].is("decision") && n["status"].is("settled") && n["id"] != ledger["origin"]
        })
        .collect();
    let count = decisions.iter().filter(|node| accepted(node)).count();
    let mut lines = vec![format!(
        "Accepted as suggested: {count}; chosen otherwise: {}.",
        decisions.len() - count
    )];
    let mut risks = Vec::new();
    for node in decisions {
        let mut labels = Vec::new();
        if node["history"]
            .items()
            .iter()
            .any(|h| h["state"]["status"].is("settled") && h["state"]["answer"] != node["answer"])
        {
            labels.push("reverses an earlier answer");
        }
        let reversed = !labels.is_empty();
        if node["authority"].is("delegated") {
            labels.push("delegated");
        }
        let delegated = node["authority"].is("delegated");
        if accepted(node) {
            labels.push("accepted exactly as suggested");
        }
        if !labels.is_empty() {
            let title = node["label"]
                .string()
                .unwrap_or_else(|| node["id"].string().unwrap_or(""));
            risks.push((
                if reversed {
                    0
                } else if delegated {
                    1
                } else {
                    3
                },
                format!(
                    "{title}: {} — {}",
                    node["answer"].string().unwrap_or(""),
                    labels.join("; ")
                ),
            ));
        }
    }
    for assumed in ledger["assumed"].items() {
        risks.push((
            2,
            format!(
                "Assumed: {} — {}",
                assumed["text"].string().unwrap_or(""),
                assumed["why"].string().unwrap_or("")
            ),
        ));
    }
    risks.sort_by_key(|(priority, _)| *priority);
    lines.extend(risks.into_iter().map(|(_, line)| line));
    lines
}

#[cfg(test)]
mod tests;
