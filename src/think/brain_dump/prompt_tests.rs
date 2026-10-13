//! The prompts reach Pi byte for byte as recorded. The recorded copies in
//! `tests/fixtures/prompts/` were taken from main before the prompts moved into their own files
//! (#38); a wording change must update them on purpose, with an eval behind it.
use super::*;

const SKILL: &str = "# Seed Me\nIgnored preamble.\n### Shape the working draft\nWORKING INSTRUCTIONS SAMPLE\nSecond line.\n### Size gate\nIgnored tail.\n";

/// The system prompts Pi receives for one send: the reading, then the decision review.
#[cfg(unix)]
fn sent_prompts() -> (String, String) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    std::fs::write(&skill, SKILL).unwrap();
    let program = dir.path().join("pi");
    std::fs::write(&program, r#"#!/usr/bin/env python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
kind=r.get('kind','shape')
p=sys.argv[sys.argv.index('--system-prompt')+1]
(root/('prompt-'+kind)).write_bytes(p.encode('utf-8'))
if kind=='decision-review':print(json.dumps(dict(decisions=[])));sys.exit(0)
print(json.dumps(dict(parts=dict(who='I',outcome='judge the result',why='it matters',done_when='a judged result exists'),questions=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pi = PiHost::new(program, "test/model".into(), skill).unwrap();
    let request = BoardRequest {
        ask_counts: Default::default(),
        sources: vec![Source {
            id: 1,
            text: "the restaurateur judges the result".into(),
            in_reply_to: None,
        }],
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    pi.reshape(request, &AtomicBool::new(false)).unwrap();
    let read =
        |kind: &str| std::fs::read_to_string(dir.path().join(format!("prompt-{kind}"))).unwrap();
    (read("shape"), read("decision-review"))
}

fn recorded(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/prompts")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[cfg(unix)]
#[test]
fn prompts_reach_pi_byte_for_byte_as_recorded() {
    let (reading, review) = sent_prompts();
    assert_eq!(reading, recorded("reading.txt"));
    assert_eq!(review, recorded("decision_review.txt"));
}

/// Fields the decoder still accepts from older replies but the prompt no longer asks for.
const LEGACY: &[&str] = &["uncertain", "framings", "outcome"];

fn sorted<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort_unstable();
    names
}

/// The prompt's schema line, split into its top-level entries at commas outside brackets.
fn schema_entries(line: &str) -> Vec<&str> {
    let mut entries = vec![];
    let (mut depth, mut start) = (0, 0);
    for (i, c) in line.char_indices() {
        match c {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                entries.push(line[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    entries.push(line[start..].trim().trim_end_matches('.'));
    entries
}

/// The names inside the first `{...}` of an entry, without "optional".
fn braced(entry: &str) -> Vec<&str> {
    let inner = &entry[entry.find('{').unwrap() + 1..entry.find('}').unwrap()];
    sorted(
        inner
            .split(',')
            .map(|f| f.trim().trim_start_matches("optional ")),
    )
}

#[test]
fn the_reading_prompt_names_the_fields_its_decoder_reads() {
    let line = reading::PROMPT
        .lines()
        .find_map(|l| l.strip_prefix("Return JSON: "))
        .expect("the reading prompt has a schema line");
    let mut top = vec![];
    for entry in schema_entries(line) {
        let name = entry
            .split(['(', '{'])
            .next()
            .unwrap()
            .trim()
            .trim_start_matches("optional ");
        match name {
            "parts" => assert_eq!(braced(entry), sorted(reading::PARTS.iter().copied())),
            "decisions" => assert_eq!(braced(entry), sorted(reading::DECISION.iter().copied())),
            "questions" => assert_eq!(braced(entry), sorted(reading::QUESTION.iter().copied())),
            "supports" => assert!(entry.contains(&reading::ANCHOR.join("/")), "{entry}"),
            _ => {}
        }
        top.extend(name.split(" or "));
    }
    let decoded = reading::READING
        .iter()
        .copied()
        .filter(|f| !LEGACY.contains(f));
    assert_eq!(sorted(top), sorted(decoded));
    // The span rule names the anchor fields the decoder reads.
    assert!(reading::PROMPT.contains(&format!("use {}:", reading::ANCHOR.join("/"))));
}

#[test]
fn the_review_prompt_names_the_fields_its_merge_reads() {
    let example = review::PROMPT
        .lines()
        .find_map(|l| l.strip_prefix("Return ONLY JSON "))
        .and_then(|l| l.rsplit_once("]}").map(|(json, _)| format!("{json}]}}")))
        .expect("the review prompt has a JSON example");
    let example: serde_json::Value = serde_json::from_str(&example).unwrap();
    let object = example.as_object().unwrap();
    assert_eq!(sorted(object.keys().map(String::as_str)), ["decisions"]);
    let decision = object["decisions"][0].as_object().unwrap();
    let mut expected = reading::DECISION.to_vec();
    expected.push(review::COVERS);
    assert_eq!(
        sorted(decision.keys().map(String::as_str)),
        sorted(expected)
    );
    // Every part the review may name is a part think knows.
    let parts = review::PROMPT
        .lines()
        .find_map(|l| l.strip_prefix("part is one of "))
        .unwrap();
    for part in parts.trim_end_matches('.').split(", ") {
        serde_json::from_value::<Part>(part.into()).unwrap_or_else(|_| panic!("{part}"));
    }
}

#[test]
fn notes_never_reach_the_model() {
    for file in [reading::PROMPT, review::PROMPT] {
        let sent = reading::assemble(file);
        assert!(file.lines().any(|l| l.starts_with('#')));
        assert!(!sent.lines().any(|l| l.starts_with('#')), "{sent}");
    }
}
