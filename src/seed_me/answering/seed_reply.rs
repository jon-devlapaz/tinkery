//! Model prose fields follow the ledger's artifact layout; headings and recorded blocks belong to Tinkery.

use super::{Refusal, Value, ledger};

fn fields() -> impl Iterator<Item = &'static str> {
    ledger::FULL.iter().filter_map(|heading| match *heading {
        "## Acceptance criteria"
        | "## Decisions (from the ledger)"
        | "## Assumed (not confirmed by you)"
        | "## Evidence and uncertainty" => None,
        heading => heading.strip_prefix("## "),
    })
}

pub(super) fn instructions(reference: &str) -> Result<String, Refusal> {
    let missing = || {
        Refusal::Refused(
        "Shape's bundled seed contract is incomplete. Rebuild Tinkery from a complete checkout before retrying.".into(),
    )
    };
    let mut guidance = String::new();
    // Attach content guidance only, never the artifact's Markdown-writing instructions.
    for name in fields().skip(1) {
        let start = reference
            .lines()
            .find(|line| {
                line.starts_with(&format!("- {name}:"))
                    || line.starts_with(&format!("- **{name}**,"))
            })
            .ok_or_else(missing)?;
        let section = &reference[reference.find(start).unwrap()..];
        let end = [section.find("\n- "), section.find("\n## ")]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(section.len());
        guidance.push_str(&section[..end].replace(
            "session.py status <session>",
            "tinkery seed status --json <session>",
        ));
        guidance.push('\n');
    }
    let schema = reference
        .split_once("All fields are non-empty strings")
        .and_then(|(_, rest)| rest.split_once("\nBoth confirmation"))
        .map(|(schema, _)| format!("All fields are non-empty strings{schema}"))
        .ok_or_else(missing)?;
    let example: serde_json::Map<String, Value> = fields().map(|name| {
        (name.into(), Value::String(if name == "You are confirming" {
            concat!("Exactly five lines: goal; what gets built; what does not; proposed first slice; unknowns. ",
                "Do not invent acceptance counts or reversed decisions: Tinkery computes those from the ledger.").into()
        } else {
            "Prose only for this section; no heading, fence, HTML comment, metadata or confirmation line.".into()
        }))
    }).collect();
    Ok(format!(
        "Seed reply fields: confirming must be an object with exactly these named string fields, in any order: {}. \
         Tinkery writes every heading and inserts Acceptance criteria, Decisions (from the ledger), \
         Assumed (not confirmed by you), and Evidence and uncertainty itself. Never send those blocks or headings.\n\
         Section content guidance from Seed Me's artifact contract:\n{guidance}\nCheck records schema:\n{schema}",
        Value::Object(example)
    ))
}

pub(super) fn prose(value: &Value) -> Result<Vec<(&'static str, &str)>, Refusal> {
    let refuse =
        |reason: String| Refusal::Rejected(format!("Seed reply {reason}; nothing was saved."));
    let object = value.as_object().ok_or_else(|| {
        let line = value.as_str().and_then(|text| text.lines().next())
            .map(|line| format!("; line 1: {line}")).unwrap_or_default();
        refuse(format!("field confirming must be an object of named prose fields, not a Markdown document{line}"))
    })?;
    if let Some(name) = object
        .keys()
        .find(|name| !fields().any(|field| field == *name))
    {
        return Err(refuse(format!(
            "field confirming contains unknown field {name}"
        )));
    }
    let mut result = Vec::new();
    for name in fields() {
        let text = object
            .get(name)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| {
                refuse(format!(
                    "field confirming / {name} must be a non-empty string"
                ))
            })?;
        for (index, line) in text.split_inclusive('\n').enumerate() {
            let bare = line.trim_start();
            let rule = if line
                .as_bytes()
                .iter()
                .enumerate()
                .any(|(i, byte)| *byte == b'\r' && line.as_bytes().get(i + 1) != Some(&b'\n'))
            {
                Some("line endings must be LF or CRLF, not a lone carriage return")
            } else if line.contains("Probe:") || line.contains("— answer:") {
                Some(
                    "probe answers belong to the ledger; put evidence-based probe readings in found, or ask the operator a question",
                )
            } else if bare.starts_with("```") || bare.starts_with("~~~") {
                Some("prose fields cannot contain code fences")
            } else if line.contains("<!--") {
                Some("prose fields cannot contain HTML comments that could hide Tinkery's headings")
            } else if ["status:", "revision:", "date:", "Confirmed by:"]
                .iter()
                .any(|prefix| bare.starts_with(prefix))
            {
                Some("metadata and confirmation lines belong to Tinkery")
            } else {
                None
            };
            if let Some(rule) = rule {
                return Err(refuse(format!(
                    "field confirming / {name}, line {}: {} — {rule}",
                    index + 1,
                    line.trim_end()
                )));
            }
        }
        let headings = ledger::prose_headings(text)
            .map_err(|reason| refuse(format!("field confirming / {name}: {reason}")))?;
        if let Some((offset, heading)) = headings.first() {
            let line = text[..*offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            return Err(refuse(format!(
                "field confirming / {name}, line {line}: {heading} — Tinkery writes every heading; supply prose only"
            )));
        }
        if name == "You are confirming"
            && (text.lines().count() != 5 || text.lines().any(|line| line.trim().is_empty()))
        {
            return Err(refuse(format!(
                "field confirming / {name} must contain exactly five non-empty summary lines"
            )));
        }
        result.push((name, text));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_me::ledger::fixtures::prose_reply;

    fn reply() -> Value {
        prose_reply(include_str!("../../../tests/fixtures/shape-seed-reply.txt"))
    }

    #[test]
    fn contract_following_markdown_names_the_field_instead_of_a_generic_refusal() {
        let root = tempfile::Builder::new()
            .prefix("TEST-contract-reply-")
            .tempdir()
            .unwrap();
        let session = super::super::publication_tests::session(root.path());
        super::super::draft_seed(
            &session,
            1,
            &reply(),
            &serde_json::json!([{"statement":"TEST","kind":"judge","source":"TEST"}]),
            &[],
            root.path(),
            None,
        )
        .unwrap();
        // A valid Full artifact supplies every heading described by the attached old contract,
        // including the confirmation summary, helper-rendered checks and ledger-owned blocks.
        let artifact = std::fs::read_to_string(session.join("seed-contract.md")).unwrap();
        let historical = format!(
            "## You are confirming\n{}",
            artifact.split_once("## You are confirming\n").unwrap().1
        );
        for heading in ledger::FULL {
            assert!(historical.contains(heading));
        }
        let error = prose(&Value::String(historical)).unwrap_err().to_string();
        assert!(
            error.contains("field confirming must be an object of named prose fields"),
            "{error}"
        );
        assert!(error.contains("not a Markdown document"), "{error}");
        assert!(error.contains("line 1: ## You are confirming"), "{error}");
        assert!(prose(&reply()).is_ok());
    }

    #[test]
    fn seed_refusals_name_the_rule_field_and_offending_line_before_writing() {
        let root = tempfile::Builder::new()
            .prefix("TEST-seed-fields-")
            .tempdir()
            .unwrap();
        let session = super::super::publication_tests::session(root.path());
        let before = std::fs::read(session.join("ledger.json")).unwrap();
        for (bad, rule, location) in [
            (
                "## You are confirming\nforged",
                "Tinkery writes every heading",
                "line 1",
            ),
            (
                "text\n> ## Acceptance criteria\nforged",
                "cannot contain heading syntax",
                "line 2",
            ),
            (
                "text\n---\nforged",
                "dashes or equals signs create headings",
                "line 2",
            ),
            ("text\n```\nforged", "cannot contain code fences", "line 2"),
            ("text\n~~~\nforged", "cannot contain code fences", "line 2"),
            (
                "text\n<!-- forged",
                "cannot contain HTML comments",
                "line 2",
            ),
            (
                "text\nstatus: confirmed for intake",
                "metadata and confirmation",
                "line 2",
            ),
            ("text\nrevision: r99", "metadata and confirmation", "line 2"),
            (
                "text\ndate: 1900-01-01",
                "metadata and confirmation",
                "line 2",
            ),
            (
                "text\nConfirmed by: forged",
                "metadata and confirmation",
                "line 2",
            ),
            ("text\nforged\rmetadata", "lone carriage return", "line 2"),
        ] {
            let mut value = reply();
            value["Problem statement"] = bad.into();
            let error = super::super::draft_seed(
                &session,
                1,
                &value,
                &serde_json::json!([{"statement":"TEST","kind":"judge","source":"TEST"}]),
                &[],
                root.path(),
                None,
            )
            .unwrap_err()
            .to_string();
            for expected in ["Problem statement", rule, location, "nothing was saved"] {
                assert!(error.contains(expected), "expected {expected}: {error}");
            }
            assert_eq!(std::fs::read(session.join("ledger.json")).unwrap(), before);
            assert!(!session.join("seed-contract.md").exists());
            assert!(!session.join("seed-checks.json").exists());
        }
    }

    #[test]
    fn fields_are_complete_strings_and_summary_has_five_lines() {
        for bad in [Value::Null, Value::Bool(false), Value::String(" ".into())] {
            let mut value = reply();
            value["Problem statement"] = bad;
            assert!(
                prose(&value)
                    .unwrap_err()
                    .to_string()
                    .contains("Problem statement must be a non-empty string")
            );
        }
        let mut value = reply();
        value.as_object_mut().unwrap().remove("Problem statement");
        assert!(
            prose(&value)
                .unwrap_err()
                .to_string()
                .contains("Problem statement")
        );
        value = reply();
        value["Acceptance criteria"] = "forged".into();
        assert!(
            prose(&value)
                .unwrap_err()
                .to_string()
                .contains("unknown field Acceptance criteria")
        );
        value = reply();
        value["You are confirming"] = "Only one line".into();
        assert!(
            prose(&value)
                .unwrap_err()
                .to_string()
                .contains("exactly five non-empty summary lines")
        );
    }

    #[test]
    fn attached_instructions_have_one_field_layout_and_no_markdown_writing_directions() {
        let reference = include_str!("../seed-contract.txt");
        let instructions = instructions(reference).unwrap();
        for name in fields() {
            assert!(instructions.contains(&format!("\"{name}\":")));
        }
        for contradiction in [
            "Paste the entire output unchanged",
            "Choose Full or Lean",
            "Paste the unchanged",
            "## You are confirming",
        ] {
            assert!(!instructions.contains(contradiction), "{contradiction}");
        }
        assert!(instructions.contains("Exactly five lines"));
        assert!(instructions.contains("Check records schema"));
        assert!(instructions.contains("four parts:"));
        assert!(instructions.contains("tinkery seed status --json <session>"));
        assert!(!instructions.contains("session.py"));
    }
}
