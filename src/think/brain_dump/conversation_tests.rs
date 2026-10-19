//! Think's conversation rules, with no terminal, keys or jobs.

use super::*;

const DUMP: &str = "I want mornings that start calmly.";

fn parts() -> serde_json::Value {
    serde_json::json!({
        "who": "I",
        "outcome": "start mornings calmly",
        "why": "rushed mornings spill into the day",
        "done_when": "I notice a calm start most days"
    })
}
/// The goal before its check is known.
fn without_check() -> serde_json::Value {
    let mut p = parts();
    p["done_when"] = serde_json::Value::Null;
    p
}
fn check(n: usize) -> serde_json::Value {
    serde_json::json!({"target": "done_when", "id": format!("check-{n}"), "text": format!("Which check {n}?")})
}
/// Send `words` and apply `reply` as the model's answer.
fn turn(c: &mut Conversation, words: &str, reply: serde_json::Value) {
    let staged = c.stage(words, false, false, vec![]).unwrap();
    if staged.adds_words(c) {
        c.commit(&staged);
    }
    let guess = Guess::decode(&reply.to_string(), &staged.request, &mut vec![]).unwrap();
    c.apply(staged.request, Ok(guess)).unwrap();
}
/// A conversation whose reading asks one question.
fn asking() -> Conversation {
    let mut c = Conversation::default();
    turn(
        &mut c,
        DUMP,
        serde_json::json!({"parts": without_check(), "questions": [check(1)]}),
    );
    assert_eq!(c.focused_question().unwrap().id, "check-1");
    c
}

#[test]
fn a_skipped_question_stays_skipped_when_the_model_asks_it_again() {
    let tone = serde_json::json!({"id": "tone", "text": "Calm in what way?"});
    let reply = serde_json::json!({"parts": without_check(), "questions": [tone]});
    let mut c = Conversation::default();
    turn(&mut c, DUMP, reply.clone());
    assert_eq!(c.focused_question().unwrap().id, "tone");
    assert!(c.skip_focused());
    assert!(c.focused_question().is_none());
    assert!(!c.skip_focused(), "nothing left to skip");
    let staged = c.stage("", false, false, vec![]).unwrap();
    // The request carries the skips, so the decoder or the conversation drops the repeat.
    if let Ok(guess) = Guess::decode(&reply.to_string(), &staged.request, &mut vec![]) {
        let _ = c.apply(staged.request, Ok(guess));
    }
    assert!(
        c.focused_question().is_none(),
        "a skipped question came back"
    );
    assert!(!c.nothing_open(), "a skip is not an answer");
}

#[test]
fn undoing_a_skip_restores_it_unless_it_was_answered_since() {
    let mut c = asking();
    assert_eq!(c.undo_skip(), None, "nothing to undo");
    c.skip_focused();
    assert_eq!(c.undo_skip(), Some(true));
    assert_eq!(c.focused_question().unwrap().id, "check-1");
    turn(
        &mut c,
        "a calm start is no phone before eight",
        serde_json::json!({"parts": parts()}),
    );
    assert_eq!(c.settled()[0].question.id, "check-1");
    assert_eq!(c.sources()[1].in_reply_to.as_deref(), Some("check-1"));
    let answered = c.settled()[0].question.clone();
    c.skipped_mut().push(answered);
    assert_eq!(
        c.undo_skip(),
        Some(false),
        "an answered question stays closed"
    );
    assert!(c.focused_question().is_none());
    assert!(c.nothing_open());
}

#[test]
fn added_words_stay_out_of_the_reading_until_their_scope_is_answered() {
    let mut c = Conversation::default();
    turn(&mut c, DUMP, serde_json::json!({"parts": parts()}));
    let staged = c.stage("Also fix the bike.", true, false, vec![]).unwrap();
    assert!(staged.addition);
    c.commit(&staged);
    let next = c.stage("", false, false, vec![]).unwrap();
    assert_eq!(next.request.sources.len(), 1, "added words are not sent");
    assert!(c.awaiting_scope());
    let scope = c.focused_question().unwrap().clone();
    assert!(scope.id.starts_with("scope-addition-"));
    assert!(
        c.stage("More words.", true, false, vec![]).is_err(),
        "a second addition waits for the first one's scope"
    );
    let answer = c.stage("keep it separate", false, false, vec![]).unwrap();
    assert_eq!(
        answer.request.sources.len(),
        3,
        "answering scope sends them"
    );
    c.commit(&answer);
    assert!(!c.awaiting_scope());
    assert_eq!(c.settled()[0].question.id, scope.id);
}

#[test]
fn words_after_a_composed_goal_refine_it() {
    let mut c = Conversation::default();
    turn(&mut c, DUMP, serde_json::json!({"parts": parts()}));
    let staged = c.stage("only on weekdays", false, false, vec![]).unwrap();
    assert!(!staged.addition);
    c.commit(&staged);
    assert_eq!(c.settled()[0].question.id, "goal-refinement-2");
    assert!(!c.caught_up());
}

#[test]
fn observations_and_refusal_remain_intact_without_a_manufactured_goal_question() {
    let words = [
        "Coffee was good this morning. I'm just putting this here; I haven't asked for anything to change.",
        "Nothing should change. Leave these as observations rather than asking me to define a goal.",
    ];
    let mut c = Conversation::default();
    for original in words {
        turn(
            &mut c,
            original,
            serde_json::json!({"parts": {}, "questions": []}),
        );
        assert!(c.focused_question().is_none());
        assert!(c.board().unwrap().goal().is_none());
        assert!(c.caught_up());
        assert!(c.last_failure().is_none());
        assert!(!c.nothing_open(), "unknown intent is not a complete goal");
    }
    assert_eq!(
        c.sources()
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        words
    );
}

#[test]
fn refusing_a_previously_asked_goal_question_does_not_break_history() {
    let mut c = Conversation::default();
    turn(
        &mut c,
        "These are observations, not a project.",
        serde_json::json!({
            "parts": {}, "questions": [{"target": "outcome", "id": "recorded-observation-query",
                "text": "What should change when this works?"}]
        }),
    );
    turn(
        &mut c,
        "Nothing should change. I am not choosing a project.",
        serde_json::json!({"parts": {}, "questions": []}),
    );
    assert!(c.caught_up());
    assert!(c.last_failure().is_none());
    assert!(c.focused_question().is_none());
    assert!(c.board().unwrap().goal().is_none());
    assert_eq!(
        c.sources()[1].text,
        "Nothing should change. I am not choosing a project."
    );
    assert_eq!(
        c.sources()[1].in_reply_to.as_deref(),
        Some("recorded-observation-query")
    );
    assert_eq!(c.settled()[0].question.id, "recorded-observation-query");
}

#[test]
fn a_failed_reply_keeps_the_previous_reading() {
    let mut c = asking();
    let before = c.board().unwrap().parts.compose();
    let staged = c.stage("tea first", false, false, vec![]).unwrap();
    c.commit(&staged);
    assert_eq!(
        c.apply(staged.request, Err("provider down".into())),
        Err("provider down".into())
    );
    assert_eq!(c.last_failure(), Some("provider down"));
    assert_eq!(c.board().unwrap().parts.compose(), before);
    assert!(!c.caught_up(), "the answer is kept, unread");
}

#[test]
fn extracts_are_exact_unique_and_bounded() {
    let mut c = Conversation::default();
    turn(&mut c, DUMP, serde_json::json!({"parts": parts()}));
    let f = c.extract(1, 0, 6).unwrap();
    assert_eq!((f.id.as_str(), f.text.as_str()), ("f1", "I want"));
    assert!(c.extract(1, 0, 6).is_err(), "no duplicates");
    assert!(c.extract(2, 0, 1).is_err(), "unknown source");
    assert!(c.extract(1, 1, 2).is_err(), "whitespace only");
    let mut ranges = (0..DUMP.len()).flat_map(|s| (s + 1..=DUMP.len()).map(move |e| (s, e)));
    while c.fragments().len() < 32 {
        let (start, end) = ranges.next().unwrap();
        let _ = c.extract(1, start, end);
    }
    assert_eq!(c.fragments().len(), 32);
    assert!(c.extract(1, 2, 6).unwrap_err().contains("32 cards"));
}

#[test]
fn turns_keep_every_question_answer_and_scope_in_the_order_they_happened() {
    use crate::think_record::Event;
    let mut c = asking();
    let staged = c
        .stage("TEST: also the welcome email", true, false, vec![])
        .unwrap();
    c.commit(&staged);
    let staged = c
        .stage("TEST: keep it separate", false, false, vec![])
        .unwrap();
    c.commit(&staged);
    let events: Vec<_> = c.turns().iter().map(|t| t.event.clone()).collect();
    let asked = Event::Asked {
        id: "check-1".into(),
        part: Some(Part::DoneWhen),
        question: "Which check 1?".into(),
    };
    let scope = c.settled().last().unwrap().question.clone();
    assert_eq!(
        events,
        [
            Event::Words {
                source: 1,
                text: DUMP.into()
            },
            asked.clone(),
            Event::Words {
                source: 2,
                text: "TEST: also the welcome email".into()
            },
            Event::ScopeAsked {
                id: scope.id.clone(),
                question: scope.text.clone(),
                source: 2
            },
            Event::ScopeAnswered {
                id: scope.id,
                question: scope.text,
                source: 3,
                answer: "TEST: keep it separate".into(),
            },
            asked,
        ]
    );
    assert!(c.turns().windows(2).all(|w| w[0].at <= w[1].at));
}
