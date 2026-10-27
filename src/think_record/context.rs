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

fn timestamp(at: &str) -> Result<(), String> {
    time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339)
        .map(|_| ())
        .map_err(|_| "Invalid reading-context timestamp.".into())
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

    pub(super) fn validate<'a>(
        &self,
        turns: impl Iterator<Item = (&'a Event, String)>,
    ) -> Result<(), String> {
        let sources: Vec<_> = turns
            .filter_map(|(event, at)| submitted(event).map(|source| (source, at)))
            .collect();
        if sources.first().is_some_and(|(source, _)| *source == 0)
            || sources.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err("Reading context requires ordered unique source numbers.".into());
        }
        let mut next = 0;
        for entry in &self.events {
            match entry {
                Entry::Submitted { source } => {
                    let Some((expected, at)) = sources.get(next) else {
                        return Err("Reading context repeats or invents submitted words.".into());
                    };
                    if source != expected {
                        return Err("Reading context disagrees with submitted-source order.".into());
                    }
                    timestamp(at)?;
                    next += 1;
                }
                Entry::Presented { at, board } => {
                    timestamp(at)?;
                    // Preserve this historical snapshot as data. Today's model decoder or board
                    // verification must not reinterpret it, including its frozen framing text.
                    let framings = board.get("framings").and_then(Value::as_array);
                    let extras = board.get("other_goals").and_then(Value::as_array);
                    let deferred = board.get("deferred").and_then(Value::as_array);
                    if next == 0
                        || !board.is_object()
                        || framings.is_none_or(|rows| rows.iter().any(|r| !r["text"].is_string()))
                        || extras.is_none_or(|rows| rows.iter().any(|r| !r.is_string()))
                        || deferred.is_none_or(|rows| rows.iter().any(|r| !r.is_string()))
                        || !(framings.is_some_and(|rows| {
                            rows.iter()
                                .any(|r| r["text"].as_str().is_some_and(|s| !s.trim().is_empty()))
                        }) || [extras, deferred].into_iter().flatten().any(|rows| {
                            rows.iter()
                                .any(|r| r.as_str().is_some_and(|s| !s.trim().is_empty()))
                        }))
                    {
                        return Err("Reading context has an invalid presentation snapshot.".into());
                    }
                }
            }
        }
        if next != sources.len() {
            return Err("Reading context is missing submitted words.".into());
        }
        Ok(())
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
    let turns: Vec<_> = turns.collect();
    let context = serde_json::from_value::<ReadingContext>(value.clone()).and_then(|context| {
        context
            .validate(turns.iter().map(|(event, at)| (*event, at.clone())))
            .map(|_| context)
            .map_err(serde::de::Error::custom)
    });
    let Ok(context) = context else {
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
    let mut sequence = vec![
        "Recorded order; presentation does not establish that someone read it.".into(),
        String::new(),
    ];
    let mut readings = Vec::new();
    let mut reading = 0;
    let mut submitted_times = turns
        .iter()
        .filter_map(|(event, at)| submitted(event).map(|_| at));
    for entry in context.events {
        match entry {
            Entry::Presented { at, board } => {
                let strings = |field: &str| {
                    board[field]
                        .as_array()
                        .expect("validated list")
                        .iter()
                        .map(|item| item.as_str().expect("validated string").to_owned())
                        .collect()
                };
                readings.push(RecordedReading {
                    at: at.clone(),
                    main: board["framings"]
                        .as_array()
                        .expect("validated framings")
                        .iter()
                        .map(|item| {
                            item["text"]
                                .as_str()
                                .expect("validated framing text")
                                .to_owned()
                        })
                        .collect(),
                    other_goals: strings("other_goals"),
                    deferred: strings("deferred"),
                });
                reading = readings.len();
                sequence.push(format!("{at}  Reading {reading} presented"));
                lines.push(format!(
                    "Presented reading {reading} at {at}\nVerified board associated with the frame:\n{}",
                    serde_json::to_string_pretty(&board).expect("JSON snapshot serializes")
                ));
            }
            Entry::Submitted { source } => {
                let at = submitted_times.next().expect("validated source");
                let context = if reading == 0 {
                    "before any recorded presentation".into()
                } else {
                    format!("after reading {reading}")
                };
                let relation = if reading == 0 {
                    "no earlier reading recorded".into()
                } else {
                    format!("after reading {reading}")
                };
                sequence.push(format!("{at}  Submission {source} · {relation}"));
                lines.push(format!("Submitted source {source} at {at}, {context}."));
            }
        }
    }
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
