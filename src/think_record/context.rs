//! Optional reading context: ordered terminal presentations and references to submitted originals.
//! It explains sequence, never intent, attention or model meaning; old histories stay independent.

use super::{Event, utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

/// In-memory observations saved only with an explicitly confirmed Think record.
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingContext {
    events: Vec<Entry>,
}

/// Saved main reading text and its original groupings from one validated presentation.
/// These strings are copied verbatim, never recomposed from parts or decoded under new rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedReading {
    /// The presentation timestamp exactly as recorded, including its timezone.
    pub at: String,
    /// Reader-owned heading with the reading number and UTC shown time, dated when needed.
    pub heading: String,
    /// Each saved framing text, retaining empty strings and item boundaries.
    pub main: Vec<String>,
    /// Items classified as other goals in this reading, with their exact wording.
    pub other_goals: Vec<String>,
    /// Items deferred for the seed, with their exact wording.
    pub deferred: Vec<String>,
}

pub(super) struct ContextHistory {
    pub sequence: String,
    pub details: String,
    pub readings: Vec<RecordedReading>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Entry {
    Presented { at: String, board: Value },
    Submitted { source: usize },
}

fn submitted(event: &Event) -> Option<usize> {
    match event {
        Event::Words { source, .. }
        | Event::Answered { source, .. }
        | Event::ScopeAnswered { source, .. } => Some(*source),
        _ => None,
    }
}

#[derive(Deserialize)]
struct Snapshot {
    framings: Vec<Framing>,
    other_goals: Vec<String>,
    deferred: Vec<String>,
}

#[derive(Deserialize)]
struct Framing {
    text: String,
}

struct ValidatedEntry<'a> {
    at: String,
    time: time::OffsetDateTime,
    content: Content<'a>,
}

enum Content<'a> {
    Presented {
        board: &'a Value,
        snapshot: Snapshot,
    },
    Submitted {
        label: String,
    },
}

fn timestamp(at: &str) -> Result<time::OffsetDateTime, String> {
    time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339)
        .map(|at| at.to_offset(time::UtcOffset::UTC))
        .map_err(|_| "Invalid reading-context timestamp.".into())
}

fn date(date: time::Date) -> String {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {} {}",
        date.day(),
        months[date.month() as usize - 1],
        date.year()
    )
}

fn clock(at: time::OffsetDateTime) -> String {
    format!("{:02}:{:02}:{:02}", at.hour(), at.minute(), at.second())
}

impl ReadingContext {
    /// Record newly committed source words by their existing number, without copying their text.
    pub fn submitted(&mut self, source: usize) {
        self.events.push(Entry::Submitted { source });
    }

    /// Record the verified board associated with successful terminal output, not a screen replay.
    /// The caller serializes its board; this shared format never depends on a phase's types.
    pub fn presented(&mut self, board: Value, at: SystemTime) {
        self.events.push(Entry::Presented {
            at: utc(at, ':'),
            board,
        });
    }

    fn validate<'a, 'event>(
        &'a self,
        turns: impl Iterator<Item = (&'event Event, String)>,
    ) -> Result<Vec<ValidatedEntry<'a>>, String> {
        let sources: Vec<_> = turns
            .filter_map(|(event, at)| submitted(event).map(|source| (source, event, at)))
            .collect();
        if sources.first().is_some_and(|(source, ..)| *source == 0)
            || sources.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err("Reading context requires ordered unique source numbers.".into());
        }
        let mut next = 0;
        let mut entries = Vec::new();
        for entry in &self.events {
            match entry {
                Entry::Submitted { source } => {
                    let Some((expected, event, at)) = sources.get(next) else {
                        return Err("Reading context repeats or invents submitted words.".into());
                    };
                    if source != expected {
                        return Err("Reading context disagrees with submitted-source order.".into());
                    }
                    let label = event.source_label().ok_or("Expected submitted words.")?;
                    entries.push(ValidatedEntry {
                        at: at.clone(),
                        time: timestamp(at)?,
                        content: Content::Submitted { label },
                    });
                    next += 1;
                }
                Entry::Presented { at, board } => {
                    // Serde also accepts positional structs; recorded boards and framings require objects.
                    if !board.is_object()
                        || board["framings"]
                            .as_array()
                            .is_some_and(|rows| rows.iter().any(|row| !row.is_object()))
                    {
                        return Err("Reading context has an invalid presentation snapshot.".into());
                    }
                    // Only these historical fields are interpreted; retain the full frozen board.
                    let snapshot = Snapshot::deserialize(board)
                        .map_err(|_| "Reading context has an invalid presentation snapshot.")?;
                    if next == 0
                        || !snapshot
                            .framings
                            .iter()
                            .map(|row| &row.text)
                            .chain(&snapshot.other_goals)
                            .chain(&snapshot.deferred)
                            .any(|text| !text.trim().is_empty())
                    {
                        return Err("Reading context has an invalid presentation snapshot.".into());
                    }
                    entries.push(ValidatedEntry {
                        at: at.clone(),
                        time: timestamp(at)?,
                        content: Content::Presented { board, snapshot },
                    });
                }
            }
        }
        if next != sources.len() {
            return Err("Reading context is missing submitted words.".into());
        }
        Ok(entries)
    }

    pub(super) fn is_valid<'a>(&self, turns: impl Iterator<Item = (&'a Event, String)>) -> bool {
        self.validate(turns).is_ok()
    }
}

pub(super) fn history<'a>(
    value: &Value,
    turns: impl Iterator<Item = (&'a Event, String)>,
) -> ContextHistory {
    if value.is_null() {
        return ContextHistory {
            sequence: "Reading sequence was not recorded.".into(),
            details: "Reading context: not recorded.".into(),
            readings: vec![],
        };
    }
    let context = serde_json::from_value::<ReadingContext>(value.clone());
    let entries = context
        .as_ref()
        .ok()
        .and_then(|context| context.validate(turns).ok());
    let Some(entries) = entries else {
        return ContextHistory {
            sequence: "Reading sequence unavailable; saved words remain below.".into(),
            details:
                "Reading context: unavailable (invalid metadata). Originals remain independent."
                    .into(),
            readings: vec![],
        };
    };
    let mut lines = vec![
        "Reading context: terminal output, not proof of attention or a complete screen replay."
            .into(),
    ];
    let heading_date = entries.first().map(|entry| entry.time.date());
    let mut sequence = vec![heading_date.map_or_else(
        || "Recorded sequence".into(),
        |day| format!("Recorded sequence · {}, UTC", date(day)),
    )];
    let mut current_date = heading_date;
    let mut readings = Vec::new();
    let mut reading = 0;
    // List order is evidence, even when wall clocks go backward.
    for ValidatedEntry { at, time, content } in entries {
        if current_date != Some(time.date()) {
            sequence.push(date(time.date()));
            current_date = Some(time.date());
        }
        let shown = clock(time);
        match content {
            Content::Presented { board, snapshot } => {
                reading += 1;
                let day = if Some(time.date()) == heading_date {
                    String::new()
                } else {
                    format!(", {}", date(time.date()))
                };
                readings.push(RecordedReading {
                    at: at.clone(),
                    heading: format!("Reading {reading} · shown {shown}{day}"),
                    main: snapshot.framings.into_iter().map(|row| row.text).collect(),
                    other_goals: snapshot.other_goals,
                    deferred: snapshot.deferred,
                });
                sequence.push(format!("{shown}  Reading {reading} shown"));
                lines.push(format!(
                    "Presented reading {reading} at {at}\nVerified board associated with the frame:\n{}",
                    serde_json::to_string_pretty(board).expect("JSON snapshot serializes")
                ));
            }
            Content::Submitted { label } => {
                let relation = if reading == 0 {
                    String::new()
                } else {
                    format!(" · after reading {reading}")
                };
                sequence.push(format!("{shown}  {label}{relation}"));
                lines.push(format!("{label}{relation} at {at}."));
            }
        }
    }
    sequence.push("Shown means it reached the screen, not that it was read.".into());
    ContextHistory {
        sequence: sequence.join("\n"),
        details: lines.join("\n\n"),
        readings,
    }
}

// Cargo tracks option_env inputs itself. No runtime repository lookup can confuse the
// executable's source with the app repository, and unstamped builds remain honestly unknown.
pub(super) fn build_source(commit: Option<&str>, dirty: Option<&str>) -> Option<Value> {
    let (Some(commit), Some(dirty)) = (commit, dirty) else {
        return None;
    };
    if !valid_commit(commit) {
        return None;
    }
    let dirty = match dirty {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    Some(serde_json::json!({"commit": commit, "dirty": dirty}))
}

fn valid_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit.bytes().all(|b| b.is_ascii_hexdigit())
        && commit.bytes().any(|b| b != b'0')
}

pub(super) fn source_details(value: &Value) -> String {
    if value.is_null() {
        return "Tinkery source: unknown (unstamped build).".into();
    }
    match (
        value.get("commit").and_then(Value::as_str),
        value.get("dirty").and_then(Value::as_bool),
    ) {
        (Some(commit), Some(dirty)) if valid_commit(commit) => format!(
            "Tinkery source (supplied at build time, not authenticated): {commit}; modified: {dirty}.{}",
            if dirty {
                " Commit identifies the base, not the modified source."
            } else {
                ""
            }
        ),
        _ => "Tinkery source: unavailable (invalid build metadata).".into(),
    }
}

#[cfg(test)]
mod tests;
