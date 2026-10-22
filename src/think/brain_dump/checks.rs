//! The record each send leaves, shown by F5 "why" and written to the log: which check ran, what
//! it decided and why, and where the person's words landed. It records; it never changes a
//! reading or a decision. A check that rejects keeps the model's raw reply so the failure can
//! be read in full.

use super::*;
use std::time::Instant;

/// Where one of the person's words appears in the reading.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    /// The word, as the person wrote it.
    pub term: String,
    /// The reading's fields it appears in; empty when it appears nowhere. Not appearing is
    /// recorded, not treated as a failure.
    pub locations: Vec<String>,
}
/// One check's verdict on one send. The fields are the log's format, so renaming one breaks
/// readers of older logs; `version` changes when the format does.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    /// The record format's version.
    pub version: u8,
    /// What decoding or the review adjusted in the reply on the way in, one line each.
    #[serde(default)]
    pub normalizations: Vec<String>,
    /// A hash of the request, tying the decisions about one send together.
    pub request_key: String,
    /// Which check ran, such as `encoded-request-cap`.
    pub check: String,
    /// `blocking` when a rejection stops the reading from being shown.
    pub mode: String,
    /// `pass` or `reject`.
    pub decision: String,
    /// Why, in plain words.
    pub reason: String,
    /// How long the check took, including the model call.
    pub elapsed_ms: u64,
    /// Where each of the person's words landed.
    pub coverage: Vec<Coverage>,
    /// The model's reply as received, when there was one.
    pub raw: Option<String>,
    /// The decoded reading, when decoding succeeded.
    pub candidate: Option<Guess>,
}
impl Decision {
    pub(super) fn new(
        check: &str,
        mode: &str,
        decision: &str,
        reason: String,
        started: Instant,
    ) -> Self {
        Self {
            version: 3,
            normalizations: vec![],
            request_key: String::new(),
            check: check.into(),
            mode: mode.into(),
            decision: decision.into(),
            reason,
            elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            coverage: vec![],
            raw: None,
            candidate: None,
        }
    }
    /// The record as F5 shows it.
    pub fn text(&self) -> String {
        format!(
            "{} · {} · {} · {} ms · request {}\n{}{}",
            self.check,
            self.mode,
            self.decision,
            self.elapsed_ms,
            self.request_key,
            self.reason.clone()
                + &self
                    .normalizations
                    .iter()
                    .map(|s| format!("\nBoundary: {s}"))
                    .collect::<String>(),
            self.coverage
                .iter()
                .map(|c| format!(
                    "\n{}: {}",
                    c.term,
                    if c.locations.is_empty() {
                        "not found on board".into()
                    } else {
                        c.locations.join(", ")
                    }
                ))
                .collect::<String>()
        )
    }
}
pub(super) fn request_key(request: &BoardRequest) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_vec(request)
        .expect("validated request serializes")
        .hash(&mut hash);
    format!("{:016x}", hash.finish())
}
pub(super) fn coverage(request: &BoardRequest, g: &Guess) -> Vec<Coverage> {
    coverage_board(request, &serde_json::to_value(g).expect("guess serializes"))
}
fn coverage_board(request: &BoardRequest, g: &serde_json::Value) -> Vec<Coverage> {
    let mut terms = HashSet::new();
    for s in &request.sources {
        for word in s.text.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric());
            if (w.len() > 1 && w.len() < 32 && w.chars().all(|c| c.is_ascii_uppercase()))
                || w.chars().any(|c| c.is_ascii_digit())
                || [
                    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
                ]
                .contains(&w.to_lowercase().as_str())
            {
                terms.insert(w.to_owned());
            }
        }
    }
    let mut terms = terms.into_iter().collect::<Vec<_>>();
    terms.sort();
    terms
        .into_iter()
        .map(|term| {
            let contains = |s: &str| {
                s.split(|c: char| !c.is_alphanumeric())
                    .any(|w| w.eq_ignore_ascii_case(&term))
            };
            let mut locations = vec![];
            for (i, f) in g
                .get("framings")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .enumerate()
            {
                if f.get("text").and_then(|v| v.as_str()).is_some_and(contains) {
                    locations.push(format!("reading {}", i + 1));
                }
            }
            if g.get("outcome")
                .and_then(|v| v.as_str())
                .is_some_and(contains)
            {
                locations.push("outcome".into());
            }
            for option in g
                .get("alternatives")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                if option
                    .as_object()
                    .is_some_and(|o| o.values().filter_map(|v| v.as_str()).any(contains))
                {
                    locations.push(format!(
                        "candidate: {}",
                        option
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(invalid label)")
                    ));
                }
            }
            for q in g
                .get("questions")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                if q.get("text").and_then(|v| v.as_str()).is_some_and(contains) {
                    locations.push(format!(
                        "question: {}",
                        q.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(invalid ID)")
                    ));
                }
            }
            for span in g
                .get("misfits")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
            {
                if span
                    .get("quote")
                    .and_then(|v| v.as_str())
                    .is_some_and(contains)
                {
                    locations.push(format!(
                        "unresolved source {}",
                        span.get("source").unwrap_or(&serde_json::Value::Null)
                    ));
                }
            }
            Coverage { term, locations }
        })
        .collect()
}
pub(super) fn log(host: &PiHost, d: &Decision) {
    if let Ok(mut history) = host.brain_history.lock() {
        history.push(serde_json::to_string(d).expect("decision is serializable"));
    }
}
pub(super) fn why(history: &[String]) -> String {
    history
        .iter()
        .filter_map(|s| serde_json::from_str::<Decision>(s).ok())
        .map(|d| {
            let attempt = if d.decision == "reject" {
                d.raw
                    .as_ref()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .map(|g| format!("\nRejected attempt — not confirmable\n{}", attempt_text(&g)))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            format!("{}{}", d.text(), attempt)
                .chars()
                .filter(|c| *c == '\n' || !c.is_control())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
/// The checks as a think record keeps them: each as it serializes, except that the model's
/// reply stays only where F5 shows it, on a rejection. A passing check's reply is already in
/// its decoded `candidate`, so keeping both would only make long sessions' records large.
pub(super) fn recorded(history: &[String]) -> Vec<serde_json::Value> {
    history
        .iter()
        .filter_map(|s| serde_json::from_str::<Decision>(s).ok())
        .map(|mut d| {
            if d.decision != "reject" {
                d.raw = None;
            }
            serde_json::to_value(d).expect("decision is serializable")
        })
        .collect()
}
fn attempt_text(g: &serde_json::Value) -> String {
    let text = |v: Option<&serde_json::Value>| {
        v.map(|v| {
            v.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Invalid text field: {v}"))
        })
        .unwrap_or_else(|| "Missing text".into())
    };
    let readings = g
        .get("framings")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|f| agent_text(&text(f.get("text"))))
        .collect::<Vec<_>>()
        .join("\n\n");
    let options = g
        .get("alternatives")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|o| {
            o.as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| format!("{k}: {}", text(Some(v))))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|| format!("Invalid candidate: {o}"))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let questions = g
        .get("questions")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|q| text(q.get("text")))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{readings}\n\nSuccess criteria (unaccepted)\n{}\n\nPossible approaches (including unsupported fields)\n{options}\n\nQuestions\n{questions}\n\nRaw evidence retains every field; this summary is not an accepted board.",
        text(g.get("parts").and_then(|parts| parts.get("done_when")))
    )
}
