//! Replayable action sequences (#827): a confirmed goal, two dependent decisions and a seed draft,
//! driven only through Shape's public calls. After each step the saved files must agree with a
//! small expectation written from the ledger contract, not from its transition code.

use super::*;
use std::{collections::BTreeMap, fs, path::PathBuf};

type Files = BTreeMap<std::ffi::OsString, Vec<u8>>;

const GOAL: &str = "TEST goal";

fn files(session: &Path) -> Files {
    fs::read_dir(session)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.file_name(), fs::read(e.path()).unwrap())
        })
        .filter(|(name, _)| name != ".writer.lock")
        .collect()
}

fn question(id: &str, after: &[&str]) -> Ask {
    Ask {
        id: id.into(),
        label: format!("TEST {id}"),
        question: format!("TEST {id}?"),
        gate: format!("TEST {id} gate"),
        found: vec![],
        resolves: None,
        supported_by: None,
        prerequisites: (!after.is_empty()).then(|| json!(after)),
        suggestion: None,
        assumed: vec![],
        options: vec![],
        against: None,
        confidence: None,
    }
}

/// A confirmed goal, decision `store`, and decision `sync` that waits for `store`.
fn start() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("TEST-sequence-")
        .tempdir()
        .unwrap();
    let session = publication_tests::session(root.path());
    let v = read(&session).unwrap().version();
    let v = ask(&session, v, &question("store", &[]), root.path()).unwrap();
    ask(&session, v, &question("sync", &["store"]), root.path()).unwrap();
    (root, session)
}

fn answer_both(session: &Path, store: &str, sync: &str) {
    for (id, words) in [("store", store), ("sync", sync)] {
        let mut v = read(session).unwrap().version();
        if let Some(next) = promote(session, v).unwrap() {
            v = next;
        }
        answer(session, v, id, words, None).unwrap();
    }
}

fn draft(session: &Path, folder: &Path) -> String {
    draft_seed(
        session,
        read(session).unwrap().version(),
        &ledger::fixtures::prose_reply(include_str!(
            "../../../tests/fixtures/shape-seed-reply.txt"
        )),
        &serde_json::from_str(include_str!("../../../tests/fixtures/shape-checks.json")).unwrap(),
        &[],
        folder,
        None,
    )
    .unwrap()
}

fn seed(session: &Path) -> String {
    fs::read_to_string(session.join(SEED)).unwrap()
}

fn confirm(session: &Path, revision: &str) -> Result<(), Refusal> {
    confirm_seed(
        session,
        read(session).unwrap().version(),
        revision,
        &seed(session),
        "TEST yes to this seed only",
        None,
    )
}

fn verified(session: &Path) -> bool {
    ledger::status(session).unwrap().to_json().unwrap()["handoff"]["verified"] == true
}

fn answer_of(session: &Path, id: &str) -> Option<String> {
    ledger::load(session).unwrap().to_json().unwrap()["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id && n["status"] == "settled")
        .and_then(|n| n["answer"].as_str().map(str::to_owned))
}

/// The contract-derived expectation checked after every step: the goal and its original words
/// never change, and a verified handoff exists only for the answers that were confirmed.
fn holds(session: &Path, confirmed: Option<(&str, &str)>) {
    let ledger = read(session).unwrap();
    assert_eq!(ledger.goal(), Some(GOAL));
    assert_eq!(answer_of(session, "goal").as_deref(), Some(GOAL));
    let current = (answer_of(session, "store"), answer_of(session, "sync"));
    let expected = confirmed
        .is_some_and(|(store, sync)| current == (Some(store.to_owned()), Some(sync.to_owned())));
    assert_eq!(verified(session), expected, "{current:?} vs {confirmed:?}");
}

/// A refused call must leave every saved file byte for byte as it was.
fn refused<T: std::fmt::Debug>(
    session: &Path,
    call: impl FnOnce() -> Result<T, Refusal>,
) -> Refusal {
    let before = files(session);
    let error = call().expect_err("the call was expected to refuse");
    assert_eq!(
        files(session),
        before,
        "a refusal changed saved files: {error:?}"
    );
    error
}

#[test]
fn s1_ordinary_path_then_reopen_is_read_only_and_identical() {
    let (root, session) = start();
    holds(&session, None);
    answer_both(&session, "TEST local", "TEST never");
    holds(&session, None);
    let r1 = draft(&session, root.path());
    holds(&session, None);
    confirm(&session, &r1).unwrap();
    holds(&session, Some(("TEST local", "TEST never")));
    end(&session, Ending::Completed, None).unwrap();
    holds(&session, Some(("TEST local", "TEST never")));
    let saved = files(&session);
    let shown = super::super::shaping::read(&session).unwrap();
    let v = read(&session).unwrap().version();
    refused(&session, || {
        respond(
            &session,
            v,
            "store",
            &ledger::AnswerAction::Revise("TEST synced".into()),
            None,
        )
    });
    refused(&session, || end(&session, Ending::Stopped, None));
    refused(&session, || confirm(&session, &r1));
    refused(&session, || {
        ask(&session, v, &question("late", &[]), root.path())
    });
    // A repeat of the same ending is a no-op, not a second write.
    end(&session, Ending::Completed, None).unwrap();
    assert_eq!(files(&session), saved);
    assert_eq!(super::super::shaping::read(&session).unwrap(), shown);
    holds(&session, Some(("TEST local", "TEST never")));
}

#[test]
fn s2_decision_changed_after_draft_refuses_the_old_confirmation() {
    let (root, session) = start();
    answer_both(&session, "TEST local", "TEST never");
    let r1 = draft(&session, root.path());
    let (old_version, old_seed) = (read(&session).unwrap().version(), seed(&session));
    respond(
        &session,
        old_version,
        "store",
        &ledger::AnswerAction::Revise("TEST synced".into()),
        None,
    )
    .unwrap();
    // The dependent reopens; the old answer is gone from current state, kept in history.
    assert_eq!(answer_of(&session, "sync"), None);
    holds(&session, None);
    // The displayed r1 (old version, old bytes) cannot be confirmed.
    assert_eq!(
        refused(&session, || confirm_seed(
            &session,
            old_version,
            &r1,
            &old_seed,
            "TEST yes",
            None
        )),
        Refusal::Stale
    );
    // With a fresh version but r1's bytes, open decisions still refuse.
    let why = |r: Refusal| format!("{r:?}");
    let open = why(refused(&session, || confirm(&session, &r1)));
    assert!(open.contains("unresolved nodes remain: sync"), "{open}");
    let v = read(&session).unwrap().version();
    answer(&session, v, "sync", "TEST hourly", None).unwrap();
    holds(&session, None);
    // r1 now has every decision settled, but its Decisions block is stale.
    let stale = why(refused(&session, || confirm(&session, &r1)));
    assert!(stale.contains("decisions"), "{stale}");
    let r2 = draft(&session, root.path());
    assert_ne!(r1, r2);
    // The displayed r1 bytes, at the fresh version, are not the saved r2 draft.
    let v = read(&session).unwrap().version();
    let changed = why(refused(&session, || {
        confirm_seed(&session, v, &r1, &old_seed, "TEST yes", None)
    }));
    assert!(changed.contains("The draft changed"), "{changed}");
    confirm(&session, &r2).unwrap();
    holds(&session, Some(("TEST synced", "TEST hourly")));
    // A confirmed seed edited afterwards neither verifies nor completes.
    let confirmed = seed(&session);
    fs::write(session.join(SEED), format!("{confirmed} ")).unwrap();
    assert!(!verified(&session));
    let tampered = why(refused(&session, || end(&session, Ending::Completed, None)));
    assert!(tampered.contains("seed changed"), "{tampered}");
    fs::write(session.join(SEED), confirmed).unwrap();
    end(&session, Ending::Completed, None).unwrap();
    holds(&session, Some(("TEST synced", "TEST hourly")));
}

#[test]
fn s3_stale_retries_and_repeats_never_write() {
    let (_root, session) = start();
    // Asking made `store` current, so there is nothing to promote.
    let v1 = read(&session).unwrap().version();
    assert_eq!(promote(&session, v1).unwrap(), None);
    for stale in [v1 - 1, v1 + 1] {
        assert_eq!(
            refused(&session, || answer(
                &session,
                stale,
                "store",
                "TEST local",
                None
            )),
            Refusal::Stale
        );
        assert_eq!(
            refused(&session, || promote(&session, stale)),
            Refusal::Stale
        );
    }
    let v2 = answer(&session, v1, "store", "TEST local", None).unwrap();
    holds(&session, None);
    // A retry of the same answer, from the old or the current version, finds the question
    // settled and writes nothing. (Observed: the open-question check runs before the version
    // check, so the old-version retry says "no longer open", not Stale.)
    for (version, words) in [(v1, "TEST local"), (v2, "TEST local"), (v2, "TEST other")] {
        assert_eq!(
            refused(&session, || answer(&session, version, "store", words, None)),
            Refusal::Refused("The question store is no longer open.".into())
        );
    }
    assert_eq!(answer_of(&session, "store").as_deref(), Some("TEST local"));
    holds(&session, None);
}

#[test]
fn s4_decision_changed_after_confirmation_cannot_complete_and_keeps_the_goal() {
    let (root, session) = start();
    answer_both(&session, "TEST local", "TEST never");
    let r1 = draft(&session, root.path());
    confirm(&session, &r1).unwrap();
    holds(&session, Some(("TEST local", "TEST never")));
    let confirmed_seed = seed(&session);
    let v = read(&session).unwrap().version();
    respond(
        &session,
        v,
        "store",
        &ledger::AnswerAction::Revise("TEST synced".into()),
        None,
    )
    .unwrap();
    holds(&session, Some(("TEST local", "TEST never")));
    refused(&session, || end(&session, Ending::Completed, None));
    assert_eq!(
        seed(&session),
        confirmed_seed,
        "the confirmed seed was rewritten"
    );
    let v = read(&session).unwrap().version();
    answer(&session, v, "sync", "TEST never", None).unwrap();
    // Settling the same words again does not revive the r1 confirmation: store still differs.
    holds(&session, Some(("TEST local", "TEST never")));
    refused(&session, || end(&session, Ending::Completed, None));
    // Shape shows the seed as unverified. Shape offers Revise only from a review flag, and a
    // flag needs a new finding, which no step adds after confirmation; this ledger-level
    // sequence is not an ordinary Shape path. Only stopping remains.
    assert_eq!(
        super::super::shaping::read(&session).unwrap().stage,
        super::super::shaping::Shaped::Unverified {
            reason: "the decisions changed after revision r1 was confirmed".into()
        }
    );
    let redraft = refused(&session, || {
        draft_seed(
            &session,
            read(&session).unwrap().version(),
            &ledger::fixtures::prose_reply(include_str!(
                "../../../tests/fixtures/shape-seed-reply.txt"
            )),
            &serde_json::from_str(include_str!("../../../tests/fixtures/shape-checks.json"))
                .unwrap(),
            &[],
            root.path(),
            None,
        )
    });
    assert_eq!(
        redraft,
        Refusal::Refused(
            "A seed that isn't a draft already exists; Shape never replaces it.".into()
        )
    );
    end(&session, Ending::Stopped, None).unwrap();
    holds(&session, Some(("TEST local", "TEST never")));
}
