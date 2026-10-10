use super::tests::settle;
use super::*;

#[test]
fn final_native_boards_replay_exactly_without_provider_or_helper_calls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/evidence/2026-10-09-goal-shape");
    let calls = std::fs::read_to_string(root.join("call-trace/calls.jsonl")).unwrap();
    for line in calls.lines().skip(25) {
        let call: serde_json::Value = serde_json::from_str(line).unwrap();
        if call["kind"] != "shape" {
            continue;
        }
        let index = call["index"].as_u64().unwrap();
        let input: serde_json::Value = serde_json::from_slice(
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
        let mut matched = None;
        for name in [
            "books",
            "onboarding-emails",
            "trap",
            "ledger",
            "voice",
            "operator-goal",
        ] {
            let dir = root.join("after").join(name);
            for file in ["00-first.json", "01-answer.json"] {
                let path = dir.join(file);
                if !path.exists() {
                    continue;
                }
                let snapshot: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
                if snapshot["sources"] == input["sources"] {
                    let decision = snapshot["decisions_cumulative"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .rev()
                        .find(|d| d["mode"] == "blocking")
                        .unwrap();
                    matched = Some((
                        snapshot["board"].clone(),
                        decision["raw"].as_str().unwrap().to_owned(),
                    ));
                    break;
                }
            }
        }
        let (expected, raw) = matched.expect("each final shape has its captured board");
        let parsed = Guess::decode(&raw, &r, &mut vec![]).unwrap();
        let board = board::Board::verify(parsed, &r).unwrap();
        assert_eq!(
            serde_json::to_value(board).unwrap(),
            expected,
            "final shape {index}"
        );
    }
}

#[test]
fn skipped_target_blocks_a_renamed_and_reworded_model_question() {
    let a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![Question {
            target: Some(Part::Proof),
            id: "old-proof".into(),
            text: "Old proof?".into(),
        }],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let g=Guess::decode(&serde_json::json!({"parts":parts(),"open":["proof"],"questions":[{"target":"proof","id":"renamed-proof","text":"A new way of asking proof?"}]}).to_string(),&r,&mut vec![]).unwrap();
    assert!(g.questions.is_empty());
    assert!(g.open.contains(&Part::Proof));
    assert!(board::Board::verify(g, &r).unwrap().questions.is_empty());
}

#[test]
fn full_person_voice_and_multiple_clauses_keep_words_without_duplicate_prefixes() {
    let p = GoalParts {
        outcome: Some("I want learners to feel smarter. I do not want new targets.".into()),
        boundaries: Some("No dashboard. It must work on phones.".into()),
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "I want learners to feel smarter; I do not want new targets. No dashboard; It must work on phones."
    );
    let p = GoalParts {
        outcome: Some("read 2.5 hours; keep \"same. words\"".into()),
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "I can read 2.5 hours; keep \"same. words\"."
    );
}
#[test]
fn trailing_commas_normalize_only_outside_strings_and_preserve_claimed_span_guards() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let mut changes = vec![];
    let g = Guess::decode(
        r#"{"parts":{"outcome":"retain literal ,} and \\\"quotes\\\"",},"open":["proof",],}"#,
        &r,
        &mut changes,
    )
    .unwrap();
    assert_eq!(
        g.parts.outcome.as_deref(),
        Some("retain literal ,} and \\\"quotes\\\"")
    );
    assert!(changes.iter().any(|s| s.contains("trailing JSON")));
    a.apply_result(r.clone(), Ok(g));
    assert!(a.guess.is_some());
    for raw in [
        r#"{"parts": {"outcome": "a"} "open": [],}"#,
        r#"{"supports":[{"source":99,"quote":"An intact original",},],}"#,
    ] {
        assert!(Guess::decode(raw, &r, &mut vec![]).is_err());
    }
}

#[test]
fn delayed_results_cannot_bypass_current_ask_counts_or_skipped_targets() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let g = Repeated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    a.ask_counts.insert(Part::Proof, 2);
    a.apply_result(r.clone(), Ok(g));
    assert!(a.focused_question().is_none());
    assert_eq!(a.ask_counts[&Part::Proof], 2);
    assert!(!a.goal_ready());
    a.ask_counts.clear();
    a.skipped.push(Question {
        target: Some(Part::Proof),
        id: "local-skip".into(),
        text: "Skipped proof".into(),
    });
    let g = Repeated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    a.apply_result(r, Ok(g));
    assert!(a.focused_question().is_none());
    assert!(!a.goal_ready());
}
#[test]
fn unsent_local_words_never_show_completion_and_unknown_fields_are_not_authority() {
    let mut a = app(
        serde_json::json!({"parts":parts(),"confirmed":true,"settled":[{}],"open":["unknown"]}),
    );
    assert!(a.goal_ready());
    assert!(a.settled.is_empty() && a.receipt.is_none());
    a.paste("new local words");
    assert!(!a.goal_ready());
    a.review_goal();
    assert!(a.goal_review.is_none());
}

fn parts() -> GoalParts {
    GoalParts {
        situation: Some("Each morning".into()),
        outcome: Some("tell whether yesterday went well".into()),
        why: Some("I know what to do".into()),
        proof: Some("useful changes per hour are better than my usual week".into()),
        boundaries: Some("no targets, no dashboard; cost only when it's way off".into()),
    }
}
fn app(raw: serde_json::Value) -> BrainDump {
    let mut a =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.sources.push(Source {
        id: 1,
        text: "An intact original".into(),
        in_reply_to: None,
    });
    a.applied = 1;
    let r = BoardRequest {
        ask_counts: Default::default(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: None,
        skipped: vec![],
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let guess = Guess::decode(&raw.to_string(), &r, &mut vec![]).unwrap();
    a.guess = Some(board::Board::verify(guess, &r).unwrap());
    a
}
#[test]
fn complete_composition_has_two_sentences_and_preserves_criteria_and_boundaries() {
    assert_eq!(
        parts().compose().unwrap(),
        "Each morning, I can tell whether yesterday went well, so I know what to do. It works if useful changes per hour are better than my usual week; no targets, no dashboard; cost only when it's way off."
    );
}
#[test]
fn every_combination_of_missing_parts_omits_dangling_connectives_and_never_invents_values() {
    let full = parts();
    for mask in 0..32 {
        let keep = |p: Part| {
            if mask & (1 << (p as u8)) != 0 {
                full.get(p).map(str::to_owned)
            } else {
                None
            }
        };
        let partial = GoalParts {
            situation: keep(Part::Situation),
            outcome: keep(Part::Outcome),
            why: keep(Part::Why),
            proof: keep(Part::Proof),
            boundaries: keep(Part::Boundaries),
        };
        if mask == 0 {
            assert!(partial.compose().is_none());
            continue;
        }
        let text = partial.compose().unwrap();
        assert!(!text.starts_with([',', ';']));
        assert!(!text.contains(". ;"));
        assert!(!text.contains("so ."));
        assert!(!text.contains("if ;"));
        for p in Part::ALL {
            if let Some(value) = partial.get(p) {
                assert!(text.contains(value));
            } else {
                assert!(!text.contains(full.get(p).unwrap()));
            }
        }
    }
    assert_eq!(
        GoalParts {
            boundaries: Some("no dashboard".into()),
            ..Default::default()
        }
        .compose()
        .as_deref(),
        Some("no dashboard.")
    );
    assert_eq!(
        GoalParts {
            proof: Some("it feels right".into()),
            ..Default::default()
        }
        .compose()
        .as_deref(),
        Some("It works if it feels right.")
    );
}
#[test]
fn punctuation_unicode_and_existing_connectives_do_not_change_words() {
    let p = GoalParts {
        situation: Some(" When tired. ".into()),
        outcome: Some("I can keep Café 👩‍💻 notes.".into()),
        why: Some("so I remember \"why?\"".into()),
        proof: Some("It works if I recognise them.".into()),
        boundaries: Some("no new numbers.".into()),
    };
    assert_eq!(
        p.compose().unwrap(),
        "When tired, I can keep Café 👩‍💻 notes, so I remember \"why?\". It works if I recognise them; no new numbers."
    );
}
#[test]
fn readiness_is_all_five_present_and_not_vague_skipped_or_pending_scope() {
    let mut a = app(serde_json::json!({"parts":parts(),"open":[]}));
    assert!(a.goal_ready());
    assert!(
        snapshot(160, 40, &mut a, false)
            .unwrap()
            .contains("Nothing I'm unsure about")
    );
    for p in Part::ALL {
        let mut missing = serde_json::to_value(parts()).unwrap();
        missing[p.name()] = serde_json::Value::Null;
        let mut a = app(serde_json::json!({"parts":missing}));
        assert!(!a.goal_ready());
        assert!(
            !snapshot(160, 40, &mut a, false)
                .unwrap()
                .contains("Nothing I'm unsure about")
        );
        let mut a = app(serde_json::json!({"parts":parts(),"open":[p]}));
        assert!(!a.goal_ready());
        a.review_goal();
        assert!(
            a.goal_review
                .as_ref()
                .unwrap()
                .affirmation
                .unresolved_notes
                .iter()
                .any(|s| s.contains(p.label()))
        );
    }
    a.skipped.push(Question {
        target: Some(Part::Proof),
        id: "skipped".into(),
        text: "Which proof?".into(),
    });
    assert!(!a.goal_ready());
    a.scope_pending = Some((
        1,
        Question {
            target: None,
            id: "scope-addition-1".into(),
            text: "Scope?".into(),
        },
    ));
    a.skipped.clear();
    assert!(!a.goal_ready());
}
#[test]
fn coarse_present_criteria_are_not_missing_and_no_queue_is_not_completion() {
    let mut a = app(serde_json::json!({"parts":parts()}));
    assert!(a.goal_ready());
    a.review_goal();
    assert_eq!(
        a.goal_review.as_ref().unwrap().affirmation.goal,
        parts().compose().unwrap()
    );
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .is_empty()
    );
    let mut blank = app(serde_json::json!({"parts":{}}));
    assert!(!blank.goal_ready());
    blank.review_goal();
    assert!(blank.goal_review.is_none());
    assert!(blank.receipt.is_none());
}
struct Repeated;
impl BoardHost for Repeated {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        Guess::decode(&serde_json::json!({"parts":parts(),"open":["proof"],"questions":[{"target":"proof","id":format!("proof-{}",r.sources.len()),"text":format!("Proof follow-up {}?",r.sources.len())}]}).to_string(),&r,&mut vec![])
    }
}
#[test]
fn at_most_two_asks_then_the_part_stays_open_for_seed() {
    let mut a =
        BrainDump::with_host(Arc::new(Repeated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.paste("original");
    a.submit();
    settle(&mut a);
    assert_eq!(a.ask_counts[&Part::Proof], 1);
    a.paste("first answer");
    a.submit();
    settle(&mut a);
    assert_eq!(a.ask_counts[&Part::Proof], 2);
    assert!(a.focused_question().is_some());
    a.paste("second answer");
    a.submit();
    settle(&mut a);
    assert!(a.focused_question().is_none());
    assert_eq!(a.ask_counts[&Part::Proof], 2);
    assert!(!a.goal_ready());
    assert_eq!(a.open_parts(), [Part::Proof]);
    a.review_goal();
    assert!(a.goal_review.is_some());
    assert!(a.handoff_job.is_none());
}
#[test]
fn a_skipped_target_is_not_reasked_and_undo_is_not_another_ask() {
    let mut a =
        BrainDump::with_host(Arc::new(Repeated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.paste("original");
    a.submit();
    settle(&mut a);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    assert!(a.focused_question().is_none());
    assert!(!a.goal_ready());
    a.undo_skip();
    assert!(a.focused_question().is_some());
    assert_eq!(a.ask_counts[&Part::Proof], 1);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    let r = BoardRequest {
        ask_counts: a.ask_counts.clone(),
        sources: a.sources.clone(),
        fragments: vec![],
        previous: a.guess.as_ref().map(|b| b.wire()),
        skipped: a.skipped.clone(),
        answered: vec![],
        settled: vec![],
        layout: vec![],
    };
    let g = Repeated
        .reshape(r.clone(), &AtomicBool::new(false))
        .unwrap();
    a.apply_result(r, Ok(g));
    assert!(a.focused_question().is_none());
    assert!(!a.goal_ready());
    a.review_goal();
    assert!(
        a.goal_review
            .as_ref()
            .unwrap()
            .affirmation
            .unresolved
            .iter()
            .any(|q| q.target == Some(Part::Proof))
    );
}
#[test]
fn quiet_review_freezes_present_parts_and_identical_goal_without_missing_lines() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = app(
            serde_json::json!({"parts":{"outcome":"see five emails in the first week","boundaries":"phones; no gamification; Kirra's claim remains unverified"},"open":["why"]}),
        );
        let reading = a.guess.as_ref().unwrap().framings[0].text.clone();
        assert_eq!(a.guess.as_ref().unwrap().parts.compose().unwrap(), reading);
        assert!(a.paper().contains(&reading));
        a.review_goal();
        let frozen = a.goal_review.as_ref().unwrap();
        assert_eq!(frozen.affirmation.goal, reading);
        assert_eq!(frozen.affirmation.unresolved_notes.len(), 3);
        let mut seen = String::new();
        for _ in 0..30 {
            seen.push_str(&snapshot(w, h, &mut a, false).unwrap());
            a.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        }
        for term in [
            "You are confirming",
            "I can",
            "not",
            "Still open",
            "type confirm",
        ] {
            assert!(seen.contains(term), "{term} at {w}x{h}");
        }
        assert!(!seen.contains("Not supplied") && !seen.contains("Doesn’t fit yet"));
        super::tests::amend_board(&mut a, |g| g.parts.outcome = Some("different".into()));
        assert_eq!(a.goal_review.as_ref().unwrap().affirmation.goal, reading);
        assert_eq!(
            a.goal_review.as_ref().unwrap().parts.outcome.as_deref(),
            Some("see five emails in the first week")
        );
        assert!(a.handoff_job.is_none());
    }
}
