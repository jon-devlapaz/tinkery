//! The decision review: a second call after each reading that names decisions the draft settled
//! silently. It holds the prompt, what the review sees (the person's words, their answers and the
//! draft parts, not the template), and how its decisions join the reading's.
use super::*;

/// The decision review prompt as written: one rule per line, with notes on why.
pub(super) const PROMPT: &str = include_str!("prompts/decision_review.txt");
/// Indices of shaping decisions a review explicitly identifies as the same choice.
pub(super) const COVERS: &str = "covers";

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
        let reading = reading::read(raw, &mut vec![]).ok()?;
        let draft = reading.get("parts").cloned().unwrap_or_default();
        let decisions = reading
            .get("decisions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
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
            "kind": "decision-review", "words": words, "answers": answered, "draft": draft, "decisions": decisions
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

/// Merges a review into a reading's raw reply. Only explicitly covered shaping choices are
/// replaced; all others remain, including different choices about the same goal part. Once
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
    let required_missing = reading::read_parts(obj.get("parts"), &mut vec![])
        .missing()
        .next()
        .is_some();
    let original = reading::items(obj.get("decisions"), "decisions", &mut vec![]);
    let mut covered = HashSet::new();
    let mut decisions = vec![];
    let mut asks = vec![];
    for mut d in found {
        let coverage = d.remove(COVERS);
        let Ok(choice) =
            serde_json::from_value::<OpenDecision>(serde_json::Value::Object(d.clone()))
        else {
            changes.push("Ignored unreadable review decision; shaping choices retained".into());
            continue;
        };
        let safe =
            |s: &str| !s.trim().is_empty() && s.chars().all(|c| c == '\n' || !c.is_control());
        let distinct: HashSet<_> = choice.answers.iter().map(|a| a.trim()).collect();
        if !safe(&choice.text)
            || distinct.len() < 2
            || choice.answers.iter().any(|a| !safe(a))
            || choice.question.as_ref().is_some_and(|q| !safe(q))
        {
            changes.push("Ignored invalid review decision; shaping choices retained".into());
            continue;
        }
        if let Some(indices) = coverage.as_ref().and_then(|v| v.as_array()) {
            let indices: Option<Vec<usize>> = indices
                .iter()
                .map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()))
                .collect();
            if let Some(indices) = indices.filter(|indices| {
                indices
                    .iter()
                    .all(|i| original.get(*i).and_then(|v| v.get("part")) == d.get("part"))
            }) {
                covered.extend(indices);
            } else {
                changes.push("Ignored invalid decision coverage; shaping choices retained".into());
            }
        }
        if let Some(text) = &choice.question {
            asks.push(serde_json::json!({"target": choice.part, "text": text}));
        }
        decisions.push(serde_json::Value::Object(d));
    }
    if decisions.is_empty() {
        return None;
    }
    let added = decisions.len();
    let mut kept: Vec<_> = original
        .into_iter()
        .enumerate()
        .filter_map(|(i, d)| (!covered.contains(&i)).then_some(d))
        .collect();
    kept.extend(decisions);
    let decisions = kept;
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
