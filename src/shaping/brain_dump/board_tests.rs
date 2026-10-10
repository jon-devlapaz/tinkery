use super::*;
fn request() -> BoardRequest {
    BoardRequest {
        ask_counts: Default::default(),
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
        r#"{"parts":{"who":"I","outcome":"remember books","unexpected":"ignore"},"alternatives":{"label":"Maybe","precondition":"unknown"},"misfits":[{"text":"Unsure about a relationship"}],"confirmed":true,"settled":[{"source":99}]}"#,
    );
    assert!(matches!(b.framings[0].supports, board::Grounding::Uncited));
    assert_eq!(b.parts.outcome.as_deref(), Some("remember books"));
    assert_eq!(b.parts.missing().count(), 2);
    assert_eq!(b.questions[0].target, Some(Part::Why));
    assert!(b.misfits.is_empty());
    assert_eq!(b.unresolved_notes, ["Unsure about a relationship"]);
    assert!(b.alternatives[0].benefit.is_none());
    for key in ["unexpected", "precondition", "confirmed", "settled"] {
        assert!(changes.iter().any(|c| c.contains(key)));
    }
    assert_eq!(b.wire().parts, b.parts);
}
#[test]
fn lengths_and_optional_types_are_not_boundary_rules_and_one_question_is_presented() {
    let input = serde_json::json!({"parts":{"who":"I","outcome":vec!["word";100].join(" "),"why":{},"done_when":null},"open":["done_when","nonsense"],"questions":(0..9).map(|i|serde_json::json!({"target":"done_when","text":format!("Unresolved {i} without question mark")})).collect::<Vec<_>>(),"alternatives":vec!["same";7]});
    let (b, changes) = parsed(&input.to_string());
    assert_eq!(b.framings.len(), 1);
    assert_eq!(b.questions.len(), 1);
    assert_eq!(b.alternatives.len(), 7);
    assert_eq!(b.presented(0).len(), 1);
    assert!(b.parts.why.is_none() && b.parts.done_when.is_none());
    assert!(changes.iter().any(|s| s.contains("unknown open part")));
    assert!(changes.iter().any(|s| s.contains("Extra questions")));
}
#[test]
fn malformed_json_fails_but_empty_partial_parts_display_without_fabrication() {
    for raw in ["not JSON", "[]"] {
        assert!(board::parse(raw, &request(), &mut vec![]).is_err());
    }
    for raw in [
        r#"{}"#,
        r#"{"parts":{"outcome":3,"why":null},"questions":[null,{"text":4}],"alternatives":[true]}"#,
        r#"{"framings":["Do not infer this old goal"]}"#,
    ] {
        let (b, _) = parsed(raw);
        assert!(b.framings.is_empty());
        assert_eq!(b.parts.missing().count(), 4);
        let mut app = BrainDump {
            sources: request().sources,
            applied: 1,
            guess: Some(b),
            ..Default::default()
        };
        for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let screen = snapshot(w, h, &mut app, false).unwrap();
            assert!(!screen.contains("Nothing I'm unsure about"));
            assert!(!screen.contains("Do not infer this old goal"));
        }
    }
}
#[test]
fn claimed_citations_remain_exact_and_grapheme_safe_even_without_parts() {
    for span in [
        serde_json::json!({"source":2,"quote":"Exact"}),
        serde_json::json!({"source":1,"quote":"invented"}),
        serde_json::json!({"source":1,"quote":"👩"}),
        serde_json::json!({"source":1,"quote":"Exact","occurrence":1}),
        serde_json::json!({"source":"1","quote":"Exact"}),
        serde_json::json!({"quote":"Exact"}),
    ] {
        for raw in [
            serde_json::json!({"supports":[span.clone()]}),
            serde_json::json!({"framings":[{"supports":[span.clone()]}]}),
            serde_json::json!({"misfits":[span]}),
        ] {
            assert!(
                board::parse(&raw.to_string(), &request(), &mut vec![]).is_err(),
                "{raw}"
            );
        }
    }
    let (b, _) = parsed(
        r#"{"parts":{"who":"I","outcome":"read"},"supports":[{"source":1,"quote":"Café 👩‍💻"}]}"#,
    );
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
        target: None,
        id: "skipped".into(),
        text: "Leave this aside".into(),
    });
    let mut changes = vec![];
    let g=board::parse(r#"{"questions":[{"id":"done","text":"Again"},{"id":"new","text":"Leave this aside"},{"id":"fresh","text":"A real uncertainty"},{"id":"other","text":"A real uncertainty"}]}"#,&r,&mut changes).unwrap();
    assert_eq!(g.questions.len(), 1);
    assert_eq!(g.questions[0].id, "fresh");
    assert_eq!(r.answered, ["done"]);
    assert_eq!(r.skipped.len(), 1);
    assert!(
        board::parse(
            r#"{"questions":[{"id":"scope-addition-99","text":"Take over scope"}]}"#,
            &r,
            &mut vec![]
        )
        .is_err()
    );
}
#[test]
fn normalization_escapes_controls_and_logs_fences_without_changing_originals() {
    let raw = "```json\n{\"parts\":{\"who\":\"I\",\"outcome\":\"Meaning\\u001b[31m\"},\"misfits\":[\"Uncertain\"]}\n```";
    let (b, changes) = parsed(raw);
    assert!(!b.framings[0].text.contains('\x1b'));
    assert!(b.framings[0].text.contains("\\u{1b}"));
    assert!(changes.iter().any(|s| s.contains("code fence")));
    assert!(
        changes
            .iter()
            .any(|s| s.contains("Escaped display control"))
    );
    assert_eq!(request().sources[0].text, "Exact Café 👩‍💻 words.");
    let (_, changes) = parsed(r#"{"parts":{},"\u001b[31m":true}"#);
    assert!(changes.iter().all(|s| !s.contains('\x1b')));
}
#[test]
fn historical_raw_captures_display_without_inferred_parts_or_provider_helpers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (folder, names) in [
        (
            "docs/evidence/2026-10-09-log-only/initial-eval",
            vec!["onboarding-emails", "trap"],
        ),
        (
            "docs/evidence/2026-10-09-subtraction/live-eval",
            vec!["books", "onboarding-emails", "trap"],
        ),
    ] {
        for name in names {
            let dir = root.join(folder).join(name);
            let records: serde_json::Value =
                serde_json::from_slice(&std::fs::read(dir.join("decisions.json")).unwrap())
                    .unwrap();
            let mut r = request();
            r.sources[0].text = std::fs::read_to_string(dir.join("original.txt")).unwrap();
            let mut changes = vec![];
            let g = Guess::decode(
                records["decisions"][0]["raw"].as_str().unwrap(),
                &r,
                &mut changes,
            )
            .unwrap();
            let b = board::Board::verify(g, &r).unwrap();
            assert_eq!(b.parts, GoalParts::default());
            assert!(b.framings.is_empty());
            assert!(!changes.is_empty());
            let mut app = BrainDump {
                sources: r.sources,
                applied: 1,
                guess: Some(b),
                ..Default::default()
            };
            for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
                snapshot(w, h, &mut app, false).unwrap();
            }
            assert!(!app.goal_ready());
            assert!(app.receipt.is_none());
        }
    }
}
#[test]
fn freeform_readings_are_ignored_and_notes_survive_without_becoming_a_goal() {
    let (b, changes) = parsed(
        r#"{"parts":{"must_not":"no dashboard"},"framings":["Reading zero","Reading one"],"misfits":["Unresolved note"]}"#,
    );
    assert_eq!(b.framings[0].text, "Must not: no dashboard.");
    assert!(changes.iter().any(|s| s.contains("Ignored freeform")));
    assert_eq!(b.unresolved_notes, ["Unresolved note"]);
    let mut app =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    app.sources = request().sources;
    app.applied = 1;
    app.guess = Some(b);
    app.review_goal();
    let frozen = app.goal_review.unwrap();
    assert_eq!(frozen.affirmation.goal, "Must not: no dashboard.");
    assert!(
        frozen
            .affirmation
            .unresolved_notes
            .iter()
            .any(|s| s == "Unresolved note")
    );
    assert!(app.receipt.is_none());
}
