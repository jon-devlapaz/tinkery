//! A verified reading: a [`Board`] exists only once a decoded reply passes the checks against
//! the request (exact spans, application-owned questions, ask counts and skips).
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
    pub parts: GoalParts,
    pub other_goals: Vec<String>,
    pub deferred: Vec<String>,
    pub open: Vec<Part>,
    pub framings: Vec<Reading>,
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
    pub fn verify(mut g: Guess, request: &BoardRequest) -> Result<Self, String> {
        g.validate(request)?;
        for value in [
            &mut g.parts.who,
            &mut g.parts.outcome,
            &mut g.parts.when,
            &mut g.parts.why,
            &mut g.parts.done_when,
        ] {
            *value = value.take().map(|s| agent_text(&s));
        }
        for values in [
            &mut g.parts.must,
            &mut g.parts.must_not,
            &mut g.other_goals,
            &mut g.deferred,
            &mut g.unresolved_notes,
        ] {
            for value in values {
                *value = agent_text(value);
            }
        }
        g.parts.normalize_constraints();
        g.parts.distinct_check();
        if let Some(missing) = g.parts.missing().next() {
            g.questions
                .retain(|q| q.target.is_none_or(|p| p == missing));
            for q in &mut g.questions {
                q.target = Some(missing);
            }
            if g.questions.is_empty() {
                let who = g.parts.get(Part::Who).unwrap_or("the person this is for");
                let text = match missing {
                    Part::Who => "Who is this for?".into(),
                    Part::Outcome => format!("What should change for {who}?"),
                    Part::Why => format!("What is going wrong for {who}?"),
                    _ => format!(
                        "What would show that {}?",
                        g.parts.goal_line().unwrap_or_else(|| "this worked".into())
                    ),
                };
                g.questions.push(Question {
                    target: Some(missing),
                    id: format!(
                        "missing-{}-{}",
                        missing.name(),
                        request.sources.iter().map(|s| s.id).max().unwrap_or(0)
                    ),
                    text,
                });
            }
        }
        for part in g.questions.iter().filter_map(|q| q.target) {
            if !g.open.contains(&part) {
                g.open.push(part);
            }
        }
        g.validate(request)?;
        g.questions.retain(|q| {
            q.target.is_none_or(|p| {
                !request.skipped.iter().any(|s| s.target == Some(p))
                    && (request.ask_counts.get(&p).copied().unwrap_or(0) < 2
                        || request
                            .previous
                            .as_ref()
                            .is_some_and(|old| old.questions.iter().any(|s| same_question(s, q))))
            })
        });
        g.questions.truncate(1);
        let composed = g.parts.compose();
        let outcome = g.parts.done_when.clone().filter(|s| !s.trim().is_empty());
        let supports = g
            .framings
            .into_iter()
            .flat_map(|f| f.supports)
            .collect::<Vec<_>>();
        let verify = |anchor: Anchor| -> Result<VerifiedAnchor, String> {
            let range = anchor.range(&request.sources)?;
            Ok(VerifiedAnchor { anchor, range })
        };
        Ok(Self(BoardData {
            parts: g.parts,
            other_goals: g.other_goals,
            deferred: g.deferred,
            open: g.open,
            framings: composed
                .into_iter()
                .map(|text| {
                    Ok(Reading {
                        text,
                        supports: {
                            let spans = supports.clone().into_iter().map(verify).collect::<Result<
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
            outcome,
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
        &self.framings
    }
    pub fn wire(&self) -> Guess {
        Guess {
            parts: self.parts.clone(),
            other_goals: self.other_goals.clone(),
            deferred: self.deferred.clone(),
            open: self.open.clone(),
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
