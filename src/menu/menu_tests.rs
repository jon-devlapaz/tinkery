//! Menu and tend never wait on session discovery. The fake source is held open, so a
//! regression that reads inside paint, tick or input fails instead of passing quietly.
use super::*;
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
        h.discovery.requested() && !h.discovery.running() && h.discovery.fresh()
    });
}
fn home(source: Source) -> Home {
    Home::new(
        Box::new(BrainDump::default),
        PathBuf::from("/TEST/sessions"),
        PathBuf::from("/TEST/no-substrate"),
    )
    .with_discovery_source(source)
}
fn screen(h: &mut Home) -> String {
    snapshot(100, 30, h, true).unwrap()
}

fn garden(count: usize) -> Home {
    let mut h = home(Arc::new(|_| panic!("input and painting never read")))
        .with_seed_me("/TEST/SKILL.md".into());
    let mut tend = Tend::new();
    tend.apply(
        &found(
            (0..count)
                .map(|i| sess(&i.to_string(), Stage::GoalSaved, "unknown"))
                .collect(),
        ),
        true,
        1,
    );
    tend.current = true;
    h.view = View::Tend(tend);
    h
}
fn selected(h: &Home) -> usize {
    let View::Tend(tend) = &h.view else { panic!() };
    tend.selected
}
fn press(h: &mut Home, column: u16, row: u16, now: Instant) {
    h.handle_mouse_at(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, 100, 30),
        now,
    );
}

#[test]
fn garden_keys_step_clamp_and_use_the_last_rendered_columns() {
    let mut h = garden(23);
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 1, "the pre-render stride defaults to one");
    screen(&mut h);
    for (code, expected) in [
        (KeyCode::Right, 2),
        (KeyCode::Char('j'), 3),
        (KeyCode::Down, 7),
        (KeyCode::Up, 3),
        (KeyCode::Up, 3),
        (KeyCode::Left, 2),
        (KeyCode::Char('k'), 1),
        (KeyCode::Left, 0),
        (KeyCode::Left, 0),
    ] {
        h.handle_key(key(code));
        assert_eq!(selected(&h), expected);
    }
    snapshot(80, 24, &mut h, true).unwrap();
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 3, "resize changes the stride after render");
    for _ in 0..30 {
        h.handle_key(key(KeyCode::Right));
    }
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Char('c')));
    assert!(h.take_copy_request().is_none(), "garden cannot copy");
    h.handle_key(key(KeyCode::Enter));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(tend.detail);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Right));
    let View::Tend(tend) = &h.view else { panic!() };
    assert_eq!(tend.scroll, 1);
    assert_eq!(tend.selected, 22);
    h.handle_key(key(KeyCode::Char('c')));
    assert_eq!(
        h.take_copy_request().unwrap(),
        crate::seed_me::continuation_prompt(Path::new("/TEST/SKILL.md"), Path::new("/TEST/22"))
    );
    h.handle_key(key(KeyCode::Esc));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(!tend.detail && tend.scroll == 0);
    h.handle_key(key(KeyCode::F(1)));
    assert!(
        screen(&mut h)
            .contains("Arrows / j k move · Enter or a double click opens the saved goal.")
    );
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::Esc));
    assert!(matches!(h.view, View::Menu) && !h.quit);
    let mut h = garden(1);
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::F(10)));
    assert!(matches!(h.view, View::Menu) && !h.help && !h.quit);
}

#[test]
fn garden_press_selects_and_only_a_timely_same_session_press_opens() {
    let now = Instant::now();
    for (elapsed, opens) in [(499, true), (500, true), (501, false)] {
        let mut h = garden(7);
        screen(&mut h);
        // Second cell at 100 columns, including its two ring rows.
        press(&mut h, 24, 4, now);
        assert_eq!(selected(&h), 1);
        press(&mut h, 24, 5, now + Duration::from_millis(elapsed));
        let View::Tend(tend) = &h.view else { panic!() };
        assert_eq!(tend.detail, opens);
    }
    let mut h = garden(7);
    screen(&mut h);
    press(&mut h, 24, 3, now);
    press(&mut h, 45, 3, now + Duration::from_millis(100));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(
        !tend.detail && tend.selected == 2,
        "different goals aren't a double click"
    );
}

#[test]
fn late_read_cannot_redirect_a_double_press_to_a_different_session() {
    let now = Instant::now();
    let mut h = garden(7);
    screen(&mut h);
    press(&mut h, 24, 3, now);
    let View::Tend(tend) = &mut h.view else {
        panic!()
    };
    let mut sessions = tend.sessions.clone();
    sessions.insert(0, sess("new", Stage::GoalSaved, "unknown"));
    tend.apply(&found(sessions), true, 2);
    assert_eq!(tend.selected, 2);
    screen(&mut h);
    press(&mut h, 24, 3, now + Duration::from_millis(100));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(!tend.detail && tend.current().unwrap().path == Path::new("/TEST/0"));
    // The same identity moving to another cell still counts as a double press.
    let mut h = garden(7);
    screen(&mut h);
    press(&mut h, 24, 3, now);
    let View::Tend(tend) = &mut h.view else {
        panic!()
    };
    let mut sessions = tend.sessions.clone();
    sessions.insert(0, sess("new", Stage::GoalSaved, "unknown"));
    tend.apply(&found(sessions), true, 2);
    screen(&mut h);
    press(&mut h, 45, 3, now + Duration::from_millis(100));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(tend.detail && tend.current().unwrap().path == Path::new("/TEST/1"));
}

#[test]
fn garden_presses_ignore_blank_rows_help_detail_and_discovery_states() {
    let now = Instant::now();
    for state in [
        "garden",
        "help",
        "detail",
        "reading",
        "wanted",
        "unavailable",
        "empty",
    ] {
        let mut h = garden(7);
        screen(&mut h);
        if state == "help" {
            h.handle_key(key(KeyCode::F(1)));
        }
        let View::Tend(tend) = &mut h.view else {
            panic!()
        };
        match state {
            "detail" => tend.detail = true,
            "reading" => tend.pending = true,
            "wanted" => tend.want = Some(("/TEST/wanted".into(), None)),
            "unavailable" => tend.error = Some("unavailable".into()),
            "empty" => tend.sessions.clear(),
            _ => {}
        }
        let detail = tend.detail;
        let row = if state == "garden" { 6 } else { 3 };
        press(&mut h, 24, row, now);
        press(&mut h, 24, row, now + Duration::from_millis(100));
        let View::Tend(tend) = &h.view else { panic!() };
        assert_eq!(tend.selected, 0, "{state}");
        assert_eq!(tend.detail, detail, "{state}");
    }
    let mut h = garden(7);
    for (column, row) in [(0, 3), (99, 3), (2, 26), (2, 27), (2, 28), (2, 29)] {
        press(&mut h, column, row, now);
    }
    assert_eq!(selected(&h), 0);
}

#[test]
fn garden_test_session_plain_text_renders() {
    let mut h = garden(7);
    let View::Tend(tend) = &mut h.view else {
        panic!()
    };
    for (session, goal) in tend.sessions.iter_mut().zip([
        "Find invoices",
        "Read a calm theme",
        "Keep goals intact",
        "Plan the next seed",
        "Review saved work",
        "Make room to think",
        "Leave manual control",
    ]) {
        session.goal = goal.into();
    }
    for (width, height) in [(100, 30), (80, 24)] {
        let screen = snapshot(width, height, &mut h, true).unwrap();
        println!("TEST garden {width}×{height}:\n{screen}");
        assert_eq!(screen.split('\n').count(), height as usize);
        assert!(screen.contains("› Find invoices") && screen.contains("goal saved · time unknown"));
    }
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
        assert!(!frame.contains("Reading") && !frame.contains("No Seed Me"));
    }
    assert_eq!(h.count(1), None, "unknown is not shown as a number or zero");
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
    assert!(matches!(h.view, View::Tend(_)));
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
        gate.finished.load(SeqCst) >= 1 && !h.discovery.running()
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
        h.open(1);
        h.handle_key(key(KeyCode::Esc));
        h.open(0);
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
    assert!(!frame.contains("No Seed Me sessions"), "{frame}");
    assert!(frame.to_lowercase().contains("esc back"));
    assert!(
        !frame.contains("move") && !frame.contains("enter open"),
        "the key bar offers only what is possible now:\n{frame}"
    );
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Enter));
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(!tend.detail, "nothing to open yet");
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::F(10)));
    assert!(matches!(h.view, View::Menu), "F10 leaves tend");
    assert!(!h.quit);
    h.handle_key(key(KeyCode::Char('2')));
    h.handle_key(key(KeyCode::Esc));
    assert!(matches!(h.view, View::Menu));
    let selected = h.selected;
    gate.release(8);
    settle(&mut h);
    assert!(
        matches!(h.view, View::Menu),
        "a late result never reopens tend"
    );
    assert_eq!(h.selected, selected, "or moves the menu selection");
    assert_eq!(
        h.count(1),
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
    let since = h.discovery.running_since().expect("a read is pending");
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
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(!tend.detail, "nothing to open yet");
    h.handle_key(key(KeyCode::Esc));
    assert!(matches!(h.view, View::Menu), "Esc still goes back");
    // The wait belongs to the read, not the visit: reopening meets the same slow read.
    h.open(1);
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
    let since = h.discovery.running_since().expect("a read is pending");
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
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(tend.detail && tend.error.is_none());
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
    h.open(1);
    wait_until("the second read", || gate.entered.load(SeqCst) == 2);
    let since = h
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
    h.open(1);
    let frame = screen(&mut h);
    assert!(
        frame.contains("Reading your sessions. Esc goes back."),
        "{frame}"
    );
    wait_until("the second read", || gate.entered.load(SeqCst) == 2);
    let since = h
        .discovery
        .running_since()
        .expect("the second read is running");
    h.tick_at(since + tend::STILL_READING_AFTER * 10);
    let frame = screen(&mut h);
    assert!(
        frame.contains("Still reading your sessions. Esc goes back."),
        "{frame}"
    );
    assert!(!frame.contains("No Seed Me sessions"), "{frame}");
    assert!(!frame.contains("enter open"), "{frame}");
    gate.release(8);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(frame.contains("goal a"), "{frame}");
    assert!(
        frame.contains("↑↓←→ move   enter open   esc back"),
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
    h.open(1);
    gate.release(1);
    settle(&mut h);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Down));
    let View::Tend(tend) = &h.view else { panic!() };
    assert_eq!(tend.current().unwrap().path, PathBuf::from("/TEST/c"));
    // A newer session appears at the top; the reader's selection does not move to it.
    let mut newer = vec![sess("n", Stage::GoalSaved, "2026-10-10T10:00:00Z")];
    newer.extend(old.clone());
    gate.set(found(newer.clone()));
    h.open(1);
    gate.release(1);
    settle(&mut h);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Down));
    let View::Tend(tend) = &h.view else { panic!() };
    let chosen = tend.current().unwrap().path.clone();
    // Open detail, then let a read land that edits that session's goal and drops another.
    h.handle_key(key(KeyCode::Enter));
    let mut edited = newer.clone();
    edited.iter_mut().find(|s| s.path == chosen).unwrap().goal = "EDITED elsewhere".into();
    edited.remove(0);
    gate.set(found(edited));
    h.discovery.request(&h.sessions_root.clone());
    gate.release(2);
    settle(&mut h);
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(tend.detail);
    assert_eq!(tend.current().unwrap().path, chosen);
    assert_ne!(
        tend.current().unwrap().goal,
        "EDITED elsewhere",
        "frozen while read"
    );
    h.handle_key(key(KeyCode::Esc));
    h.tick();
    let View::Tend(tend) = &h.view else { panic!() };
    assert_eq!(
        tend.current().unwrap().path,
        chosen,
        "selection follows identity"
    );
    assert_eq!(tend.current().unwrap().goal, "EDITED elsewhere");
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
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(tend.detail && tend.error.is_none());
    assert_eq!(tend.current().unwrap().path, target);
    assert_eq!(tend.current().unwrap().goal, "goal new");
}

#[test]
fn receipt_detail_for_a_session_a_complete_read_lacks_is_explicit() {
    let gate = Gate::new(found(vec![]));
    gate.release(4);
    let mut h = home(gate.source());
    h.open_tend_session(Path::new("/TEST/missing"));
    settle(&mut h);
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(
        tend.error
            .as_ref()
            .unwrap()
            .contains("Saved session is unavailable")
    );
    assert!(tend.sessions.is_empty() && !tend.detail);
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
    assert!(matches!(h.view, View::Menu));
    h.handle_key(key(KeyCode::Char('4')));
    h.handle_key(key(KeyCode::Esc));
    gate.release(4);
    settle(&mut h);
    assert!(matches!(h.view, View::Menu));
    h.open(1);
    let View::Tend(tend) = &h.view else { panic!() };
    assert!(
        tend.want.is_none() && !tend.detail,
        "the old target is gone"
    );
}

#[test]
fn counts_are_exact_only_after_complete_current_reads() {
    let sessions = vec![
        sess("a", Stage::GoalSaved, "2026-10-09T10:00:00Z"),
        sess("b", Stage::SeedReady, "2026-10-08T10:00:00Z"),
        sess("c", Stage::Ended, "2026-10-07T10:00:00Z"),
    ];
    let gate = Gate::new(found(sessions.clone()));
    gate.release(1);
    let mut h = home(gate.source());
    assert_eq!(h.count(1), None, "not known before any read");
    settle(&mut h);
    assert_eq!(
        h.count(1),
        Some("2".into()),
        "ended sessions are not active"
    );
    // Partial evidence: valid records are kept but no number is claimed.
    gate.set(Ok(Found {
        sessions: sessions.clone(),
        unreadable: 1,
    }));
    gate.release(1);
    h.open(1);
    settle(&mut h);
    assert_eq!(h.count(1), None, "incomplete is not an exact result");
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
    h.open(1);
    settle(&mut h);
    assert_eq!(h.count(1), None);
    assert!(!screen(&mut h).contains("No Seed Me sessions"));
    // Complete and empty is the only way to "no sessions".
    gate.set(found(vec![]));
    gate.release(1);
    h.open(1);
    settle(&mut h);
    assert!(screen(&mut h).contains("No Seed Me sessions yet"));
    assert_eq!(h.count(1), None, "zero is never printed");
}

#[test]
fn only_unreadable_records_never_read_as_no_sessions() {
    let gate = Gate::new(Ok(Found {
        sessions: vec![],
        unreadable: 2,
    }));
    gate.release(2);
    let mut h = home(gate.source());
    h.open(1);
    settle(&mut h);
    let frame = screen(&mut h);
    assert!(
        frame.contains("2 saved records couldn't be read"),
        "{frame}"
    );
    assert!(!frame.contains("No Seed Me sessions"), "{frame}");
}

#[test]
fn returning_from_think_and_reopening_tend_read_again_and_hide_a_stale_count() {
    let tmp = tempfile::tempdir().unwrap();
    write_session(tmp.path(), "first", "2026-10-09T10:00:00Z");
    let mut h = Home::new(
        Box::new(BrainDump::default),
        tmp.path().to_path_buf(),
        tmp.path().join("no-substrate"),
    );
    settle(&mut h);
    assert_eq!(h.count(1), Some("1".into()));
    h.open(0);
    write_session(tmp.path(), "saved-in-think", "2026-10-10T10:00:00Z");
    if let View::Think(dump) = &mut h.view {
        dump.handle_key(key(KeyCode::F(10)));
    }
    h.tick();
    assert!(matches!(h.view, View::Menu));
    assert_eq!(
        h.count(1),
        None,
        "the old number is not shown while rereading"
    );
    settle(&mut h);
    assert_eq!(h.count(1), Some("2".into()), "count refreshed after saving");
    write_session(tmp.path(), "later", "2026-10-11T10:00:00Z");
    h.open(1);
    settle(&mut h);
    let View::Tend(tend) = &h.view else { panic!() };
    assert_eq!(tend.sessions.len(), 3, "deliberate reopening reads again");
    assert_eq!(tend.sessions[0].path.file_name().unwrap(), "later");
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
    let mut h = Home::new(
        Box::new(BrainDump::default),
        tmp.path().to_path_buf(),
        tmp.path().join("no-substrate"),
    );
    settle(&mut h);
    assert_eq!(h.count(1), Some((total - total / 5).to_string()));
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
fn menu_startup_and_navigation_never_run_the_measure_subprocess() {
    let tmp = tempfile::tempdir().unwrap();
    let substrate = tmp.path().join("substrate");
    fs::create_dir_all(substrate.join("tink_substrate")).unwrap();
    fs::write(substrate.join("tink_substrate/__init__.py"), "").unwrap();
    fs::write(
        substrate.join("tink_substrate/__main__.py"),
        "open('measure-ran', 'w').write('ran')\nprint('report')\n",
    )
    .unwrap();
    let mut h = Home::new(
        Box::new(BrainDump::default),
        tmp.path().join("sessions"),
        substrate.clone(),
    );
    settle(&mut h);
    for code in [
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Up,
    ] {
        h.handle_key(key(code));
        screen(&mut h);
    }
    h.open(1);
    settle(&mut h);
    h.handle_key(key(KeyCode::Esc));
    assert!(
        !substrate.join("measure-ran").exists(),
        "measure stays behind its action"
    );
    h.handle_key(key(KeyCode::Char('6')));
    assert!(
        substrate.join("measure-ran").exists(),
        "explicit opening still runs it"
    );
}
