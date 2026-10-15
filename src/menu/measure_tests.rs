//! Measure's menu boundary, help ownership and existing quit routes; no real ledger reads.

use super::*;

fn home(root: &Path) -> Home {
    let mut home = Home::new(
        Box::new(BrainDump::default),
        Ok(root.to_path_buf()),
        root.join("missing"),
        Ok(root.join("TEST-links")),
    );
    home.open(6);
    let watchdog = Instant::now() + Duration::from_secs(5);
    while matches!(&home.view, View::Measure(r) if !r.failed()) {
        assert!(Instant::now() < watchdog);
        home.tick();
        std::thread::yield_now();
    }
    home
}

fn press(home: &mut Home, code: KeyCode) {
    home.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn measure_help_steps_back_and_q_always_returns_to_a_closed_help_menu() {
    let root = tempfile::tempdir().unwrap();
    for layer in 0..3 {
        let mut h = home(root.path());
        if layer > 0 {
            press(&mut h, KeyCode::F(1));
        }
        if layer > 1 {
            press(&mut h, KeyCode::Char('h'));
        }
        press(&mut h, KeyCode::Char('q'));
        assert!(!h.quit && !h.help && matches!(h.view, View::Menu));
        assert!(
            snapshot(80, 24, &mut h, true)
                .unwrap()
                .contains("? help   q quit")
        );
    }
    let mut h = home(root.path());
    press(&mut h, KeyCode::Char('?'));
    press(&mut h, KeyCode::Char('h'));
    press(&mut h, KeyCode::Esc);
    assert!(
        snapshot(80, 24, &mut h, true)
            .unwrap()
            .contains("Right now")
    );
    press(&mut h, KeyCode::Esc);
    assert!(
        snapshot(80, 24, &mut h, true)
            .unwrap()
            .contains("Ledger report unavailable")
    );
    press(&mut h, KeyCode::Esc);
    assert!(!h.quit && !h.help && matches!(h.view, View::Menu));
}

#[test]
fn measure_keeps_f10_and_ctrl_c_quit_routes_in_every_help_layer() {
    let root = tempfile::tempdir().unwrap();
    for layer in 0..3 {
        for quit in [
            KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            let mut h = home(root.path());
            if layer > 0 {
                press(&mut h, KeyCode::Char('?'));
            }
            if layer > 1 {
                press(&mut h, KeyCode::Char('h'));
            }
            h.handle_key(quit);
            assert!(h.quit);
        }
    }
}

#[test]
fn blocked_measure_keeps_navigation_help_and_late_result_ownership() {
    let root = tempfile::tempdir().unwrap();
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for no_color in [false, true] {
            let mut h = Home::new(
                Box::new(BrainDump::default),
                Ok(root.path().join("TEST-sessions")),
                root.path().join("TEST-ledger"),
                Ok(root.path().join("TEST-links")),
            );
            let (reader, entered, release) = measure::Reader::blocked();
            h.measure = reader;
            h.open(6);
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(
                snapshot(width, height, &mut h, no_color)
                    .unwrap()
                    .contains("Reading ledger…")
            );
            h.open(6); // Same visit coalesces rather than cancelling/restarting.
            press(&mut h, KeyCode::Char('?'));
            assert!(
                snapshot(width, height, &mut h, no_color)
                    .unwrap()
                    .contains("Right now")
            );
            press(&mut h, KeyCode::Esc);
            press(&mut h, KeyCode::Esc);
            assert!(matches!(h.view, View::Menu));
            press(&mut h, KeyCode::Down);
            h.open(6);
            assert!(
                snapshot(width, height, &mut h, no_color)
                    .unwrap()
                    .contains("Finishing the previous read…")
            );
            assert!(entered.try_recv().is_err());
            release.send(()).unwrap();
            let watchdog = Instant::now() + Duration::from_secs(5);
            loop {
                h.tick();
                if snapshot(width, height, &mut h, no_color)
                    .unwrap()
                    .contains("Previous read stopped.")
                {
                    break;
                }
                assert!(Instant::now() < watchdog);
                std::thread::yield_now();
            }
            press(&mut h, KeyCode::Esc);
            h.tick();
            assert!(matches!(h.view, View::Menu));
            assert!(!root.path().join("TEST-sessions").exists());
        }
    }
}

#[test]
fn failure_help_ignores_menu_shortcuts_and_releases() {
    let root = tempfile::tempdir().unwrap();
    let mut h = home(root.path());
    press(&mut h, KeyCode::F(1));
    let before = snapshot(100, 30, &mut h, false).unwrap();
    for code in [KeyCode::Char('1'), KeyCode::Char('7'), KeyCode::Enter] {
        press(&mut h, code);
        assert_eq!(snapshot(100, 30, &mut h, false).unwrap(), before);
    }
    let mut release = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    h.handle_key(release);
    assert_eq!(snapshot(100, 30, &mut h, false).unwrap(), before);
}
