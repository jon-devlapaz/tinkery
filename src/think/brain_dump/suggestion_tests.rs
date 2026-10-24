use super::*;
use crate::think_record::Event;
use serde_json::json;

fn raw(value: serde_json::Value) -> String {
    json!({"parts":{"who":"I", "outcome":"track books", "why":"remember them"},
        "open":["who"], "questions":[{"id":"audience", "target":"who",
            "text":"Who is this for?", "suggestion":value}]})
    .to_string()
}
fn fixture() -> BrainDump {
    let mut a = super::super::goal_tests::board();
    let request = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    let guess = Guess::decode(
        &raw(json!({"answer":"Just me.", "reason":"You described your own books."})),
        &request,
        &mut vec![],
    )
    .unwrap();
    a.conversation.apply(request, Ok(guess)).unwrap();
    a
}
fn use_it(a: &mut BrainDump) {
    a.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
}

#[test]
fn suggestion_copies_without_submitting_or_overwriting() {
    let mut a = fixture();
    let sources = serde_json::to_value(a.conversation.sources()).unwrap();
    let goal = a.conversation.board().unwrap().goal().map(str::to_owned);
    assert!(a.input.text.is_empty());
    use_it(&mut a);
    assert_eq!(a.input.text, "Just me.");
    assert!(!a.running());
    assert_eq!(
        serde_json::to_value(a.conversation.sources()).unwrap(),
        sources
    );
    assert!(a.conversation.settled().is_empty());
    assert_eq!(a.conversation.board().unwrap().goal(), goal.as_deref());
    assert!(
        !a.conversation
            .turns()
            .iter()
            .any(|t| matches!(t.event, Event::Answered { .. }))
    );
    a.input = Note::new("My exact draft\r\n");
    use_it(&mut a);
    assert_eq!(a.input.text, "My exact draft\r\n");
}

#[test]
fn unaccepted_suggestions_are_recorded_but_never_become_model_context() {
    let mut a = fixture();
    assert!(
        a.conversation
            .turns()
            .iter()
            .any(|t| matches!(&t.event, Event::Suggested {answer, ..} if answer == "Just me."))
    );
    let request = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    assert!(
        !serde_json::to_string(&request)
            .unwrap()
            .contains("Just me.")
    );
    use_it(&mut a);
    a.input.insert(" Actually, my family.").unwrap();
    let staged = a
        .conversation
        .stage(&a.input.text, false, false, vec![])
        .unwrap();
    a.conversation.commit(&staged);
    assert_eq!(
        a.conversation.sources().last().unwrap().text,
        "Just me. Actually, my family."
    );
    assert!(
        matches!(&a.conversation.turns().last().unwrap().event, Event::Answered {answer, ..} if answer == "Just me. Actually, my family.")
    );
}

#[test]
fn malformed_suggestions_preserve_questions_and_review_decisions() {
    let a = fixture();
    let r = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    for value in [
        json!(null),
        json!("guess"),
        json!({"answer":"x"}),
        json!({"answer":"", "reason":"why"}),
        json!({"answer":"x\u{1b}","reason":"why"}),
    ] {
        let guess = Guess::decode(&raw(value.clone()), &r, &mut vec![]).unwrap();
        assert_eq!(guess.questions[0].text, "Who is this for?");
        assert!(guess.questions[0].suggestion.is_none());
        let review = Review::decode(&json!({"decisions":[{"part":"who", "text":"Audience", "answers":["Me","Family"], "question":"Who is this for?", "suggestion":value}]}).to_string(), &mut vec![]).unwrap();
        let guess =
            Guess::decode_reviewed(&raw(json!(null)), Some(&review), &r, &mut vec![]).unwrap();
        assert!(!guess.decisions.is_empty());
        assert!(guess.decisions[0].suggestion.is_none());
    }
}

#[test]
fn changed_question_discards_suggestion_and_scope_has_honest_fallback() {
    let mut a = fixture();
    let r = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    a.conversation
        .apply(r.clone(), Guess::decode(&raw(json!(null)), &r, &mut vec![]))
        .unwrap();
    assert!(!a.can_use_suggestion());
    let staged = a
        .conversation
        .stage("Another thought", true, false, vec![])
        .unwrap();
    a.conversation.commit(&staged);
    assert!(a.conversation.awaiting_scope());
    assert!(
        description(a.conversation.focused_question().unwrap()).contains("No suggested answer")
    );
}

#[test]
fn suggestion_and_reason_are_reachable_at_all_sizes_without_calls() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for mono in [false, true] {
            let mut a = fixture();
            let mut seen = String::new();
            for _ in 0..6 {
                seen.push_str(&super::super::goal_tests::right_text(
                    &snapshot(w, h, &mut a, mono).unwrap(),
                    w,
                ));
                a.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
            }
            assert!(seen.contains("Just me."), "{seen}");
            assert!(seen.contains("You described your own books."), "{seen}");
            assert!(a.input.text.is_empty());
            assert!(!a.running());
        }
    }
}

#[test]
fn review_question_keeps_its_own_suggestion_and_rewording_clears_it() {
    let a = fixture();
    let r = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    let proposal = json!({"answer":"No sharing yet.", "reason":"You described personal use."});
    let review = Review::decode(&json!({"decisions":[{"part":"who", "text":"Audience", "answers":["Me","Family"], "question":"Include family?", "suggestion":proposal}]}).to_string(), &mut vec![]).unwrap();
    let reply = json!({"parts":{"who":"I", "outcome":"track books", "why":"remember", "done_when":"I find a book"}}).to_string();
    let guess = Guess::decode_reviewed(&reply, Some(&review), &r, &mut vec![]).unwrap();
    let board = board::Board::verify(guess, &r).unwrap();
    assert_eq!(board.questions[0].text, "Include family?");
    assert_eq!(
        board.questions[0].suggestion.as_ref().unwrap().answer,
        "No sharing yet."
    );
    assert!(
        !serde_json::to_string(&board.wire())
            .unwrap()
            .contains("No sharing yet.")
    );

    let reply = json!({"parts":{"who":"I", "outcome":"track books", "why":"remember"},
        "decisions":[{"part":"done_when", "text":"Success", "answers":["Find a book", "Finish a book"],
            "question":"Find or finish?", "suggestion":proposal}]}).to_string();
    let guess = Guess::decode(&reply, &r, &mut vec![]).unwrap();
    let board = board::Board::verify(guess, &r).unwrap();
    assert_eq!(board.questions[0].text, "What would tell you this worked?");
    assert!(board.questions[0].suggestion.is_none());
    assert!(board.decisions[0].suggestion.is_none());
}

#[test]
fn unavailable_and_stale_suggestions_never_change_the_draft() {
    for state in ["pending", "behind", "new dump", "missing"] {
        let mut a = fixture();
        match state {
            "pending" => {
                a.shaping.ready = Some((
                    a.conversation
                        .stage("", false, false, vec![])
                        .unwrap()
                        .request,
                    Err("TEST failed reply".into()),
                ))
            }
            "behind" => a.conversation.set_applied(0),
            "new dump" => a.add_more = true,
            _ => a.conversation.set_board(None),
        }
        use_it(&mut a);
        assert!(a.input.text.is_empty(), "{state}");
        assert!(!a.running());
    }
}

#[test]
fn review_input_excludes_unaccepted_suggestions_but_keeps_choices() {
    let a = fixture();
    let r = a
        .conversation
        .stage("", false, false, vec![])
        .unwrap()
        .request;
    let reply = json!({"parts":{"who":"I"}, "decisions":[{"part":"who", "text":"Audience", "answers":["Me","Family"], "question":"Who?", "suggestion":{"answer":"PROPOSAL ONLY", "reason":"ASSUMPTION ONLY"}}]}).to_string();
    let input = super::super::review::input(&r, &reply).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&input).unwrap();
    assert_eq!(value["decisions"][0]["answers"], json!(["Me", "Family"]));
    assert!(value["decisions"][0].get("suggestion").is_none());
    assert!(!String::from_utf8(input).unwrap().contains("PROPOSAL ONLY"));
}

#[test]
fn next_dump_draft_is_unsent_and_edited_words_have_separate_provenance() {
    let mut a = BrainDump::default();
    a.offer_draft(crate::think_record::DraftSuggestion {
        text: "TEST proposal".into(),
        provenance: "TEST provenance not model context".into(),
    });
    for (w, h) in [(80, 24), (100, 30)] {
        for mono in [false, true] {
            let shown = snapshot(w, h, &mut a, mono).unwrap();
            assert!(shown.contains("Tinkery's guess"));
        }
    }
    a.tick();
    assert!(!a.running() && a.conversation.sources().is_empty());
    assert!(a.saved().is_none());
    a.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    a.paste("  TEST my correction\r\n");
    assert_eq!(a.input.text, "  TEST my correction\r\n");
    a.handle_key(KeyEvent::from(KeyCode::F(2)));
    assert_eq!(a.conversation.sources()[0].text, "  TEST my correction\r\n");
    assert!(matches!(&a.conversation.turns()[0].event,
        Event::DraftSuggested { text, provenance } if text == "TEST proposal" && provenance.contains("TEST provenance")));
    assert!(matches!(&a.conversation.turns()[1].event,
        Event::Words { text, .. } if text == "  TEST my correction\r\n"));
    let request = a.shaping.request.as_ref().unwrap();
    let wire = serde_json::to_string(request).unwrap();
    assert!(!wire.contains("TEST proposal") && !wire.contains("TEST provenance"));
}
