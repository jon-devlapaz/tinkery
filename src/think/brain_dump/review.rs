//! The decision review: a second call after each reading that names decisions the draft settled
//! silently. It holds the prompt, what the review sees (the person's words, their answers and the
//! draft parts, not the template), and how its decisions join the reading's.
use super::*;

/// The decision review prompt as written: one rule per line, with notes on why.
pub(super) const PROMPT: &str = include_str!("prompts/decision_review.txt");
/// The one field a review decision adds to a reading's [`reading::DECISION`]: the question to ask.
pub(super) const QUESTION: &str = "question";

impl PiHost {
    /// Runs the review on a reading's raw reply. Returns the reply with the review's decisions
    /// merged in, or `None` to keep the reply as it was.
    pub(super) fn review_decisions(
        &self,
        request: &BoardRequest,
        raw: &str,
        cancelled: &AtomicBool,
        changes: &mut Vec<String>,
    ) -> Option<String> {
        let draft = reading::read(raw, &mut vec![])
            .ok()?
            .get("parts")
            .cloned()
            .unwrap_or_default();
        let text_of = |id: usize| {
            request
                .sources
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.text.clone())
        };
        let answered: Vec<_> = request
            .settled
            .iter()
            .map(|s| serde_json::json!({"question": s.question.text, "answer": text_of(s.source)}))
            .collect();
        let words: Vec<_> = request
            .sources
            .iter()
            .filter(|s| !request.settled.iter().any(|a| a.source == s.id))
            .map(|s| s.text.clone())
            .collect();
        let input = serde_json::to_vec(&serde_json::json!({
            "kind": "decision-review", "words": words, "answers": answered, "draft": draft
        }))
        .ok()?;
        match self.complete(input, reading::assemble(PROMPT), cancelled) {
            Ok(review) => merge(raw, &review, changes),
            Err(e) => {
                changes.push(format!("Decision review skipped: {e}"));
                None
            }
        }
    }
}

/// Merges a review into a reading's raw reply. The review's decisions join the reading's; once
/// required parts are present, its top question is asked first. Board validation still applies
/// (ask counts, skips, duplicates), so a dropped review question falls back to the reading's own.
/// An empty or unreadable review returns `None`, leaving the reading as it was.
pub(super) fn merge(raw: &str, review: &str, changes: &mut Vec<String>) -> Option<String> {
    let mut board = reading::read(raw, &mut vec![]).ok()?;
    let review = match reading::read(review, &mut vec![]) {
        Ok(v) => v,
        Err(e) => {
            changes.push(format!("Ignored decision review: {e}"));
            return None;
        }
    };
    let found: Vec<_> = reading::items(review.get("decisions"), "review decisions", &mut vec![])
        .into_iter()
        .filter_map(|v| v.as_object().cloned())
        .collect();
    let obj = board.as_object_mut()?;
    if found.is_empty() {
        return None;
    }
    let required_missing = Part::REQUIRED.iter().any(|p| {
        let key = serde_json::to_value(p).ok();
        obj.get("parts")
            .and_then(|parts| parts.get(key.as_ref()?.as_str()?))
            .and_then(|v| v.as_str())
            .is_none_or(|s| s.trim().is_empty())
    });
    let mut decisions = reading::items(obj.get("decisions"), "decisions", &mut vec![]);
    let added = found.len();
    let mut asks = vec![];
    for mut d in found {
        if let Some(text) = d.remove(QUESTION).filter(|q| q.is_string()) {
            asks.push(serde_json::json!({"target": d.get("part"), "text": text}));
        }
        decisions.push(serde_json::Value::Object(d));
    }
    let mut questions = reading::items(obj.get("questions"), "questions", &mut vec![]);
    if required_missing {
        questions.extend(asks);
    } else {
        asks.extend(questions);
        questions = asks;
    }
    changes.push(format!("Decision review added {added} decision(s)"));
    obj.insert("decisions".into(), decisions.into());
    obj.insert("questions".into(), questions.into());
    serde_json::to_string(&board).ok()
}
