use super::*;
use std::{collections::VecDeque, ops::Deref};

/// Nonempty collections can only be made by the verified board constructor.
/// ```compile_fail
/// use tinkery::shaping::brain_dump::board::{NonEmpty, Reading};
/// let empty = NonEmpty::<Reading>(vec![]);
/// ```
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NonEmpty<T>(Vec<T>);
impl<T> Deref for NonEmpty<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VerifiedAnchor {
    #[serde(flatten)]
    anchor: Anchor,
    #[serde(skip)]
    range: std::ops::Range<usize>,
}
impl Deref for VerifiedAnchor {
    type Target = Anchor;
    fn deref(&self) -> &Anchor {
        &self.anchor
    }
}
impl VerifiedAnchor {
    pub fn range(&self, sources: &[Source]) -> Result<std::ops::Range<usize>, String> {
        if self.anchor.range(sources)? != self.range {
            return Err("Verified source changed".into());
        }
        Ok(self.range.clone())
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum Grounding {
    Uncited,
    Cited(NonEmpty<VerifiedAnchor>),
}
impl Deref for Grounding {
    type Target = [VerifiedAnchor];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Uncited => &[],
            Self::Cited(spans) => spans,
        }
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Reading {
    pub text: String,
    pub supports: Grounding,
}
#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub label: String,
    pub benefit: Option<String>,
    pub cost: Option<String>,
    pub undo_cost: Option<String>,
}
impl Candidate {
    pub fn wire(&self) -> Approach {
        Approach {
            label: self.label.clone(),
            benefit: self.benefit.clone().unwrap_or_default(),
            cost: self.cost.clone().unwrap_or_default(),
            undo_cost: self.undo_cost.clone().unwrap_or_default(),
        }
    }
    pub fn text(&self) -> String {
        let mut text = agent_text(&self.label);
        for (label, value) in [
            ("Benefit", &self.benefit),
            ("Cost", &self.cost),
            ("Undo cost", &self.undo_cost),
        ] {
            if let Some(value) = value {
                text.push_str(&format!("\n{label}: {}", agent_text(value)));
            }
        }
        text
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct BoardData {
    pub framings: NonEmpty<Reading>,
    pub outcome: Option<String>,
    pub questions: VecDeque<Question>,
    pub alternatives: Vec<Candidate>,
    pub misfits: Vec<VerifiedAnchor>,
    pub unresolved_notes: Vec<String>,
}
/// Constructed only from checked input; no mutable access or model authority fields.
/// ```compile_fail
/// use tinkery::shaping::brain_dump::board::Board;
/// fn replace_history(board: &mut Board) {
///     board.questions.clear();
/// }
/// ```
#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct Board(BoardData);
impl Deref for Board {
    type Target = BoardData;
    fn deref(&self) -> &BoardData {
        &self.0
    }
}
impl Board {
    pub fn verify(g: Guess, request: &BoardRequest) -> Result<Self, String> {
        g.validate(request)?;
        let verify = |anchor: Anchor| -> Result<VerifiedAnchor, String> {
            let range = anchor.range(&request.sources)?;
            Ok(VerifiedAnchor { anchor, range })
        };
        Ok(Self(BoardData {
            framings: NonEmpty(
                g.framings
                    .into_iter()
                    .map(|f| {
                        Ok(Reading {
                            text: f.text,
                            supports: {
                                let spans = f.supports.into_iter().map(verify).collect::<Result<
                                    Vec<_>,
                                    String,
                                >>(
                                )?;
                                if spans.is_empty() {
                                    Grounding::Uncited
                                } else {
                                    Grounding::Cited(NonEmpty(spans))
                                }
                            },
                        })
                    })
                    .collect::<Result<_, String>>()?,
            ),
            outcome: (!g.outcome.is_empty()).then_some(g.outcome),
            questions: g.questions.into(),
            alternatives: g
                .alternatives
                .into_iter()
                .map(|a| {
                    let optional = |s: String| (!s.is_empty()).then_some(s);
                    Candidate {
                        label: a.label,
                        benefit: optional(a.benefit),
                        cost: optional(a.cost),
                        undo_cost: optional(a.undo_cost),
                    }
                })
                .collect(),
            misfits: g
                .misfits
                .into_iter()
                .map(verify)
                .collect::<Result<_, String>>()?,
            unresolved_notes: g.unresolved_notes,
        }))
    }
    pub fn presented(&self, _selected: usize) -> &[Reading] {
        &self.framings[..1]
    }
    pub fn wire(&self) -> Guess {
        Guess {
            uncertain: false,
            framings: self
                .framings
                .iter()
                .map(|r| Framing {
                    text: r.text.clone(),
                    supports: r.supports.iter().map(|s| s.anchor.clone()).collect(),
                })
                .collect(),
            outcome: self.outcome.clone().unwrap_or_default(),
            questions: self.questions.iter().cloned().collect(),
            alternatives: self.alternatives.iter().map(Candidate::wire).collect(),
            misfits: self.misfits.iter().map(|a| a.anchor.clone()).collect(),
            unresolved_notes: self.unresolved_notes.clone(),
        }
    }
}

pub(super) fn parse(
    raw: &str,
    request: &BoardRequest,
    changes: &mut Vec<String>,
) -> Result<Guess, String> {
    let text = raw.trim();
    let text = if text.starts_with("```") && text.ends_with("```") {
        changes.push("Removed surrounding JSON code fence".into());
        text.split_once('\n')
            .map(|(_, s)| s[..s.len() - 3].trim())
            .unwrap_or(text)
    } else {
        text
    };
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Unreadable board response: {e}"))?;
    let obj = value.as_object().ok_or("No readable board object")?;
    extras(
        obj,
        &[
            "uncertain",
            "framings",
            "outcome",
            "misfits",
            "unresolved_notes",
            "questions",
            "alternatives",
        ],
        "board",
        changes,
    );
    if obj.contains_key("uncertain") {
        changes.push("Legacy uncertainty flag ignored as a count/authority signal; all readings remain provisional".into());
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
        if let Some(text) = text {
            if supports.is_empty() {
                changes.push(format!("Reading {i} is uncited; no source highlight"));
            }
            framings.push(Framing { text, supports });
        } else {
            changes.push(format!("Dropped reading {i}: no readable text"));
        }
    }
    if framings.is_empty() {
        return Err("No readable interpretation in the response".into());
    }
    let outcome =
        display(obj.get("outcome"), "optional desired experience", changes).unwrap_or_default();
    let mut questions = vec![];
    for (i, v) in items(obj.get("questions"), "questions", changes)
        .iter()
        .enumerate()
    {
        let o = v.as_object();
        if let Some(o) = o {
            extras(o, &["id", "text"], &format!("question {i}"), changes);
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
        let q = Question { id, text };
        if q.id.starts_with("scope-addition-") {
            return Err("Model attempted an application-owned scope question ID".into());
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
        } else {
            questions.push(q);
        }
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
            && !["source", "quote", "occurrence"]
                .iter()
                .any(|k| o.contains_key(*k))
        {
            extras(o, &["text"], &format!("unresolved note {i}"), changes);
        }
        if object.is_some_and(|o| {
            ["source", "quote", "occurrence"]
                .iter()
                .any(|k| o.contains_key(*k))
        }) {
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
    let mut guess = Guess {
        uncertain: false,
        framings,
        outcome,
        misfits,
        unresolved_notes,
        questions,
        alternatives,
    };
    // One reading only: if the model is torn, the question decides. Extras are logged, never shown.
    if guess.framings.len() > 1 {
        changes.push(format!(
            "{} readings received; the first is presented, the rest are dropped",
            guess.framings.len()
        ));
        guess.framings.truncate(1);
    }
    guess.validate(request)?;
    if guess.questions.len() > 1 {
        changes.push(format!(
            "{} questions retained; up to 1 presented, remainder available in details",
            guess.questions.len()
        ));
    }
    Ok(guess)
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
fn items(
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
    extras(o, &["source", "quote", "occurrence"], where_, changes);
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
