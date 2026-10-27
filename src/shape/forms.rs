//! Unsaved operator forms, pinned to the answer and version that opened them. The ledger alone
//! validates contents; editing, changing fields and cancellation never publish anything.

use super::words::Words;
use crate::seed_me::ledger::AnswerAction;

pub(super) struct Form {
    pub(super) id: String,
    pub(super) expected: u64,
    fields: Fields,
}

enum Fields {
    Defer {
        gate: String,
        reason: Words,
        revisit: Box<Words>,
        condition: bool,
    },
    Revise(Words),
}

impl Form {
    pub(super) fn defer(id: String, expected: u64, gate: String) -> Self {
        Self {
            id,
            expected,
            fields: Fields::Defer {
                gate,
                reason: Words::new(""),
                revisit: Box::new(Words::new("")),
                condition: false,
            },
        }
    }

    pub(super) fn revise(id: String, expected: u64) -> Self {
        Self {
            id,
            expected,
            fields: Fields::Revise(Words::new("")),
        }
    }

    pub(super) fn words(&mut self) -> &mut Words {
        match &mut self.fields {
            Fields::Defer {
                reason,
                revisit,
                condition,
                ..
            } => {
                if *condition {
                    revisit
                } else {
                    reason
                }
            }
            Fields::Revise(words) => words,
        }
    }

    pub(super) fn label(&self) -> &'static str {
        match &self.fields {
            Fields::Defer {
                condition: false, ..
            } => "reason to defer",
            Fields::Defer { .. } => "revisit condition",
            Fields::Revise(_) => "replacement answer",
        }
    }

    pub(super) fn bar(&self) -> &'static str {
        match &self.fields {
            Fields::Defer {
                condition: false, ..
            } => "F2 next field",
            Fields::Defer { .. } => "F2 save deferral",
            Fields::Revise(_) => "F2 save revision and look",
        }
    }

    pub(super) fn description(&self) -> String {
        match &self.fields {
            Fields::Defer { gate, reason, condition, .. } => {
                let reason = if *condition { format!("\nReason: {}", reason.text()) } else { String::new() };
                format!("Defer this question. You decide it can wait.\nGate: {gate}{reason}\nTab changes fields; F2 advances or saves. Esc cancels.")
            }
            Fields::Revise(_) => concat!(
                "Replace this answer. F2 saves the revision and looks into the next. ",
                "Recorded dependents come first, without a model call; otherwise one step runs. Esc cancels."
            ).into(),
        }
    }

    pub(super) fn toggle(&mut self) {
        if let Fields::Defer { condition, .. } = &mut self.fields {
            *condition = !*condition;
        }
    }

    pub(super) fn advance(&mut self) -> bool {
        if let Fields::Defer { condition, .. } = &mut self.fields
            && !*condition
        {
            *condition = true;
            false
        } else {
            true
        }
    }

    pub(super) fn action(&self) -> AnswerAction {
        match &self.fields {
            Fields::Defer {
                reason, revisit, ..
            } => AnswerAction::Defer {
                reason: reason.text().to_owned(),
                revisit: revisit.text().to_owned(),
            },
            Fields::Revise(words) => AnswerAction::Revise(words.text().to_owned()),
        }
    }
}
