use serde_json::{Value, json};
use std::path::PathBuf;
use tinkery::shaping::brain_dump::{BoardRequest, Guess, Settled, Source, board::Board};
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    let calls = std::fs::read_to_string(root.join("call-trace/calls.jsonl")).unwrap();
    let mut results = vec![];
    for line in calls.lines().skip(59) {
        let call: Value = serde_json::from_str(line).unwrap();
        if call["kind"] != "shape" {
            continue;
        }
        let index = call["index"].as_u64().unwrap();
        let input: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("call-trace/{index:02}-input.json"))).unwrap(),
        )
        .unwrap();
        let r = BoardRequest {
            ask_counts: serde_json::from_value(input["ask_counts"].clone()).unwrap(),
            sources: input["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| Source {
                    id: s["id"].as_u64().unwrap() as usize,
                    text: s["text"].as_str().unwrap().into(),
                    in_reply_to: s["in_reply_to"].as_str().map(str::to_owned),
                })
                .collect(),
            fragments: vec![],
            previous: serde_json::from_value(input["previous"].clone()).unwrap(),
            skipped: serde_json::from_value(input["skipped"].clone()).unwrap(),
            answered: serde_json::from_value(input["answered"].clone()).unwrap(),
            settled: input["settled"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| Settled {
                    question: serde_json::from_value(s["question"].clone()).unwrap(),
                    source: s["source"].as_u64().unwrap() as usize,
                })
                .collect(),
            layout: vec![],
        };
        let dirs = if input["sources"].as_array().unwrap().len()>1 && input["sources"][0]["text"]==serde_json::Value::String(std::fs::read_to_string(root.join("revised-2/inputs/ledger.txt")).unwrap()) {
            vec![root.join(if call["index"].as_u64().unwrap()>=81 {"revised-2/ledger-refinement"}else{"revised-2/ledger-followup"})]
        } else if call["index"].as_u64().unwrap()>=75 {
            vec![root.join("revised-2/final-checks/mom-scams"),root.join("revised-2/final-checks/multi-topic-ci")]
        } else {
            [
                "books",
                "onboarding-emails",
                "trap",
                "ledger",
                "voice",
                "operator-goal",
                "vague-onboarding",
                "mom-scams",
                "multi-topic-ci",
            ]
            .iter()
            .map(|n| root.join("revised-2").join(n))
            .collect()
        };
        let mut found = None;
        for dir in dirs {
            for file in ["00-first.json", "01-answer.json", "02-answer.json"] {
                let path = dir.join(file);
                if !path.exists() {
                    continue;
                }
                let snapshot: Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                if snapshot["sources"] == input["sources"] {
                    found = Some((path, snapshot));
                }
            }
        }
        let (path, snapshot) = found.expect("shape has snapshot");
        let decision = snapshot["decisions_cumulative"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .find(|d| d["mode"] == "blocking")
            .unwrap();
        let mut changes = vec![];
        let parsed = Guess::decode(decision["raw"].as_str().unwrap(), &r, &mut changes).unwrap();
        let board = Board::verify(parsed, &r).unwrap();
        let exact = serde_json::to_value(&board).unwrap() == snapshot["board"];
        results.push(json!({"call":index,"snapshot":path.strip_prefix(&root).unwrap(),"exact_board":exact,"surface_change_only":!exact,"board":board,"parts":board.parts,"rendered_goal":board.framings.first().map(|r|r.text.clone()),"normalizations":changes}));
    }
    std::fs::write(
        root.join("revised-2/current-code-replay.json"),
        serde_json::to_string_pretty(&results).unwrap() + "\n",
    )
    .unwrap();
}
