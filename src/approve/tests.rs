use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn seed(goal: &str) -> ReadySeed {
    ReadySeed {
        goal: goal.into(),
        revision: "r1".into(),
        acceptance: "## Acceptance criteria\n- TEST exact check  \n".into(),
        commands: Some("python3 _system/scripts/sdlc.py new <run>\nTEST intake".into()),
        confirmation_note: None,
    }
}

fn view() -> Approve {
    let mut view = Approve::open(None);
    view.waiting = false;
    view.found = Some(Ok(Found {
        seeds: vec![seed("TEST oldest"), seed("TEST newest")],
        ..Found::default()
    }));
    view
}

fn screen(view: &mut Approve, width: u16, height: u16, help: bool) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| view.render(f, f.area(), Palette::new(true), help))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn one_seed_at_a_time_and_copy_never_runs_a_command() {
    let mut view = view();
    let mut help = false;
    assert!(view.take_copy_request().is_none());
    let shown = screen(&mut view, 80, 24, false);
    assert!(shown.contains("TEST oldest") && !shown.contains("TEST newest"));
    view.key(KeyEvent::from(KeyCode::Char('c')), &mut help);
    assert_eq!(
        view.take_copy_request().unwrap(),
        view.seeds()[0].commands.clone().unwrap()
    );
    assert!(view.take_copy_request().is_none());
    view.copy_result(false);
    assert!(screen(&mut view, 80, 24, false).contains("Couldn't copy; try again."));
    view.key(KeyEvent::from(KeyCode::Char('n')), &mut help);
    assert!(screen(&mut view, 80, 24, false).contains("TEST newest"));
    view.key(KeyEvent::from(KeyCode::F(1)), &mut help);
    view.key(KeyEvent::from(KeyCode::Char('c')), &mut help);
    view.key(KeyEvent::from(KeyCode::Char('p')), &mut help);
    assert!(view.take_copy_request().is_none());
    assert_eq!(view.selected, 1);
    assert!(!view.key(KeyEvent::from(KeyCode::Esc), &mut help));
    assert!(!help);
    assert!(view.key(KeyEvent::from(KeyCode::Esc), &mut help));
}

#[test]
fn agent_confirmation_notice_keeps_copy_and_missing_checkout_guidance() {
    for (width, height) in [(80, 24), (100, 30)] {
        let mut view = view();
        let Some(Ok(found)) = &mut view.found else {
            panic!()
        };
        found.seeds[0].confirmation_note =
            Some("Agent-confirmed seed. Intake requires tink-sdlc 1.38.0 or newer.".into());
        let shown = screen(&mut view, width, height, false);
        assert!(shown.contains("Agent-confirmed seed."));
        assert!(
            shown
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("Copying runs nothing.")
        );
        assert!(shown.contains("c copy command"));
        view.key(KeyEvent::from(KeyCode::Char('c')), &mut false);
        assert_eq!(view.take_copy_request(), view.seeds()[0].commands);
        let Some(Ok(found)) = &mut view.found else {
            panic!()
        };
        found.seeds[0].commands = None;
        view.key(KeyEvent::from(KeyCode::End), &mut false);
        let shown = screen(&mut view, width, height, false);
        assert!(shown.contains("Agent-confirmed seed."));
        assert!(shown.contains("Cannot copy:"));
        view.key(KeyEvent::from(KeyCode::Char('c')), &mut false);
        assert!(view.take_copy_request().is_none());
    }
}

#[test]
fn long_readings_reach_the_tail_and_keep_exit_visible() {
    for (width, height) in [(80, 24), (100, 30)] {
        let mut view = view();
        let Some(Ok(found)) = &mut view.found else {
            panic!()
        };
        found.seeds[0].goal = (0..100).map(|i| format!("TEST goal row {i}\n")).collect();
        found.seeds[0].commands = None;
        let before = screen(&mut view, width, height, false);
        assert!(before.contains("TEST goal row 0"));
        assert!(!before.contains("Cannot copy:"));
        view.key(KeyEvent::from(KeyCode::End), &mut false);
        let tail = screen(&mut view, width, height, false);
        assert!(tail.contains("Cannot copy:") && tail.contains("esc back"));
        assert!(!tail.contains("c copy commands"));
        view.key(KeyEvent::from(KeyCode::Home), &mut false);
        assert!(screen(&mut view, width, height, false).contains("TEST goal row 0"));
    }
}

#[test]
fn unknown_and_incomplete_are_never_a_complete_empty_list() {
    let mut view = Approve::open(None);
    let shown = screen(&mut view, 80, 24, false);
    assert!(shown.contains("Reading sessions"));
    assert!(shown.contains("Can't tell which seeds have started; none are hidden."));
    view.found = Some(Ok(Found {
        unavailable: 2,
        reasons: vec!["TEST failure".into()],
        ..Found::default()
    }));
    let shown = screen(&mut view, 80, 24, false);
    assert!(
        shown.contains("may be incomplete") && shown.contains("No verified seeds could be listed")
    );
    assert!(!shown.contains("No verified seeds are waiting"));
    assert!(screen(&mut view, 80, 24, true).contains("TEST failure"));
}

#[test]
fn abandoned_read_cannot_publish_or_spawn_a_second_worker() {
    let (tx, rx) = mpsc::channel();
    let mut reader = Reader {
        running: Some((rx, Arc::new(AtomicBool::new(false)))),
    };
    reader.cancel();
    assert!(!reader.start(Vec::new(), None, Found::default()));
    tx.send(Found {
        seeds: vec![seed("TEST stale")],
        ..Found::default()
    })
    .unwrap();
    let mut reopened = Approve::open(None);
    reader.poll(Some(&mut reopened));
    assert!(reopened.found.is_none());
    assert!(reader.running.is_none());
}

#[test]
fn a_clipped_reading_says_where_the_hidden_command_lies() {
    for (width, height) in [(80, 24), (100, 30)] {
        let mut view = view();
        assert!(!screen(&mut view, width, height, false).contains("more"));
        let Some(Ok(found)) = &mut view.found else {
            panic!()
        };
        found.seeds[0].goal = (0..40).map(|i| format!("TEST goal row {i}\n")).collect();
        let top = screen(&mut view, width, height, false);
        assert!(
            top.contains("↓ more") && !top.contains("TEST intake"),
            "{top}"
        );
        view.key(KeyEvent::from(KeyCode::Down), &mut false);
        assert!(screen(&mut view, width, height, false).contains("↕ more"));
        view.key(KeyEvent::from(KeyCode::End), &mut false);
        let tail = screen(&mut view, width, height, false);
        assert!(
            tail.contains("↑ more") && tail.contains("TEST intake"),
            "{tail}"
        );
        assert!(tail.contains("esc back"), "{tail}");
    }
}
