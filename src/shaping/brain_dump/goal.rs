use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Who,
    Outcome,
    When,
    Why,
    DoneWhen,
    Keep,
    Avoid,
}
impl Part {
    pub const ALL: [Self; 7] = [
        Self::Who,
        Self::Outcome,
        Self::When,
        Self::Why,
        Self::DoneWhen,
        Self::Keep,
        Self::Avoid,
    ];
    pub const REQUIRED: [Self; 4] = [Self::Who, Self::Outcome, Self::Why, Self::DoneWhen];
    pub fn name(self) -> &'static str {
        match self {
            Self::Who => "who",
            Self::Outcome => "outcome",
            Self::When => "when",
            Self::Why => "why",
            Self::DoneWhen => "done_when",
            Self::Keep => "keep",
            Self::Avoid => "avoid",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Who => "Who",
            Self::Outcome => "Outcome",
            Self::When => "When",
            Self::Why => "Why",
            Self::DoneWhen => "Done when",
            Self::Keep => "Keep",
            Self::Avoid => "Avoid",
        }
    }
}

fn case_first(text: &str, upper: bool) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        let first = if upper {
            first.to_uppercase().collect::<String>()
        } else {
            first.to_lowercase().collect::<String>()
        };
        first + chars.as_str()
    })
}

fn prose_clause(text: &str) -> String {
    let chars = text
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .chars()
        .collect::<Vec<_>>();
    let mut quote = None;
    let mut clauses = vec![];
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
            clauses.push(current.trim().to_owned());
            current.clear();
        } else if c == '\n' {
            current.push(' ');
        } else {
            current.push(c);
        }
    }
    clauses.push(current.trim().to_owned());
    clauses
        .into_iter()
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, s)| if i == 0 { s } else { case_first(&s, false) })
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
    pub keep: Option<String>,
    pub avoid: Option<String>,
}
impl GoalParts {
    pub fn get(&self, part: Part) -> Option<&str> {
        let value = match part {
            Part::Who => &self.who,
            Part::Outcome => &self.outcome,
            Part::When => &self.when,
            Part::Why => &self.why,
            Part::DoneWhen => &self.done_when,
            Part::Keep => &self.keep,
            Part::Avoid => &self.avoid,
        };
        value.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
    pub fn missing(&self) -> impl Iterator<Item = Part> + '_ {
        Part::REQUIRED
            .into_iter()
            .filter(|p| self.get(*p).is_none())
    }
    pub fn compose(&self) -> Option<String> {
        let clause = |part| self.get(part).map(prose_clause).filter(|s| !s.is_empty());
        let mut lines = vec![];
        let finish = |text: String| {
            if text.ends_with(['.', '!', '?']) {
                text
            } else {
                text + "."
            }
        };
        if let (Some(who), Some(outcome)) = (clause(Part::Who), clause(Part::Outcome)) {
            let outcome = outcome
                .strip_prefix("can ")
                .or_else(|| outcome.strip_prefix("Can "))
                .unwrap_or(&outcome);
            let mut goal = format!("{} can {}", case_first(&who, true), outcome);
            if let Some(when) = clause(Part::When) {
                goal.push_str(&format!(", {when}"));
            }
            lines.push(finish(goal));
        }
        for part in [Part::Why, Part::DoneWhen, Part::Keep, Part::Avoid] {
            if let Some(value) = clause(part) {
                lines.push(finish(format!("{}: {value}", part.label())));
            }
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}
