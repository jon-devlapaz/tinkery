//! Where one session's seed stands, read only through Seed Me's helper (`read` and `status`),
//! plus the seed draft's text when one waits for confirmation. Read-only: it never publishes,
//! confirms or ends anything, and keeps no copy of the ledger.

use super::answering::UNFOUND;
use serde_json::Value;
use std::path::Path;

/// One session as Shape shows it: the confirmed goal exactly as recorded, and where its seed stands.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaping {
    /// The ledger's confirmed goal, byte for byte; `None` when the session records none.
    pub goal: Option<String>,
    /// Where the seed stands.
    pub stage: Shaped,
    /// The ledger version this was read at; a change made from what's shown must name it.
    pub version: Option<u64>,
    /// Each settled decision after the goal, in ledger order, exactly as recorded.
    pub decided: Vec<Decided>,
}

/// One settled decision: what was decided, the answer as recorded, and who settled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    /// The node's label, or its id when it has none.
    pub title: String,
    /// The recorded answer, byte for byte.
    pub answer: String,
    /// Seed Me's authority word: `user` means the operator's own words; anything else isn't.
    pub authority: String,
}

/// Where a session's seed stands. Each says only what Seed Me's files show; nothing is guessed.
#[derive(Debug, Clone, PartialEq)]
pub enum Shaped {
    /// Seed Me's current question, as the ledger records it. The ledger keeps no options.
    Question {
        /// The node's id, which an answer settles.
        id: String,
        /// The node's label, or its id when it has none.
        title: String,
        /// The question, or the decision to make when no question is recorded.
        ask: String,
        /// The node's evidence, one entry per line: what was checked, then what was observed.
        found: Vec<String>,
        /// Seed Me's suggestion, when one is recorded.
        suggestion: Option<String>,
    },
    /// Active with no current question and no seed: Seed Me hasn't asked its next question.
    Waiting,
    /// A seed draft waiting for the operator to confirm it.
    Drafted {
        /// The draft's revision label, when it has one.
        revision: Option<String>,
        /// The draft exactly as saved, when it could be read.
        text: Option<String>,
    },
    /// A confirmed seed that `status` verifies against this session's confirmation record.
    Ready {
        /// The confirmed revision label, when it has one.
        revision: Option<String>,
    },
    /// A seed that says it is confirmed but that `status` can't verify, with Seed Me's reason.
    Unverified {
        /// Seed Me's own words for why.
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

/// Read `session` through the helper beside `skill` (`read`, then `status`). Blocks on two helper
/// calls; run it off the paint path. Refuses a missing helper, a helper refusal or invalid JSON.
pub fn read(skill: &Path, session: &Path) -> Result<Shaping, String> {
    let skill = skill
        .canonicalize()
        .map_err(|e| format!("Cannot locate Seed Me: {e}"))?;
    let helper = skill
        .parent()
        .ok_or("Seed Me skill has no directory")?
        .join("scripts/session.py");
    if !helper.is_file() {
        return Err(format!("Missing official helper: {}", helper.display()));
    }
    let call = |command: &str| -> Result<Value, String> {
        let text = super::save::output(&helper, &[command.as_ref(), session.as_os_str()])?;
        serde_json::from_str(&text).map_err(|e| format!("Invalid helper JSON: {e}"))
    };
    let ledger = call("read")?;
    let status = call("status")?;
    let mut shaping = classify(&ledger, &status);
    if let Shaped::Drafted { text, .. } = &mut shaping.stage {
        *text = std::fs::read_to_string(session.join("seed-contract.md")).ok();
    }
    Ok(shaping)
}

/// Where the seed stands, from the helper's `read` and `status` output. An open question comes
/// before a seed, so a reopened decision is never hidden behind an older draft.
fn classify(ledger: &Value, status: &Value) -> Shaping {
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
                authority: text(n, "authority").unwrap_or_else(|| "unrecorded".into()),
            })
        })
        .collect();
    let shaping = |goal: Option<String>, stage| Shaping {
        goal,
        stage,
        version,
        decided: decided.clone(),
    };
    let unknown = |what: &str| shaping(goal.clone(), Shaped::Unknown { what: what.into() });
    let Some(state) = text(status, "status") else {
        return unknown("Seed Me reported no session status.");
    };
    if state != "active" {
        return shaping(goal, Shaped::Ended { status: state });
    }
    if goal.as_deref().is_none_or(|g| g.trim().is_empty()) {
        return unknown("This session has no confirmed goal.");
    }
    if let Some(id) = text(status, "current_question") {
        let node = nodes.iter().find(|n| text(n, "id").as_deref() == Some(&id));
        let Some(node) = node else {
            return unknown("Seed Me's current question isn't in its ledger.");
        };
        let Some(ask) = text(node, "question").or_else(|| text(node, "gate")) else {
            return unknown("Seed Me's current question has no wording.");
        };
        let found = node
            .get("evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| match entry {
                Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
                Value::Object(_) => {
                    let unfound = text(entry, "check")
                        .and_then(|c| c.split_once(UNFOUND).map(|(_, why)| why.to_owned()))
                        .map(|why| format!(" (citation not found: {why})"))
                        .unwrap_or_default();
                    match (text(entry, "checked"), text(entry, "observed")) {
                        (Some(checked), Some(observed)) => {
                            Some(format!("{checked}: {observed}{unfound}"))
                        }
                        (None, observed) => observed,
                        (Some(_), None) => None,
                    }
                }
                _ => None,
            })
            .collect();
        return shaping(
            goal,
            Shaped::Question {
                title: text(node, "label").unwrap_or_else(|| id.clone()),
                id,
                ask,
                found,
                suggestion: text(node, "recommendation"),
            },
        );
    }
    let seed = status.get("seed").filter(|s| !s.is_null());
    let Some(seed) = seed else {
        return shaping(goal, Shaped::Waiting);
    };
    let revision = text(seed, "revision");
    let handoff = status.get("handoff");
    let stage = match seed.get("line_1").and_then(Value::as_str) {
        Some("status: draft") => Shaped::Drafted {
            revision,
            text: None,
        },
        Some("status: simulated") => Shaped::Practice,
        Some("status: confirmed for intake") => {
            match handoff
                .and_then(|h| h.get("verified"))
                .and_then(Value::as_bool)
            {
                Some(true) => Shaped::Ready { revision },
                _ => Shaped::Unverified {
                    reason: handoff
                        .and_then(|h| text(h, "reason"))
                        .unwrap_or_else(|| "Seed Me gave no reason.".into()),
                },
            }
        }
        _ => Shaped::Unknown {
            what: "The seed's first line isn't one Seed Me writes.".into(),
        },
    };
    shaping(goal, stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
                title: "Where records live".into(),
                ask: "Keep records local or synced?".into(),
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
                title: "storage".into(),
                ask: "storage choice".into(),
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
                    authority: "user".into(),
                },
                Decided {
                    title: "lint".into(),
                    answer: "Clippy.".into(),
                    authority: "delegated".into(),
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
                revision: Some("r2".into())
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
