use super::*;

fn key(app: &mut BrainDump, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn thought_capture_refuses_invalid_boundaries_and_keeps_the_original() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-thought-")
        .tempdir()
        .unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let repo = crate::repo::Repo::enclosing(root.path());
    for (words, store, origin, expected) in [
        (
            "".to_string(),
            Ok(root.path().join("empty")),
            repo.clone(),
            "Write a thought",
        ),
        (
            "x".repeat(131073),
            Ok(root.path().join("large")),
            repo.clone(),
            "Write a thought",
        ),
        (
            "TEST original".into(),
            Err("TEST unsafe storage".into()),
            repo.clone(),
            "TEST unsafe storage",
        ),
        (
            "TEST original".into(),
            Ok(root.path().join("no-repo")),
            None,
            "Start Tinkery in a repository",
        ),
    ] {
        let mut app = BrainDump::default().with_thoughts(store, origin);
        app.paste(&words);
        snapshot(100, 30, &mut app, true).unwrap();
        key(&mut app, KeyCode::F(3));
        let screen = snapshot(100, 30, &mut app, true).unwrap();
        assert!(screen.contains(expected), "{screen}");
        assert_eq!(*app.exit(), Exit::Stay);
        assert!(app.saved().is_none());
        if !words.is_empty() {
            app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
            app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
            assert_eq!(app.take_copy_request().as_deref(), Some(words.as_str()));
        }
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn submitting_a_visit_removes_thought_capture_authority() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-thought-")
        .tempdir()
        .unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let store = root.path().join("thoughts");
    let mut app = BrainDump::default()
        .with_thoughts(Ok(store.clone()), crate::repo::Repo::enclosing(root.path()));
    app.paste("I want a small page to keep my project notes because I forget them.");
    snapshot(100, 30, &mut app, true).unwrap();
    key(&mut app, KeyCode::F(2));
    key(&mut app, KeyCode::F(3));
    for _ in 0..100 {
        app.tick();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    app.paste("TEST answer must not become a thought");
    key(&mut app, KeyCode::F(3));
    assert!(!store.exists());
    assert_eq!(*app.exit(), Exit::Stay);
    assert!(app.saved().is_none());
}

#[test]
fn fresh_input_names_both_actions_and_ctrl_s_keeps_exact_words_without_sending() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-thought-")
        .tempdir()
        .unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let store = root.path().join("thoughts");
    let mut app = BrainDump::default()
        .with_thoughts(Ok(store.clone()), crate::repo::Repo::enclosing(root.path()));
    for (w, h) in [(80, 24), (100, 30)] {
        let screen = snapshot(w, h, &mut app, true).unwrap();
        let bar = screen.lines().last().unwrap().trim();
        assert_eq!(
            bar, "enter clarify goal   ^S keep thought   ^J newline   esc menu",
            "{screen}"
        );
        assert!(
            screen
                .lines()
                .nth(1)
                .unwrap()
                .contains("tinkery · tinkery-TEST"),
            "{screen}"
        );
        assert!(
            !screen.contains("F3") && !screen.contains("no model call"),
            "{screen}"
        );
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    app.paste("TEST kept\r\nwords");
    assert_eq!(app.input.text, "\n\nTEST kept\r\nwords");
    assert!(
        app.conversation.sources().is_empty(),
        "a new line or paste sent"
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert_eq!(
        *app.exit(),
        Exit::Garden {
            words: "\n\nTEST kept\r\nwords".into(),
            duplicate: false
        }
    );
    assert!(app.conversation.sources().is_empty());
    assert_eq!(std::fs::read_dir(&store).unwrap().count(), 1);
}

#[test]
fn enter_sends_like_f2_and_ctrl_s_after_a_send_says_why_nothing_was_kept() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-thought-")
        .tempdir()
        .unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let store = root.path().join("thoughts");
    let mut app = BrainDump::default()
        .with_thoughts(Ok(store.clone()), crate::repo::Repo::enclosing(root.path()));
    app.paste("I want a small page to keep my project notes because I forget them.");
    snapshot(100, 30, &mut app, true).unwrap();
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.conversation.sources().len(), 1, "Enter did not send");
    for _ in 0..100 {
        app.tick();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    let screen = snapshot(100, 30, &mut app, true).unwrap();
    assert!(
        screen.contains("Nothing kept. Keep thought works only before the"),
        "{screen}"
    );
    assert!(!store.exists());
    assert_eq!(*app.exit(), Exit::Stay);
}
