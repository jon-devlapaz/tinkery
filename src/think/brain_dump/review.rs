//! The decision review: a second call after each reading that names decisions the draft settled
//! silently. It holds the prompt, what the review sees (the person's words, their answers and the
//! draft parts, not the template), its reply's decoder, and how its decisions join the reading's.
use super::*;

/// The decision review prompt as written: one rule per line, with notes on why.
pub(super) const PROMPT: &str = include_str!("prompts/decision_review.txt");
/// Indices of shaping decisions a review explicitly identifies as the same choice.
pub(super) const COVERS: &str = "covers";

impl PiHost {
    /// Runs the review on a reading's raw reply. Returns the decisions it found, or `None` when
    /// it found none or couldn't run, leaving the reading as it was.
    pub(super) fn review_decisions(
        &self,
        request: &BoardRequest,
        raw: &str,
        cancelled: &AtomicBool,
        changes: &mut Vec<String>,
    ) -> Option<Review> {
        let input = input(request, raw)?;
        match self.complete(
            Request::DecisionReview,
            input,
            reading::assemble(PROMPT),
            cancelled,
        ) {
            Ok(review) => Review::decode(&review, changes),
            Err(e) => {
                changes.push(format!("Decision review skipped: {e}"));
                None
            }
        }
    }
}

/// Prepares the decision review's input from a readable shaping reply, before board validation.
/// Unreadable JSON yields no review; a readable but invalid board is still reviewed.
/// Contains original words, settled answers, draft parts and shaping decisions, never standing instructions.
pub fn input(request: &BoardRequest, raw: &str) -> Option<Vec<u8>> {
    let (draft, mut decisions) = reading::draft(raw)?;
    if let Some(decisions) = decisions.as_array_mut() {
        for decision in decisions
            .iter_mut()
            .filter_map(serde_json::Value::as_object_mut)
        {
            decision.remove("suggestion");
        }
    }
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
    serde_json::to_vec(&serde_json::json!({
        "kind": "decision-review", "words": words, "answers": answered, "draft": draft, "decisions": decisions
    }))
    .ok()
}

/// A decision review's reply, decoded: the choices it found, most consequential first. Each is
/// checked on its own here; whether it may replace a shaping decision is decided when the reading
/// joins it ([`Guess::decode_reviewed`]). The review never edits the reading's reply.
#[derive(Clone, Debug)]
pub struct Review {
    found: Vec<Found>,
}
#[derive(Clone, Debug)]
struct Found {
    choice: OpenDecision,
    /// The shaping decisions it says it covers: `None` when it names none, `Err` when the
    /// indices it names aren't indices.
    covers: Option<Result<Vec<usize>, ()>>,
}

impl Review {
    /// Reads a review reply. Unreadable or invalid decisions are noted and left out; a reply with
    /// none left is `None`, so the reading stays as it was.
    pub fn decode(review: &str, changes: &mut Vec<String>) -> Option<Self> {
        let review = match reading::read(review, &mut vec![]) {
            Ok(v) => v,
            Err(e) => {
                changes.push(format!("Ignored decision review: {e}"));
                return None;
            }
        };
        let mut found = vec![];
        for v in reading::items(review.get("decisions"), "review decisions", &mut vec![]) {
            let Some(mut d) = v.as_object().cloned() else {
                continue;
            };
            let covers = d.remove(COVERS);
            let Ok(mut choice) =
                serde_json::from_value::<OpenDecision>(serde_json::Value::Object(d))
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
            let mut answers: Vec<String> = vec![];
            for a in choice.answers {
                if !answers.contains(&a) {
                    answers.push(a);
                }
            }
            choice.answers = answers;
            let covers = covers.as_ref().and_then(|v| v.as_array()).map(|indices| {
                indices
                    .iter()
                    .map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or(()))
                    .collect()
            });
            found.push(Found { choice, covers });
        }
        (!found.is_empty()).then_some(Self { found })
    }

    /// The reading's decisions with this review joined in. A shaping decision is replaced only
    /// when a review decision about the same part names its index; every other one remains,
    /// including different choices about the same part. `shaping` pairs each decision with its
    /// index in the reply the review saw.
    pub(super) fn join(
        &self,
        shaping: Vec<(usize, OpenDecision)>,
        changes: &mut Vec<String>,
    ) -> Vec<OpenDecision> {
        let mut covered: HashSet<usize> = HashSet::new();
        for found in &self.found {
            match &found.covers {
                Some(Ok(indices))
                    if indices.iter().all(|i| {
                        shaping
                            .iter()
                            .any(|(j, d)| j == i && d.part == found.choice.part)
                    }) =>
                {
                    covered.extend(indices.iter().copied())
                }
                Some(_) => changes
                    .push("Ignored invalid decision coverage; shaping choices retained".into()),
                None => {}
            }
        }
        changes.push(format!(
            "Decision review added {} decision(s)",
            self.found.len()
        ));
        shaping
            .into_iter()
            .filter(|(i, _)| !covered.contains(i))
            .map(|(_, d)| d)
            .chain(self.found.iter().map(|f| f.choice.clone()))
            .collect()
    }

    /// The questions the review asks, most consequential first, with the part each would change.
    pub(super) fn questions(&self) -> impl Iterator<Item = &OpenDecision> {
        self.found
            .iter()
            .filter(|f| f.choice.question.is_some())
            .map(|f| &f.choice)
    }
}
