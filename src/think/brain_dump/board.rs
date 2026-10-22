//! A verified reading: a [`Board`] exists only once a decoded reply passes the checks against
//! the request (exact spans, application-owned questions, ask counts and skips).
use super::*;
use std::{collections::VecDeque, ops::Deref};

/// Nonempty collections can only be made by the verified board constructor.
/// ```compile_fail
/// use tinkery::think::brain_dump::board::{NonEmpty, Reading};
/// let empty = NonEmpty::<Reading>(vec![]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmpty<T>(Vec<T>);
impl<T> Deref for NonEmpty<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}
/// An [`Anchor`] checked against the sources when the board was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAnchor {
    anchor: Anchor,
    range: std::ops::Range<usize>,
}
impl Deref for VerifiedAnchor {
    type Target = Anchor;
    fn deref(&self) -> &Anchor {
        &self.anchor
    }
}
impl VerifiedAnchor {
    /// The span to highlight, refused if `sources` no longer hold the words it was checked
    /// against.
    pub fn range(&self, sources: &[Source]) -> Result<std::ops::Range<usize>, String> {
        if self.anchor.range(sources)? != self.range {
            return Err("Verified source changed".into());
        }
        Ok(self.range.clone())
    }
}
/// Whether a reading cites the person's words. Uncited is allowed and shown plainly; it is
/// not a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Grounding {
    /// No citations.
    Uncited,
    /// At least one exact, verified span.
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
/// The reading as shown: the composed goal block and what it cites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The goal block.
    pub text: String,
    /// The person's words it rests on.
    pub supports: Grounding,
}
/// A proposed approach; an empty benefit or cost is `None` rather than shown blank.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// The approach in a few words.
    pub label: String,
    /// What it gains.
    pub benefit: Option<String>,
    /// What it costs to do.
    pub cost: Option<String>,
    /// What it costs to back out of later.
    pub undo_cost: Option<String>,
}
impl Candidate {
    /// The approach in the model's wire shape, for the next request.
    pub fn wire(&self) -> Approach {
        Approach {
            label: self.label.clone(),
            benefit: self.benefit.clone().unwrap_or_default(),
            cost: self.cost.clone().unwrap_or_default(),
            undo_cost: self.undo_cost.clone().unwrap_or_default(),
        }
    }
    /// The approach as shown: label, then each known benefit and cost on its own line.
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
/// What a verified board holds, readable through [`Board`] but never writable. The fields mean
/// what [`Guess`]'s do, after verification.
#[derive(Clone, Debug)]
pub struct BoardData {
    /// The goal's parts, cleaned and with constraints normalised.
    pub parts: GoalParts,
    /// Separate goals, kept aside.
    pub other_goals: Vec<String>,
    /// Things explicitly put off.
    pub deferred: Vec<String>,
    /// Parts still open, including every part a question targets.
    pub open: Vec<Part>,
    /// The displayed goal block and what it cites; `None` while the parts can't compose one.
    /// Read the block through [`Board::goal`], so nothing downstream composes it again.
    pub reading: Option<Reading>,
    /// The done-when part, if any.
    pub outcome: Option<String>,
    /// The one question to ask, if any.
    pub questions: VecDeque<Question>,
    /// Proposed approaches.
    pub alternatives: Vec<Candidate>,
    /// The person's words that don't fit the reading.
    pub misfits: Vec<VerifiedAnchor>,
    /// Open choices and explicit links to their questions, retained even when asking is skipped.
    pub decisions: Vec<OpenDecision>,
    /// Open intent that isn't a question yet.
    pub unresolved_notes: Vec<String>,
}
/// Constructed only from checked input; no mutable access or model authority fields.
/// ```compile_fail
/// use tinkery::think::brain_dump::board::Board;
/// fn replace_history(board: &mut Board) {
///     board.questions.clear();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Board(BoardData);
impl Deref for Board {
    type Target = BoardData;
    fn deref(&self) -> &BoardData {
        &self.0
    }
}
impl Board {
    /// The only way to make a board. Refuses a guess that fails [`Guess::validate`]; otherwise
    /// strips the model's self-labels, normalises constraints, drops a why or done-when that
    /// only repeats the outcome, and keeps one question about missing or open intent. Missing
    /// fields do not displace a consequential question or require template completion.
    /// A selected question bounded by success alternatives becomes open-ended without repeating
    /// the headline. Other questions stay intact; no missing field or answer length creates one.
    /// A part skipped or already asked twice isn't asked again.
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
        for d in &g.decisions {
            if !g.open.contains(&d.part) {
                g.open.push(d.part);
            }
        }
        g.parts.normalize_constraints();
        g.parts.distinct_check();
        if let Some(selected) = g
            .questions
            .first()
            .filter(|q| {
                q.target == Some(Part::DoneWhen)
                    && g.parts.get(Part::DoneWhen).is_none()
                    && g.decisions
                        .iter()
                        .any(|d| d.covered_by(std::slice::from_ref(q)))
            })
            .cloned()
        {
            let text = "What would tell you this worked?";
            for d in g
                .decisions
                .iter_mut()
                .filter(|d| d.covered_by(std::slice::from_ref(&selected)))
            {
                if d.question.as_deref() != Some(text) {
                    d.suggestion = None;
                }
                d.question = Some(text.into());
            }
            let q = &mut g.questions[0];
            if q.text != text {
                q.suggestion = None;
            }
            q.text = text.into();
            if request.answered.contains(&q.id)
                || request
                    .settled
                    .iter()
                    .any(|s| same_question(&s.question, q))
                || request.skipped.iter().any(|s| same_question(s, q))
            {
                g.questions.clear();
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
            reading: composed
                .map(|text| -> Result<_, String> {
                    let spans = supports
                        .into_iter()
                        .map(verify)
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Reading {
                        text,
                        supports: if spans.is_empty() {
                            Grounding::Uncited
                        } else {
                            Grounding::Cited(NonEmpty(spans))
                        },
                    })
                })
                .transpose()?,
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
            decisions: g.decisions,
        }))
    }
    /// The goal block exactly as shown. Review freezes it and refinement compares it; it is
    /// never recomposed from the parts after verification.
    pub fn goal(&self) -> Option<&str> {
        self.reading.as_ref().map(|r| r.text.as_str())
    }
    /// Test-only: replaces the displayed block, leaving the parts alone. Verification never
    /// makes such a board; tests use it to prove consumers read the block instead of
    /// recomposing it from the parts.
    #[cfg(test)]
    pub(super) fn with_displayed_goal(mut self, text: &str) -> Self {
        if let Some(reading) = &mut self.0.reading {
            reading.text = text.into();
        }
        self
    }
    /// The board in the model's wire shape, sent back as `previous` so the next reading
    /// refines this one.
    pub fn wire(&self) -> Guess {
        Guess {
            parts: self.parts.clone(),
            other_goals: self.other_goals.clone(),
            deferred: self.deferred.clone(),
            open: self.open.clone(),
            uncertain: false,
            framings: self
                .reading
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
            decisions: self.decisions.clone(),
        }
    }
}
