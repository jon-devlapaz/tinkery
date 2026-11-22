//! Tend's garden, detail and linking keys and presses, driven through its place surface: the
//! keys and presses the menu forwards, and a landed read. Nothing here starts an agent.
use super::*;
use crossterm::event::MouseButton;
use ratatui::{Terminal, backend::TestBackend};
use std::path::Path;

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

/// Tend as the menu holds it: keys and presses in, and whether it asked to leave.
struct H {
    tend: Tend,
    left: bool,
}

impl H {
    fn handle_key(&mut self, key: KeyEvent) {
        if matches!(self.tend.key(key), Next::Menu) {
            self.left = true;
        }
    }
    fn handle_mouse_at(&mut self, event: MouseEvent, area: Rect, now: Instant) {
        let body = Rect::new(2, 3, area.width - 4, area.height - 4);
        self.tend.pointer(event, body, now);
    }
}

fn screen_at(h: &mut H, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            h.tend.render(
                frame,
                Rect::new(2, 3, width - 4, height - 4),
                Palette::new(true),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn screen(h: &mut H) -> String {
    screen_at(h, 100, 30)
}

fn bare() -> Tend {
    Tend::new(
        Err("TEST links unavailable".into()),
        None,
        Err("TEST no records".into()),
        Box::new(|| Err("TEST no connections".into())),
    )
}

fn garden(count: usize) -> H {
    let mut tend = bare();
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
    H { tend, left: false }
}

fn selected(h: &H) -> usize {
    h.tend.selected
}

fn press(h: &mut H, column: u16, row: u16, now: Instant) {
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

fn goal_point(h: &mut H, name: &str) -> (u16, u16) {
    let frame = screen(h);
    let (row, line) = frame
        .lines()
        .enumerate()
        .skip(3)
        .take(23)
        .find(|(_, line)| line.contains(name))
        .unwrap_or_else(|| panic!("{name}: {frame}"));
    let col = line[..line.find(name).unwrap()].chars().count();
    ((col + name.chars().count() / 2) as u16, row as u16)
}

fn press_goal(h: &mut H, name: &str, now: Instant) {
    let (x, y) = goal_point(h, name);
    press(h, x, y, now);
}

#[test]
fn ended_goals_never_enter_garden_selection_keys_mouse_or_detail() {
    let stages = [
        Stage::GoalSaved,
        Stage::SeedDrafted,
        Stage::SeedUnchecked,
        Stage::Unknown,
        Stage::Simulated,
    ];
    let mut h = garden(0);
    let tend = &mut h.tend;
    let sessions = stages
        .iter()
        .enumerate()
        .flat_map(|(i, &stage)| {
            [
                sess(&format!("ended-{i}"), Stage::Completed, "unknown"),
                sess(&i.to_string(), stage, "unknown"),
            ]
        })
        .collect();
    tend.apply(&found(sessions), true, 1);
    assert_eq!(
        tend.sessions.iter().map(|s| s.stage).collect::<Vec<_>>(),
        stages
    );
    assert_eq!(tend.ended, 5);
    let frame = screen(&mut h);
    assert!(frame.contains("5 ended not shown"), "{frame}");
    assert!(!frame.contains("goal ended"), "{frame}");
    for code in [
        KeyCode::Right,
        KeyCode::Char('j'),
        KeyCode::Left,
        KeyCode::Char('k'),
        KeyCode::Down,
        KeyCode::Up,
    ] {
        for _ in 0..7 {
            h.handle_key(key(code));
            let tend = &h.tend;
            assert_ne!(tend.current().unwrap().stage, Stage::Completed);
        }
    }
    let now = Instant::now();
    for (index, &stage) in stages.iter().enumerate() {
        press_goal(
            &mut h,
            &format!("goal {index}"),
            now + Duration::from_secs(index as u64),
        );
        assert_eq!(selected(&h), index);
        h.handle_key(key(KeyCode::Enter));
        let tend = &h.tend;
        assert!(tend.detail);
        assert_eq!(tend.current().unwrap().stage, stage);
        h.handle_key(key(KeyCode::Esc));
    }
}

#[test]
fn all_ended_is_not_empty_and_incomplete_evidence_stays_visible() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-ended-notes-")
        .tempdir()
        .unwrap();
    let mut h = garden(0);
    let tend = &mut h.tend;
    tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
    for count in [1, 2] {
        for unreadable in [0, 1, 2] {
            let tend = &mut h.tend;
            tend.apply(
                &Ok(Found {
                    sessions: (0..count)
                        .map(|i| sess(&format!("ended-{i}"), Stage::Completed, "unknown"))
                        .collect(),
                    unreadable,
                    failures: Vec::new(),
                }),
                true,
                1,
            );
            for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
                let frame = screen_at(&mut h, width, height);
                let text = frame.split_whitespace().collect::<Vec<_>>().join(" ");
                if unreadable == 0 {
                    assert!(text.contains(&format!("No active goals. {count} ended not shown. Confirm a goal in think to start one.")), "{frame}");
                } else {
                    assert!(!text.contains("No active goals"), "{frame}");
                    let records = if unreadable == 1 { "record" } else { "records" };
                    assert!(
                        text.contains(&format!("{unreadable} saved {records} couldn't be read")),
                        "{frame}"
                    );
                    assert_eq!(text.matches("couldn't be read").count(), 1, "{frame}");
                }
                assert!(!text.contains("No saved sessions"), "{frame}");
                assert!(
                    text.contains(&format!("{count} ended not shown")),
                    "{frame}"
                );
                assert_eq!(text.matches("ended not shown").count(), 1);
                assert_eq!(frame.lines().last().unwrap().trim(), "esc back");
                for code in [
                    KeyCode::Right,
                    KeyCode::Down,
                    KeyCode::Char('j'),
                    KeyCode::Enter,
                ] {
                    h.handle_key(key(code));
                }
                press(&mut h, 3, 3, Instant::now());
                let tend = &h.tend;
                assert!(tend.current().is_none() && !tend.detail);
            }
        }
    }
    let tend = &mut h.tend;
    tend.apply(&found(vec![]), true, 2);
    let frame = screen(&mut h);
    assert!(frame.contains("No saved sessions yet. Confirm a goal in think to start one."));
    assert!(!frame.contains("ended not shown"));
}

#[test]
fn wanted_session_is_matched_before_filtering_and_selected_by_path_after_it() {
    for stage in [Stage::GoalSaved, Stage::Completed] {
        let mut tend = bare();
        tend.want = Some(("/TEST/alias".into(), Some("/TEST/target".into())));
        tend.apply(
            &found(vec![
                sess("ended", Stage::Completed, "unknown"),
                sess("target", stage, "unknown"),
            ]),
            false,
            1,
        );
        assert!(tend.want.is_none() && !tend.pending && tend.error.is_none());
        assert_eq!(tend.detail, stage != Stage::Completed);
        if stage != Stage::Completed {
            assert_eq!(tend.selected, 0);
            assert_eq!(tend.current().unwrap().path, Path::new("/TEST/target"));
        } else {
            assert!(tend.current().is_none());
        }
    }
}

#[test]
fn garden_keys_step_one_goal_in_newest_order_and_clamp_after_resize() {
    let mut h = garden(23);
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 1, "the pre-render stride defaults to one");
    screen(&mut h);
    for (code, expected) in [
        (KeyCode::Right, 2),
        (KeyCode::Char('j'), 3),
        (KeyCode::Down, 4),
        (KeyCode::Up, 3),
        (KeyCode::Up, 2),
        (KeyCode::Left, 1),
        (KeyCode::Char('k'), 0),
        (KeyCode::Left, 0),
        (KeyCode::Left, 0),
    ] {
        h.handle_key(key(code));
        assert_eq!(selected(&h), expected);
    }
    screen_at(&mut h, 80, 24);
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 1, "resize never changes selection order");
    for _ in 0..30 {
        h.handle_key(key(KeyCode::Right));
    }
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Enter));
    let tend = &h.tend;
    assert!(tend.detail);
    h.handle_key(key(KeyCode::Down));
    h.handle_key(key(KeyCode::Right));
    let tend = &h.tend;
    assert_eq!(tend.scroll, 1);
    assert_eq!(tend.selected, 22);
    h.handle_key(key(KeyCode::Esc));
    let tend = &h.tend;
    assert!(!tend.detail && tend.scroll == 0);
    h.handle_key(key(KeyCode::F(1)));
    assert!(
        screen(&mut h)
            .contains("Arrows / j k move · Enter or a double click opens the selected original.")
    );
    h.handle_key(key(KeyCode::Down));
    assert_eq!(selected(&h), 22);
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::Esc));
    assert!(h.left);
    let mut h = garden(1);
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::F(10)));
    assert!(h.left);
}

#[test]
fn linking_routes_keys_and_single_click_and_help_guards_hidden_actions() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-input-")
        .tempdir()
        .unwrap();
    let root = tmp.path().join("TEST-links");
    let mut h = garden(7);
    let tend = &mut h.tend;
    tend.needs = needs::Store::new(Ok(root.clone()));
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        screen_at(&mut h, width, height);
        h.handle_key(key(KeyCode::Char('l')));
        let frame = screen_at(&mut h, width, height);
        assert!(
            frame.contains("↑↓←→ pick   enter link   esc cancel"),
            "{frame}"
        );
        h.handle_key(key(KeyCode::Enter)); // self-pick
        h.handle_key(key(KeyCode::Right));
        let frame = screen_at(&mut h, width, height);
        assert!(
            frame.contains("› goal 0") && frame.contains("→ goal 1"),
            "{frame}"
        );
        assert!(frame.contains("goal 0 needs goal 1"), "{frame}");
        h.handle_key(key(KeyCode::F(1)));
        for code in [KeyCode::Char('l'), KeyCode::Enter, KeyCode::Right] {
            h.handle_key(key(code));
        }
        assert_eq!(selected(&h), 1);
        h.handle_key(key(KeyCode::Esc));
        h.handle_key(key(KeyCode::Esc));
        assert_eq!(selected(&h), 0);
        assert!(!root.exists());
    }
    screen(&mut h);
    h.handle_key(key(KeyCode::Char('l')));
    press_goal(&mut h, "goal 1", Instant::now());
    let tend = &h.tend;
    assert!(!tend.is_linking() && !tend.detail);
    assert_eq!(selected(&h), 0);
    assert!(tend.needs.read().into_links().unwrap().contains("0", "1"));
    h.handle_key(key(KeyCode::Char('l')));
    h.handle_key(key(KeyCode::Char('j')));
    assert!(screen(&mut h).contains("enter unlink"));
    h.handle_key(key(KeyCode::Enter));
    assert_eq!(selected(&h), 0);
    assert!(screen(&mut h).contains("l link"));
    let tend = &h.tend;
    assert!(!tend.needs.read().into_links().unwrap().contains("0", "1"));
}

#[test]
fn link_key_is_ignored_outside_garden_and_repeated_presses_do_not_start_linking() {
    for state in [
        "detail",
        "help",
        "how",
        "reading",
        "wanted",
        "unavailable",
        "empty",
        "repeat",
    ] {
        let mut h = garden(2);
        if matches!(state, "help" | "how") {
            h.handle_key(key(KeyCode::F(1)));
            if state == "how" {
                h.handle_key(key(KeyCode::Char('h')));
            }
        }
        let tend = &mut h.tend;
        match state {
            "detail" => tend.detail = true,
            "reading" => tend.pending = true,
            "wanted" => tend.want = Some(("/TEST/wanted".into(), None)),
            "unavailable" => tend.error = Some("TEST unavailable".into()),
            "empty" => tend.sessions.clear(),
            _ => {}
        }
        let mut event = key(KeyCode::Char('l'));
        if state == "repeat" {
            event.kind = KeyEventKind::Repeat;
        }
        h.handle_key(event);
        let tend = &h.tend;
        assert!(!tend.is_linking() && tend.notice.is_empty(), "{state}");
    }
}

#[test]
fn garden_press_selects_and_only_a_timely_same_session_press_opens() {
    let now = Instant::now();
    for (elapsed, opens) in [(499, true), (500, true), (501, false)] {
        let mut h = garden(7);
        screen(&mut h);
        // Hit the ring through its displayed label.
        press_goal(&mut h, "goal 1", now);
        assert_eq!(selected(&h), 1);
        press_goal(&mut h, "goal 1", now + Duration::from_millis(elapsed));
        let tend = &h.tend;
        assert_eq!(tend.detail, opens);
    }
    let mut h = garden(7);
    screen(&mut h);
    press_goal(&mut h, "goal 1", now);
    press_goal(&mut h, "goal 2", now + Duration::from_millis(100));
    let tend = &h.tend;
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
    press_goal(&mut h, "goal 1", now);
    let tend = &mut h.tend;
    let mut sessions = tend.sessions.clone();
    sessions.insert(0, sess("new", Stage::GoalSaved, "unknown"));
    tend.apply(&found(sessions), true, 2);
    assert_eq!(tend.selected, 2);
    screen(&mut h);
    press_goal(&mut h, "goal 0", now + Duration::from_millis(100));
    let tend = &h.tend;
    assert!(!tend.detail && tend.current().unwrap().path == Path::new("/TEST/0"));
    // The same identity moving to another position still counts as a double press.
    let mut h = garden(7);
    screen(&mut h);
    press_goal(&mut h, "goal 1", now);
    let tend = &mut h.tend;
    let mut sessions = tend.sessions.clone();
    sessions.insert(0, sess("new", Stage::GoalSaved, "unknown"));
    tend.apply(&found(sessions), true, 2);
    screen(&mut h);
    press_goal(&mut h, "goal 1", now + Duration::from_millis(100));
    let tend = &h.tend;
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
        let tend = &mut h.tend;
        match state {
            "detail" => tend.detail = true,
            "reading" => tend.pending = true,
            "wanted" => tend.want = Some(("/TEST/wanted".into(), None)),
            "unavailable" => tend.error = Some("unavailable".into()),
            "empty" => tend.sessions.clear(),
            _ => {}
        }
        let detail = tend.detail;
        let row = if state == "garden" { 26 } else { 3 };
        press(&mut h, 0, row, now);
        press(&mut h, 0, row, now + Duration::from_millis(100));
        let tend = &h.tend;
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
    let tend = &mut h.tend;
    let mut sessions = tend.sessions.clone();
    for (session, goal) in sessions.iter_mut().zip([
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
    tend.apply(&found(sessions), true, tend.seen + 1);
    for (width, height) in [(100, 30), (80, 24)] {
        let screen = screen_at(&mut h, width, height);
        println!("TEST garden {width}×{height}:\n{screen}");
        assert_eq!(screen.split('\n').count(), height as usize);
        assert!(screen.contains("› Find invoices") && screen.contains("goal saved · time unknown"));
    }
}

#[test]
fn page_keys_move_a_screen_of_goal_detail_less_one_row() {
    let mut h = garden(1);
    let tend = &mut h.tend;
    tend.sessions[0].goal = (0..200).map(|i| format!("TEST line {i}\n")).collect();
    tend.detail = true;
    screen(&mut h);
    let tend = &h.tend;
    let page = tend.page;
    assert!(page > 5, "{page} rows");
    h.handle_key(key(KeyCode::PageDown));
    let tend = &h.tend;
    assert_eq!(tend.scroll, page - 1);
    h.handle_key(key(KeyCode::PageUp));
    let tend = &h.tend;
    assert_eq!(tend.scroll, 0);
}

#[test]
fn shifted_graph_view_keys_work_but_modified_actions_stay_blocked() {
    for ch in ['H', 'J', 'K', 'L', '+', '=', '-'] {
        let mut plain = garden(7);
        let mut shifted = garden(7);
        let before = screen(&mut shifted);
        screen(&mut plain);
        plain.handle_key(key(KeyCode::Char(ch)));
        shifted.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT));
        let after = screen(&mut shifted);
        assert_ne!(before, after, "{ch}");
        assert_eq!(screen(&mut plain), after, "{ch}");
        assert_eq!(selected(&shifted), 0);
    }
    let mut h = garden(7);
    let before = screen(&mut h);
    for modifiers in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    ] {
        for ch in ['H', 'J', 'K', 'L', '+', '=', '-', 'l', 'c'] {
            h.handle_key(KeyEvent::new(KeyCode::Char(ch), modifiers));
            assert_eq!(screen(&mut h), before);
        }
    }
    for code in [KeyCode::Char('l'), KeyCode::Enter, KeyCode::Down] {
        h.handle_key(KeyEvent::new(code, KeyModifiers::SHIFT));
        assert_eq!(screen(&mut h), before);
    }
}

#[test]
fn graph_selection_keys_reveal_the_goal_after_panning_it_offscreen() {
    let mut h = garden(7);
    screen(&mut h);
    for _ in 0..80 {
        h.handle_key(key(KeyCode::Char('H')));
    }
    assert!(!screen(&mut h).contains("› goal 0"));
    h.handle_key(key(KeyCode::Up)); // clamped at newest, but reveal it again
    assert!(screen(&mut h).contains("› goal 0"));
    for _ in 0..80 {
        h.handle_key(key(KeyCode::Char('H')));
    }
    h.handle_key(key(KeyCode::Down));
    assert!(screen(&mut h).contains("› goal 1"));
}

#[test]
fn a_toggles_only_tend_garden_and_never_repeats_or_conflicts_with_shape() {
    let mut h = garden(2);
    h.tend.repo_paths = Some([h.tend.sessions[0].path.clone()].into_iter().collect());
    h.tend.toggle_scope(); // all
    h.tend.toggle_scope(); // this repo
    assert_eq!(h.tend.sessions.len(), 1);
    h.handle_key(KeyEvent::new_with_kind(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Repeat,
    ));
    assert_eq!(h.tend.sessions.len(), 1);
    assert!(matches!(h.tend.key(key(KeyCode::Char('s'))), Next::Stay));
    h.handle_key(key(KeyCode::F(1)));
    h.handle_key(key(KeyCode::Char('a')));
    h.handle_key(key(KeyCode::Esc));
    assert_eq!(h.tend.sessions.len(), 1, "help ignores a");
    h.handle_key(key(KeyCode::Char('a')));
    assert_eq!(h.tend.sessions.len(), 2);
    h.handle_key(key(KeyCode::Enter));
    h.handle_key(key(KeyCode::Char('a')));
    assert!(h.tend.detail && h.tend.sessions.len() == 2);
    h.handle_key(key(KeyCode::Esc));
    h.handle_key(key(KeyCode::Char('a')));
    assert_eq!(h.tend.sessions.len(), 1);
    assert!(!h.left);
}

#[test]
fn navigation_keys_never_ask_for_find_connections_and_only_f_does() {
    let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = asked.clone();
    let mut tend = Tend::new(
        Err("TEST links unavailable".into()),
        Some(Path::new("/TEST/repo")),
        Err("TEST no records".into()),
        Box::new(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err("TEST connections refused".into())
        }),
    );
    tend.apply(&found(vec![sess("a", Stage::GoalSaved, "")]), true, 1);
    tend.current = true;
    tend.repo_paths = Some([PathBuf::from("/TEST/a")].into_iter().collect());
    let mut h = H { tend, left: false };
    for code in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Char('a'),
        KeyCode::Char('a'),
        KeyCode::F(1),
        KeyCode::Char('h'),
        KeyCode::Esc,
        KeyCode::Esc,
        KeyCode::Char('l'),
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Char('r'),
        KeyCode::Esc,
        KeyCode::Esc,
    ] {
        h.handle_key(key(code));
    }
    assert_eq!(asked.load(std::sync::atomic::Ordering::SeqCst), 0);
    h.handle_key(key(KeyCode::F(1)));
    assert!(
        matches!(h.tend.key(key(KeyCode::Char('t'))), Next::Stay),
        "help hides t"
    );
    h.handle_key(key(KeyCode::F(1)));
    assert!(matches!(
        h.tend.key(key(KeyCode::Char('t'))),
        Next::Think(None)
    ));
    h.handle_key(key(KeyCode::Char('f')));
    assert_eq!(asked.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(screen(&mut h).contains("TEST connections refused"));
}
