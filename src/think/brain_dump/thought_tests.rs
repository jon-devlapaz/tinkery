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
