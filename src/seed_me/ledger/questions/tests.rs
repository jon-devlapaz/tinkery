//! Reopening a saved question preserves the original words and its disclosure rule.

use super::*;
use crate::seed_me::ledger;
use serde_json::json;

const AT: &str = "2026-11-26T10:00:00+00:00";
const ASK: &str = "Which records?";
const A: &str = "Option A: Local; offline; needs\nmanual backups. Undo cost: cheap.";
const B: &str = "Option B: Public; line one;\nline two. Undo cost: hard.";

#[test]
fn read_old_multiline_and_incomplete_hard_offers_without_losing_words() {
    let recorded = format!("{ASK}\n{A}\n{B}\nAgainst my suggestion: First line.\nAnother line.");
    let old = read_question(&json!({"question": recorded})).unwrap();
    assert_eq!(old.ask, ASK);
    assert_eq!(
        old.options,
        [(A.into(), Undo::Cheap), (B.into(), Undo::Hard)]
    );
    assert_eq!(old.against.as_deref(), Some("First line.\nAnother line."));
    assert!(old.hard());

    for footer in [
        format!("{B}\nExtra text."),
        format!("{B}\n{A}"),
        format!("{A}\n{B}\n{B}"),
    ] {
        let recorded = format!("{ASK}\n{footer}");
        let old = read_question(&json!({"question": recorded})).unwrap();
        assert_eq!(old.ask, ASK);
        assert_eq!(old.options, [(footer, Undo::Hard)]);
        assert!(old.hard());

        let literal = read_question(&json!({"question": recorded, "choices": null})).unwrap();
        assert_eq!(literal.ask, recorded);
        assert!(!literal.hard());
    }
}

#[test]
fn changing_how_a_settled_question_reads_requires_review_of_its_dependents() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-choices-")
        .tempdir()
        .unwrap();
    ledger::fixtures::confirmed(root.path(), "TEST goal", "r1");
    let original = ledger::load(root.path()).unwrap();
    let mut state = ledger::editable(&original).unwrap();
    let mut source = state["nodes"].items()[0].clone();
    source["id"] = "storage".into();
    source.set("question", Value::String(format!("{ASK}\n{A}\n{B}")));
    let mut dependent = source.clone();
    dependent["id"] = "dependent".into();
    dependent["prerequisites"] = Value::Array(vec!["storage".into()]);
    state["nodes"].list_mut().extend([source, dependent]);
    let reason = "TEST source and dependent".into();
    let saved =
        ledger::publish(root.path(), &state, &original["version"], &reason, None, AT).unwrap();
    let mut changed = ledger::editable(&saved).unwrap();
    let options = json!([{"choice":"A","tradeoff":"first","undo":"cheap"},{"choice":"B","tradeoff":"second","undo":"hard"}]);
    for choices in [Value::Null, Value::from(json!({"options":options}))] {
        changed["nodes"].list_mut()[1].set("choices", choices);
        let error =
            ledger::transition(&saved, &changed, &saved["version"], &reason, None, AT).unwrap_err();
        assert_eq!(
            error,
            "affected settlement requires explicit revalidation with evidence: dependent"
        );
    }
    let mut invalid = ledger::editable(&saved).unwrap();
    invalid["nodes"].list_mut()[1].set("choices", Value::from(json!({"options":[]})));
    assert!(ledger::validate(&invalid).unwrap_err().contains("options"));
    invalid["nodes"].list_mut()[1].set("choices", Value::Null);
    invalid["nodes"].list_mut()[1].remove("question");
    assert!(
        ledger::validate(&invalid)
            .unwrap_err()
            .contains("decision with a question")
    );
}
