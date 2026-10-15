//! Measure's menu boundary, help ownership and existing quit routes; no real ledger reads.

use super::*;

fn home(root: &Path) -> Home {
    let mut home = Home::new(
        Box::new(BrainDump::default),
        Ok(root.to_path_buf()),
        root.join("missing"),
        Ok(root.join("TEST-links")),
    );
    home.open(5);
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
fn failure_help_ignores_menu_shortcuts_and_releases() {
    let root = tempfile::tempdir().unwrap();
    let mut h = home(root.path());
    press(&mut h, KeyCode::F(1));
    let before = snapshot(100, 30, &mut h, false).unwrap();
    for code in [KeyCode::Char('1'), KeyCode::Char('6'), KeyCode::Enter] {
        press(&mut h, code);
        assert_eq!(snapshot(100, 30, &mut h, false).unwrap(), before);
    }
    let mut release = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    h.handle_key(release);
    assert_eq!(snapshot(100, 30, &mut h, false).unwrap(), before);
}
