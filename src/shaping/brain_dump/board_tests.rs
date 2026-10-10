use super::tests::{Recording, settle};
use super::*;
fn request() -> BoardRequest {
    BoardRequest {
        sources: vec![Source {
            id: 1,
            text: "Exact Café 👩‍💻 words.".into(),
            in_reply_to: None,
        }],
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    }
}
fn parsed(raw: &str) -> (board::Board, Vec<String>) {
    let r = request();
    let mut changes = vec![];
    let g = board::parse(raw, &r, &mut changes).unwrap();
    (board::Board::verify(g, &r).unwrap(), changes)
}
#[test]
fn omissions_extras_and_narrative_notes_never_fabricate_citations_or_authority() {
    let (b, changes) = parsed(
        r#"{"framings":"A provisional reading","alternatives":{"label":"Maybe","precondition":"unknown"},"misfits":[{"text":"Unsure about a relationship"}],"confirmed":true,"settled":[{"source":99}]}"#,
    );
    assert!(matches!(b.framings[0].supports, board::Grounding::Uncited));
    assert!(b.outcome.is_none() && b.questions.is_empty() && b.misfits.is_empty());
    assert_eq!(b.unresolved_notes, ["Unsure about a relationship"]);
    assert!(b.alternatives[0].benefit.is_none());
    assert!(
        changes.iter().any(|c| c.contains("precondition"))
            && changes.iter().any(|c| c.contains("confirmed"))
    );
    assert!(changes.iter().any(|c| c.contains("no source highlight")));
    assert_eq!(b.wire().framings.len(), 1);
}
#[test]
fn counts_word_lengths_labels_and_question_punctuation_are_not_boundary_rules() {
    let input = serde_json::json!({"framings":(0..5).map(|i|format!("reading {i} {}",vec!["word";100].join(" "))).collect::<Vec<_>>(),"questions":(0..9).map(|i|format!("Unresolved {i} without question mark")).collect::<Vec<_>>(),"alternatives":vec!["same";7]});
    let (b, changes) = parsed(&input.to_string());
    assert_eq!(
        b.framings.len(),
        1,
        "one reading: the question decides, not a second guess"
    );
    assert!(b.framings[0].text.starts_with("reading 0"));
    assert_eq!(b.questions.len(), 9);
    assert_eq!(b.alternatives.len(), 7);
    assert_eq!(b.presented(0).len(), 1);
    assert!(
        changes
            .iter()
            .any(|c| c.contains("5 readings received; the first is presented"))
    );
}
#[test]
fn malformed_json_and_no_readable_core_fail_but_optional_types_do_not() {
    for raw in ["not JSON", "[]", r#"{"framings":[null,"",{"text":3}]}"#] {
        assert!(board::parse(raw, &request(), &mut vec![]).is_err());
    }
    let (b, changes) = parsed(
        r#"{"framings":[{"text":"Meaning","supports":null}],"outcome":{},"questions":[null,{"text":4}],"alternatives":[true]}"#,
    );
    assert_eq!(b.framings.len(), 1);
    assert!(b.outcome.is_none() && b.questions.is_empty() && b.alternatives.is_empty());
    assert!(!changes.is_empty());
}
#[test]
fn claimed_citations_remain_exact_and_grapheme_safe() {
    for span in [
        serde_json::json!({"source":2,"quote":"Exact"}),
        serde_json::json!({"source":1,"quote":"invented"}),
        serde_json::json!({"source":1,"quote":"👩"}),
        serde_json::json!({"source":1,"quote":"Exact","occurrence":1}),
        serde_json::json!({"source":"1","quote":"Exact"}),
        serde_json::json!({"quote":"Exact"}),
    ] {
        for field in ["supports", "misfits"] {
            let raw = if field == "supports" {
                serde_json::json!({"framings":[{"text":"Meaning","supports":[span]}]})
            } else {
                serde_json::json!({"framings":["Meaning"],"misfits":[span]})
            };
            assert!(
                board::parse(&raw.to_string(), &request(), &mut vec![]).is_err(),
                "{raw}"
            );
        }
    }
    let (b, _) =
        parsed(r#"{"framings":[{"text":"Meaning","supports":[{"source":1,"quote":"Café 👩‍💻"}]}]}"#);
    let span = &b.framings[0].supports[0];
    assert_eq!(
        &request().sources[0].text[span.range(&request().sources).unwrap()],
        "Café 👩‍💻"
    );
    let mut changed = request().sources;
    changed[0].text = changed[0].text.replace("Café", "Cafe");
    assert!(span.range(&changed).is_err());
}
#[test]
fn local_history_and_scope_identity_cannot_be_replaced_by_model_output() {
    let mut r = request();
    r.answered.push("done".into());
    r.skipped.push(Question {
        id: "skipped".into(),
        text: "Leave this aside".into(),
    });
    let mut changes = vec![];
    let g=board::parse(r#"{"framings":"Meaning","questions":[{"id":"done","text":"Again"},{"id":"new","text":"Leave this aside"},{"id":"fresh","text":"A real uncertainty"},{"id":"other","text":"A real uncertainty"}]}"#,&r,&mut changes).unwrap();
    assert_eq!(g.questions.len(), 1);
    assert_eq!(g.questions[0].id, "fresh");
    assert_eq!(r.answered, ["done"]);
    assert_eq!(r.skipped.len(), 1);
    assert!(board::parse(r#"{"framings":"Meaning","questions":[{"id":"scope-addition-99","text":"Take over scope"}]}"#,&r,&mut vec![]).is_err());
}
#[test]
fn normalization_escapes_controls_and_logs_fences_without_changing_originals() {
    let raw = "\x60\x60\x60json\n{\"framings\":\"Meaning\\u001b[31m\",\"misfits\":[\"Uncertain\"]}\n\x60\x60\x60";
    let (b, changes) = parsed(raw);
    assert!(!b.framings[0].text.contains('\x1b'));
    assert!(b.framings[0].text.contains("\\u{1b}"));
    assert!(changes.iter().any(|c| c.contains("code fence")));
    assert!(
        changes
            .iter()
            .any(|c| c.contains("Escaped display control"))
    );
    assert_eq!(request().sources[0].text, "Exact Café 👩‍💻 words.");
    let (_, changes) = parsed(r#"{"framings":"Meaning","\\u001b[31m":true}"#);
    assert!(changes.iter().all(|s| !s.contains('\x1b')));
}
#[test]
fn historical_latest_shape_rejections_replay_without_becoming_new_live_attempts() {
    for name in ["onboarding-emails", "trap"] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/evidence/2026-10-09-log-only");
        let records: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("initial-eval").join(name).join("decisions.json")).unwrap(),
        )
        .unwrap();
        let raw = records["decisions"][0]["raw"].as_str().unwrap();
        let mut r = request();
        r.sources[0].text =
            std::fs::read_to_string(root.join("initial-eval").join(name).join("original.txt"))
                .unwrap();
        let mut changes = vec![];
        let g = board::parse(raw, &r, &mut changes).unwrap();
        let b = board::Board::verify(g, &r).unwrap();
        assert_eq!(b.framings.len(), 1);
        assert!(!b.alternatives.is_empty());
        if name == "onboarding-emails" {
            assert!(changes.iter().any(|c| c.contains("precondition")));
        }
    }
}
#[test]
fn final_boundary_replays_all_three_live_boards_without_a_provider_or_helper() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/evidence/2026-10-09-subtraction/live-eval");
    for name in ["books", "onboarding-emails", "trap"] {
        let dir = root.join(name);
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("decisions.json")).unwrap()).unwrap();
        let mut r = request();
        r.sources[0].text = std::fs::read_to_string(dir.join("original.txt")).unwrap();
        let mut changes = vec![];
        let g = Guess::decode(
            record["decisions"][0]["raw"].as_str().unwrap(),
            &r,
            &mut changes,
        )
        .unwrap();
        let b = board::Board::verify(g, &r).unwrap();
        let mut live: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("first-board.json")).unwrap()).unwrap();
        // Recorded before one-reading-only (2026-10-09): only the first reading is kept now.
        if let Some(f) = live["framings"].as_array_mut() {
            f.truncate(1);
        }
        assert_eq!(
            serde_json::to_value(b).unwrap(),
            live,
            "Final-code board differs for {name}"
        );
        assert!(!changes.is_empty());
    }
}
#[test]
fn extra_readings_are_dropped_and_logged_and_notes_reach_the_goal_review() {
    let mut app = BrainDump::with_host(Arc::new(Recording::default()))
        .with_seed_me("/tmp/Tinkery-TEST-unused/SKILL.md".into(), None);
    app.real = true;
    app.paste("Exact Café 👩‍💻 words.");
    app.submit();
    settle(&mut app);
    let raw = r#"{"framings":["Reading zero","Reading one","Extra reading two"],"misfits":["Unresolved note"]}"#;
    let mut changes = vec![];
    let g = board::parse(raw, &request(), &mut changes).unwrap();
    assert!(changes.iter().any(|c| c.contains("3 readings received")));
    app.apply_result(request(), Ok(g));
    let view = snapshot(100, 30, &mut app, false).unwrap();
    assert!(view.contains("Reading zero"));
    assert!(!view.contains("Reading one") && !app.paper().contains("Extra reading two"));
    app.review_goal();
    let goal = app.goal_review.unwrap();
    assert_eq!(goal.affirmation.goal, "Reading zero");
    assert_eq!(goal.affirmation.unresolved_notes, ["Unresolved note"]);
    assert!(app.receipt.is_none());
}
