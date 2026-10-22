//! Provisional answers belong to their question. Copying one only edits the local draft;
//! missing or malformed suggestions never prevent answering, and none supplies model context.
use super::*;

/// An editable model proposal and its stated basis, never an operator answer or permission.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Suggestion {
    /// The proposed answer, kept separate from submitted words.
    pub answer: String,
    /// Why this answer fits the supplied context, including any assumptions.
    pub reason: String,
}

pub(super) fn read(value: Option<&serde_json::Value>) -> Option<Suggestion> {
    let suggestion: Suggestion = serde_json::from_value(value?.clone()).ok()?;
    [&suggestion.answer, &suggestion.reason]
        .iter()
        .all(|s| !s.trim().is_empty() && s.chars().all(|c| c == '\n' || !c.is_control()))
        .then_some(suggestion)
}

pub(super) fn decode<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Suggestion>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(read(Some(&value)))
}

pub(super) fn description(question: &Question) -> String {
    match &question.suggestion {
        Some(s) => format!("Suggested answer: {}\nWhy: {}", s.answer, s.reason),
        None => "No suggested answer available. Use your own words or skip.".into(),
    }
}

impl BrainDump {
    pub(super) fn can_use_suggestion(&self) -> bool {
        !self.running()
            && self.shaping.ready.is_none()
            && self.conversation.caught_up()
            && !self.add_more
            && !self.voice_active()
            && self.input.text.is_empty()
            && self
                .conversation
                .focused_question()
                .is_some_and(|q| q.suggestion.is_some())
    }

    pub(super) fn use_suggestion(&mut self) {
        if !self.input.text.is_empty() {
            self.refuse("Your draft is kept. Clear it first to use the suggestion.");
            return;
        }
        if !self.can_use_suggestion() {
            self.refuse("No suggestion ready to use. Write your answer or skip.");
            return;
        }
        let answer = self
            .conversation
            .focused_question()
            .unwrap()
            .suggestion
            .as_ref()
            .unwrap()
            .answer
            .clone();
        self.focus_writing();
        if self.input.insert(&answer).is_err() {
            self.refuse(layout::LIMIT_TEXT);
        }
    }
}

#[cfg(test)]
#[path = "suggestion_tests.rs"]
mod tests;
