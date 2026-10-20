//! Acceptance records own their rendered section; commands remain inert data, never executions.

use super::{
    Result, Value,
    python::{text, whitespace},
    require,
};
use regex::Regex;
use std::{fs, path::Path};

pub(super) const FILE: &str = "seed-checks.json";

/// Refuse unknown fields, blank values and non-relative working directories without coercion.
pub fn validate_checks(records: &Value) -> Result<()> {
    require(
        records.array().is_some() && records.truth(),
        "checks must be a non-empty JSON array",
    )?;
    for (index, record) in records.items().iter().enumerate() {
        let prefix = format!("check {}", index + 1);
        require(
            record.object().is_some(),
            format!("{prefix} must be an object"),
        )?;
        let (required, optional): (&[&str], &[&str]) = match record["kind"].string() {
            Some("cmd") => (
                &["statement", "kind", "source", "cmd", "expect", "cwd"],
                &[],
            ),
            Some("provisional") => (
                &["statement", "kind", "source", "expect", "cwd", "missing"],
                &["cmd"],
            ),
            Some("judge") => (&["statement", "kind", "source"], &["how_to_judge"]),
            _ => return Err(format!("{prefix} has unknown kind")),
        };
        let mut missing: Vec<_> = required
            .iter()
            .filter(|k| !record.has(k))
            .copied()
            .collect();
        missing.sort();
        require(
            missing.is_empty(),
            format!("{prefix} missing fields: {}", missing.join(", ")),
        )?;
        let mut extra: Vec<_> = record
            .fields()
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !required.contains(k) && !optional.contains(k))
            .collect();
        extra.sort();
        require(
            extra.is_empty(),
            format!("{prefix} has unknown fields: {}", extra.join(", ")),
        )?;
        for (field, value) in record.fields() {
            require(
                text(value) && !value.string().unwrap_or("").contains('\0'),
                format!("{prefix}.{field} must be a non-empty string without NUL"),
            )?;
        }
        if let Some(cwd) = record.get("cwd").and_then(Value::string) {
            let drive = cwd.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                && cwd.as_bytes().get(1) == Some(&b':');
            require(
                !cwd.starts_with('/')
                    && !cwd.split('/').any(|s| s == "..")
                    && !cwd.contains(['\\', '\n', '\r'])
                    && !drive,
                format!(
                    "{prefix}.cwd must be repo-relative, without '..', absolute paths or backslashes"
                ),
            )?;
        }
    }
    Ok(())
}

/// Read strict check records without writing or running any of them.
pub fn read_checks(directory: &Path) -> Result<Value> {
    let path = directory.join(FILE);
    require(
        path.is_file(),
        "checks are missing: use `seed checks set <session> <input.json>`",
    )?;
    let records = Value::parse(&fs::read_to_string(path).map_err(|e| e.to_string())?)?;
    validate_checks(&records)?;
    Ok(records)
}

pub(super) fn digest(records: &Value) -> Result<String> {
    Ok(super::seed::sha256(
        records.sorted().compact(false)?.as_bytes(),
    ))
}

fn prose(value: &str) -> String {
    let leading = value.bytes().take_while(|c| *c == b' ').count();
    let mut out = "&#32;".repeat(leading);
    let numeric = Regex::new(r"^\d+[.)]")
        .unwrap()
        .find(value)
        .map(|m| m.end() - 1);
    for (i, c) in value.char_indices().skip_while(|(i, _)| *i < leading) {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if whitespace(c) && c != ' ' => out.push_str(&format!("&#{};", c as u32)),
            c => {
                if "\\`*_[]#~".contains(c) || (i == 0 && "+-=".contains(c)) || numeric == Some(i) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
    }
    out
}
fn command(value: &str) -> String {
    if value.contains("<!--")
        || value
            .chars()
            .any(|c| c < ' ' || (whitespace(c) && c != ' '))
    {
        return prose(value);
    }
    let longest = value.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let pad = if value.starts_with(['`', ' ']) || value.ends_with(['`', ' ']) {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{value}{pad}{fence}")
}

/// Render literal, one-line acceptance cards from records, never from model Markdown.
pub fn checks_block(records: &Value) -> Result<String> {
    validate_checks(records)?;
    let blocks: Vec<_> = records
        .items()
        .iter()
        .map(|r| {
            let statement = r["statement"].string().unwrap();
            let (label, detail) = match r["kind"].string().unwrap() {
                "cmd" => ("check", command(r["cmd"].string().unwrap())),
                "judge" => (
                    "you judge",
                    prose(
                        r.get("how_to_judge")
                            .and_then(Value::string)
                            .unwrap_or(statement),
                    ),
                ),
                _ => ("not yet runnable", prose(r["missing"].string().unwrap())),
            };
            format!("{}\n{label}: {detail}", prose(statement))
        })
        .collect();
    Ok(format!(
        "## Acceptance criteria\n\n{}\n\n",
        blocks.join("\n\n")
    ))
}

pub(super) fn legacy_block(records: &Value) -> Result<String> {
    validate_checks(records)?;
    Ok(format!(
        "## Acceptance criteria\n\n```json\n{}```\n\n",
        records.sorted().pretty()?
    ))
}

pub(super) const FULL: &[&str] = &[
    "## You are confirming",
    "## Problem statement",
    "## Proposed outcome",
    "## Acceptance criteria",
    "## Affected users and systems",
    "## Constraints and boundaries",
    "## Decisions (from the ledger)",
    "## Assumed (not confirmed by you)",
    "## Knowledge map",
    "## Evidence and uncertainty",
    "## Suggested first slice",
    "## Risks and verification",
    "## Open questions and deferrals",
    "## Owners and next steps",
    "## Downstream handoff",
];
pub(super) const LEAN: &[&str] = &[
    "## You are confirming",
    "## Now  (the agent keeps these current)",
    "## Goal",
    "## Decisions (from the ledger)",
    "## Assumed (not confirmed by you)",
    "## Constraints and exclusions",
    "## Acceptance criteria",
    "## Open questions",
    "## Knowledge map",
    "## Evidence",
];
fn without_comments(raw: &str, inside: &mut bool) -> String {
    let (mut out, mut rest) = (String::new(), raw);
    while !rest.is_empty() {
        if *inside {
            let Some(end) = rest.find("-->") else { break };
            rest = &rest[end + 3..];
            *inside = false;
        } else if let Some(start) = rest.find("<!--") {
            out.push_str(&rest[..start]);
            rest = &rest[start + 4..];
            *inside = true;
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

/// Locate only literal template headings; refuse hidden, nested, Setext or unterminated headings.
pub fn document_headings(body: &str) -> Result<Vec<(usize, String)>> {
    let opening = Regex::new(r"^ {0,3}(`{3,}|~{3,})(.*)$").unwrap();
    let setext = Regex::new(r"^(?:=+|-+)[ \t]*$").unwrap();
    let container = Regex::new(r"^(?:>[ \t]*|(?:[-+*]|\d+[.)])[ \t]+)").unwrap();
    let heading = Regex::new(r"^#{1,6}(?:[ \t]+|$)").unwrap();
    let (mut headings, mut fence, mut comment, mut offset) = (vec![], None::<String>, false, 0);
    for line in body.split_inclusive('\n') {
        let raw = super::seed::split_terminator(line).0;
        let start = offset;
        offset += line.len();
        if let Some(ref f) = fence {
            let close = format!(r"^ {{0,3}}{}{{{},}}[ \t]*$", &f[..1], f.len());
            if Regex::new(&close).unwrap().is_match(raw) {
                fence = None;
            }
            continue;
        }
        if !comment
            && let Some(c) = opening.captures(raw)
            && (c[1].starts_with('~') || !c[2].contains('`'))
        {
            fence = Some(c[1].into());
            continue;
        }
        let visible = without_comments(raw, &mut comment);
        let mut candidate = visible.trim_start_matches(whitespace);
        loop {
            require(
                !setext.is_match(candidate),
                "checks refuse injected Setext heading syntax; body prose cannot contain headings",
            )?;
            let Some(m) = container.find(candidate) else {
                break;
            };
            candidate = candidate[m.end()..].trim_start_matches(whitespace);
        }
        if heading.is_match(candidate) {
            require(
                raw == candidate && visible == raw && line == format!("{raw}\n"),
                "checks refuse injected, indented, commented or non-literal heading lines",
            )?;
            headings.push((start, raw.into()));
        }
    }
    Ok(headings)
}

pub(super) fn require_seed_checks(directory: &Path, allow_legacy: bool) -> Result<Value> {
    let records = read_checks(directory)?;
    let body = super::seed::seed_lines(directory)?.concat();
    require(
        !body
            .as_bytes()
            .iter()
            .enumerate()
            .any(|(i, c)| *c == b'\r' && body.as_bytes().get(i + 1) != Some(&b'\n')),
        "checks refuse lone CR: seed line endings must be LF or CRLF",
    )?;
    let headings = document_headings(&body)?;
    require(
        headings.first().is_some_and(|(_, s)| {
            s.strip_prefix("# ").is_some_and(|title| {
                !title.contains(['\r', '\n'])
                    && title.chars().next_back().is_some_and(|c| !whitespace(c))
            })
        }),
        "checks need one leading H1 title and canonical Full or Lean section headings",
    )?;
    let titles: Vec<_> = headings.iter().skip(1).map(|(_, s)| s.as_str()).collect();
    require(
        titles == FULL || titles == LEAN,
        "checks refuse injected, missing, reordered or non-canonical headings; use the Full or Lean template",
    )?;
    let index = headings
        .iter()
        .position(|(_, s)| s == "## Acceptance criteria")
        .unwrap();
    let section = &body[headings[index].0..headings.get(index + 1).map_or(body.len(), |h| h.0)];
    require(
        section == checks_block(&records)? || (allow_legacy && section == legacy_block(&records)?),
        "checks do not match the rendered Acceptance criteria section; use `seed checks render`",
    )?;
    Ok(records)
}
