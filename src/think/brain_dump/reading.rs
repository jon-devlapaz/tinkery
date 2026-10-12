//! The reading: think's contract with the model for each shaping call. It holds the request
//! think sends ([`BoardRequest`]), the prompt, the reply's wire shape ([`Guess`]) and the
//! lenient decoder that reads a reply into it. [`board`](super::board) then verifies it.
use super::*;

/// The reading prompt as written: one rule per line, with notes on why. See its header.
pub(super) const PROMPT: &str = include_str!("prompts/reading.txt");

/// The system prompt for one shaping call, with Seed Me's shaping guidance in its place.
pub(super) fn prompt(seed_me_shaping: &str) -> String {
    assemble(PROMPT).replace("{seed_me_shaping}", seed_me_shaping)
}

/// A prompt file as the model reads it. Notes (lines starting with `#`) are dropped, lines in a
/// paragraph are joined with one space, and each blank line becomes one newline.
pub(super) fn assemble(file: &str) -> String {
    let mut prompt = String::new();
    let mut in_paragraph = false;
    for line in file.lines().filter(|l| !l.starts_with('#')) {
        if line.is_empty() {
            if !prompt.is_empty() {
                prompt.push('\n');
            }
            in_paragraph = false;
        } else {
            if in_paragraph {
                prompt.push(' ');
            }
            prompt.push_str(line);
            in_paragraph = true;
        }
    }
    prompt
}

/// The fields of a reading the decoder reads. The prompt's schema line names the same fields, less
/// three that older replies carried (`uncertain`, `framings`, `outcome`); `prompt_tests` keeps the
/// two equal.
pub(super) const READING: &[&str] = &[
    "parts",
    "other_goals",
    "deferred",
    "open",
    "supports",
    "uncertain",
    "framings",
    "outcome",
    "misfits",
    "unresolved_notes",
    "questions",
    "alternatives",
    "decisions",
];
/// The fields of `parts`.
pub(super) const PARTS: &[&str] = &[
    "who",
    "outcome",
    "when",
    "why",
    "done_when",
    "must",
    "must_not",
];
/// The fields of one decision. The decision review adds a question to these.
pub(super) const DECISION: &[&str] = &["part", "text", "answers"];
/// The fields of one question.
pub(super) const QUESTION: &[&str] = &["id", "text", "target"];
/// The fields of a claimed source span.
pub(super) const ANCHOR: &[&str] = &["source", "quote", "occurrence"];

#[derive(Clone, Debug, Serialize)]
pub struct Source {
    pub id: usize,
    pub text: String,
    pub in_reply_to: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Fragment {
    pub id: String,
    pub source: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub source: usize,
    pub quote: String,
    pub occurrence: usize,
}
impl Anchor {
    pub fn range(&self, sources: &[Source]) -> Result<std::ops::Range<usize>, String> {
        let source = sources
            .iter()
            .find(|s| s.id == self.source)
            .ok_or("Unknown annotation source")?;
        if self.quote.trim().is_empty() {
            return Err("Empty annotation".into());
        }
        let start = source
            .text
            .match_indices(&self.quote)
            .nth(self.occurrence)
            .map(|(i, _)| i)
            .ok_or("Annotation is not an exact source substring at that occurrence")?;
        let end = start + self.quote.len();
        if !grapheme_boundary(&source.text, start) || !grapheme_boundary(&source.text, end) {
            return Err("Annotation cuts a grapheme; no highlight applied".into());
        }
        Ok(start..end)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Framing {
    pub text: String,
    pub supports: Vec<Anchor>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    #[serde(default)]
    pub target: Option<Part>,
    pub id: String,
    pub text: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Guess {
    #[serde(default)]
    pub parts: GoalParts,
    #[serde(default)]
    pub other_goals: Vec<String>,
    #[serde(default)]
    pub deferred: Vec<String>,
    #[serde(default)]
    pub open: Vec<Part>,
    #[serde(default)]
    pub unresolved_notes: Vec<String>,
    pub uncertain: bool,
    pub framings: Vec<Framing>,
    pub outcome: String,
    pub misfits: Vec<Anchor>,
    pub questions: Vec<Question>,
    pub alternatives: Vec<Approach>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Settled {
    pub question: Question,
    pub source: usize,
}
#[derive(Clone, Serialize)]
pub struct BoardRequest {
    pub ask_counts: std::collections::BTreeMap<Part, u8>,
    pub sources: Vec<Source>,
    pub fragments: Vec<Fragment>,
    pub previous: Option<Guess>,
    pub skipped: Vec<Question>,
    pub answered: Vec<String>,
    pub settled: Vec<Settled>,
    pub layout: Vec<(String, f64, f64)>,
}

pub(super) fn same_question(a: &Question, b: &Question) -> bool {
    let words = |s: &str| {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    a.id == b.id || words(&a.text) == words(&b.text)
}
pub(super) fn grapheme_boundary(text: &str, index: usize) -> bool {
    index == text.len() || text.grapheme_indices(true).any(|(i, _)| i == index)
}
fn safe(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().all(|c| c == '\n' || !c.is_control())
}
impl Guess {
    pub fn decode(
        raw: &str,
        request: &BoardRequest,
        normalizations: &mut Vec<String>,
    ) -> Result<Self, String> {
        parse(raw, request, normalizations)
    }
    pub fn validate(&self, request: &BoardRequest) -> Result<(), String> {
        if self
            .parts
            .must
            .iter()
            .chain(&self.parts.must_not)
            .chain(&self.other_goals)
            .chain(&self.deferred)
            .any(|s| s.chars().any(|c| c != '\n' && c.is_control()))
        {
            return Err("Unsafe display control".into());
        }
        for part in Part::ALL {
            if self
                .parts
                .get(part)
                .is_some_and(|s| s.chars().any(|c| c != '\n' && c.is_control()))
            {
                return Err("Unsafe display control".into());
            }
        }
        for frame in &self.framings {
            if frame.text.chars().any(|c| c != '\n' && c.is_control()) {
                return Err("Unsafe display control".into());
            }
            for span in &frame.supports {
                span.range(&request.sources)?;
            }
        }
        for span in &self.misfits {
            span.range(&request.sources)?;
        }
        for text in std::iter::once(&self.outcome)
            .chain(self.unresolved_notes.iter())
            .chain(
                self.alternatives
                    .iter()
                    .flat_map(|a| [&a.label, &a.benefit, &a.cost, &a.undo_cost]),
            )
        {
            if text.chars().any(|c| c != '\n' && c.is_control()) {
                return Err("Unsafe display control".into());
            }
        }
        let mut ids = HashSet::new();
        for q in &self.questions {
            if q.id.starts_with("scope-addition-")
                || !safe(&q.id)
                || !safe(&q.text)
                || !ids.insert(&q.id)
                || request.answered.contains(&q.id)
                || request
                    .settled
                    .iter()
                    .any(|s| same_question(&s.question, q))
                || request.skipped.iter().any(|s| same_question(s, q))
            {
                return Err("Invalid application-owned question identity/history".into());
            }
        }
        Ok(())
    }
}

pub(super) fn parse(
    raw: &str,
    request: &BoardRequest,
    changes: &mut Vec<String>,
) -> Result<Guess, String> {
    let value = read(raw, changes)?;
    let obj = value.as_object().ok_or("No readable board object")?;
    extras(obj, READING, "board", changes);
    if obj.contains_key("uncertain") {
        changes.push("Legacy uncertainty flag ignored as a count/authority signal; all readings remain provisional".into());
    }
    let mut parts = GoalParts::default();
    let object = obj.get("parts").and_then(serde_json::Value::as_object);
    if let Some(object) = object {
        extras(object, PARTS, "goal parts", changes);
    }
    parts.who = display(object.and_then(|o| o.get("who")), "who", changes);
    parts.outcome = display(object.and_then(|o| o.get("outcome")), "outcome", changes);
    parts.when = display(object.and_then(|o| o.get("when")), "when", changes);
    parts.why = display(object.and_then(|o| o.get("why")), "why", changes);
    parts.done_when = display(
        object.and_then(|o| o.get("done_when")),
        "done_when",
        changes,
    );
    parts.must = items(object.and_then(|o| o.get("must")), "must", changes)
        .iter()
        .filter_map(|v| display(Some(v), "must", changes))
        .collect();
    parts.must_not = items(object.and_then(|o| o.get("must_not")), "must not", changes)
        .iter()
        .filter_map(|v| display(Some(v), "must not", changes))
        .collect();
    for value in parts
        .must
        .iter()
        .chain(&parts.must_not)
        .filter(|v| crate::goal::looks_compound(v))
    {
        changes.push(format!(
            "A constraint item looks like more than one constraint; shown as one line as written: {value}"
        ));
    }
    parts.distinct_check();
    let other_goals = items(obj.get("other_goals"), "other goals", changes)
        .iter()
        .filter_map(|v| display(Some(v), "other goal", changes))
        .collect();
    let deferred = items(obj.get("deferred"), "deferred", changes)
        .iter()
        .filter_map(|v| display(Some(v), "deferred", changes))
        .collect();
    let mut open = vec![];
    for value in items(obj.get("open"), "open parts", changes) {
        if let Ok(part) = serde_json::from_value::<Part>(value.clone()) {
            if !open.contains(&part) {
                open.push(part);
            }
        } else {
            changes.push(format!("Ignored unknown open part {value}"));
        }
    }
    // Typed decisions: a choice only the person can make, with answers that would change
    // a goal line. Each keeps its part open (so it can be asked and blocks readiness) and
    // travels to review and the seed as a note.
    let mut decision_notes = vec![];
    for (i, v) in items(obj.get("decisions"), "decisions", changes)
        .iter()
        .enumerate()
    {
        let Some(o) = v.as_object() else {
            changes.push(format!("Ignored decision {i}: not an object"));
            continue;
        };
        extras(o, DECISION, &format!("decision {i}"), changes);
        let part = o
            .get("part")
            .and_then(|v| serde_json::from_value::<Part>(v.clone()).ok());
        let text = display(o.get("text"), &format!("decision {i}"), changes);
        let mut answers: Vec<String> = vec![];
        for a in items(o.get("answers"), &format!("decision {i} answers"), changes) {
            if let Some(a) = display(Some(&a), &format!("decision {i} answer"), changes)
                && !answers.contains(&a)
            {
                answers.push(a);
            }
        }
        let (Some(part), Some(text)) = (part, text) else {
            changes.push(format!("Ignored decision {i}: needs a known part and text"));
            continue;
        };
        if answers.len() < 2 {
            changes.push(format!(
                "Ignored decision {i}: needs two answers that change the goal"
            ));
            continue;
        }
        if !open.contains(&part) {
            open.push(part);
        }
        decision_notes.push(format!("Open decision: {text} ({})", answers.join(" / ")));
    }
    let mut framings = vec![];
    for (i, v) in items(obj.get("framings"), "framings", changes)
        .iter()
        .enumerate()
    {
        let (text, supports) = if let Some(o) = v.as_object() {
            extras(o, &["text", "supports"], &format!("reading {i}"), changes);
            (
                display(o.get("text"), &format!("reading {i}"), changes),
                anchors(o.get("supports"), &format!("reading {i} supports"), changes)?,
            )
        } else {
            (display(Some(v), &format!("reading {i}"), changes), vec![])
        };
        if supports.is_empty() {
            changes.push(format!("Reading {i} is uncited; no source highlight"));
        }
        framings.push(Framing {
            text: text.unwrap_or_default(),
            supports,
        });
    }
    if obj.contains_key("framings") {
        changes.push("Ignored freeform reading text; goal comes only from parts".into());
    }
    let mut supports = anchors(obj.get("supports"), "goal supports", changes)?;
    supports.extend(framings.into_iter().flat_map(|f| f.supports));
    let framings = vec![Framing {
        text: parts.compose().unwrap_or_default(),
        supports,
    }];
    let outcome = parts.done_when.clone().unwrap_or_default();
    let mut questions = vec![];
    for (i, v) in items(obj.get("questions"), "questions", changes)
        .iter()
        .enumerate()
    {
        let o = v.as_object();
        if let Some(o) = o {
            extras(o, QUESTION, &format!("question {i}"), changes);
        }
        let Some(text) = display(
            o.and_then(|o| o.get("text")).or(Some(v)),
            &format!("question {i}"),
            changes,
        ) else {
            continue;
        };
        let id = display(
            o.and_then(|o| o.get("id")),
            &format!("question {i} ID"),
            changes,
        )
        .unwrap_or_else(|| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            let id = format!("model-{:016x}", h.finish());
            changes.push(format!("Allocated question ID {id}"));
            id
        });
        let target = o
            .and_then(|o| o.get("target"))
            .and_then(|v| serde_json::from_value::<Part>(v.clone()).ok());
        let target = target
            .or_else(|| open.first().copied())
            .or_else(|| parts.missing().next());
        if o.and_then(|o| o.get("target")).is_none() {
            changes.push(format!(
                "Question {id} target assigned from first open/missing part"
            ));
        }
        if let Some(part) = target {
            if parts.missing().next().is_none() && !open.contains(&part) {
                changes.push(format!("Ignored question {id}: no missing/open target"));
                continue;
            }
        } else {
            changes.push(format!(
                "Ignored untargeted question {id}: no open/missing part"
            ));
            continue;
        }
        let q = Question { target, id, text };
        if q.id.starts_with("scope-addition-") || q.id.starts_with("goal-refinement-") {
            return Err("Model attempted an application-owned question ID".into());
        }
        if questions.iter().any(|old| same_question(old, &q))
            || request.answered.contains(&q.id)
            || request
                .settled
                .iter()
                .any(|s| same_question(&s.question, &q))
            || request.skipped.iter().any(|s| same_question(s, &q))
        {
            changes.push(format!(
                "Excluded duplicate/answered/skipped question {}",
                q.id
            ));
        } else if q.target.is_some_and(|p| {
            request.skipped.iter().any(|s| s.target == Some(p))
                || (request.ask_counts.get(&p).copied().unwrap_or(0) >= 2
                    && !request
                        .previous
                        .as_ref()
                        .is_some_and(|old| old.questions.iter().any(|s| same_question(s, &q))))
        }) {
            changes.push(format!(
                "Part question {} left open for seed: skipped or asked twice",
                q.id
            ));
        } else {
            questions.push(q);
        }
    }
    if questions.len() > 1 {
        changes.push("Extra questions dropped; one targeted question".into());
        questions.truncate(1);
    }
    let mut alternatives = vec![];
    for (i, v) in items(obj.get("alternatives"), "candidates", changes)
        .iter()
        .enumerate()
    {
        let o = v.as_object();
        if let Some(o) = o {
            extras(
                o,
                &["label", "benefit", "cost", "undo_cost"],
                &format!("candidate {i}"),
                changes,
            );
        }
        let Some(label) = display(
            o.and_then(|o| o.get("label")).or(Some(v)),
            &format!("candidate {i} label"),
            changes,
        ) else {
            continue;
        };
        let mut optional = |key: &str| {
            display(
                o.and_then(|o| o.get(key)),
                &format!("candidate {i} {key}"),
                changes,
            )
            .unwrap_or_default()
        };
        alternatives.push(Approach {
            label,
            benefit: optional("benefit"),
            cost: optional("cost"),
            undo_cost: optional("undo_cost"),
        });
    }
    let mut misfits = vec![];
    let mut unresolved_notes = vec![];
    for value in items(obj.get("supports"), "uncited supports", changes)
        .into_iter()
        .chain(
            items(obj.get("open"), "open prose", changes)
                .into_iter()
                .filter(|v| v.as_str().is_some_and(|s| s.split_whitespace().count() > 1)),
        )
    {
        if !value
            .as_object()
            .is_some_and(|o| ANCHOR.iter().any(|key| o.contains_key(*key)))
        {
            let text = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            if let Some(text) = display(
                Some(&serde_json::Value::String(text)),
                "uncited note",
                changes,
            ) {
                unresolved_notes.push(text);
                changes.push(
                    "Retained narrative support/open prose as uncited note; no highlight".into(),
                );
            }
        }
    }
    for (i, v) in items(obj.get("misfits"), "unresolved", changes)
        .into_iter()
        .chain(items(
            obj.get("unresolved_notes"),
            "unresolved notes",
            changes,
        ))
        .enumerate()
    {
        let object = v.as_object();
        if let Some(o) = object
            && !ANCHOR.iter().any(|k| o.contains_key(*k))
        {
            extras(o, &["text"], &format!("unresolved note {i}"), changes);
        }
        if object.is_some_and(|o| ANCHOR.iter().any(|k| o.contains_key(*k))) {
            misfits.push(anchor(&v, &format!("unresolved {i}"), changes)?);
        } else if let Some(note) = display(
            object.and_then(|o| o.get("text")).or(Some(&v)),
            &format!("unresolved note {i}"),
            changes,
        ) {
            unresolved_notes.push(note);
            changes.push(format!(
                "Retained unresolved {i} as an uncited note; no source highlight"
            ));
        }
    }
    unresolved_notes.extend(decision_notes);
    let guess = Guess {
        parts,
        other_goals,
        deferred,
        open,
        uncertain: false,
        framings,
        outcome,
        misfits,
        unresolved_notes,
        questions,
        alternatives,
    };
    guess.validate(request)?;
    if guess.questions.len() > 1 {
        changes.push(format!(
            "{} questions retained; up to 1 presented, remainder available in details",
            guess.questions.len()
        ));
    }
    Ok(guess)
}
pub(super) fn read(raw: &str, changes: &mut Vec<String>) -> Result<serde_json::Value, String> {
    let text = raw.trim();
    let text = if text.starts_with("```") && text.ends_with("```") {
        changes.push("Removed surrounding JSON code fence".into());
        text.split_once('\n')
            .map(|(_, s)| s[..s.len() - 3].trim())
            .unwrap_or(text)
    } else {
        text
    };
    serde_json::from_str(text)
        .or_else(|original| {
            let normalized = without_trailing_commas(text);
            if normalized == text {
                return Err(original);
            }
            let value = serde_json::from_str(&normalized)?;
            changes.push(
                "Removed trailing JSON collection comma outside strings; raw retained".into(),
            );
            Ok(value)
        })
        .map_err(|e| format!("Unreadable board response: {e}"))
}
// A separate review call sees only the person's words, their answers and the draft
fn without_trailing_commas(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &byte) in bytes.iter().enumerate() {
        if !quoted
            && byte == b','
            && bytes[i + 1..]
                .iter()
                .find(|b| !b.is_ascii_whitespace())
                .is_some_and(|b| matches!(b, b'}' | b']'))
        {
            continue;
        }
        out.push(byte);
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        }
    }
    String::from_utf8(out).expect("Removing ASCII commas preserves UTF-8")
}
fn extras(
    obj: &serde_json::Map<String, serde_json::Value>,
    known: &[&str],
    where_: &str,
    changes: &mut Vec<String>,
) {
    for key in obj.keys().filter(|k| !known.contains(&k.as_str())) {
        changes.push(format!(
            "Ignored extra {where_} field {key:?}; retained in raw evidence; never authority"
        ));
    }
}
pub(super) fn items(
    value: Option<&serde_json::Value>,
    where_: &str,
    changes: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    match value {
        Some(serde_json::Value::Array(v)) => v.clone(),
        None | Some(serde_json::Value::Null) => {
            changes.push(format!("Absent {where_}: empty collection"));
            vec![]
        }
        Some(v) => {
            changes.push(format!("Accepted singleton {where_}"));
            vec![v.clone()]
        }
    }
}
fn display(
    value: Option<&serde_json::Value>,
    where_: &str,
    changes: &mut Vec<String>,
) -> Option<String> {
    let Some(s) = value
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    else {
        changes.push(format!("Absent/unreadable optional {where_}"));
        return None;
    };
    let escaped = s
        .chars()
        .map(|c| {
            if c != '\n' && c.is_control() {
                changes.push(format!("Escaped display control in {where_}"));
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect::<String>();
    Some(escaped)
}
fn anchors(
    value: Option<&serde_json::Value>,
    where_: &str,
    changes: &mut Vec<String>,
) -> Result<Vec<Anchor>, String> {
    items(value, where_, changes)
        .iter()
        .enumerate()
        .filter(|(_, v)| {
            v.as_object()
                .is_some_and(|o| ANCHOR.iter().any(|key| o.contains_key(*key)))
        })
        .map(|(i, v)| anchor(v, &format!("{where_} {i}"), changes))
        .collect()
}
fn anchor(
    v: &serde_json::Value,
    where_: &str,
    changes: &mut Vec<String>,
) -> Result<Anchor, String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("Invalid claimed source span at {where_}"))?;
    extras(o, ANCHOR, where_, changes);
    let source = o
        .get("source")
        .and_then(|s| s.as_u64())
        .and_then(|s| usize::try_from(s).ok())
        .ok_or_else(|| format!("Invalid source reference at {where_}"))?;
    let quote = o
        .get("quote")
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("Missing exact source quote at {where_}"))?
        .to_owned();
    let occurrence = match o.get("occurrence") {
        None => {
            changes.push(format!("Defaulted {where_} occurrence to zero"));
            0
        }
        Some(v) => v
            .as_u64()
            .and_then(|s| usize::try_from(s).ok())
            .ok_or_else(|| format!("Invalid source occurrence at {where_}"))?,
    };
    Ok(Anchor {
        source,
        quote,
        occurrence,
    })
}
