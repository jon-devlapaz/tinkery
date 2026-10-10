use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Who,
    Outcome,
    When,
    Why,
    DoneWhen,
    Must,
    MustNot,
}
impl Part {
    pub const ALL: [Self; 7] = [
        Self::Who,
        Self::Outcome,
        Self::When,
        Self::Why,
        Self::DoneWhen,
        Self::Must,
        Self::MustNot,
    ];
    pub const REQUIRED: [Self; 4] = [Self::Who, Self::Outcome, Self::Why, Self::DoneWhen];
    pub fn name(self) -> &'static str {
        match self {
            Self::Who => "who",
            Self::Outcome => "outcome",
            Self::When => "when",
            Self::Why => "why",
            Self::DoneWhen => "done_when",
            Self::Must => "must",
            Self::MustNot => "must_not",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Who => "Who",
            Self::Outcome => "Outcome",
            Self::When => "When",
            Self::Why => "Why",
            Self::DoneWhen => "Done when",
            Self::Must => "Must",
            Self::MustNot => "Must not",
        }
    }
}
fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}
pub(super) fn identity(text: &str) -> String {
    text.trim()
        .trim_end_matches(['.', ',', ';', '!', '?'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn clauses(text: &str) -> Vec<String> {
    let chars = text
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .chars()
        .collect::<Vec<_>>();
    let mut quote = None;
    let mut parts = vec![];
    let mut current = String::new();
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
            && (c == ';'
                || (matches!(c, '.' | '!' | '?')
                    && !decimal
                    && next.is_some_and(char::is_whitespace)))
        {
            parts.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(if c == '\n' { ' ' } else { c });
        }
    }
    parts.push(current.trim().to_owned());
    parts.into_iter().filter(|s| !s.is_empty()).collect()
}
fn prose(text: &str) -> String {
    clauses(text)
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            if i == 0 {
                s
            } else {
                let mut chars = s.chars();
                chars.next().map_or_else(String::new, |c| {
                    c.to_lowercase().collect::<String>() + chars.as_str()
                })
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct GoalParts {
    pub who: Option<String>,
    pub outcome: Option<String>,
    pub when: Option<String>,
    pub why: Option<String>,
    pub done_when: Option<String>,
    #[serde(default)]
    pub must: Vec<String>,
    #[serde(default)]
    pub must_not: Vec<String>,
}
impl GoalParts {
    pub fn get(&self, part: Part) -> Option<&str> {
        let value = match part {
            Part::Who => self.who.as_ref(),
            Part::Outcome => self.outcome.as_ref(),
            Part::When => self.when.as_ref(),
            Part::Why => self.why.as_ref(),
            Part::DoneWhen => self.done_when.as_ref(),
            Part::Must => self.must.first(),
            Part::MustNot => self.must_not.first(),
        };
        value
            .map(String::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
    pub fn missing(&self) -> impl Iterator<Item = Part> + '_ {
        Part::REQUIRED
            .into_iter()
            .filter(|p| self.get(*p).is_none())
    }
    pub fn goal_line(&self) -> Option<String> {
        let (who, outcome) = (self.get(Part::Who)?, self.get(Part::Outcome)?);
        let raw_who = prose(who);
        let who = capitalise(&raw_who);
        let mut outcome = prose(outcome);
        if outcome
            .get(..raw_who.len())
            .is_some_and(|prefix| identity(prefix) == identity(&raw_who))
            && outcome
                .get(raw_who.len()..)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        {
            outcome = outcome[raw_who.len()..].trim_start().to_owned();
        }
        while outcome.starts_with("can can ") {
            outcome = outcome[4..].to_owned();
        }
        let mut goal = format!("{who} {outcome}");
        if let Some(when) = self.get(Part::When).map(prose)
            && !identity(&goal).contains(&identity(&when))
        {
            goal.push_str(&format!(", {when}"));
        }
        Some(goal)
    }
    pub(super) fn distinct_check(&mut self) {
        if let Some(why) = self.get(Part::Why)
            && self
                .get(Part::Outcome)
                .is_some_and(|outcome| identity(why) == identity(outcome))
        {
            self.why = None;
        }
        if let Some(done) = self.get(Part::DoneWhen)
            && (self
                .goal_line()
                .is_some_and(|line| identity(&line) == identity(done))
                || self
                    .get(Part::Outcome)
                    .is_some_and(|outcome| identity(outcome) == identity(done)))
        {
            self.done_when = None;
        }
    }
    pub fn compose(&self) -> Option<String> {
        let finish = |s: String| {
            if s.ends_with(['.', '!', '?']) {
                s
            } else {
                s + "."
            }
        };
        let mut lines = vec![];
        if let Some(goal) = self.goal_line() {
            lines.push(finish(goal));
        }
        for part in [Part::Why, Part::DoneWhen] {
            if let Some(value) = self.get(part) {
                lines.push(finish(format!("{}: {}", part.label(), prose(value))));
            }
        }
        for (label, values) in [("Must", &self.must), ("Must not", &self.must_not)] {
            let mut seen = std::collections::BTreeSet::new();
            for value in values.iter().flat_map(|s| clauses(s)) {
                if seen.insert(identity(&value)) {
                    lines.push(finish(format!("{label}: {value}")));
                }
            }
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}
