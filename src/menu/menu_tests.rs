//! Menu and tend never wait on session discovery. The fake source is held open, so a
//! regression that reads inside paint, tick or input fails instead of passing quietly.
use super::*;
use crate::data_roots::DataRoots;
use crate::tend::{Found, Outcome, Session, Stage, scan};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    sync::{
        Condvar, Mutex,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sess(id: &str, stage: Stage, updated: &str) -> Session {
    Session {
        id: Some(id.into()),
        path: PathBuf::from(format!("/TEST/{id}")),
        goal: format!("goal {id}"),
        stage,
        updated: updated.into(),
    }
}
fn found(sessions: Vec<Session>) -> Outcome {
    Ok(Found {
        sessions,
        unreadable: 0,
        failures: Vec::new(),
    })
}

/// A source that blocks until given a permit, counting entries, finishes and peak workers.
#[derive(Default)]
struct Gate {
    permits: Mutex<usize>,
    wake: Condvar,
    outcome: Mutex<Option<Outcome>>,
    entered: AtomicUsize,
    finished: AtomicUsize,
    live: AtomicUsize,
    peak: AtomicUsize,
}
impl Gate {
    fn new(outcome: Outcome) -> Arc<Self> {
        let gate = Arc::new(Self::default());
        *gate.outcome.lock().unwrap() = Some(outcome);
        gate
    }
    fn source(self: &Arc<Self>) -> Source {
        let gate = self.clone();
        Arc::new(move |_root| {
            gate.entered.fetch_add(1, SeqCst);
            let now = gate.live.fetch_add(1, SeqCst) + 1;
            gate.peak.fetch_max(now, SeqCst);
            let mut permits = gate.permits.lock().unwrap();
            let deadline = Instant::now() + Duration::from_secs(4);
            while *permits == 0 {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                permits = gate.wake.wait_timeout(permits, left).unwrap().0;
            }
            *permits = permits.saturating_sub(1);
            drop(permits);
            gate.live.fetch_sub(1, SeqCst);
            gate.finished.fetch_add(1, SeqCst);
            gate.outcome.lock().unwrap().clone().unwrap()
        })
    }
    fn release(&self, n: usize) {
        *self.permits.lock().unwrap() += n;
        self.wake.notify_all();
    }
    fn set(&self, outcome: Outcome) {
        *self.outcome.lock().unwrap() = Some(outcome);
    }
}
fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn settle(h: &mut Home) {
    wait_until("discovery to settle", || {
        h.tick();
        h.kept.discovery.requested() && !h.kept.discovery.running() && h.kept.discovery.fresh()
    });
}
#[test]
fn places_keep_digit_arrow_alias_and_row_navigation() {
    for (index, (place, name)) in [
        (Place::Think, "think"),
        (Place::Tend, "tend"),
        (Place::Shape, "shape"),
        (Place::Approve, "approve"),
        (Place::Watch, "watch"),
        (Place::Taste, "taste"),
        (Place::Measure, "measure"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut direct = home(Arc::new(|_| found(vec![])));
        direct.open(place);
        let expected = screen(&mut direct);
        let heading = if name == "think" {
            "What's on your mind?"
        } else {
            name
        };
        assert!(expected.contains(heading), "{name}: {expected}");
        let mut digit = home(Arc::new(|_| found(vec![])));
        digit.handle_key(key(KeyCode::Char(char::from(b'1' + index as u8))));
        assert_eq!(screen(&mut digit), expected, "digit for {name}");
        for code in [
            KeyCode::Down,
            KeyCode::Char('j'),
            KeyCode::Up,
            KeyCode::Char('k'),
        ] {
            let steps = if matches!(code, KeyCode::Down | KeyCode::Char('j')) {
                PLACES.len() + index
            } else {
                PLACES.len() - index
            };
            let mut arrows = home(Arc::new(|_| found(vec![])));
            for _ in 0..steps {
                arrows.handle_key(key(code));
            }
            arrows.handle_key(key(KeyCode::Enter));
            assert_eq!(screen(&mut arrows), expected, "{code:?} for {name}");
        }
        let mut mouse = home(Arc::new(|_| found(vec![])));
        mouse.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 5,
                row: FIRST_ROW + index as u16 * ROW_GAP,
                modifiers: KeyModifiers::NONE,
            },
            Rect::new(0, 0, 100, 30),
        );
        assert_eq!(screen(&mut mouse), expected, "row for {name}");
    }
}

#[test]
fn watch_f10_returns_home_from_help_without_quitting() {
    for help in [true, false] {
        let mut h = home(Arc::new(|_| found(vec![])));
        h.open(Place::Watch);
        if help {
            h.handle_key(key(KeyCode::F(1)));
        }
        assert_eq!(screen(&mut h).contains("What Watch does"), help);
        h.handle_key(key(KeyCode::F(10)));
        assert!(
            at(&mut h) == "menu" && !h.help && !h.quit,
            "F10 from Watch (help={help}) must return Home without quitting"
        );
        h.handle_key(key(KeyCode::F(10)));
        assert!(h.quit, "F10 on Home still quits");
        for mut ignored in [
            key(KeyCode::F(10)),
            KeyEvent::new(KeyCode::F(10), KeyModifiers::ALT),
        ] {
            let mut h = home(Arc::new(|_| found(vec![])));
            h.open(Place::Watch);
            if help {
                h.handle_key(key(KeyCode::F(1)));
            }
            if ignored.modifiers == KeyModifiers::NONE {
                ignored.kind = KeyEventKind::Repeat;
            }
            h.handle_key(ignored);
            assert!(at(&mut h) == "watch" && !h.quit);
        }
    }
}

fn home(source: Source) -> Home {
    home_in(source, None)
}

fn home_in(source: Source, repo: Option<crate::repo::Repo>) -> Home {
    Home::new(Launch {
        repo,
        ..Launch::practice(DataRoots {
            sessions: Ok(PathBuf::from("/TEST/sessions")),
            tend: Err("TEST links unavailable".into()),
            ..DataRoots::from_home(None)
        })
    })
    .with_discovery_source(source)
}
fn screen(h: &mut Home) -> String {
    snapshot(100, 30, h, true).unwrap()
}

/// A Think visit standing in for one whose goal at `path` was saved and read back.
pub(super) fn think_saved(h: &mut Home, path: &Path) {
    h.clear();
    let mut dump = h.kept.think();
    dump.set_saved(crate::seed_me::Receipt::test(path.to_path_buf()));
    h.push(Box::new(dump), None);
}

/// Which screen shows, read from its title row: "menu", a place's name, or "think" for
/// Think's own full screen, which has no place list.
pub(super) fn at(h: &mut Home) -> String {
    let screen = snapshot(100, 30, h, true).unwrap();
    let row = screen.lines().nth(1).unwrap_or_default().trim();
    let title = row.split("  ").next().unwrap_or_default();
    if title == "tinkery"
        && screen
            .lines()
            .nth(4)
            .is_some_and(|row| row.starts_with("  1  think"))
    {
        "menu".into()
    } else if let Some(place) = title.strip_prefix("tinkery › ") {
        place.into()
    } else if title.starts_with("measure") {
        "measure".into()
    } else {
        "think".into()
    }
}

/// Whether Tend shows a saved goal's detail, read from its key bar.
pub(super) fn in_detail(h: &mut Home) -> bool {
    screen(h)
        .lines()
        .last()
        .is_some_and(|bar| bar.contains("history   esc back   F10 menu"))
}

/// Tend's selected-goal line under the garden: the goal's first line, then its stage.
pub(super) fn selected_line(h: &mut Home) -> String {
    let frame = screen(h);
    frame
        .lines()
        .nth(27)
        .unwrap_or_default()
        .trim_start()
        .to_string()
}

/// Tend's "How it works" help, as words, leaving help closed again.
pub(super) fn how_it_works(h: &mut Home) -> String {
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::Char('h')));
    let text = screen(h).split_whitespace().collect::<Vec<_>>().join(" ");
    h.handle_key(key(KeyCode::F(1)));
    text
}

#[test]
fn blocked_discovery_still_paints_six_places_and_handles_queued_input() {
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    let mut h = home(gate.source());
    h.tick();
    wait_until("the worker to start", || gate.entered.load(SeqCst) == 1);
    for _ in 0..50 {
        let frame = screen(&mut h);
        for name in ["think", "tend", "approve", "watch", "taste", "measure"] {
            assert!(frame.contains(name), "{frame}");
        }
        assert!(!frame.contains("Reading") && !frame.contains("No saved"));
    }
    assert_eq!(
        h.count(Place::Tend),
        None,
        "unknown is not shown as a number or zero"
    );
    assert_eq!(gate.entered.load(SeqCst), 1, "painting never reads sources");
    // Queued navigation and quit, handled while the source is still blocked.
    for code in [
        KeyCode::Char('j'),
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Char('k'),
    ] {
        h.handle_key(key(code));
        h.tick();
    }
    h.handle_key(key(KeyCode::Char('2')));
    h.tick();
    assert!(at(&mut h) == "tend");
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::Char('q')));
    assert!(h.quit, "quit is handled before discovery finishes");
    assert_eq!(
        gate.finished.load(SeqCst),
        0,
        "the source never returned first"
    );
    gate.release(8);
    wait_until("late landing", || {
        h.tick();
        gate.finished.load(SeqCst) >= 1 && !h.kept.discovery.running()
    });
    assert!(gate.peak.load(SeqCst) <= 1, "one worker at a time");
}

#[test]
fn many_requests_while_blocked_stay_bounded_to_one_worker() {
    let gate = Gate::new(found(vec![]));
    let mut h = home(gate.source());
    h.tick();
    wait_until("start", || gate.entered.load(SeqCst) == 1);
    for _ in 0..20 {
        h.open(Place::Tend);
        h.handle_key(key(KeyCode::Esc));
        h.open(Place::Think);
        h.handle_key(key(KeyCode::F(10)));
        h.tick();
    }
    assert_eq!(gate.live.load(SeqCst), 1);
    gate.release(100);
    settle(&mut h);
    assert_eq!(gate.peak.load(SeqCst), 1);
    assert!(
        gate.entered.load(SeqCst) <= 2,
        "requests coalesce into one follow-up"
    );
}

#[test]
fn tend_opened_during_discovery_is_responsive_and_never_says_no_sessions() {
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    let mut h = home(gate.source());
    h.tick();
    h.handle_key(key(KeyCode::Char('2')));
    let frame = screen(&mut h);
    assert!(
        frame.contains("Reading your sessions. Esc goes back."),
        "{frame}"
    );
    assert!(!frame.contains("No saved sessions"), "{frame}");
    assert!(frame.to_lowercase().contains("esc back"));
    assert!(
        !frame.contains("move") && !frame.contains("enter open"),
        "the key bar offers only what is possible now:\n{frame}"
    );
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Enter));
    assert!(!in_detail(&mut h), "nothing to open yet");
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::F(10)));
    assert!(at(&mut h) == "menu", "F10 leaves tend");
    assert!(!h.quit);
    h.handle_key(key(KeyCode::Char('2')));
    h.handle_key(key(KeyCode::Esc));
    assert!(at(&mut h) == "menu");
    let selected = h.selected;
    gate.release(8);
    settle(&mut h);
    assert!(at(&mut h) == "menu", "a late result never reopens tend");
    assert_eq!(h.selected, selected, "or moves the menu selection");
    assert_eq!(
        h.count(Place::Tend),
        Some("1".into()),
        "the count appears once complete"
    );
}

#[test]
fn a_slow_read_changes_one_quiet_line_and_starts_nothing() {
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    let mut h = home(gate.source());
    h.tick();
    wait_until("the worker to start", || gate.entered.load(SeqCst) == 1);
    h.handle_key(key(KeyCode::Char('2')));
    // The clock is an input of the real tick, so no test sleeps: the read's start plus a wait.
    let since = h.kept.discovery.running_since().expect("a read is pending");
    let threshold = tend::STILL_READING_AFTER;
    h.tick_at(since + threshold - Duration::from_millis(1));
    let before = screen(&mut h);
    assert!(
        before.contains("Reading your sessions. Esc goes back."),
        "{before}"
    );
    assert!(!before.contains("Still reading"), "{before}");
    h.tick_at(since + threshold);
    let after = screen(&mut h);
    assert!(
        after.contains("Still reading your sessions. Esc goes back."),
        "{after}"
    );
    assert!(!after.contains("Reading your sessions"), "{after}");
    // Only that line changed: no spinner, no loading screen.
    assert_eq!(before.lines().count(), after.lines().count());
    let changed = before.lines().zip(after.lines()).filter(|(a, b)| a != b);
    assert_eq!(changed.count(), 1, "{before}\n{after}");
    // Nothing started, and the view is as live as before.
    assert_eq!(gate.entered.load(SeqCst), 1, "no replacement worker");
    assert_eq!(gate.live.load(SeqCst), 1);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Enter));
    assert!(!in_detail(&mut h), "nothing to open yet");
    h.handle_key(key(KeyCode::Esc));
    assert!(at(&mut h) == "menu", "Esc still goes back");
    // The wait belongs to the read, not the visit: reopening meets the same slow read.
    h.open(Place::Tend);
    h.tick_at(since + threshold);
    let reopened = screen(&mut h);
    assert!(
        reopened.contains("Still reading your sessions."),
        "{reopened}"
    );
    assert_eq!(gate.entered.load(SeqCst), 1, "still one worker");
    // When the read lands the line is gone, however late the clock says it is.
    gate.release(8);
    settle(&mut h);
    h.tick_at(since + threshold * 10);
    let landed = screen(&mut h);
    assert!(landed.contains("goal a"), "{landed}");
    assert!(!landed.to_lowercase().contains("reading"), "{landed}");
}

#[test]
fn a_slow_read_behind_a_saved_session_says_so_too() {
    let gate = Gate::new(found(vec![]));
    let mut h = home(gate.source());
    h.tick();
    wait_until("the worker to start", || gate.entered.load(SeqCst) == 1);
    h.open_tend_session(Path::new("/TEST/new"));
    let since = h.kept.discovery.running_since().expect("a read is pending");
    h.tick_at(since + tend::STILL_READING_AFTER - Duration::from_millis(1));
    assert!(screen(&mut h).contains("Reading your sessions. Esc goes back."));
    h.tick_at(since + tend::STILL_READING_AFTER);
    let frame = screen(&mut h);
    assert!(frame.contains("Still reading your sessions."), "{frame}");
    assert!(!frame.contains("unavailable"), "{frame}");
    gate.set(found(vec![sess(
        "new",
        Stage::GoalSaved,
        "2026-10-10T10:00:00Z",
    )]));
    gate.release(8);
    settle(&mut h);
    assert!(in_detail(&mut h) && !screen(&mut h).contains("unavailable"));
}

#[test]
fn a_slow_reread_leaves_a_list_already_on_screen_alone() {
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    gate.release(1);
    let mut h = home(gate.source());
    settle(&mut h);
    // Reopening reads again, and this read gets no permit: the last result shows meanwhile.
    h.open(Place::Tend);
    wait_until("the second read", || gate.entered.load(SeqCst) == 2);
    let since = h
        .kept
        .discovery
        .running_since()
        .expect("the second read is running");
    let before = screen(&mut h);
    h.tick_at(since + tend::STILL_READING_AFTER * 10);
    let after = screen(&mut h);
    assert!(after.contains("goal a"), "{after}");
    assert!(!after.to_lowercase().contains("reading"), "{after}");
    assert_eq!(before, after, "a slow re-read changes nothing on screen");
    gate.release(8);
    settle(&mut h);
}

#[test]
fn a_slow_reread_behind_an_empty_list_never_says_no_sessions() {
    let gate = Gate::new(found(vec![]));
    gate.release(1);
    let mut h = home(gate.source());
    settle(&mut h);
    // A goal was saved since the empty read; the read that would contain it is slow.
    gate.set(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    h.open(Place::Tend);
    let frame = screen(&mut h);
    assert!(
        frame.contains("Reading your sessions. Esc goes back."),
        "{frame}"
    );
    wait_until("the second read", || gate.entered.load(SeqCst) == 2);
    let since = h
        .kept
        .discovery
        .running_since()
        .expect("the second read is running");
    h.tick_at(since + tend::STILL_READING_AFTER * 10);
    let frame = screen(&mut h);
    assert!(
        frame.contains("Still reading your sessions. Esc goes back."),
        "{frame}"
    );
    assert!(!frame.contains("No saved sessions"), "{frame}");
    assert!(!frame.contains("enter open"), "{frame}");
    gate.release(8);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(frame.contains("goal a"), "{frame}");
    assert!(
        frame.contains("↑↓←→ move   enter open   l link   esc back"),
        "{frame}"
    );
}

#[test]
fn late_results_keep_the_selected_session_by_identity_and_do_not_replace_detail() {
    let old = [
        sess("a", Stage::GoalSaved, "2026-10-09T10:00:00Z"),
        sess("b", Stage::GoalSaved, "2026-10-08T10:00:00Z"),
        sess("c", Stage::GoalSaved, "2026-10-07T10:00:00Z"),
    ];
    let gate = Gate::new(found(old.to_vec()));
    gate.release(1);
    let mut h = home(gate.source());
    settle(&mut h);
    h.open(Place::Tend);
    gate.release(1);
    settle(&mut h);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Down));
    assert!(selected_line(&mut h).starts_with("goal c "));
    // A newer session appears at the top; the reader's selection does not move to it.
    let mut newer = vec![sess("n", Stage::GoalSaved, "2026-10-10T10:00:00Z")];
    newer.extend(old.clone());
    gate.set(found(newer.clone()));
    h.open(Place::Tend);
    gate.release(1);
    settle(&mut h);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Down));
    let line = selected_line(&mut h);
    let chosen = PathBuf::from(format!("/TEST/{}", &line[5..6]));
    // Open detail, then let a read land that edits that session's goal and drops another.
    h.handle_key(key(KeyCode::Enter));
    let mut edited = newer.clone();
    edited.iter_mut().find(|s| s.path == chosen).unwrap().goal = "EDITED elsewhere".into();
    edited.remove(0);
    gate.set(found(edited));
    h.kept
        .discovery
        .request(h.kept.sessions_root.as_deref().unwrap());
    gate.release(2);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(in_detail(&mut h));
    assert!(frame.contains(&line[..6]), "{frame}");
    assert!(!frame.contains("EDITED elsewhere"), "frozen while read");
    h.handle_key(key(KeyCode::Esc));
    h.tick();
    assert!(
        selected_line(&mut h).starts_with("EDITED elsewhere "),
        "selection follows identity"
    );
}

#[test]
fn shape_return_waits_for_fresh_discovery_and_cannot_reopen_exited_detail() {
    for leave_detail in [false, true] {
        let gate = Gate::new(found(vec![sess("a", Stage::GoalSaved, "")]));
        gate.release(1);
        let mut h = home(gate.source());
        h.open(Place::Tend);
        settle(&mut h);
        h.handle_key(key(KeyCode::Enter));
        h.handle_key(key(KeyCode::Char('s')));
        assert!(at(&mut h) == "shape");
        // A read already in flight cannot satisfy the return's later request.
        h.kept.request_sessions();
        h.handle_key(key(KeyCode::Esc));
        gate.set(found(vec![sess("a", Stage::SeedDrafted, "")]));
        let version = h.kept.discovery.version();
        gate.release(1);
        wait_until("older read to land", || {
            h.tick();
            h.kept.discovery.version() > version
        });
        // Still waiting on the goal it asked for: the older read was not applied.
        assert!(in_detail(&mut h));
        assert!(screen(&mut h).contains("Reading your sessions."));
        if leave_detail {
            h.handle_key(key(KeyCode::Esc));
        }
        gate.set(found(vec![sess("a", Stage::SeedUnchecked, "")]));
        gate.release(1);
        settle(&mut h);
        assert_eq!(
            in_detail(&mut h),
            !leave_detail,
            "a late read reopened detail"
        );
        let frame = screen(&mut h);
        assert!(!frame.contains("Reading your sessions."), "{frame}");
        if leave_detail {
            assert!(selected_line(&mut h).contains(Stage::SeedUnchecked.label()));
        } else {
            assert!(
                frame.contains(Stage::SeedUnchecked.next().unwrap()),
                "{frame}"
            );
        }
        assert_eq!(gate.peak.load(SeqCst), 1);
    }
}

#[test]
fn landed_discovery_does_not_cancel_or_redirect_link_picking() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-refresh-")
        .tempdir()
        .unwrap();
    let sessions = vec![
        sess("a", Stage::GoalSaved, ""),
        sess("b", Stage::GoalSaved, ""),
    ];
    let gate = Gate::new(found(sessions.clone()));
    gate.release(4);
    let mut h = home(gate.source());
    h.kept.link_root = Ok(tmp.path().join("TEST-links"));
    h.open(Place::Tend);
    settle(&mut h);
    h.handle_key(key(KeyCode::Char('l')));
    h.handle_key(key(KeyCode::Right));
    let mut newer = sessions;
    newer.reverse();
    gate.set(found(newer));
    h.kept
        .discovery
        .request(h.kept.sessions_root.as_deref().unwrap());
    settle(&mut h);
    assert!(screen(&mut h).contains("esc cancel"), "still picking");
    assert!(screen(&mut h).contains("goal a needs goal b"));
    h.handle_key(key(KeyCode::Enter));
    h.tick();
    assert!(!screen(&mut h).contains("esc cancel"));
    assert!(selected_line(&mut h).starts_with("goal a "));
    let links = tend::needs::Store::new(Ok(tmp.path().join("TEST-links"))).read();
    assert!(links.into_links().unwrap().contains("a", "b"));
}

#[test]
fn receipt_detail_waits_for_a_read_that_can_contain_the_session() {
    let target = PathBuf::from("/TEST/new");
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    let mut h = home(gate.source());
    h.tick();
    wait_until("first read started", || gate.entered.load(SeqCst) == 1);
    h.open_tend_session(&target);
    assert!(screen(&mut h).contains("Reading your sessions"));
    // The read that began before the save lands without it: still waiting, not "unavailable".
    gate.release(1);
    wait_until("first read to land", || {
        h.tick();
        gate.finished.load(SeqCst) == 1 && gate.entered.load(SeqCst) == 2
    });
    let frame = screen(&mut h);
    assert!(frame.contains("Reading your sessions"), "{frame}");
    assert!(!frame.contains("unavailable"), "{frame}");
    gate.set(found(vec![
        sess("new", Stage::GoalSaved, "2026-10-10T10:00:00Z"),
        sess("a", Stage::GoalSaved, "2026-10-09T10:00:00Z"),
    ]));
    gate.release(1);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(
        in_detail(&mut h) && !frame.contains("unavailable"),
        "{frame}"
    );
    assert!(
        frame.contains("goal new") && !frame.contains("goal a"),
        "{frame}"
    );
}

#[test]
fn receipt_detail_for_a_session_a_complete_read_lacks_is_explicit() {
    let gate = Gate::new(found(vec![]));
    gate.release(4);
    let mut h = home(gate.source());
    h.open_tend_session(Path::new("/TEST/missing"));
    settle(&mut h);
    assert!(screen(&mut h).contains("Saved session unavailable. F1 then h for details."));
    assert!(how_it_works(&mut h).contains("Saved session is unavailable: /TEST/missing"));
    assert!(!in_detail(&mut h));
}

#[test]
fn a_late_receipt_target_does_not_resurrect_an_exited_visit() {
    let gate = Gate::new(found(vec![sess(
        "a",
        Stage::GoalSaved,
        "2026-10-09T10:00:00Z",
    )]));
    let mut h = home(gate.source());
    h.open_tend_session(Path::new("/TEST/a"));
    h.handle_key(key(KeyCode::Esc));
    assert!(at(&mut h) == "menu");
    h.handle_key(key(KeyCode::Char('4')));
    h.handle_key(key(KeyCode::Esc));
    gate.release(4);
    settle(&mut h);
    assert!(at(&mut h) == "menu");
    h.open(Place::Tend);
    let frame = screen(&mut h);
    assert!(
        !in_detail(&mut h) && frame.contains("goal a") && !frame.contains("Reading"),
        "the old target is gone: {frame}"
    );
}

#[test]
fn counts_are_exact_only_after_complete_current_reads() {
    let sessions = vec![
        sess("a", Stage::GoalSaved, "2026-10-09T10:00:00Z"),
        sess("b", Stage::SeedUnchecked, "2026-10-08T10:00:00Z"),
        sess("c", Stage::Completed, "2026-10-07T10:00:00Z"),
    ];
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-count-notes-")
        .tempdir()
        .unwrap();
    let gate = Gate::new(found(sessions.clone()));
    gate.release(1);
    let mut h = home(gate.source());
    h.kept.link_root = Ok(tmp.path().join("TEST-links"));
    assert_eq!(h.count(Place::Tend), None, "not known before any read");
    settle(&mut h);
    assert_eq!(
        h.count(Place::Tend),
        Some("2".into()),
        "ended sessions are not active"
    );
    // Partial evidence: valid records are kept but no number is claimed.
    gate.set(Ok(Found {
        sessions: sessions.clone(),
        unreadable: 1,
        failures: Vec::new(),
    }));
    gate.release(1);
    h.open(Place::Tend);
    settle(&mut h);
    assert_eq!(
        h.count(Place::Tend),
        None,
        "incomplete is not an exact result"
    );
    let frame = screen(&mut h);
    assert!(
        frame.contains("1 saved record couldn't be read; this view may be missing sessions."),
        "{frame}"
    );
    assert!(
        frame.contains("goal a") && frame.contains("goal b"),
        "{frame}"
    );
    // A failed root is not "no sessions" and not a count.
    gate.set(Err("Cannot read /TEST: boom".into()));
    gate.release(1);
    h.open(Place::Tend);
    settle(&mut h);
    assert_eq!(h.count(Place::Tend), None);
    assert!(!screen(&mut h).contains("No saved sessions"));
    // Complete and empty is the only way to "no sessions".
    gate.set(found(vec![]));
    gate.release(1);
    h.open(Place::Tend);
    settle(&mut h);
    assert!(screen(&mut h).contains("No saved sessions yet"));
    assert_eq!(h.count(Place::Tend), None, "zero is never printed");
}

#[test]
fn only_unreadable_records_never_read_as_no_sessions() {
    let gate = Gate::new(Ok(Found {
        sessions: vec![],
        unreadable: 2,
        failures: Vec::new(),
    }));
    gate.release(2);
    let mut h = home(gate.source());
    h.open(Place::Tend);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(
        frame.contains("2 saved records couldn't be read"),
        "{frame}"
    );
    assert!(!frame.contains("No saved sessions"), "{frame}");
}

#[test]
fn returning_from_think_and_reopening_tend_read_again_and_hide_a_stale_count() {
    let tmp = tempfile::tempdir().unwrap();
    write_session(tmp.path(), "first", "2026-10-09T10:00:00Z");
    let mut h = Home::new(Launch::practice(DataRoots {
        sessions: Ok(tmp.path().to_path_buf()),
        tend: Ok(tmp.path().join("TEST-links")),
        ..DataRoots::from_home(None)
    }));
    settle(&mut h);
    assert_eq!(h.count(Place::Tend), Some("1".into()));
    h.open(Place::Think);
    write_session(tmp.path(), "saved-in-think", "2026-10-10T10:00:00Z");
    h.handle_key(key(KeyCode::Esc));
    h.tick();
    assert!(at(&mut h) == "menu");
    assert_eq!(
        h.count(Place::Tend),
        None,
        "the old number is not shown while rereading"
    );
    settle(&mut h);
    assert_eq!(
        h.count(Place::Tend),
        Some("2".into()),
        "count refreshed after saving"
    );
    write_session(tmp.path(), "later", "2026-10-11T10:00:00Z");
    h.open(Place::Tend);
    settle(&mut h);
    assert_eq!(
        h.count(Place::Tend),
        Some("3".into()),
        "deliberate reopening reads again"
    );
    let frame = screen(&mut h);
    assert!(frame.contains("goal later"), "{frame}");
}

fn write_session(root: &Path, id: &str, updated: &str) {
    let dir = root.join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("ledger.json"),
        json!({"status": "active", "goal": format!("goal {id}"), "updated_at": updated})
            .to_string(),
    )
    .unwrap();
}

#[test]
fn scan_reads_thousands_once_each_newest_first_with_exact_active_count() {
    let tmp = tempfile::tempdir().unwrap();
    let total = 3000;
    for i in 0..total {
        let dir = tmp.path().join(format!("s{i:05}"));
        fs::create_dir_all(&dir).unwrap();
        let status = if i % 5 == 0 { "completed" } else { "active" };
        fs::write(
            dir.join("ledger.json"),
            json!({"status": status, "goal": format!("goal {i}"),
                   "updated_at": format!("2026-10-{:02}T{:02}:{:02}:{:02}Z", 1 + i / 1440 % 28, i / 60 % 24, i % 60, 0)})
            .to_string(),
        )
        .unwrap();
    }
    let found = scan(tmp.path()).unwrap();
    assert!(found.complete());
    assert_eq!(found.sessions.len(), total);
    let unique: HashSet<_> = found.sessions.iter().map(|s| &s.path).collect();
    assert_eq!(unique.len(), total, "every session exactly once");
    let times: Vec<_> = found
        .sessions
        .iter()
        .map(|s| tend::timestamp(&s.updated).unwrap())
        .collect();
    assert!(times.windows(2).all(|w| w[0] >= w[1]), "newest first");
    let mut h = Home::new(Launch::practice(DataRoots {
        sessions: Ok(tmp.path().to_path_buf()),
        tend: Ok(tmp.path().join("TEST-links")),
        ..DataRoots::from_home(None)
    }));
    settle(&mut h);
    assert_eq!(h.count(Place::Tend), Some((total - total / 5).to_string()));
}

#[test]
fn equal_timestamps_have_path_order_independent_of_creation_order() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-sort-")
        .tempdir()
        .unwrap();
    for name in ["z", "a", "m"] {
        write_session(tmp.path(), name, "2026-10-09T10:00:00Z");
    }
    let found = scan(tmp.path()).unwrap();
    assert_eq!(
        found
            .sessions
            .iter()
            .map(|s| s.goal.as_str())
            .collect::<Vec<_>>(),
        ["goal a", "goal m", "goal z"]
    );
}

#[test]
fn scan_completeness_semantics() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Missing root: nothing has been saved yet. Complete and empty.
    assert_eq!(scan(&root.join("missing")), Ok(Found::default()));
    // Root that cannot be listed: failed, never empty.
    fs::write(root.join("a-file"), "x").unwrap();
    assert!(scan(&root.join("a-file")).is_err());
    // Mixed root: valid records survive; non-sessions are ignored; unreadable ones are counted.
    let sessions = root.join("sessions");
    write_session(&sessions, "good", "2026-10-09T10:00:00Z");
    fs::create_dir_all(sessions.join("empty-dir")).unwrap();
    fs::write(sessions.join("stray-file"), "x").unwrap();
    fs::create_dir_all(sessions.join("bad-json")).unwrap();
    fs::write(sessions.join("bad-json/ledger.json"), "not json").unwrap();
    fs::create_dir_all(sessions.join("no-status")).unwrap();
    fs::write(sessions.join("no-status/ledger.json"), "{}").unwrap();
    fs::create_dir_all(sessions.join("ledger-is-dir/ledger.json")).unwrap();
    fs::create_dir_all(sessions.join("bad-utf8")).unwrap();
    fs::write(sessions.join("bad-utf8/ledger.json"), [0xff, 0xfe]).unwrap();
    let found = scan(&sessions).unwrap();
    assert_eq!(found.sessions.len(), 1);
    assert_eq!(found.sessions[0].goal, "goal good");
    assert_eq!(
        found.unreadable, 4,
        "bad-json, no-status, ledger-is-dir, bad-utf8"
    );
    assert!(!found.complete());
    // Only genuinely absent records leave a root complete.
    let clean = root.join("clean");
    write_session(&clean, "x", "2026-10-09T10:00:00Z");
    fs::create_dir_all(clean.join("not-a-session")).unwrap();
    assert!(scan(&clean).unwrap().complete());
}

#[test]
fn unavailable_session_root_never_scans_or_claims_an_empty_list() {
    let reason = "HOME is unavailable; pass --seed-session-root PATH";
    let mut h = Home::new(Launch::practice(DataRoots {
        sessions: Err(reason.into()),
        tend: Err("TEST links unavailable".into()),
        ..DataRoots::from_home(None)
    }))
    .with_discovery_source(Arc::new(|_| panic!("No root must never reach a source")));
    h.tick();
    h.discover_for_snapshot(Duration::from_secs(1));
    assert!(!h.kept.discovery.running());
    assert_eq!(h.count(Place::Tend), None);
    for _ in 0..2 {
        h.open(Place::Tend);
        h.tick();
        assert!(how_it_works(&mut h).contains(reason));
        let frame = snapshot(80, 24, &mut h, false).unwrap();
        assert!(frame.contains("unavailable"), "{frame}");
        assert!(!frame.contains("No saved sessions"), "{frame}");
        assert!(!h.kept.discovery.running());
    }
}

#[test]
fn links_destination_survives_menu_reopening_and_standalone_receipt_handoff() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-wiring-")
        .tempdir()
        .unwrap();
    let sessions = tmp.path().join("TEST-sessions");
    write_session(&sessions, "goal", "2026-10-09T10:00:00Z");
    let links = tmp.path().join("TEST-links");
    fs::create_dir_all(&links).unwrap();
    let bytes = b"TEST deliberately unreadable; navigation must leave it alone";
    fs::write(links.join("needs.json"), bytes).unwrap();
    for receipt in [false, true] {
        let mut h = Home::new(Launch::practice(DataRoots {
            sessions: Ok(sessions.clone()),
            tend: Ok(links.clone()),
            ..DataRoots::from_home(None)
        }));
        if receipt {
            h.open_tend_session(&sessions.join("goal"));
        }
        for _ in 0..2 {
            h.open(Place::Tend);
            settle(&mut h);
            assert!(selected_line(&mut h).starts_with("goal goal "));
            assert!(
                how_it_works(&mut h).contains("Cannot read links: needs.json is not valid JSON.")
            );
            assert_eq!(fs::read(links.join("needs.json")).unwrap(), bytes);
            h.handle_key(key(KeyCode::Esc));
        }
    }
}

#[test]
fn unavailable_linking_does_not_stop_tend_reading_sessions() {
    let reason = "TEST linking unavailable; pass --link-root PATH";
    let mut h = home(Arc::new(|_| {
        found(vec![sess("goal", Stage::GoalSaved, "")])
    }));
    h.kept.link_root = Err(reason.into());
    h.open(Place::Tend);
    settle(&mut h);
    assert!(selected_line(&mut h).starts_with("goal goal "));
    assert!(!screen(&mut h).contains("unavailable"));
    assert!(how_it_works(&mut h).contains(reason));
}

#[test]
fn the_menu_names_the_repository_once_and_says_nothing_outside_one() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("TEST-repo-here");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    let mut h = home_in(
        Arc::new(|_| found(vec![])),
        crate::repo::Repo::enclosing(&root),
    );
    let menu = screen(&mut h);
    let title = menu.lines().nth(1).unwrap();
    assert!(title.contains("tinkery  in TEST-repo-here"), "{title}");
    assert_eq!(menu.matches("TEST-repo-here").count(), 1, "{menu}");
    let mut plain = home(Arc::new(|_| found(vec![])));
    assert!(!screen(&mut plain).contains(" in "));
}

#[test]
fn a_standalone_agent_launch_keeps_its_shape_hold_after_saving_a_goal() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-receipt-launch-")
        .tempdir()
        .unwrap();
    let sessions = tmp.path().join("TEST-sessions");
    let human = crate::confirmation::GoalConfirmation::terminal(Some("human")).unwrap();
    let goal = "TEST Keep my words.\nWhy: TEST.\nDone when: TEST the next question shows.";
    let session =
        crate::shape::command::new_goal(sessions.clone(), goal, "TEST origin", &human).unwrap();
    let roots = DataRoots {
        sessions: Ok(sessions.clone()),
        tend: Ok(tmp.path().join("TEST-links")),
        ..DataRoots::from_home(None)
    };
    for (agent, held) in [(true, true), (false, false)] {
        let confirmation =
            crate::confirmation::GoalConfirmation::terminal(agent.then_some("agent")).unwrap();
        let launch = Launch {
            agent,
            declared: agent,
            confirmation,
            ..Launch::practice(roots.clone())
        };
        let mut h = Home::alone(launch);
        think_saved(&mut h, &session);
        h.handle_key(key(KeyCode::Esc));
        h.tick();
        assert_eq!(at(&mut h), "menu");
        h.open(Place::Shape);
        settle(&mut h);
        h.handle_key(key(KeyCode::Enter));
        let mut text = String::new();
        wait_until("the session read", || {
            h.tick();
            text = screen(&mut h)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            !text.contains("Checking this Tink.")
        });
        assert_eq!(
            text.contains("An agent answers Shape with --shape-pi. Shape only reads"),
            held,
            "{text}"
        );
    }
}
