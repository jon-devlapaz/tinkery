use super::*;
use serde_json::{Value, json};
use std::fs;

/// A Seed Me skill whose helper keeps its state in a TEST folder and records what it was asked
/// to publish. `tamper` makes the helper save the origin with another authority.
fn fake_skill(dir: &Path, tamper: bool) -> PathBuf {
    let scripts = dir.join("skill/scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(dir.join("skill/SKILL.md"), "TEST").unwrap();
    fs::write(
        scripts.join("session.py"),
        r#"import json, sys
from pathlib import Path
session = Path(__file__).resolve().parent.parent / "TEST-session"
state_file = session / "state.json"
def load():
    return json.loads(state_file.read_text())
cmd = sys.argv[1]
if cmd == "init":
    assert sys.argv[2] == "--root" and sys.argv[3], "Tinkery must pass its resolved root"
    session.mkdir()
    state_file.write_text(json.dumps({"status": "active", "operator": "human", "version": 1, "origin": None, "goal": None, "nodes": []}))
    print(json.dumps({"session": str(session)}))
elif cmd == "read":
    print(json.dumps(load()))
elif cmd == "publish":
    payload = json.loads(Path(sys.argv[3]).read_text())
    (session / "published.json").write_text(json.dumps(payload))
    state = load()
    state.update(payload["state"])
    if (session.parent / "tamper").exists():
        state["nodes"][0]["authority"] = "model"
    state_file.write_text(json.dumps(state))
    print("{}")
elif cmd == "status":
    print(json.dumps({"status": "active", "snapshot_current": True, "viewer": "live http://127.0.0.1:9/ledger-view.html"}))
"#,
    )
    .unwrap();
    fs::write(
        scripts.join("viewer.py"),
        "import time\nprint('http://127.0.0.1:9/ledger-view.html', flush=True)\ntime.sleep(30)\n",
    )
    .unwrap();
    if tamper {
        fs::write(dir.join("skill/tamper"), "").unwrap();
    }
    dir.join("skill/SKILL.md")
}

fn affirmed() -> Affirmed {
    Affirmed {
        goal: "TEST goal".into(),
        outcome: "PR ready".into(),
        options: vec![Choice {
            label: "A".into(),
            benefit: "b".into(),
            cost: "c".into(),
            undo_cost: "u".into(),
        }],
        originals: vec![
            Original {
                id: 0,
                in_reply_to: None,
                text: "  dump 👩‍💻\n".into(),
            },
            Original {
                id: 1,
                in_reply_to: Some("q0".into()),
                text: "answer".into(),
            },
        ],
        context: r#"{"TEST":"context"}"#.into(),
        source: "TEST typed confirm".into(),
    }
}

/// The publish payload with its two clock readings replaced, so it can be compared.
fn published(skill: &Path) -> Value {
    let path = skill.parent().unwrap().join("TEST-session/published.json");
    let mut payload: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    payload["reason"] = "TEST reason".into();
    for entry in payload["state"]["nodes"][0]["evidence"]
        .as_array_mut()
        .unwrap()
    {
        entry["at"] = "TEST at".into();
    }
    payload
}

#[test]
fn confirm_publishes_the_origin_seed_me_expects_and_reads_it_back() {
    let tmp = tempfile::tempdir().unwrap();
    let skill = fake_skill(tmp.path(), false);
    let mut receipt = Config::new(skill.clone(), None)
        .confirm(affirmed())
        .unwrap();
    receipt.stop_viewer().unwrap();
    assert_eq!(receipt.goal, "TEST goal");
    assert_eq!(receipt.ledger, "http://127.0.0.1:9/ledger-view.html");
    assert_eq!(
        receipt.saved_view(),
        receipt.session.join("ledger-view.html")
    );
    let evidence = |checked: &str, observed: &str| json!({"checked": checked, "at": "TEST at", "observed": observed});
    assert_eq!(
        published(&skill),
        json!({
            "expected_version": 1,
            "reason": "TEST reason",
            "state": {
                "status": "active",
                "draft": {"goal": "TEST goal", "outcome": "PR ready", "options": ["A — benefit: b; cost: c; undo cost: u"]},
                "goal": "TEST goal",
                "origin": "goal",
                "current_question": null,
                "nodes": [{
                    "id": "goal",
                    "kind": "decision",
                    "status": "settled",
                    "prerequisites": [],
                    "evidence": [
                        evidence("Tinkery original 0", "  dump 👩‍💻\n"),
                        evidence("Tinkery original 1 / reply to q0", "answer"),
                        evidence("Tinkery answer/question context; questions remain provisional, not settled Seed Me decisions", r#"{"TEST":"context"}"#),
                    ],
                    "owner": "User",
                    "gate": "Explicit affirmation of the complete displayed goal; not seed or implementation approval",
                    "answer": "TEST goal",
                    "authority": "user",
                    "authority_source": "TEST typed confirm"
                }]
            }
        })
    );
}

#[test]
fn confirm_refuses_a_read_back_that_does_not_match_the_affirmed_goal() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Config::new(fake_skill(tmp.path(), true), None);
    let error = config.confirm(affirmed()).err().unwrap();
    assert_eq!(error, "Helper read-back did not match the affirmed goal");
    assert!(config.recovery_path().is_some());
}

#[test]
fn evidence_labels_are_plain_and_leave_original_bytes_untouched() {
    let mut original = Original {
        id: 2,
        text: "  both\n".into(),
        in_reply_to: Some("compounding-referent".into()),
    };
    assert_eq!(
        original.label(),
        "Tinkery original 2 / reply to compounding-referent"
    );
    assert_eq!(original.text, "  both\n");
    original.in_reply_to = None;
    assert_eq!(original.label(), "Tinkery original 2");
}

#[test]
fn reading_a_session_never_guesses() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(matches!(read(tmp.path()), Record::NotASession));
    fs::write(tmp.path().join("ledger.json"), r#"{"goal":"g"}"#).unwrap();
    assert!(matches!(read(tmp.path()), Record::Unreadable(_)));
    fs::write(
        tmp.path().join("ledger.json"),
        r#"{"status":"active","goal":"  "}"#,
    )
    .unwrap();
    fs::write(tmp.path().join("seed-contract.md"), "status: draft\n").unwrap();
    let Record::Session(session) = read(tmp.path()) else {
        panic!("a ledger with a status is a session")
    };
    assert_eq!(session.stage, Stage::Unknown);
    fs::write(
        tmp.path().join("ledger.json"),
        r#"{"status":"active","goal":"g","updated_at":"t"}"#,
    )
    .unwrap();
    let Record::Session(session) = read(tmp.path()) else {
        panic!("a ledger with a status is a session")
    };
    assert_eq!(
        (
            session.goal.as_str(),
            session.stage,
            session.updated.as_str()
        ),
        ("g", Stage::SeedDrafted, "t")
    );
}

#[test]
fn session_identity_comes_only_from_a_nonempty_control_free_ledger_string() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-id-")
        .tempdir()
        .unwrap();
    for (value, expected) in [
        (Some(json!("TEST-id")), Some("TEST-id")),
        (Some(json!("  verbatim 界  ")), Some("  verbatim 界  ")),
        (None, None),
        (Some(json!(null)), None),
        (Some(json!(42)), None),
        (Some(json!("")), None),
        (Some(json!("bad\nline")), None),
        (Some(json!("bad\u{7f}")), None),
        (Some(json!("bad\u{85}")), None),
    ] {
        let mut ledger = json!({"status":"active", "goal":"Not the session id"});
        if let Some(value) = value {
            ledger["session_id"] = value;
        }
        fs::write(tmp.path().join("ledger.json"), ledger.to_string()).unwrap();
        let Record::Session(session) = read(tmp.path()) else {
            panic!("readable ledger")
        };
        assert_eq!(session.id.as_deref(), expected);
    }
}

#[test]
fn default_root_is_under_the_given_home() {
    for home in [None, Some(""), Some("TEST-relative"), Some(".")] {
        let error = default_root(home.map(std::ffi::OsStr::new)).unwrap_err();
        assert!(error.contains("--seed-session-root PATH"));
    }
    assert_eq!(
        default_root(Some(std::ffi::OsStr::new("/TEST/home"))).unwrap(),
        Path::new("/TEST/home/.local/share/seed-me/sessions")
    );
}

#[test]
fn an_empty_explicit_root_never_means_the_current_folder() {
    let error = resolve_root(Some(PathBuf::new())).unwrap_err();
    assert!(error.contains("--seed-session-root"), "{error}");
}
