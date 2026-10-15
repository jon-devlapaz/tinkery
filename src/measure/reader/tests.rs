//! Barrier-owned lifecycle and real synthetic subprocess regressions; no installed ledger is read.

use super::*;
use std::sync::mpsc;

fn wait(reader: &mut Reader, visible: &mut Reading) {
    let watchdog = Instant::now() + Duration::from_secs(5);
    while reader.job.is_some() {
        assert!(Instant::now() < watchdog, "reader failed to clean up");
        reader.poll(Some(visible), Instant::now());
        thread::yield_now();
    }
}

#[test]
fn blocked_start_retains_capacity_until_cleanup_and_cannot_publish_on_reopen() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    let source: Source = Arc::new(move |_, _| {
        entered_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
        Reading::unavailable(Cause::Exit, "old result".into())
    });
    let mut reader = Reader::with_source(source);
    let mut first = reader.open(Path::new("/TEST"), Instant::now());
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(first.waiting_state());
    reader.cancel();
    let mut reopened = reader.open(Path::new("/TEST"), Instant::now());
    assert!(reopened.request.is_none());
    assert_eq!(reader.next, 1);
    release_tx.send(()).unwrap();
    wait(&mut reader, &mut reopened);
    assert!(
        matches!(reopened.report, super::super::Report::Waiting(s) if s.starts_with("Previous read stopped"))
    );
    assert!(!matches!(reopened.report, super::super::Report::Failed(_)));
    reader.poll(Some(&mut first), Instant::now());
    assert!(
        first.waiting_state(),
        "departed visit must not be published"
    );
}

#[test]
fn deadline_includes_start_and_is_checked_again_at_acceptance() {
    for offset in [
        DEADLINE - Duration::from_nanos(1),
        DEADLINE,
        DEADLINE + Duration::from_nanos(1),
    ] {
        let now = Instant::now();
        let source: Source = Arc::new(|_, _| Reading::waiting(None, "completed fake"));
        let mut reader = Reader::with_source(source);
        let mut visible = reader.open(Path::new("/TEST"), now);
        while !reader.job.as_ref().unwrap().task.is_finished() {
            thread::yield_now();
        }
        reader.poll(Some(&mut visible), now + offset);
        if offset < DEADLINE {
            assert!(visible.waiting_state());
        } else {
            assert_eq!(visible.report_cause(), Some(&Cause::Timeout));
        }
        assert!(reader.job.is_none());
    }
}

#[test]
fn blocked_preparation_obeys_cancel_and_deadline_without_publishing_a_ready_report() {
    for cancelled in [false, true] {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        let source: Source = Arc::new(move |_, _| {
            let report = super::super::Report::Complete("café 界 👩‍💻".repeat(100_000));
            entered_tx.send(()).unwrap(); // Raw result ready; preparation has not finished.
            release_rx.lock().unwrap().recv().unwrap();
            Reading::with_report(report)
        });
        let mut reader = Reader::with_source(source);
        let now = Instant::now();
        let mut visible = reader.open(Path::new("/TEST"), now);
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        if cancelled {
            reader.cancel();
        }
        reader.poll(
            Some(&mut visible),
            if cancelled { now } else { now + DEADLINE },
        );
        assert!(reader.job.is_some());
        if !cancelled {
            assert_eq!(visible.report_cause(), Some(&Cause::Timeout));
        }
        release_tx.send(()).unwrap();
        wait(&mut reader, &mut visible);
        assert!(!matches!(visible.report, super::super::Report::Complete(_)));
    }
}

#[test]
fn exceptional_shutdown_warns_once_and_does_not_call_pending_cleanup_complete() {
    let (mut reader, entered, release) = Reader::blocked();
    reader.open(Path::new("/TEST"), Instant::now());
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut notices = vec![];
    reader.drain(
        |notice| {
            notices.push(notice.to_string());
            release.send(()).unwrap();
        },
        || Duration::from_secs(3),
        thread::yield_now,
    );
    assert_eq!(
        notices,
        ["Ledger reader cleanup is still pending; exit time is unknown."]
    );
    assert!(reader.job.is_none());
}

#[test]
fn cancellation_and_completion_event_orders_do_not_resurrect_cancelled_visits() {
    for cancel_before_poll in [false, true] {
        let mut reader = Reader::with_source(Arc::new(|_, _| {
            Reading::unavailable(Cause::Exit, "finished".into())
        }));
        let mut visible = reader.open(Path::new("/TEST"), Instant::now());
        while !reader.job.as_ref().unwrap().task.is_finished() {
            thread::yield_now();
        }
        if cancel_before_poll {
            reader.cancel();
        }
        reader.poll(Some(&mut visible), Instant::now());
        assert_eq!(visible.failed(), !cancel_before_poll);
        assert!(reader.job.is_none());
    }
}

#[test]
fn worker_panic_keeps_cleanup_uncertainty_visible_after_deadline_and_on_reopen() {
    for elapsed in [Duration::ZERO, DEADLINE, DEADLINE + Duration::from_nanos(1)] {
        let mut reader =
            Reader::with_source(Arc::new(|_, _| panic!("synthetic TEST worker panic")));
        let now = Instant::now();
        let mut visible = reader.open(Path::new("/TEST"), now);
        let watchdog = Instant::now() + Duration::from_secs(5);
        while !reader.job.as_ref().unwrap().task.is_finished() {
            assert!(Instant::now() < watchdog);
            thread::yield_now();
        }
        reader.poll(Some(&mut visible), now + elapsed);
        assert_eq!(
            visible.report_cause(),
            Some(if elapsed < DEADLINE {
                &Cause::Collection
            } else {
                &Cause::Timeout
            })
        );
        assert_cleanup_disclosed(&mut visible);
        assert!(reader.closed);
        assert!(
            reader
                .cleanup_error
                .as_ref()
                .unwrap()
                .contains("cleanup could not be confirmed")
        );
        let mut retry = reader.open(Path::new("/TEST"), Instant::now());
        assert_cleanup_disclosed(&mut retry);
        assert!(retry.text[2].contains("further reads are refused"));
        assert_eq!(reader.next, 1);
        assert!(reader.job.is_none());
        let mut notices = vec![];
        reader.shutdown(|notice| notices.push(notice.to_string()));
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("cleanup could not be confirmed"));
    }
}

fn assert_cleanup_disclosed(reading: &mut Reading) {
    let super::super::Report::Failed(failure) = &reading.report else {
        panic!("expected failure")
    };
    assert!(
        failure.details.contains("cleanup could not be confirmed"),
        "{}",
        failure.details
    );
    assert!(reading.text[2].contains("cleanup could not be confirmed"));
    reading.how = true;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| reading.render(frame, frame.area(), crate::Palette::new(true), true))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = (0..24)
        .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.contains("cleanup could not be confirmed"),
        "{rendered}"
    );
}

#[test]
fn cancel_before_source_returns_keeps_latch_and_shutdown_waits_for_cleanup() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    let source: Source = Arc::new(move |_, control| {
        entered_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
        assert_eq!(control.reason_at(Instant::now()), CANCEL);
        Reading::waiting(None, "stopped")
    });
    let mut reader = Reader::with_source(source);
    reader.open(Path::new("/TEST"), Instant::now());
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    reader.cancel();
    release_tx.send(()).unwrap();
    reader.shutdown(|_| panic!("ordinary shutdown should not warn"));
    assert!(reader.job.is_none());
    assert!(reader.closed);
    assert!(
        reader
            .open(Path::new("/TEST"), Instant::now())
            .request
            .is_none()
    );
}
