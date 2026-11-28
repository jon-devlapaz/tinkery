//! Shape's view of where one session's seed stands: the shared classification, plus the question,
//! findings, decisions, review lines and draft text Shape shows. Read-only: it never publishes,
//! confirms or ends anything, and keeps no copy of the ledger.

use super::answering::{Authority, NOTHING_FOUND, UNFOUND, claim_suffix};
use crate::seed_me::{
    ledger,
    ready::{self, Stand},
    retry::Base,
};
use serde_json::Value;
use std::path::Path;

/// One session as Shape shows it: the confirmed goal exactly as recorded, and where its seed stands.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaping {
    /// Exact confirmed-seed identity from this verified read; navigation only, never permission.
    pub ready_target: Option<ready::SeedTarget>,
    /// The ledger's confirmed goal, byte for byte; `None` when the session records none.
    pub goal: Option<String>,
    /// The saved record explicitly lacks a success criterion; unknown older records are not absence.
    pub success_open: bool,
    /// Where the seed stands.
    pub stage: Shaped,
    /// The ledger version this was read at; a change made from what's shown must name it.
    pub version: Option<u64>,
    /// Each settled decision after the goal, in ledger order, exactly as recorded.
    pub decided: Vec<Decided>,
    /// Open questions in ledger order, naming the earlier answers each held question waits for.
    pub open: Vec<String>,
    /// Confirmation attribution and consumer requirements for a verified seed; not permission.
    pub confirmation_note: Option<String>,
    /// Ledger-computed risks, displayed without recomputing them.
    pub confirmation: Vec<String>,
    /// The ledger's latest completed teach-back for the exact draft, with display labels.
    pub teach_back: Vec<String>,
    /// The single eligible deferred prerequisite, offered only while Waiting.
    pub deferred: Option<ledger::DeferredPrerequisite>,
    /// The exact draft checks, in the order shown.
    pub(crate) checks: Vec<ledger::DraftCheck>,
    /// Drafting guesses bound to the exact draft, or a notice when the receipt was invalid.
    pub(crate) review: Option<Result<ledger::CompletionReview, &'static str>>,
    /// The human sign-off the draft's or ready seed's judged checks require, if any.
    pub signoff: Option<String>,
}

impl Shaping {
    /// The draft's proposed failure for this check if its statement still matches.
    pub(crate) fn failure_case(&self, check: &ledger::DraftCheck) -> Option<&str> {
        self.review
            .as_ref()
            .and_then(|review| review.as_ref().ok())
            .and_then(|review| review.failure_cases.get(check.index))
            .filter(|case| case.statement == check.statement())
            .map(|case| case.case.as_str())
    }

    /// Drafting guesses followed by ledger risks. The screen may place cases beside their checks.
    pub(crate) fn review_lines(&self, cases_in_cards: bool) -> impl Iterator<Item = String> + '_ {
        let lines = match &self.review {
            Some(Ok(review)) if cases_in_cards => review.supporting_lines(),
            Some(Ok(review)) => review.report_lines(),
            Some(Err(notice)) => vec![(*notice).into()],
            None => vec![],
        };
        lines.into_iter().chain(self.confirmation.iter().cloned())
    }

    /// The draft as Shape shows it, when this is a draft whose text was read. The stored bytes
    /// and their order don't change; only the view sets the criteria apart.
    pub(crate) fn seed_view(&self) -> Option<SeedView<'_>> {
        let Shaped::Drafted {
            text: Some(seed), ..
        } = &self.stage
        else {
            return None;
        };
        let (before, criteria, after) = match seed.split_once("\n## Acceptance criteria\n") {
            Some((before, section)) => {
                let end = section.find("\n## ").map_or(section.len(), |end| end + 1);
                (before, Some(&section[..end]), &section[end..])
            }
            None => (seed.as_str(), None, ""),
        };
        let criteria = criteria.map(|criteria| {
            if self.checks.is_empty() {
                Criteria::Readable(ledger::readable_acceptance(criteria).trim_end().to_owned())
            } else {
                Criteria::Checks(&self.checks)
            }
        });
        // The first block's status and revision lines are metadata, not the seed's words.
        let mut header = true;
        let prose = before
            .lines()
            .chain(after.trim_end_matches('\n').split_terminator('\n'))
            .filter(|line| {
                header &= !line.trim().is_empty();
                !(header && (line.starts_with("status:") || line.starts_with("revision:")))
            })
            .map(
                |line| match line.strip_prefix("## ").or_else(|| line.strip_prefix("# ")) {
                    Some(heading) => SeedLine::Heading(heading),
                    None => SeedLine::Words(line),
                },
            )
            .collect();
        Some(SeedView {
            prose,
            criteria,
            nothing_found: seed.contains(NOTHING_FOUND),
        })
    }
}

/// A seed draft as Shape shows it: its criteria first, then its other words in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SeedView<'a> {
    /// The seed's lines outside its acceptance criteria, in order, without its metadata lines.
    pub prose: Vec<SeedLine<'a>>,
    /// The acceptance criteria, when the seed has that section.
    pub criteria: Option<Criteria<'a>>,
    /// Whether the evidence section says no step recorded what it checked.
    pub nothing_found: bool,
}

/// One line of a seed's words, exactly as saved apart from a heading's `#` marks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SeedLine<'a> {
    /// A section or title heading.
    Heading(&'a str),
    /// Any other line, blank lines included.
    Words(&'a str),
}

/// A draft's acceptance criteria: its exact check records, or the section's readable words when
/// no records match the draft.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Criteria<'a> {
    /// The exact draft checks, in order.
    Checks(&'a [ledger::DraftCheck]),
    /// The section in readable words.
    Readable(String),
}

/// One settled decision: what was decided, the answer as recorded, and who settled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    /// The node's label, or its id when it has none.
    pub title: String,
    /// The recorded answer, byte for byte.
    pub answer: String,
    /// Who settled it: only [`Authority::User`] means the operator's own words.
    pub authority: Authority,
}

/// Where a session's seed stands. Each says only what the ledger's files show; nothing is guessed.
#[derive(Debug, Clone, PartialEq)]
pub enum Shaped {
    /// The current question and any recorded choices, shared by the screen and command.
    Question {
        /// The node's id, which an answer settles.
        id: String,
        /// The node's label, or its id when it has none.
        title: String,
        /// What this question settles, available before answering or deferring.
        gate: String,
        /// The question and its alternatives, with the undo costs that govern disclosure.
        question: ledger::QuestionView,
        /// The node's evidence, one entry per line: what was checked, then what was observed.
        found: Vec<String>,
        /// The recorded suggestion, when one is recorded.
        suggestion: Option<String>,
    },
    /// A settled node flagged by the ledger, shown before any question or seed.
    Review {
        /// The node to keep or, for an operator-answered decision, revise.
        id: String,
        /// Ledger-owned eligibility: findings and the confirmed goal are keep-only.
        revisable: bool,
        /// The node's label or ID.
        title: String,
        /// The unchanged settled answer.
        answer: String,
        /// The ledger's reasons and their source facts.
        found: Vec<String>,
    },
    /// Active with no current question and no seed: no next question has been recorded.
    Waiting,
    /// A seed draft waiting for the operator to confirm it.
    Drafted {
        /// The draft's revision label, when it has one.
        revision: Option<String>,
        /// The draft exactly as saved, when it could be read.
        text: Option<String>,
    },
    /// A confirmed seed that `status` verifies against this session's confirmation record. It
    /// stays ready after the session is completed, since intake needs only the verified seed.
    Ready {
        /// The confirmed revision label, when it has one.
        revision: Option<String>,
        /// Whether the session is completed, so nothing more can be changed or ended.
        ended: bool,
    },
    /// A seed that says it is confirmed but that `status` can't verify, with Tinkery's reason.
    Unverified {
        /// Tinkery's reason for refusing verification.
        reason: String,
    },
    /// A practice session's seed, never a real one.
    Practice,
    /// Completed or stopped: nothing to shape.
    Ended {
        /// The ledger's status word.
        status: String,
    },
    /// The files say something this module doesn't recognise. Never guessed.
    Unknown {
        /// What wasn't recognised, in plain words.
        what: String,
    },
}

/// Read `session` through Tinkery's ledger and verifier; run disk reads off the paint path.
/// It offers no intake command: Approve owns the one that starts a build.
pub fn read(session: &Path) -> Result<Shaping, String> {
    read_with_base(session).map(|(shaping, _)| shaping)
}

/// Include the retry checkout in the same reading, so Shape uses the source it displays.
pub(crate) fn read_with_base(session: &Path) -> Result<(Shaping, Option<Base>), String> {
    let mut standing = ready::standing(session)?;
    let retry = standing.retry.take();
    Ok((from_standing(standing), retry))
}

/// Project the contract and its matching checks from a single verified read. Opens no files.
pub(super) fn from_standing(standing: ready::Standing) -> Shaping {
    let mut shaping = classify(&standing.json, &standing.status, standing.stand);
    shaping.ready_target = standing.target;
    shaping.confirmation = ledger::confirmation_summary(&standing.ledger);
    shaping.deferred = standing.deferred;
    shaping.confirmation_note = standing.note.map(Into::into);
    if let Some(checks) = &standing.checks {
        shaping.signoff = ledger::human_signoff(&checks.records);
    }
    if let Shaped::Drafted { text, revision } = &mut shaping.stage {
        *text = standing
            .seed_bytes
            .and_then(|bytes| String::from_utf8(bytes).ok());
        if let (Some(seed), Some(revision)) = (text.as_ref(), revision.as_ref()) {
            if let Some(checks) = &standing.checks {
                shaping.checks = ledger::draft_checks(&checks.records).collect();
            }
            shaping.teach_back = ledger::teach_back_report(&standing.ledger, revision, seed);
            shaping.review = ledger::completion_review(&standing.ledger, revision, seed);
        }
    }
    shaping
}

/// Where the seed stands, from the ledger and its verified `status`. An open question comes
/// before a seed, so a reopened decision is never hidden behind an older draft.
fn classify(ledger: &Value, status: &Value, stand: Stand) -> Shaping {
    let text = |value: &Value, key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
    };
    let goal = ledger
        .get("goal")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let version = ledger.get("version").and_then(Value::as_u64);
    let nodes = ledger
        .get("nodes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let origin = text(ledger, "origin");
    let decided: Vec<Decided> = nodes
        .iter()
        .filter(|n| {
            text(n, "kind").as_deref() == Some("decision")
                && text(n, "status").as_deref() == Some("settled")
                && text(n, "id") != origin
                && n.get("answer").and_then(Value::as_str) != goal.as_deref()
        })
        .filter_map(|n| {
            Some(Decided {
                title: text(n, "label").or_else(|| text(n, "id"))?,
                answer: text(n, "answer")?,
                authority: Authority::of(text(n, "authority").as_deref()),
            })
        })
        .collect();
    let success_open = ledger
        .pointer("/draft/outcome")
        .and_then(Value::as_str)
        .is_some_and(|value| value.trim().is_empty())
        && goal.as_deref().is_some_and(|goal| {
            !goal.lines().any(|line| {
                line.strip_prefix("Done when:")
                    .is_some_and(|value| !value.trim().is_empty())
            })
        });
    let shaping = |goal: Option<String>, stage| Shaping {
        ready_target: None,
        goal,
        success_open,
        stage,
        version,
        decided: decided.clone(),
        open: open_questions(nodes, &status["waiting_on"]),
        confirmation_note: None,
        confirmation: Vec::new(),
        teach_back: Vec::new(),
        deferred: None,
        checks: vec![],
        review: None,
        signoff: None,
    };
    let stage = match stand {
        Stand::Unknown(what) => Shaped::Unknown { what: what.into() },
        Stand::Ended(status) => Shaped::Ended { status },
        Stand::Ready { revision, ended } => Shaped::Ready { revision, ended },
        Stand::Waiting => Shaped::Waiting,
        Stand::Practice => Shaped::Practice,
        Stand::Drafted(revision) => Shaped::Drafted {
            revision,
            text: None,
        },
        Stand::Unverified(reason) => Shaped::Unverified { reason },
        Stand::Review(id) => {
            let node = nodes
                .iter()
                .find(|n| n["id"] == id.as_str())
                .unwrap_or(&Value::Null);
            let found = status["review_flags"][&id]
                .as_array()
                .into_iter()
                .flatten()
                .map(|flag| {
                    let source = flag["because"].as_str().unwrap_or("");
                    let answer = nodes
                        .iter()
                        .find(|n| n["id"] == source)
                        .and_then(|n| n["answer"].as_str())
                        .unwrap_or("");
                    format!(
                        "{}: {source}: {answer}",
                        flag["why"].as_str().unwrap_or("review")
                    )
                })
                .collect();
            Shaped::Review {
                revisable: status["revisable"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|v| v == &id)),
                title: text(node, "label").unwrap_or_else(|| id.clone()),
                id,
                answer: text(node, "answer").unwrap_or_default(),
                found,
            }
        }
        Stand::Question(id) => {
            let node = nodes
                .iter()
                .find(|n| text(n, "id").as_deref() == Some(&id))
                .unwrap_or(&Value::Null);
            let question = match ledger::read_question(node) {
                Ok(question) => question,
                Err(what) => return shaping(goal, Shaped::Unknown { what }),
            };
            let found = std::iter::once(node)
                .chain(
                    node["supported_by"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|id| nodes.iter().find(|n| n["id"] == *id)),
                )
                .flat_map(|node| {
                    node["evidence"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(move |entry| format_finding(node, entry))
                })
                .collect();
            Shaped::Question {
                gate: text(node, "gate").unwrap_or_default(),
                title: text(node, "label").unwrap_or_else(|| id.clone()),
                id,
                question,
                found,
                suggestion: text(node, "recommendation"),
            }
        }
    };
    shaping(goal, stage)
}

fn open_questions(nodes: &[Value], waits: &Value) -> Vec<String> {
    let title = |node: &Value| {
        node["label"]
            .as_str()
            .or_else(|| node["id"].as_str())
            .unwrap_or("")
            .to_owned()
    };
    nodes
        .iter()
        .filter(|node| node["kind"] == "decision" && node["status"] == "unresolved")
        .map(|node| {
            let parents: Vec<_> = node["id"]
                .as_str()
                .and_then(|id| waits[id].as_array())
                .into_iter()
                .flatten()
                .filter_map(|id| nodes.iter().find(|parent| parent["id"] == *id))
                .map(&title)
                .collect();
            if parents.is_empty() {
                title(node)
            } else {
                format!("{} — waits for: {}", title(node), parents.join(", "))
            }
        })
        .collect()
}

fn format_finding(node: &Value, entry: &Value) -> Option<String> {
    let mut text = format_receipt(entry)?;
    if node["claim"]["type"].is_string() {
        text.push_str(&claim_suffix(&node["claim"], None));
    }
    Some(text)
}

fn format_receipt(entry: &Value) -> Option<String> {
    let text = |value: &Value, key: &str| {
        value[key]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
    };
    match entry {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Object(_) => {
            let unfound = text(entry, "check")
                .and_then(|c| c.split_once(UNFOUND).map(|(_, why)| why.to_owned()))
                .map(|why| format!(" (citation not found: {why})"))
                .unwrap_or_default();
            match (text(entry, "checked"), text(entry, "observed")) {
                (Some(checked), Some(observed)) => Some(format!("{checked}: {observed}{unfound}")),
                (None, observed) => observed,
                (Some(_), None) => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn classify(ledger: &Value, status: &Value) -> Shaping {
        super::classify(ledger, status, ready::stand(ledger, status))
    }

    fn ledger() -> Value {
        json!({"goal": "TEST goal\nWhy: it matters.", "nodes": [
            {"id": "goal", "kind": "decision", "status": "settled"},
            {"id": "storage", "kind": "decision", "status": "unresolved", "label": "Where records live",
             "question": "Keep records local or synced?", "gate": "storage choice",
             "evidence": ["README.md:3 says local only", {"observed": "no sync client found", "check": "rg sync"}, 5],
             "recommendation": "Local, matching the README."}
        ]})
    }

    fn status(current: Option<&str>, seed: Value, handoff: Value) -> Value {
        json!({"status": "active", "current_question": current, "seed": seed, "handoff": handoff})
    }

    #[test]
    fn missing_success_is_known_absence_not_a_guess_about_older_records() {
        let mut saved = ledger();
        let waiting = status(None, Value::Null, Value::Null);
        assert!(!classify(&saved, &waiting).success_open);
        saved["draft"] = json!({"outcome": null});
        assert!(!classify(&saved, &waiting).success_open);
        saved["draft"]["outcome"] = " ".into();
        assert!(classify(&saved, &waiting).success_open);
        saved["draft"]["outcome"] = "The complete scenario passes.".into();
        assert!(!classify(&saved, &waiting).success_open);
        saved["draft"]["outcome"] = "".into();
        saved["goal"] = "TEST goal\nDone when: The complete scenario passes.".into();
        assert!(!classify(&saved, &waiting).success_open);
    }

    #[test]
    fn the_current_question_shows_what_the_ledger_records_and_nothing_else() {
        let shaped = classify(
            &ledger(),
            &status(Some("storage"), Value::Null, Value::Null),
        );
        assert_eq!(shaped.goal.as_deref(), Some("TEST goal\nWhy: it matters."));
        assert_eq!(
            shaped.stage,
            Shaped::Question {
                id: "storage".into(),
                gate: "storage choice".into(),
                title: "Where records live".into(),
                question: ledger::QuestionView {
                    ask: "Keep records local or synced?".into(),
                    options: vec![],
                    against: None,
                },
                found: vec![
                    "README.md:3 says local only".into(),
                    "no sync client found".into()
                ],
                suggestion: Some("Local, matching the README.".into()),
            }
        );
        let mut bare = ledger();
        for key in ["label", "question", "recommendation", "evidence"] {
            bare["nodes"][1].as_object_mut().unwrap().remove(key);
        }
        let shaped = classify(&bare, &status(Some("storage"), Value::Null, Value::Null));
        assert_eq!(
            shaped.stage,
            Shaped::Question {
                id: "storage".into(),
                gate: "storage choice".into(),
                title: "storage".into(),
                question: ledger::QuestionView {
                    ask: "storage choice".into(),
                    options: vec![],
                    against: None,
                },
                found: vec![],
                suggestion: None,
            }
        );
    }

    #[test]
    fn settled_decisions_after_the_goal_are_read_exactly_with_who_settled_them() {
        let mut ledger = ledger();
        ledger["origin"] = "goal".into();
        let nodes = ledger["nodes"].as_array_mut().unwrap();
        nodes[0]["answer"] = "TEST goal\nWhy: it matters.".into();
        nodes.push(
            json!({"id": "format", "kind": "decision", "status": "settled",
            "label": "Note format", "answer": " Markdown,\nas written. ", "authority": "user"}),
        );
        nodes.push(
            json!({"id": "lint", "kind": "decision", "status": "settled",
            "answer": "Clippy.", "authority": "delegated"}),
        );
        nodes.push(json!({"id": "repo", "kind": "fact", "status": "settled",
            "answer": "A Rust crate.", "authority": "evidence"}));
        let shaped = classify(&ledger, &status(None, Value::Null, Value::Null));
        assert_eq!(
            shaped.decided,
            vec![
                Decided {
                    title: "Note format".into(),
                    answer: " Markdown,\nas written. ".into(),
                    authority: Authority::User,
                },
                Decided {
                    title: "lint".into(),
                    answer: "Clippy.".into(),
                    authority: Authority::Delegated,
                },
            ]
        );
    }

    #[test]
    fn a_seed_is_ready_only_when_status_verifies_it() {
        let confirmed = json!({"line_1": "status: confirmed for intake", "revision": "r2"});
        let ready = classify(
            &ledger(),
            &status(None, confirmed.clone(), json!({"verified": true})),
        );
        assert_eq!(
            ready.stage,
            Shaped::Ready {
                revision: Some("r2".into()),
                ended: false,
            }
        );
        let stale = classify(
            &ledger(),
            &status(
                None,
                confirmed.clone(),
                json!({"verified": false, "reason": "the seed changed after revision r2 was confirmed"}),
            ),
        );
        assert_eq!(
            stale.stage,
            Shaped::Unverified {
                reason: "the seed changed after revision r2 was confirmed".into()
            }
        );
        let silent = classify(&ledger(), &status(None, confirmed, Value::Null));
        assert!(matches!(silent.stage, Shaped::Unverified { .. }));
    }

    #[test]
    fn every_other_stage_is_said_plainly_and_nothing_is_guessed() {
        let drafted = json!({"line_1": "status: draft", "revision": "r1"});
        let cases = [
            (status(None, Value::Null, Value::Null), Shaped::Waiting),
            (
                status(None, drafted, Value::Null),
                Shaped::Drafted {
                    revision: Some("r1".into()),
                    text: None,
                },
            ),
            (
                status(None, json!({"line_1": "status: simulated"}), Value::Null),
                Shaped::Practice,
            ),
            (
                json!({"status": "completed"}),
                Shaped::Ended {
                    status: "completed".into(),
                },
            ),
        ];
        for (status, expected) in cases {
            assert_eq!(classify(&ledger(), &status).stage, expected);
        }
        let unknowns = [
            (ledger(), json!({})),
            (ledger(), status(Some("missing"), Value::Null, Value::Null)),
            (
                json!({"goal": null}),
                status(None, Value::Null, Value::Null),
            ),
            (
                ledger(),
                status(None, json!({"line_1": "hello"}), Value::Null),
            ),
        ];
        for (ledger, status) in unknowns {
            assert!(
                matches!(classify(&ledger, &status).stage, Shaped::Unknown { .. }),
                "{status}"
            );
        }
    }
}
