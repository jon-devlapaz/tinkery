//! A decision question owns its exact wording and its optional pair of alternatives.
//! Only old questions without the choices field can carry a formatted option footer.

use super::{Result, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

const INVALID: &str = "Question options must be absent, empty, or exactly two complete choices with a tradeoff and a cheap, moderate, or hard undo cost.";

/// How difficult it is to undo a choice. A hard choice leaves room for an unaided answer.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Undo {
    /// The choice is cheap to undo.
    Cheap,
    /// The choice takes moderate effort to undo.
    Moderate,
    /// The operator sees the question before the choices.
    Hard,
}

impl Undo {
    fn word(self) -> &'static str {
        match self {
            Self::Cheap => "cheap",
            Self::Moderate => "moderate",
            Self::Hard => "hard",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        match word {
            "cheap" => Some(Self::Cheap),
            "moderate" => Some(Self::Moderate),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

/// A proposed answer with its tradeoff and undo cost. It records no operator decision.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    /// What the operator could choose.
    pub choice: String,
    /// The benefit and cost of making this choice.
    pub tradeoff: String,
    /// How hard this choice is to undo.
    pub undo: Undo,
}

/// Exactly two alternatives for a decision, with the optional case against its suggestion.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Choices {
    /// The two alternatives in the order offered.
    pub options: [Alternative; 2],
    /// The reason to consider an alternative to the suggestion.
    pub against: Option<String>,
}

impl Choices {
    /// Read the optional choices in a model reply. A malformed offer cannot lose its hard cost.
    pub(crate) fn from_reply(reply: &Json) -> Result<Option<Self>> {
        let Some(value) = reply.get("options") else {
            return Ok(None);
        };
        let rows = value.as_array().ok_or(INVALID)?;
        if rows.is_empty() {
            return Ok(None);
        }
        let [first, second] = rows.as_slice() else {
            return Err(INVALID.into());
        };
        let option = |row: &Json| -> Result<Alternative> {
            let field = |name: &str| row[name].as_str().map(str::trim).filter(|s| !s.is_empty());
            Ok(Alternative {
                choice: field("choice").ok_or(INVALID)?.into(),
                tradeoff: field("tradeoff").ok_or(INVALID)?.into(),
                undo: Undo::parse(&field("undo").ok_or(INVALID)?.to_ascii_lowercase())
                    .ok_or(INVALID)?,
            })
        };
        let against = match reply.get("against") {
            None | Some(Json::Null) => None,
            Some(value) => Some(
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or("Question options need nonblank text when a case against is supplied.")?
                    .into(),
            ),
        };
        Ok(Some(Self {
            options: [option(first)?, option(second)?],
            against,
        }))
    }

    /// True when either option requires the operator's instinct before disclosure.
    pub(crate) fn hard(&self) -> bool {
        self.options.iter().any(|option| option.undo == Undo::Hard)
    }

    /// Format both alternatives in their established terminal and command-line wording.
    fn lines(&self) -> [String; 2] {
        fn bare(s: &str) -> &str {
            s.trim_end_matches(['.', ';', ',', ' '])
        }
        std::array::from_fn(|i| {
            let option = &self.options[i];
            format!(
                "Option {}: {}; {}. Undo cost: {}.",
                ["A", "B"][i],
                bare(&option.choice),
                bare(&option.tradeoff),
                option.undo.word()
            )
        })
    }

    fn stored(value: &Json) -> Result<Self> {
        // Serde also accepts positional structs. The stored format requires named fields.
        if !value.is_object()
            || !value["options"]
                .as_array()
                .is_some_and(|rows| rows.iter().all(Json::is_object))
        {
            return Err(INVALID.into());
        }
        let choices = Self::deserialize(value).map_err(|_| INVALID.to_owned())?;
        if choices
            .options
            .iter()
            .any(|o| o.choice.trim().is_empty() || o.tradeoff.trim().is_empty())
            || choices
                .against
                .as_ref()
                .is_some_and(|words| words.trim().is_empty())
        {
            return Err(INVALID.into());
        }
        Ok(choices)
    }
}

/// What a stored question can show. Historical option lines retain their original punctuation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionView {
    /// Literal wording of the question, with only a recognized historical footer removed.
    pub ask: String,
    /// Display line and known undo cost of each option in its offered order.
    pub options: Vec<(String, Undo)>,
    /// Recorded case against the suggestion, when present.
    pub against: Option<String>,
}

impl QuestionView {
    /// Whether any displayed option has a hard undo cost.
    pub fn hard(&self) -> bool {
        self.options.iter().any(|(_, undo)| *undo == Undo::Hard)
    }
}

/// Read the question without changing the ledger or its confirmation digest.
pub(crate) fn read_question(node: &Json) -> Result<QuestionView> {
    let ask = node["question"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| node["gate"].as_str())
        .unwrap_or_default();
    let plain = || QuestionView {
        ask: ask.into(),
        options: vec![],
        against: None,
    };
    match node.get("choices") {
        None => Ok(legacy(ask).unwrap_or_else(plain)),
        Some(Json::Null) => Ok(plain()),
        Some(value) => {
            let choices = Choices::stored(value)?;
            Ok(QuestionView {
                ask: ask.into(),
                options: choices
                    .lines()
                    .into_iter()
                    .zip(choices.options.map(|o| o.undo))
                    .collect(),
                against: choices.against,
            })
        }
    }
}

fn legacy(stored: &str) -> Option<QuestionView> {
    // Old fields were unescaped. Slice complete blocks without interpreting their punctuation.
    const A: &str = "Option A: ";
    const B: &str = "Option B: ";
    const AGAINST: &str = "Against my suggestion: ";
    let starts = std::iter::once(0).chain(stored.match_indices('\n').map(|(at, _)| at + 1));
    let mut headings = starts.filter_map(|at| {
        [A, B, AGAINST]
            .into_iter()
            .find(|label| stored[at..].starts_with(label))
            .map(|label| (at, label))
    });
    let (start, first) = headings.find(|(_, label)| *label != AGAINST)?;
    let (prefix, footer) = stored.split_at(start);
    let ask = prefix.strip_suffix('\n').unwrap_or(prefix);
    let pair = match (first, headings.next(), headings.next(), headings.next()) {
        (A, Some((b, B)), None, None) => Some((b, stored.len(), None)),
        (A, Some((b, B)), Some((end, AGAINST)), None) => {
            let reason = &stored[end + AGAINST.len()..];
            (!reason.trim().is_empty()).then_some((b, end - 1, Some(reason)))
        }
        _ => None,
    };
    let mentions_hard = footer
        .lines()
        .any(|line| line.ends_with(" Undo cost: hard."));
    if let Some((b, end, against)) = pair {
        let a_line = &stored[start..b - 1];
        let b_line = &stored[b..end];
        let cost = |line: &str| {
            let (body, undo) = line.strip_suffix('.')?.rsplit_once(" Undo cost: ")?;
            if !body.strip_suffix('.')?.contains("; ") {
                return None;
            }
            Undo::parse(undo)
        };
        if let (Some(a_cost), Some(b_cost)) = (cost(a_line), cost(b_line))
            && !ask.trim().is_empty()
            && (!mentions_hard || a_cost == Undo::Hard || b_cost == Undo::Hard)
        {
            return Some(QuestionView {
                ask: ask.into(),
                options: vec![(a_line.into(), a_cost), (b_line.into(), b_cost)],
                against: against.map(str::to_owned),
            });
        }
    }
    // Withhold an ambiguous historical hard offer whole until the operator asks to see it.
    mentions_hard.then(|| QuestionView {
        ask: ask.into(),
        options: vec![(footer.into(), Undo::Hard)],
        against: None,
    })
}

/// Reject choices on any node that cannot own a decision question, including past revisions.
pub(in crate::seed_me::ledger) fn validate_choices(node: &Value) -> Result<()> {
    if let Some(value) = node.get("choices") {
        if !node["kind"].is("decision") || !super::text(&node["question"]) {
            return Err("Only a decision with a question can record choices.".into());
        }
        if value != &Value::Null {
            Choices::stored(&value.to_json()?)?;
        }
    }
    Ok(())
}
