//! Confirmed-goal persistence: preserve exact originals and retry one session, never start a viewer.

use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// A captured destination and the session retained across retries.
#[derive(Clone)]
pub struct Config {
    root: Result<PathBuf, String>,
    session: Arc<Mutex<Option<PathBuf>>>,
}
/// The exact goal and evidence explicitly affirmed by the operator.
#[derive(Clone)]
pub struct Affirmed {
    /// The displayed goal, saved unchanged.
    pub goal: String,
    /// The goal's done-when check.
    pub outcome: String,
    /// Proposed approaches, not accepted choices.
    pub options: Vec<Choice>,
    /// Original words, preserved verbatim.
    pub originals: Vec<Original>,
    /// Provisional interview context, never settled decisions.
    pub context: String,
    /// Who affirmed the goal and how.
    pub source: String,
}
/// One proposed approach.
#[derive(Clone)]
pub struct Choice {
    /// The approach's name.
    pub label: String,
    /// What it gains.
    pub benefit: String,
    /// What it costs.
    pub cost: String,
    /// What undoing it costs.
    pub undo_cost: String,
}
/// One untouched original dump or answer.
#[derive(Clone)]
pub struct Original {
    /// Its conversation number, starting at one.
    pub id: usize,
    /// The question it answered, if any.
    pub in_reply_to: Option<String>,
    /// Exact original words.
    pub text: String,
}
impl Original {
    /// A provenance label, without changing the original.
    pub fn label(&self) -> String {
        format!(
            "Tinkery original {}{}",
            self.id,
            self.in_reply_to
                .as_ref()
                .map_or(String::new(), |id| format!(" / reply to {id}"))
        )
    }
}
/// A goal read back unchanged from its durable session.
pub struct Receipt {
    /// The saved session folder.
    pub session: PathBuf,
    /// The exact confirmed goal.
    pub goal: String,
}
impl Receipt {
    #[cfg(test)]
    pub(crate) fn test(session: PathBuf) -> Self {
        Self {
            session,
            goal: "TEST confirmed goal".into(),
        }
    }
    /// A plain receipt for after terminal restoration.
    pub fn text(&self) -> String {
        format!(
            "Saved.\n{}\nNext: shape it into a seed in Shape.\nSession: {}",
            self.goal,
            self.session.display()
        )
    }
}
impl Config {
    /// Capture the explicit or checked HOME destination without writing anything.
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root: super::resolve_root(root),
            session: Arc::new(Mutex::new(None)),
        }
    }
    /// The session retained after an incomplete save.
    pub fn recovery_path(&self) -> Option<PathBuf> {
        self.session.lock().unwrap().clone()
    }
    /// The same destination, with no session yet.
    pub fn fresh(&self) -> Self {
        Self {
            root: self.root.clone(),
            session: Arc::new(Mutex::new(None)),
        }
    }
    /// Save through the shared Rust ledger and read back the exact goal and originals.
    /// Retries reuse the same session; an existing origin, ended session or seed is never replaced.
    pub fn confirm(&self, a: Affirmed) -> Result<Receipt, String> {
        let root = self.root.as_ref().map_err(Clone::clone)?;
        let stamp = super::answering::now();
        let mut retained = self.session.lock().unwrap();
        if retained.is_none() {
            *retained = Some(super::ledger::initialize(root, &stamp)?);
        }
        let session = retained.as_ref().unwrap();
        let current = super::ledger::load(session).map_err(
            |_| "The session could not be read. Inspect the recovery session before retrying.",
        )?;
        if current["status"].string() != Some("active")
            || current["operator"].string() != Some("human")
        {
            return Err("Session is not an active human goal session; nothing overwritten.".into());
        }
        if session.join("seed-contract.md").exists() {
            return Err("A seed file already exists; reopen this session in Shape.".into());
        }
        if current["origin"] == super::ledger::Value::Null {
            let evidence = a.originals.iter().map(|s| json!({"checked":s.label(),"at":stamp,"observed":s.text}))
                .chain(std::iter::once(json!({"checked":"Tinkery answer/question context; questions remain provisional, not settled decisions","at":stamp,"observed":a.context}))).collect::<Vec<_>>();
            let state = json!({"status":"active", "draft":{"goal":a.goal,"outcome":a.outcome,
                "options":a.options.iter().map(|o|format!("{} — benefit: {}; cost: {}; undo cost: {}",o.label,o.benefit,o.cost,o.undo_cost)).collect::<Vec<_>>()},
                "goal":a.goal,"origin":"goal","current_question":null,"nodes":[{"id":"goal","kind":"decision","status":"settled",
                "prerequisites":[],"evidence":evidence,"owner":"User","gate":"Explicit affirmation of the complete displayed goal; not seed or implementation approval",
                "answer":a.goal,"authority":"user","authority_source":a.source}],"assumed":[],"operator":"human"});
            super::ledger::publish(session, &super::ledger::Value::from(state), &current["version"],
                &"Explicit goal affirmation in Tinkery.".into(), None, &stamp)
                .map_err(|_| "The goal save could not be verified. Inspect the recovery session before retrying.")?;
        }
        let read = super::ledger::load(session).map_err(|_| {
            "The saved goal could not be read back. Inspect the recovery session before retrying."
        })?;
        let origin = read["nodes"]
            .array()
            .unwrap_or(&[])
            .iter()
            .find(|n| n["id"] == read["origin"])
            .ok_or("The saved goal has no origin. Inspect the recovery session.")?;
        if read["goal"].string() != Some(a.goal.as_str())
            || origin["answer"].string() != Some(a.goal.as_str())
            || origin["authority"].string() != Some("user")
            || origin["status"].string() != Some("settled")
        {
            return Err("This session already pins another goal. No origin was replaced.".into());
        }
        for original in &a.originals {
            if !origin["evidence"]
                .array()
                .unwrap_or(&[])
                .iter()
                .any(|e| e["observed"].string() == Some(original.text.as_str()))
            {
                return Err(
                    "Saved originals differ from this confirmation; no goal was replaced.".into(),
                );
            }
        }
        Ok(Receipt {
            session: session.clone(),
            goal: a.goal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_root_survives_fresh_dumps_and_refuses_before_writing() {
        let config = Config {
            root: super::super::default_root(None),
            session: Arc::new(Mutex::new(None)),
        };
        for config in [config.clone(), config.fresh()] {
            let error = config
                .confirm(Affirmed {
                    goal: "TEST goal".into(),
                    outcome: "TEST outcome".into(),
                    options: vec![],
                    originals: vec![],
                    context: "TEST context".into(),
                    source: "TEST".into(),
                })
                .err()
                .expect("An unavailable destination cannot save");
            assert!(error.contains("--seed-session-root PATH"), "{error}");
            assert!(config.recovery_path().is_none());
        }
    }

    #[test]
    fn unreadable_recovery_sessions_use_plain_errors_and_remain_available() {
        let directory = tempfile::Builder::new()
            .prefix("tinkery-TEST-save-refusal-")
            .tempdir()
            .unwrap();
        let recovery = directory.path().join("TEST-recovery");
        std::fs::create_dir(&recovery).unwrap();
        let config = Config::new(Some(directory.path().join("TEST-sessions")));
        *config.session.lock().unwrap() = Some(recovery.clone());
        for contents in [None, Some("TEST malformed JSON")] {
            if let Some(contents) = contents {
                std::fs::write(recovery.join("ledger.json"), contents).unwrap();
            }
            let error = config
                .confirm(Affirmed {
                    goal: "TEST unchanged goal".into(),
                    outcome: "TEST outcome".into(),
                    options: vec![],
                    originals: vec![],
                    context: "TEST context".into(),
                    source: "TEST".into(),
                })
                .err()
                .expect("An unreadable session cannot confirm a goal");
            assert_eq!(
                error,
                "The session could not be read. Inspect the recovery session before retrying."
            );
            assert_eq!(config.recovery_path(), Some(recovery.clone()));
        }
    }

    #[test]
    fn fresh_dump_keeps_the_selected_root() {
        let root = PathBuf::from("/TEST/captured-sessions");
        assert_eq!(Config::new(Some(root.clone())).fresh().root, Ok(root));
    }
}
