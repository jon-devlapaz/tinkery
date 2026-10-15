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
        turns: vec![think_record::RecordedTurn {
            at: "TEST time".into(),
            event: Event::Words {
                source: 1,
                text: original.into(),
            },
        }],
        goal: "Goal exactly.\nMust not: rewrite.".into(),
        made_in: MadeIn::Repo("/TEST/private/TEST-repo".into()),
        details: "/TEST/private-record.json\nMODEL-ONLY-HERE".into(),
    }));
    let body = screen(&mut view, (100, 30), false, false);
    assert!(body.contains("  Café 👩‍💻"));
    assert!(body.contains("Keep my words.  "));
    assert!(body.contains("Goal exactly.") && body.contains("Must not: rewrite."));
    assert!(!body.contains("MODEL") && !body.contains("/TEST"));
    assert!(screen(&mut view, (100, 30), true, true).contains("MODEL-ONLY-HERE"));
    assert!(body.contains("Made in") && body.contains("TEST-repo"));
}

#[test]
fn where_a_goal_was_made_is_said_plainly_and_never_guessed() {
    for (made_in, said) in [
        (MadeIn::NoRepo, "not in a repository"),
        (MadeIn::Unknown, "repo unknown"),
    ] {
        let mut view = View::waiting();
        view.finish(Ok(History {
            turns: vec![],
            goal: "Goal.".into(),
            made_in,
            details: String::new(),
        }));
        let body = screen(&mut view, (100, 30), false, false);
        assert!(body.contains("Made in") && body.contains(said), "{body}");
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
