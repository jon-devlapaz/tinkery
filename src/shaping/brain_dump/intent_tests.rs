use super::goal_tests::right_text;
use super::tests::settle;
use super::*;

const SOURCE: &str = "Must work offline. MUST NOT upload recordings; don't care about colours.";
const SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (160, 40), (320, 40)];

fn head() -> serde_json::Value {
    serde_json::json!({"who":"I","outcome":"keep field notes","why":"signal drops","done_when":"a week of notes syncs later"})
}
const HEAD: &str =
    "I keep field notes.\nWhy: signal drops.\nDone when: a week of notes syncs later.";
/// Decoded and verified through the same path as a provider reply.
fn app(raw: serde_json::Value) -> BrainDump {
    let mut a =
        BrainDump::with_host(Arc::new(Simulated)).with_seed_me("/missing/SKILL.md".into(), None);
    a.sources.push(Source {
        id: 1,
        text: SOURCE.into(),
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
    let g = Guess::decode(&raw.to_string(), &r, &mut vec![]).unwrap();
    a.guess = Some(board::Board::verify(g, &r).unwrap());
    a
}
pub(super) fn constraints(must: &[&str], must_not: &[&str]) -> BrainDump {
    let mut parts = head();
    parts["must"] = serde_json::json!(must);
    parts["must_not"] = serde_json::json!(must_not);
    app(serde_json::json!({ "parts": parts }))
}

#[test]
fn explicit_leading_prefixes_normalize_before_composition_and_handoff() {
    // (must, must_not) in → (must, must_not) out, then the exact constraint lines.
    #[allow(clippy::type_complexity)]
    let table: &[(&[&str], &[&str], &[&str], &[&str], &str)] = &[
        (
            &["must work offline"],
            &[],
            &["work offline"],
            &[],
            "Must: work offline.",
        ),
        (
            &["MUST Work Offline"],
            &[],
            &["Work Offline"],
            &[],
            "Must: Work Offline.",
        ),
        (
            &["must not upload recordings"],
            &[],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &["Must Not upload recordings"],
            &[],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &["don't upload recordings"],
            &[],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &[],
            &["must not upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &[],
            &["MUST NOT upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &[],
            &["Do not upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &[],
            &["DON'T upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &[],
            &["don\u{2019}t upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &["must work offline; Must not upload recordings"],
            &[],
            &["work offline"],
            &["upload recordings"],
            "Must: work offline.\nMust not: upload recordings.",
        ),
        // No duplicate constraints after normalization.
        (
            &["must not upload recordings"],
            &["upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &["must not upload recordings"],
            &["must not upload recordings"],
            &[],
            &["upload recordings"],
            "Must not: upload recordings.",
        ),
        (
            &["must work offline", "work offline"],
            &[],
            &["work offline"],
            &[],
            "Must: work offline.",
        ),
        (
            &["simple", "must work offline"],
            &[],
            &["simple", "work offline"],
            &[],
            "Must: simple.\nMust: work offline.",
        ),
        (
            &["must preserve U.S. data, e.g. maps"],
            &[],
            &["preserve U.S. data, e.g. maps"],
            &[],
            "Must: preserve U.S. data, e.g. maps.",
        ),
        (
            &["must preserve U.S. Do not upload logs"],
            &[],
            &["preserve U.S"],
            &["upload logs"],
            "Must: preserve U.S.\nMust not: upload logs.",
        ),
        (
            &["must coordinate with the U.S. Army"],
            &[],
            &["coordinate with the U.S. Army"],
            &[],
            "Must: coordinate with the U.S. Army.",
        ),
        (
            &["must coordinate with Dr. Smith"],
            &[],
            &["coordinate with Dr. Smith"],
            &[],
            "Must: coordinate with Dr. Smith.",
        ),
        (
            &["must ask Prof. Lee, Mrs. Diaz etc. first. Do not email Mr. Ng"],
            &[],
            &["ask Prof. Lee, Mrs. Diaz etc. first"],
            &["email Mr. Ng"],
            "Must: ask Prof. Lee, Mrs. Diaz etc. first.\nMust not: email Mr. Ng.",
        ),
        (
            &["must keep café 👩‍💻 names"],
            &[],
            &["keep café 👩‍💻 names"],
            &[],
            "Must: keep café 👩‍💻 names.",
        ),
    ];
    for (must, must_not, want_must, want_not, lines) in table {
        let mut a = constraints(must, must_not);
        let parts = &a.guess.as_ref().unwrap().parts;
        assert_eq!(parts.must, *want_must, "{must:?} {must_not:?}");
        assert_eq!(parts.must_not, *want_not, "{must:?} {must_not:?}");
        let composed = format!("{HEAD}\n{lines}");
        assert_eq!(a.guess.as_ref().unwrap().framings[0].text, composed);
        a.review_goal();
        let affirmation = &a.goal_review.as_ref().unwrap().affirmation;
        assert_eq!(affirmation.goal, composed);
        assert_eq!(affirmation.parts.must, *want_must);
        assert_eq!(affirmation.parts.must_not, *want_not);
        assert_eq!(affirmation.sources[0].text.as_bytes(), SOURCE.as_bytes());
        assert_eq!(a.sources[0].text.as_bytes(), SOURCE.as_bytes());
    }
}
#[test]
fn ambiguous_or_quoted_negation_is_left_unchanged() {
    for (must, must_not) in [
        (&["must not only support phones"][..], &[][..]),
        (&[], &["must not only support phones"]),
        (&["Do not just copy the old app"], &[]),
        (&["\"must ship Friday\" is the team's slogan"], &[]),
        (&["the label says \"must not upload\""], &[]),
        (&["Don't care about colours"], &[]),
        (&[], &["don't care about colours"]),
        (&[], &["don't mind slow syncs"]),
        (&[], &["must work offline"]),
        (&["mustard yellow headers"], &[]),
        (&["must  not upload recordings"], &[]),
        (&["must not: upload recordings"], &[]),
        (
            &["Display ‘must work offline; must not upload recordings’"],
            &[],
        ),
        (
            &["Display \"must work offline; must not upload recordings\""],
            &[],
        ),
        (&["Show 'I don't sync; must not upload' on the label"], &[]),
        (&["Show ‘I don’t sync; must not upload’ on the label"], &[]),
        (&["must"], &["do not"]),
    ] {
        let a = constraints(must, must_not);
        let parts = &a.guess.as_ref().unwrap().parts;
        assert_eq!(parts.must, must, "{must:?}");
        assert_eq!(parts.must_not, must_not, "{must_not:?}");
        let lines = must
            .iter()
            .map(|v| format!("Must: {v}."))
            .chain(must_not.iter().map(|v| format!("Must not: {v}.")))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            a.guess.as_ref().unwrap().framings[0].text,
            format!("{HEAD}\n{lines}")
        );
        assert_eq!(a.sources[0].text.as_bytes(), SOURCE.as_bytes());
    }
}
#[test]
fn combined_clauses_keep_names_acronyms_decimals_and_quotes() {
    let p = GoalParts {
        who: Some("I".into()),
        outcome: Some("brief Priya".into()),
        why: Some(
            "NASA data arrives late. Priya reviews 2.5 GB nightly; \"Ship it. Then rest\" is the motto. It costs hours. IT handles access. A backup exists"
                .into(),
        ),
        ..Default::default()
    };
    assert_eq!(
        p.compose().unwrap(),
        "I brief Priya.\nWhy: NASA data arrives late; Priya reviews 2.5 GB nightly; \"Ship it. Then rest\" is the motto; it costs hours; IT handles access; a backup exists."
    );
    let a = constraints(&["must keep NASA terms; Priya signs off at 2.5 GB"], &[]);
    assert_eq!(
        a.guess.as_ref().unwrap().parts.must,
        ["keep NASA terms; Priya signs off at 2.5 GB"]
    );
}
#[test]
fn timing_is_emitted_once_beside_normalized_constraints() {
    let a = app(
        serde_json::json!({"parts":{"who":"My mom","outcome":"spots a fake text before tapping anything","when":"before tapping anything","must_not":["must not need an app"]}}),
    );
    let text = &a.guess.as_ref().unwrap().framings[0].text;
    assert_eq!(
        text,
        "My mom spots a fake text before tapping anything.\nMust not: need an app."
    );
    assert_eq!(text.matches("before tapping anything").count(), 1);
}

struct Missing {
    part: Part,
}
impl BoardHost for Missing {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        let mut parts = head();
        parts[self.part.name()] = serde_json::Value::Null;
        let raw = serde_json::json!({"parts":parts,"questions":[{"target":self.part.name(),"id":format!("ask-{}",r.sources.len()),"text":format!("Which check {}?",r.sources.len())}]});
        Guess::decode(&raw.to_string(), &r, &mut vec![])
    }
}
fn missing(part: Part) -> BrainDump {
    let mut a = BrainDump::with_host(Arc::new(Missing { part }))
        .with_seed_me("/missing/SKILL.md".into(), None);
    a.paste(SOURCE);
    a.submit();
    settle(&mut a);
    assert!(a.focused_question().is_some());
    a
}
fn screens(a: &mut BrainDump) -> Vec<String> {
    SIZES
        .iter()
        .map(|&(w, h)| right_text(&snapshot(w, h, a, false).unwrap(), w))
        .collect()
}
/// Review lists every open item, and readiness is not needed to try saving.
fn review_lists_and_saves(a: &mut BrainDump, open: &[&str]) {
    a.review_goal();
    let mut seen = String::new();
    for _ in 0..30 {
        seen.push_str(&right_text(&snapshot(80, 24, a, false).unwrap(), 80));
        a.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    }
    seen.push_str(&right_text(&snapshot(80, 24, a, false).unwrap(), 80));
    assert!(seen.contains("Still open"), "{seen}");
    for item in open {
        assert!(seen.contains(item), "{item}: {seen}");
    }
    a.paste("confirm");
    a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(a.handoff_job.is_some(), "readiness was required to save");
    // The missing helper fails the save attempt; no session root is written.
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while a.handoff_job.is_some() && std::time::Instant::now() < end {
        a.goal_tick();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(a.notice.contains("Handoff not complete"));
    assert!(a.receipt.is_none());
    assert!(a.goal_config.as_ref().unwrap().recovery_path().is_none());
}
#[test]
fn ready_says_nothing_unsure() {
    let mut a = constraints(&["must work offline"], &[]);
    assert!(a.goal_ready());
    for screen in screens(&mut a) {
        assert!(screen.contains("Nothing I'm unsure about. Review when you're ready."));
        assert!(!screen.contains("Still missing") && !screen.contains("remain open"));
    }
    a.help = true;
    assert!(yohaku::help_text(&a).contains("Nothing I'm unsure about.\nF3 reviews the goal."));
}
#[test]
fn skipped_why_names_only_the_missing_part() {
    let mut a = missing(Part::Why);
    a.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    assert!(a.focused_question().is_none() && !a.goal_ready());
    for screen in screens(&mut a) {
        assert!(
            screen.contains("Still missing: Why. Review when you're ready."),
            "{screen}"
        );
        assert!(!screen.contains("Nothing I'm unsure") && !screen.contains("remain open"));
    }
    assert!(
        yohaku::help_text(&a).starts_with("Right now\nStill missing: Why.\nF3 reviews the goal.")
    );
    review_lists_and_saves(&mut a, &["Why — Which check 1?"]);
    assert_eq!(a.sources[0].text.as_bytes(), SOURCE.as_bytes());
}
#[test]
fn done_when_after_skip_or_two_asks_is_named_missing_and_reviewable() {
    let mut skipped = missing(Part::DoneWhen);
    skipped.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    let mut capped = missing(Part::DoneWhen);
    for answer in ["first answer", "second answer"] {
        capped.paste(answer);
        capped.submit();
        settle(&mut capped);
    }
    assert_eq!(capped.ask_counts[&Part::DoneWhen], 2);
    for (a, open) in [
        (&mut skipped, "Done when — Which check 1?"),
        (&mut capped, "Done when — still open"),
    ] {
        assert!(a.focused_question().is_none() && !a.goal_ready());
        for screen in screens(a) {
            assert!(
                screen.contains("Still missing: Done when. Review when you're ready."),
                "{screen}"
            );
            assert!(!screen.contains("Nothing I'm unsure") && !screen.contains("remain open"));
        }
        review_lists_and_saves(a, &[open]);
    }
}
#[test]
fn present_but_open_and_deferred_context_is_not_called_missing() {
    let mut parts = head();
    parts["must"] = serde_json::json!(["must work offline"]);
    let mut a = app(
        serde_json::json!({"parts":parts,"open":["why"],"deferred":["editing can wait"],"other_goals":["release builds"]}),
    );
    assert!(a.focused_question().is_none() && !a.goal_ready());
    for screen in screens(&mut a) {
        assert!(
            screen.contains("Some parts remain open for the seed. Review when you're ready."),
            "{screen}"
        );
        assert!(!screen.contains("Still missing") && !screen.contains("Nothing I'm unsure"));
    }
    let paper = a.paper();
    assert!(paper.contains("for the seed: editing can wait"));
    assert!(paper.contains("also in your dump: release builds"));
    review_lists_and_saves(
        &mut a,
        &[
            "Why — still open",
            "for the seed: editing can wait",
            "release builds",
        ],
    );
}
#[test]
fn no_composed_goal_names_who_without_offering_review() {
    let mut a = app(serde_json::json!({"parts":{}}));
    a.skipped.push(a.focused_question().unwrap().clone());
    for screen in screens(&mut a) {
        assert!(screen.contains("Still missing: Who."), "{screen}");
        assert!(!screen.contains("Review when you're ready"));
    }
}
struct FailsAfterFirst(std::sync::atomic::AtomicUsize);
impl BoardHost for FailsAfterFirst {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        if self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0 {
            return Err("TEST stub failure".into());
        }
        Guess::decode(
            &serde_json::json!({ "parts": head() }).to_string(),
            &r,
            &mut vec![],
        )
    }
}
#[test]
fn a_submitted_refinement_that_failed_to_apply_is_not_called_ready() {
    let mut a = BrainDump::with_host(Arc::new(FailsAfterFirst(Default::default())))
        .with_seed_me("/missing/SKILL.md".into(), None);
    a.paste(SOURCE);
    a.submit();
    settle(&mut a);
    assert!(a.goal_ready());
    a.paste("also sync over wifi only");
    a.submit();
    settle(&mut a);
    assert_ne!(a.applied, a.sources.len());
    for screen in screens(&mut a) {
        assert!(!screen.contains("Nothing I'm unsure"), "{screen}");
        assert!(
            screen.contains("Some parts remain open for the seed."),
            "{screen}"
        );
    }
    assert!(
        !yohaku::open_intent(&a)
            .unwrap()
            .contains("Nothing I'm unsure")
    );
    assert_eq!(a.sources[1].text, "also sync over wifi only");
}
#[test]
fn a_skipped_question_without_a_target_keeps_the_goal_open() {
    let mut a = constraints(&["must work offline"], &[]);
    assert!(yohaku::open_intent(&a).unwrap().starts_with("Nothing"));
    a.skipped.push(Question {
        target: None,
        id: "priority".into(),
        text: "Which goal comes first?".into(),
    });
    for screen in screens(&mut a) {
        assert!(
            screen.contains("Some parts remain open for the seed."),
            "{screen}"
        );
        assert!(!screen.contains("Nothing I'm unsure"));
    }
    review_lists_and_saves(&mut a, &["Which goal comes first?"]);
}
#[test]
fn many_distinct_prefixed_constraints_normalize_in_linear_time() {
    let mut p = GoalParts {
        must: (0..4000).map(|i| format!("must {i}")).collect(),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    p.normalize_constraints();
    assert_eq!(p.must.len(), 4000);
    assert_eq!(p.must[3999], "3999");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}
#[test]
fn clause_splitting_stays_linear_on_many_initials() {
    let long = "A. ".repeat(32 * 1024 / 3);
    let p = GoalParts {
        who: Some("I".into()),
        outcome: Some("read".into()),
        why: Some(long.clone()),
        must: vec![format!("must {long}")],
        ..Default::default()
    };
    let started = std::time::Instant::now();
    let mut parts = p.clone();
    parts.normalize_constraints();
    assert!(parts.compose().is_some());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}
