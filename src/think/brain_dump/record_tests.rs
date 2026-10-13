//! The think record as a session leaves it: written once by a confirmed save, never before.

use super::tests::amend_board;
use super::*;
use std::path::Path;

const DUMP: &str = "  TEST: calmer mornings. Café 👩‍💻\n\"no phone\" before eight.  ";
const ANSWER: &str = "TEST answer: the calm start matters most ";

fn key(a: &mut BrainDump, code: KeyCode, modifiers: KeyModifiers) {
    a.handle_key(KeyEvent::new(code, modifiers));
}
fn settle(a: &mut BrainDump) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while a.running() && std::time::Instant::now() < deadline {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(!a.running());
}
fn records(root: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(root)
        .map(|d| d.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default()
}
/// A practice-host session with a dump, a skip, its undo and an answer, writing records to `root`.
fn session(root: &Path) -> BrainDump {
    let mut a = BrainDump::with_host(Arc::new(Simulated))
        .with_seed_me("/missing/SKILL.md".into(), None)
        .with_record_root(root.to_path_buf());
    a.paste(DUMP);
    a.submit();
    settle(&mut a);
    key(&mut a, KeyCode::F(8), KeyModifiers::NONE);
    key(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    a.paste(ANSWER);
    a.submit();
    settle(&mut a);
    a
}
/// Hands the review's save a Seed Me result, as the save thread would.
fn save(a: &mut BrainDump, result: Result<seed_me::Receipt, String>) {
    let (tx, rx) = std::sync::mpsc::channel();
    a.handoff.job = Some(rx);
    tx.send(result).unwrap();
    a.goal_tick();
}
fn receipt(goal: &str) -> seed_me::Receipt {
    let mut receipt = seed_me::Receipt::test("/tmp/TEST-sessions/20261012-TEST".into());
    receipt.goal = goal.into();
    receipt
}

#[test]
fn a_confirmed_session_leaves_exactly_one_record_with_its_words_turns_and_goal() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-records");
    let mut a = session(&root);
    a.review_goal();
    assert!(records(&root).is_empty(), "opening review wrote a record");
    key(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.handoff.review.is_none());
    assert!(records(&root).is_empty(), "Esc wrote a record");
    a.review_goal();
    let shown = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
    // A later reading must not leak into the record: only the confirmed block counts.
    amend_board(&mut a, |g| {
        g.parts.outcome = Some("TEST later reading".into())
    });
    save(&mut a, Ok(receipt(&shown)));
    assert_eq!(a.notice, "Saved.");
    let files = records(&root);
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.ends_with("Z_20261012-TEST.json"), "{name}");
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&files[0]).unwrap()).unwrap();
    assert_eq!(record["schema"], 1);
    assert_eq!(record["origin"], "simulated");
    assert_eq!(record["seed_me_session"], "20261012-TEST");
    assert_eq!(
        record["goal"].as_str().unwrap(),
        shown,
        "record goal differs from the confirmed block"
    );
    let turns = record["turns"].as_array().unwrap();
    let kinds: Vec<_> = turns.iter().map(|t| t["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["words", "asked", "skipped", "restored", "asked", "answered"]
    );
    let sources = a.conversation.sources();
    assert_eq!(turns[0]["text"].as_str().unwrap(), DUMP);
    assert_eq!(turns[0]["text"].as_str().unwrap(), sources[0].text);
    assert_eq!(turns[5]["answer"].as_str().unwrap(), ANSWER);
    assert_eq!(turns[5]["answer"].as_str().unwrap(), sources[1].text);
    assert_eq!(turns[5]["id"], "priority");
    assert_eq!(turns[1]["question"], turns[5]["question"]);
    let times: Vec<_> = turns.iter().map(|t| t["at"].as_str().unwrap()).collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");
    assert_eq!(record["started"], turns[0]["at"]);
    assert!(!record["checks"].as_array().unwrap().is_empty());
    let line = a.record_line().unwrap();
    assert!(
        line.starts_with("Think record: ") && line.contains(&name),
        "{line}"
    );
    assert!(a.finish_handoff().unwrap().contains(&line));
}

#[test]
fn a_session_that_never_confirms_or_fails_to_save_leaves_no_record() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-records");
    let mut a = session(&root);
    a.review_goal();
    save(&mut a, Err("TEST helper refused".into()));
    assert!(a.handoff.receipt.is_none());
    assert!(
        records(&root).is_empty(),
        "a failed Seed Me save wrote a record"
    );
    let mut quit = session(&root);
    key(&mut quit, KeyCode::F(10), KeyModifiers::NONE);
    key(&mut quit, KeyCode::Char('y'), KeyModifiers::NONE);
    assert_eq!(*quit.exit(), Exit::Menu);
    assert!(quit.finish_handoff().is_none());
    assert!(!root.exists(), "leaving without confirming wrote a record");
}

#[test]
fn a_record_that_cannot_be_written_leaves_the_saved_goal_standing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("TEST-not-a-folder");
    std::fs::write(&blocked, "TEST").unwrap();
    let mut a = session(&blocked);
    a.review_goal();
    let shown = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
    save(&mut a, Ok(receipt(&shown)));
    assert_eq!(a.saved().unwrap().goal, shown, "the goal save must stand");
    assert!(
        a.notice
            .starts_with("Saved, but the think record wasn't written:"),
        "{}",
        a.notice
    );
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("think record wasn't written"), "{screen}");
    let printed = a.finish_handoff().unwrap();
    assert!(printed.contains("Think record not written: "), "{printed}");
}

#[test]
fn review_says_confirm_also_saves_the_record_only_when_one_will_be_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = session(&dir.path().join("TEST-records"));
    a.review_goal();
    let screen = snapshot(160, 40, &mut a, false).unwrap();
    assert!(screen.contains("save goal and record"), "{screen}");
    assert!(
        screen.contains("type confirm to save the goal and its record."),
        "{screen}"
    );
    let mut plain =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    plain.paste(DUMP);
    plain.submit();
    settle(&mut plain);
    plain.review_goal();
    let screen = snapshot(160, 40, &mut plain, false).unwrap();
    assert!(!screen.contains("record"), "{screen}");
}

/// How many shaping calls a record's checks describe.
fn shaping_checks(record: &serde_json::Value) -> usize {
    record["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "structure-spans-history")
        .count()
}

#[cfg(unix)]
#[test]
fn records_keep_the_model_reply_only_for_rejected_checks() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    std::fs::write(
        &skill,
        "# Seed Me\n### Shape the working draft\nTEST\n### Size gate\n",
    )
    .unwrap();
    let program = dir.path().join("pi");
    std::fs::write(&program, r#"#!/usr/bin/env python3
import json,sys
r=json.load(sys.stdin)
if r.get('kind')=='decision-review':print('{"decisions":[]}');sys.exit(0)
marker=r['sources'][-1]['text']
if marker=='TEST-BAD':print('TEST not a reading');sys.exit(0)
print(json.dumps(dict(parts=dict(who='I',outcome='finish TEST-GOOD-REPLY',why='the work is stuck',done_when='tests pass'),questions=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Arc::new(PiHost::new(program, "test/model".into(), skill.clone()).unwrap());
    let root = dir.path().join("TEST-records");
    let mut a = BrainDump::with_host(host)
        .with_seed_me(skill, None)
        .with_record_root(root.clone());
    a.paste("TEST-BAD");
    a.submit();
    settle(&mut a);
    a.paste("TEST-GOOD");
    a.submit();
    settle(&mut a);
    a.review_goal();
    let goal = a.handoff.review.as_ref().unwrap().affirmation.goal.clone();
    save(&mut a, Ok(receipt(&goal)));
    let file = records(&root).pop().unwrap();
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let checks: Vec<_> = record["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "structure-spans-history")
        .collect();
    assert_eq!(checks.len(), 2, "{record:#}");
    assert_eq!(checks[0]["decision"], "reject");
    assert_eq!(checks[0]["raw"], "TEST not a reading\n");
    assert_eq!(checks[1]["decision"], "pass");
    assert!(checks[1]["raw"].is_null(), "passing reply kept: {record:#}");
    assert!(
        checks[1]["candidate"]
            .to_string()
            .contains("TEST-GOOD-REPLY"),
        "the decoded reading stays: {record:#}"
    );
    // F5 still has every reply this session.
    assert!(a.diagnostics().join("\n").contains("TEST-GOOD-REPLY"));
}

#[cfg(unix)]
#[test]
fn new_goals_keep_their_own_checks_and_the_standalone_record_setting() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    std::fs::write(
        &skill,
        "# Seed Me\n### Shape the working draft\nTEST\n### Size gate\n",
    )
    .unwrap();
    let program = dir.path().join("pi");
    std::fs::write(&program, r#"#!/usr/bin/env python3
import json,sys
r=json.load(sys.stdin)
if r.get('kind')=='decision-review':print('{"decisions":[]}');sys.exit(0)
marker=r['sources'][0]['text']
print(json.dumps(dict(parts=dict(who='I',outcome='finish '+marker,why='the work is stuck',done_when='tests pass for '+marker),questions=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Arc::new(PiHost::new(program, "test/model".into(), skill.clone()).unwrap());
    let root = dir.path().join("TEST-records");
    let mut first = BrainDump::with_host(host.clone())
        .with_seed_me(skill.clone(), None)
        .with_record_root(root.clone());
    first.paste("ALPHA");
    first.submit();
    settle(&mut first);
    first.paste("Also keep this goal's second check.");
    first.submit();
    settle(&mut first);
    first.review_goal();
    let goal = first
        .handoff
        .review
        .as_ref()
        .unwrap()
        .affirmation
        .goal
        .clone();
    save(&mut first, Ok(receipt(&goal)));
    let alpha_file = records(&root).pop().unwrap();
    let alpha_bytes = std::fs::read(&alpha_file).unwrap();
    let alpha: serde_json::Value = serde_json::from_slice(&alpha_bytes).unwrap();
    assert_eq!(shaping_checks(&alpha), 2);

    // Home passes its configured prototype again; standalone uses fresh_factory.
    for (name, mut next) in [
        (
            "BRAVO",
            BrainDump::with_host(host)
                .with_seed_me(skill, None)
                .with_record_root(root.clone()),
        ),
        ("CHARLIE", first.fresh_factory()()),
    ] {
        assert!(next.records(), "a new visit lost --record-root");
        next.paste(name);
        next.submit();
        settle(&mut next);
        assert!(
            !next.diagnostics().join("\n").contains("ALPHA"),
            "previous goal in F5 history"
        );
        next.review_goal();
        let goal = next
            .handoff
            .review
            .as_ref()
            .unwrap()
            .affirmation
            .goal
            .clone();
        let mut result = seed_me::Receipt::test(format!("/tmp/TEST-sessions/{name}").into());
        result.goal = goal;
        save(&mut next, Ok(result));
        let file = records(&root)
            .into_iter()
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(&format!("_{name}.json"))
            })
            .unwrap();
        let text = std::fs::read_to_string(file).unwrap();
        assert!(!text.contains("ALPHA"));
        let record: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(shaping_checks(&record), 1);
        assert_eq!(record["turns"][0]["text"], name);
    }
    assert_eq!(std::fs::read(alpha_file).unwrap(), alpha_bytes);
}
