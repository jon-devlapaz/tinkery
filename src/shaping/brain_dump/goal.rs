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
/// Constraint identity for de-duplication: like `identity`, but only the first letter is
/// case-folded, so "Upload logs" matches "upload logs" while "use IT" stays distinct
/// from "use it".
fn constraint_identity(text: &str) -> String {
    let joined = text
        .trim()
        .trim_end_matches(['.', ',', ';', '!', '?'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = joined.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_lowercase().collect::<String>() + chars.as_str()
    })
}
fn clauses(text: &str) -> Vec<String> {
    ended_clauses(text).into_iter().map(|(s, _)| s).collect()
}
/// Clauses with the separator that ended each (`.` for the last).
fn ended_clauses(text: &str) -> Vec<(String, char)> {
    let chars = text
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .chars()
        .collect::<Vec<_>>();
    // A single quote (straight or curly) closes only at the last one of its kind that
    // isn't an apostrophe inside a word ("don’t"), so a possessive ("users' choices")
    // inside a quotation doesn't end it early.
    let closes = |kind: char| {
        (0..chars.len())
            .rev()
            .find(|&n| chars[n] == kind && !chars.get(n + 1).is_some_and(|c| c.is_alphanumeric()))
    };
    let (last_straight, last_curly) = (closes('\''), closes('’'));
    let mut quote = None;
    let mut parts = vec![];
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).and_then(|n| chars.get(n)).copied();
        let next = chars.get(i + 1).copied();
        let closing = match c {
            '\'' => last_straight == Some(i),
            '’' => last_curly == Some(i),
            _ => true,
        };
        if quote == Some(c) && closing {
            quote = None;
        } else if quote.is_none()
            && (c == '"'
                || c == '“'
                || (matches!(c, '\'' | '‘') && !previous.is_some_and(char::is_alphanumeric)))
        {
            quote = Some(match c {
                '“' => '”',
                '‘' => '’',
                c => c,
            });
        }
        let decimal = c == '.'
            && previous.is_some_and(|c| c.is_ascii_digit())
            && next.is_some_and(|c| c.is_ascii_digit());
        // A period after a lone letter or a common abbreviation ("U.S. data", "e.g. maps",
        // "Dr. Smith") is not a clause end unless a capitalised constraint starts after it
        // ("U.S. Do not upload"; "the U.S. must remain" stays one clause).
        let word = if c == '.' {
            chars[..i]
                .iter()
                .rev()
                .take_while(|c| c.is_alphabetic())
                .count()
        } else {
            0
        };
        let initial = word > 0
            && (word == 1 || {
                let w = chars[i - word..i].iter().collect::<String>();
                ABBREVIATIONS.iter().any(|a| a.eq_ignore_ascii_case(&w))
            })
            && (i - word)
                .checked_sub(1)
                .and_then(|n| chars.get(n))
                .is_none_or(|c| !c.is_alphanumeric())
            && !(starts_with_syntax(&chars[i + 1..])
                && chars[i + 1..]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| c.is_uppercase()));
        if quote.is_none()
            && (c == ';'
                || (matches!(c, '.' | '!' | '?')
                    && !decimal
                    && !initial
                    && next.is_some_and(char::is_whitespace)))
        {
            parts.push((current.trim().to_owned(), c));
            current.clear();
        } else {
            current.push(if c == '\n' { ' ' } else { c });
        }
    }
    parts.push((current.trim().to_owned(), '.'));
    parts.into_iter().filter(|(s, _)| !s.is_empty()).collect()
}
/// A value split only where the next clause starts with constraint syntax, so other
/// periods ("Dr. Smith") never turn one stated constraint into two.
fn constraint_segments(text: &str) -> Vec<String> {
    let mut segments: Vec<String> = vec![];
    let mut ended = '.';
    for (clause, end) in ended_clauses(text) {
        let starts = starts_with_syntax(&clause.chars().collect::<Vec<_>>());
        match segments.last_mut() {
            Some(last) if !starts => last.push_str(&format!("{ended} {clause}")),
            _ => segments.push(clause),
        }
        ended = end;
    }
    segments
}
/// Abbreviations whose period is not a clause end (see `ended_clauses`).
const ABBREVIATIONS: [&str; 14] = [
    "co", "dr", "etc", "inc", "jr", "ltd", "mr", "mrs", "ms", "mt", "prof", "sr", "st", "vs",
];
/// Common words lowered when they start a later clause. Any other first word (a name,
/// acronym such as "IT", quote or number) keeps its case.
const LOWERED: [&str; 34] = [
    "a", "all", "also", "an", "and", "both", "but", "each", "every", "he", "her", "his", "if",
    "it", "its", "no", "not", "or", "our", "she", "so", "some", "that", "the", "their", "then",
    "there", "these", "they", "this", "those", "we", "you", "your",
];
fn prose(text: &str) -> String {
    clauses(text)
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            let first = s.split_whitespace().next().unwrap_or_default();
            let mut chars = s.chars();
            match chars.next() {
                Some(c)
                    if i > 0
                        && LOWERED.contains(&first.to_lowercase().as_str())
                        && !first.chars().skip(1).any(char::is_uppercase) =>
                {
                    c.to_lowercase().collect::<String>() + chars.as_str()
                }
                _ => s,
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
/// Explicit leading negative syntax, matched case-insensitively; see goal-block.md.
const NEGATIVE: [&str; 4] = ["must not", "do not", "don't", "don\u{2019}t"];
/// Words after a negative prefix that make the clause something other than an exclusion.
const NOT_AN_EXCLUSION: [&str; 6] = ["only", "just", "necessarily", "always", "care", "mind"];
fn after_prefix<'a>(clause: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = clause
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .and_then(|_| clause[prefix.len()..].strip_prefix(char::is_whitespace))?
        .trim_start();
    (!rest.is_empty()).then_some(rest)
}
/// Whether `chars` (after any whitespace) begins with constraint syntax that rewriting
/// accepts (`constraint_syntax`), read from a bounded head so it never copies the rest.
/// 32 visible characters cover any supported prefix plus the word after it.
fn starts_with_syntax(chars: &[char]) -> bool {
    let start = chars
        .iter()
        .position(|c| !c.is_whitespace())
        .unwrap_or(chars.len());
    // Bounded by visible characters only, so long runs of spaces never cut the payload.
    let mut visible = 0;
    let head = chars[start..]
        .iter()
        .take_while(|c| {
            visible += usize::from(!c.is_whitespace());
            visible <= 32
        })
        .collect::<String>();
    constraint_syntax(&head, false).is_some()
}
/// Where an unambiguous leading prefix puts this clause: (is an exclusion, remainder).
fn constraint_syntax(clause: &str, negative: bool) -> Option<(bool, &str)> {
    if let Some(rest) = NEGATIVE.iter().find_map(|p| after_prefix(clause, p)) {
        // The qualifier is the leading letters/digits, so "necessarily—at" still reads
        // as "necessarily".
        let first = rest
            .split(|c: char| !c.is_alphanumeric())
            .find(|w| !w.is_empty())?
            .to_lowercase();
        return (!NOT_AN_EXCLUSION.contains(&first.as_str())).then_some((true, rest));
    }
    (!negative)
        .then(|| after_prefix(clause, "must"))
        .flatten()
        // "must not" followed by anything but a plain space ("must not:", "must not—…")
        // is unsupported syntax: leave it whole rather than strip only "must".
        .filter(|rest| {
            !rest.get(..3).is_some_and(|w| w.eq_ignore_ascii_case("not"))
                || rest[3..].starts_with(char::is_alphanumeric)
        })
        .map(|rest| (false, rest))
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
    /// Rewrites only explicit leading `must`/`must not`/`do not`/`don't` in constraints;
    /// anything else, and every original source, stays as written.
    pub(super) fn normalize_constraints(&mut self) {
        let mut lists: [Vec<(String, bool)>; 2] = Default::default();
        for (negative, values) in [
            (false, std::mem::take(&mut self.must)),
            (true, std::mem::take(&mut self.must_not)),
        ] {
            for value in values {
                let split = constraint_segments(&value);
                let rewrites = split
                    .iter()
                    .map(|c| constraint_syntax(c, negative).map(|(n, rest)| (n, rest.to_owned())))
                    .collect::<Vec<_>>();
                if rewrites.iter().all(Option::is_none) {
                    lists[usize::from(negative)].push((value, false));
                    continue;
                }
                for (segment, rewrite) in split.into_iter().zip(rewrites) {
                    match rewrite {
                        Some((to, rest)) => lists[usize::from(to)].push((rest, true)),
                        None => lists[usize::from(negative)].push((segment, false)),
                    }
                }
            }
        }
        // A rewritten constraint already stated in its category is not repeated: one
        // pass over the identities of every unrewritten entry and earlier rewrites.
        let [must, must_not] = lists.map(|list| {
            let ids = |v: &str| {
                clauses(v)
                    .iter()
                    .map(|c| constraint_identity(c))
                    .collect::<Vec<_>>()
            };
            let mut stated = list
                .iter()
                .filter(|(_, rewritten)| !rewritten)
                .flat_map(|(v, _)| ids(v))
                .collect::<std::collections::HashSet<_>>();
            list.into_iter()
                .filter_map(|(value, rewritten)| {
                    if !rewritten {
                        return Some(value);
                    }
                    let own = ids(&value);
                    let repeated = own.iter().all(|id| stated.contains(id));
                    stated.extend(own);
                    (!repeated).then_some(value)
                })
                .collect()
        });
        self.must = must;
        self.must_not = must_not;
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
                if seen.insert(constraint_identity(&value)) {
                    lines.push(finish(format!("{label}: {value}")));
                }
            }
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}
