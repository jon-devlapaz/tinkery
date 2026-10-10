use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Situation,
    Outcome,
    Why,
    Proof,
    Boundaries,
}
impl Part {
    pub const ALL: [Self; 5] = [
        Self::Situation,
        Self::Outcome,
        Self::Why,
        Self::Proof,
        Self::Boundaries,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Situation => "situation",
            Self::Outcome => "outcome",
            Self::Why => "why",
            Self::Proof => "proof",
            Self::Boundaries => "boundaries",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Situation => "when",
            Self::Outcome => "I can",
            Self::Why => "so",
            Self::Proof => "it works if",
            Self::Boundaries => "not",
        }
    }
}

fn prose_clause(text: &str) -> String {
    let chars = text
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .chars()
        .collect::<Vec<_>>();
    let mut quote = None;
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).and_then(|n| chars.get(n)).copied();
        let next = chars.get(i + 1).copied();
        if quote == Some(c) {
            quote = None;
        } else if quote.is_none()
            && (c == '"' || c == '“' || (c == '\'' && previous.is_none_or(char::is_whitespace)))
        {
            quote = Some(if c == '“' { '”' } else { c });
        }
        let decimal = c == '.'
            && previous.is_some_and(|c| c.is_ascii_digit())
            && next.is_some_and(|c| c.is_ascii_digit());
        if quote.is_none()
            && matches!(c, '.' | '!' | '?')
            && !decimal
            && next.is_some_and(char::is_whitespace)
        {
            out.push(';');
        } else if c == '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out.trim().to_owned()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct GoalParts {
    pub situation: Option<String>,
    pub outcome: Option<String>,
    pub why: Option<String>,
    pub proof: Option<String>,
    pub boundaries: Option<String>,
}
impl GoalParts {
    pub fn get(&self, part: Part) -> Option<&str> {
        let value = match part {
            Part::Situation => &self.situation,
            Part::Outcome => &self.outcome,
            Part::Why => &self.why,
            Part::Proof => &self.proof,
            Part::Boundaries => &self.boundaries,
        };
        value.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
    pub fn missing(&self) -> impl Iterator<Item = Part> + '_ {
        Part::ALL.into_iter().filter(|p| self.get(*p).is_none())
    }
    pub fn compose(&self) -> Option<String> {
        let clause = |part| self.get(part).map(prose_clause).filter(|s| !s.is_empty());
        let mut first = clause(Part::Situation).unwrap_or_default();
        if let Some(outcome) = clause(Part::Outcome) {
            if !first.is_empty() {
                first.push_str(", ");
            }
            if !outcome.to_lowercase().starts_with("i ")
                && !outcome.to_lowercase().starts_with("i’m ")
                && !outcome.to_lowercase().starts_with("i'm ")
            {
                first.push_str("I can ");
            }
            first.push_str(&outcome);
        }
        if let Some(why) = clause(Part::Why) {
            if !first.is_empty() {
                first.push_str(", so ");
            }
            first.push_str(why.strip_prefix("so ").unwrap_or(&why));
        }
        let mut second = String::new();
        if let Some(proof) = clause(Part::Proof) {
            second.push_str("It works if ");
            second.push_str(
                proof
                    .strip_prefix("It works if ")
                    .or_else(|| proof.strip_prefix("it works if "))
                    .unwrap_or(&proof),
            );
        }
        if let Some(boundaries) = clause(Part::Boundaries) {
            if !second.is_empty() {
                second.push_str("; ");
            }
            second.push_str(&boundaries);
        }
        let sentences = [first, second]
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|mut s| {
                if !s.ends_with(['.', '!', '?']) {
                    s.push('.');
                }
                s
            })
            .collect::<Vec<_>>();
        (!sentences.is_empty()).then(|| sentences.join(" "))
    }
}
