//! Read ownership, exact display and long history navigation, using only fake reads.

use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

fn outcome(label: &str) -> Outcome {
    Ok(History {
        turns: vec![],
        goal: label.into(),
        made_in: MadeIn::Unknown,
        details: label.into(),
        sequence: "Reading sequence was not recorded.".into(),
        readings: vec![],
    })
}

fn wait(reader: &mut Reader) -> Outcome {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = reader.poll() {
            return result;
        }
        assert!(Instant::now() < until, "history read never completed");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn screen(view: &mut View, size: (u16, u16), help: bool, how: bool) -> String {
    let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
    terminal
        .draw(|frame| {
            view.render(
                frame,
                Rect::new(2, 3, size.0 - 4, size.1 - 4),
                Palette::new(true),
                help,
                how,
            )
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(usize::from(size.0))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn one_worker_coalesces_requests_and_ignores_cancelled_results() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    let (started, begin) = mpsc::channel();
    let count = calls.clone();
    let mut reader = Reader {
        source: Arc::new(move |r| {
            if count.fetch_add(1, Ordering::SeqCst) == 0 {
                started.send(()).unwrap();
                held.lock().unwrap().recv().unwrap();
            }
            outcome(&r.goal)
        }),
        ..Reader::default()
    };
    reader.request("/TEST/records".into(), "/TEST/a".into(), "old".into());
    begin.recv_timeout(Duration::from_secs(3)).unwrap();
    reader.cancel();
    assert!(reader.poll().is_none());
    reader.request("/TEST/records".into(), "/TEST/b".into(), "discarded".into());
    reader.request("/TEST/records".into(), "/TEST/c".into(), "current".into());
    assert!(reader.poll().is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    release.send(()).unwrap();
    assert_eq!(wait(&mut reader).unwrap().goal, "current");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(reader.poll().is_none());
}

#[test]
fn cancelling_a_read_does_not_publish_it_or_start_another() {
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    let mut reader = Reader {
        source: Arc::new(move |_| {
            held.lock().unwrap().recv().unwrap();
            outcome("departed")
        }),
        ..Reader::default()
    };
    reader.request("/TEST/records".into(), "/TEST/a".into(), "old".into());
    reader.cancel();
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while reader.running.is_some() {
        assert!(reader.poll().is_none(), "late history escaped its view");
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(reader.wanted.is_none());
}

#[test]
fn history_keeps_words_and_only_details_disclose_technical_records() {
    let original = "  Café 👩‍💻\nKeep my words.  ";
    let mut view = View::waiting();
    view.finish(Ok(History {
        turns: vec![
            think_record::RecordedTurn {
                at: "TEST time".into(),
                event: Event::Words {
                    source: 1,
                    text: original.into(),
                },
            },
            think_record::RecordedTurn {
                at: "TEST later".into(),
                event: Event::ScopeAnswered {
                    id: "scope".into(),
                    question: "Same goal?".into(),
                    source: 4,
                    answer: "  TEST scope answer Café  ".into(),
                },
            },
        ],
        goal: "Goal exactly.\nMust not: rewrite.".into(),
        made_in: MadeIn::Repo("/TEST/private/TEST-repo".into()),
        details: "/TEST/private-record.json\nMODEL-ONLY-HERE".into(),
        sequence: "Reading sequence was not recorded.".into(),
        readings: vec![],
    }));
    let body = screen(&mut view, (100, 30), false, false);
    assert!(body.contains("  Café 👩‍💻"));
    assert!(body.contains("Keep my words.  "));
    assert!(body.contains("Original words 1"));
    assert!(body.contains("Submission 4"));
    assert!(body.contains("  TEST scope answer Café  "));
    assert!(body.contains("Goal exactly.") && body.contains("Must not: rewrite."));
    assert!(!body.contains("MODEL") && !body.contains("/TEST"));
    assert!(
        !body.contains("Readings · Tinkery's guesses"),
        "missing context added an empty section"
    );
    assert!(screen(&mut view, (100, 30), true, true).contains("MODEL-ONLY-HERE"));
    assert!(body.contains("Made in") && body.contains("TEST-repo"));
}

#[test]
fn no_known_repository_adds_nothing_to_history() {
    for made_in in [MadeIn::NoRepo, MadeIn::Unknown] {
        let mut view = View::waiting();
        view.finish(Ok(History {
            turns: vec![],
            goal: "Goal.".into(),
            made_in,
            details: String::new(),
            sequence: "Reading sequence was not recorded.".into(),
            readings: vec![],
        }));
        let body = screen(&mut view, (100, 30), false, false);
        assert!(!body.contains("Made in"), "{body}");
    }
}

#[test]
fn long_unicode_history_and_help_reach_the_end_and_resize_back() {
    let mut view = View::waiting();
    view.body = Page::plain(&format!("START\n{}\nTHE END", "界👩‍💻 café\n".repeat(70_000)));
    view.details = Page::plain(&format!(
        "DETAIL START\n{}\nDETAIL END",
        "details\n".repeat(70_000)
    ));
    view.scroll(KeyCode::End, false, false);
    assert!(screen(&mut view, (80, 24), false, false).contains("THE END"));
    assert!(view.body.offset > 65_535);
    view.scroll(KeyCode::End, true, true);
    assert!(screen(&mut view, (80, 24), true, true).contains("DETAIL END"));
    view.scroll(KeyCode::Home, false, false);
    assert!(screen(&mut view, (320, 40), false, false).contains("START"));
    assert!(screen(&mut view, (100, 30), true, true).contains("DETAIL END"));
    view.reset_help();
    assert!(screen(&mut view, (100, 30), true, true).contains("DETAIL START"));
}

#[test]
fn wrapping_keeps_every_grapheme_and_newline_without_changing_sources() {
    let raw = format!("  {}\n\nsecond\n", "界👩‍💻 café ".repeat(300));
    let mut page = Page::plain(&raw);
    page.layout(76);
    assert_eq!(page.source[0].0, raw);
    assert_eq!(
        page.rows.iter().map(|r| r.0.as_str()).collect::<String>(),
        raw.replace('\n', "")
    );
    assert!(page.rows.iter().all(|r| r.0.width() <= 76));
    assert!(page.rows.last().unwrap().0.is_empty());
}

#[test]
fn unsafe_controls_are_visible_text_and_do_not_change_saved_words() {
    let raw = "safe\n\x1b[31m\r\t\u{202e} café 👩‍💻";
    let shown = display(raw);
    assert!(shown.contains("\\u{1b}[31m\\r\\t\\u{202e}"));
    assert!(shown.contains("café 👩‍💻"));
    assert!(!shown.contains('\x1b') && !shown.contains('\u{202e}'));
    assert!(raw.contains('\x1b'));
}

#[test]
fn page_keys_move_a_screen_of_history_less_one_row() {
    for size in [(80, 24), (100, 30), (160, 40)] {
        let mut view = View::waiting();
        view.body = Page::plain(&(0..500).map(|i| format!("row {i}\n")).collect::<String>());
        screen(&mut view, size, false, false);
        let height = view.body.height;
        assert!(height > 5, "{size:?}: {height} rows");
        view.scroll(KeyCode::PageDown, false, false);
        assert_eq!(view.body.offset, usize::from(height - 1));
        view.scroll(KeyCode::PageUp, false, false);
        assert_eq!(view.body.offset, 0);
    }
}

#[test]
fn room_to_choose_sequence_is_first_and_numbered_originals_remain_whole() {
    use crate::think_record::{GoalConfirmation, Origin, ReadingContext, Session, Turn};
    use std::time::{Duration, UNIX_EPOCH};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/think-delivery-401.json"
    ))
    .unwrap();
    let words = fixture["words"].as_str().unwrap();
    let answer = fixture["correction"].as_str().unwrap();
    let goal = fixture["readings"][1]["framings"][0]["text"]
        .as_str()
        .unwrap();
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-sequence-")
        .tempdir()
        .unwrap();
    let session = root.path().join("TEST-sessions/TEST-room-to-choose");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("ledger.json"),
        serde_json::json!({
            "schema_version":1,"session_id":"TEST-room-to-choose","status":"active","goal":goal,
        })
        .to_string(),
    )
    .unwrap();
    let turns = [
        Turn {
            at: UNIX_EPOCH + Duration::from_secs(1),
            event: Event::Words {
                source: 1,
                text: words.into(),
            },
        },
        Turn {
            at: UNIX_EPOCH + Duration::from_secs(3),
            event: Event::Answered {
                id: "goal-refinement-2".into(),
                question: "What should change about this goal?".into(),
                source: 2,
                answer: answer.into(),
            },
        },
    ];
    let mut context = ReadingContext::default();
    context.submitted(1);
    context.presented(
        fixture["readings"][0].clone(),
        UNIX_EPOCH + Duration::from_secs(2),
    );
    context.submitted(2);
    context.presented(
        fixture["readings"][1].clone(),
        UNIX_EPOCH + Duration::from_secs(4),
    );
    let records = root.path().join("TEST-records");
    let path = think_record::write_with_context(
        &records,
        &Session {
            origin: &Origin::simulated(),
            turns: &turns,
            checks: vec![],
            goal,
            seed_me_session: &session,
            repo: None,
            confirmed: UNIX_EPOCH + Duration::from_secs(5),
        },
        &GoalConfirmation::terminal(Some("agent")).unwrap(),
        Some(&context),
    )
    .unwrap()
    .path;
    let bytes = std::fs::read(&path).unwrap();
    let history = think_record::read(&records, &session, goal).unwrap();
    assert_eq!(history.turns[0].event, turns[0].event);
    assert_eq!(history.turns[1].event, turns[1].event);
    for size in [(80, 24), (140, 42)] {
        let mut view = View::waiting();
        view.finish(Ok(history.clone()));
        let first = screen(&mut view, size, false, false);
        let entries = [
            "Original words 1",
            "Reading 1 shown",
            "Submission 2 · after reading 1",
            "Reading 2 shown",
        ];
        let positions: Vec<_> = entries
            .iter()
            .map(|entry| first.find(entry).expect("entry on first screen"))
            .collect();
        assert!(positions.windows(2).all(|p| p[0] < p[1]), "{first}");
        assert!(first.contains("Recorded sequence · 1 Jan 1970, UTC"));
        assert!(first.contains("00:00:03  Submission 2 · after reading 1"));
        assert!(first.contains("Shown means it reached the screen, not that it was read."));
        assert!(!first.contains("1970-01-01T"));
        assert!(!first.contains("framings") && !first.contains("structure-spans-history"));
        let mut all = first
            .lines()
            .skip(5)
            .take(usize::from(size.1 - 6))
            .collect::<Vec<_>>()
            .join("\n");
        let mut previous = first;
        for _ in 0..200 {
            view.scroll(KeyCode::Down, false, false);
            let next = screen(&mut view, size, false, false);
            if next == previous {
                break;
            }
            all.push_str(next.lines().nth(usize::from(size.1 - 2)).unwrap());
            previous = next;
        }
        assert!(all.contains("Submission 2"));
        let headings = [
            "Submission 2",
            "Readings · Tinkery's guesses",
            "Reading 1 · shown 00:00:02",
            "Reading 2 · shown 00:00:04",
            "Confirmed goal",
        ];
        let positions: Vec<_> = headings
            .iter()
            .map(|heading| all.find(heading).expect("heading reachable"))
            .collect();
        assert!(positions.windows(2).all(|p| p[0] < p[1]));
        let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        for text in [words, answer, goal] {
            assert!(
                compact(&all).contains(&compact(text)),
                "full text missing at {size:?}"
            );
        }
        let first_reading = all
            .split("Reading 1 · shown 00:00:02")
            .nth(1)
            .unwrap()
            .split("Reading 2 · shown 00:00:04")
            .next()
            .unwrap();
        let second_reading = all
            .split("Reading 2 · shown 00:00:04")
            .nth(1)
            .unwrap()
            .split("Confirmed goal")
            .next()
            .unwrap();
        let (main, extras) = first_reading
            .split_once("Also in your dump")
            .expect("first reading grouping");
        assert!(
            compact(main).contains(&compact(
                fixture["readings"][0]["framings"][0]["text"]
                    .as_str()
                    .unwrap()
            ))
        );
        for item in fixture["readings"][0]["other_goals"].as_array().unwrap() {
            assert!(compact(extras).contains(&compact(item.as_str().unwrap())));
        }
        assert!(!main.contains("Must: Build through"));
        assert!(!second_reading.contains("Also in your dump"));
        for text in fixture["readings"][1]["parts"]["must"].as_array().unwrap() {
            assert!(compact(second_reading).contains(&compact(text.as_str().unwrap())));
        }
        view.scroll(KeyCode::End, false, false);
        assert!(screen(&mut view, size, false, false).contains("Change tool configuration."));
        assert!(screen(&mut view, size, true, true).contains("Think record:"));
        view.scroll(KeyCode::Home, false, false);
        assert!(screen(&mut view, size, false, false).contains("Recorded sequence"));
    }
    assert_eq!(
        std::fs::read(path).unwrap(),
        bytes,
        "display changed the source record"
    );
}

#[test]
fn reading_guesses_are_italic_and_preserve_partial_groups_without_changing_original_styles() {
    use crate::think_record::{RecordedReading, RecordedTurn};
    use ratatui::style::Modifier;
    let mut view = View::waiting();
    view.finish(Ok(History {
        turns: vec![RecordedTurn {
            at: "TEST time".into(),
            event: Event::Words {
                source: 1,
                text: "TEST ORIGINAL ink".into(),
            },
        }],
        goal: "TEST CONFIRMED ink".into(),
        made_in: MadeIn::Unknown,
        details: "TEST DETAILS".into(),
        sequence: "TEST sequence".into(),
        readings: vec![
            RecordedReading {
                at: "2026-10-27T01:00:00Z".into(),
                heading: "Reading 1 · shown 01:00:00".into(),
                main: vec!["  TEST MAIN Café  ".into(), "TEST MAIN TWO".into()],
                other_goals: vec!["TEST OTHER FIRST".into(), "TEST OTHER SECOND".into()],
                deferred: vec!["TEST DEFERRED FIRST".into(), "TEST DEFERRED SECOND".into()],
            },
            RecordedReading {
                at: "2026-10-27T00:00:00Z".into(),
                heading: "Reading 2 · shown 00:00:00".into(),
                main: vec!["  \n  ".into()],
                other_goals: vec![],
                deferred: vec!["TEST PARTIAL Café\u{202e}".into()],
            },
        ],
    }));
    for no_color in [false, true] {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut seen = String::new();
        let mut item_boundaries = false;
        view.scroll(KeyCode::Home, false, false);
        let mut previous = String::new();
        for _ in 0..100 {
            terminal
                .draw(|frame| {
                    view.render(
                        frame,
                        Rect::new(2, 3, 76, 20),
                        Palette::new(no_color),
                        false,
                        false,
                    )
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let mut current = String::new();
            for y in 5..23 {
                let text = (2..78).map(|x| buffer[(x, y)].symbol()).collect::<String>();
                for marker in ["TEST MAIN", "TEST OTHER", "TEST DEFERRED", "TEST PARTIAL"] {
                    if let Some(column) = text.find(marker) {
                        let cell = &buffer[(2 + column as u16, y)];
                        assert!(
                            cell.modifier.contains(Modifier::ITALIC),
                            "guess lost italic: {text}"
                        );
                    }
                }
                for marker in ["TEST ORIGINAL", "TEST CONFIRMED"] {
                    if let Some(column) = text.find(marker) {
                        let cell = &buffer[(2 + column as u16, y)];
                        assert!(
                            !cell.modifier.contains(Modifier::ITALIC),
                            "original/confirmed became a guess"
                        );
                    }
                }
                current.push_str(&text);
                current.push('\n');
            }
            item_boundaries |= current
                .lines()
                .map(str::trim_end)
                .collect::<Vec<_>>()
                .join("\n")
                .contains("TEST OTHER FIRST\n\nTEST OTHER SECOND");
            seen.push_str(&current);
            if current == previous {
                break;
            }
            previous = current;
            view.scroll(KeyCode::Down, false, false);
        }
        assert!(item_boundaries, "other-goal item boundaries disappeared");
        assert!(seen.contains("  TEST MAIN Café  "));
        assert!(seen.contains("TEST MAIN TWO"));
        assert!(seen.contains("No goal text recorded for this reading."));
        assert!(seen.contains("TEST PARTIAL Café\\u{202e}"));
        assert!(!seen.contains('\u{202e}'));
        for marker in [
            "TEST OTHER FIRST",
            "TEST OTHER SECOND",
            "TEST DEFERRED FIRST",
            "TEST DEFERRED SECOND",
        ] {
            assert!(seen.contains(marker));
        }
        let first = seen.find("Reading 1 · shown 01:00:00").unwrap();
        let partial = seen.find("Reading 2 · shown 00:00:00").unwrap();
        assert!(first < partial);
    }
}
