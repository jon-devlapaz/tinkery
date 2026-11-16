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
    key(&mut app, KeyCode::Char('1'));
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
fn fresh_input_names_choose_and_ctrl_s_keeps_exact_words_without_sending() {
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
        assert_eq!(bar, "enter choose   ^J newline   esc menu", "{screen}");
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
    assert_eq!(*app.exit(), Exit::Stay);
    assert_eq!(
        app.take_thought_saved(),
        Some(("\n\nTEST kept\r\nwords".into(), false))
    );
    assert!(app.input.text.is_empty());
    assert!(app.conversation.sources().is_empty());
    assert_eq!(std::fs::read_dir(&store).unwrap().count(), 1);
}

#[test]
fn choice_sends_and_ctrl_s_after_a_send_says_why_nothing_was_kept() {
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
    key(&mut app, KeyCode::Char('1'));
    assert_eq!(app.conversation.sources().len(), 1, "Choice did not send");
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

#[test]
fn hidden_key_bar_keeps_the_choice_exit_visible() {
    let mut app = BrainDump::default().with_key_bar(false);
    snapshot(80, 24, &mut app, true).unwrap();
    app.paste("TEST words");
    key(&mut app, KeyCode::Enter);
    let choice = snapshot(80, 24, &mut app, true).unwrap();
    assert!(
        choice.lines().last().unwrap().contains("esc back"),
        "{choice}"
    );
    key(&mut app, KeyCode::Esc);
    assert!(
        snapshot(80, 24, &mut app, true)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .trim()
            .is_empty()
    );
}

#[test]
fn fresh_enter_and_choice_enter_never_send_without_a_digit() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Default)]
    struct Host(AtomicUsize);
    impl BoardHost for Host {
        fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Guess::decode(
                r#"{"parts":{"who":"I","outcome":"keep TEST notes","why":"remember","done_when":"find notes"}}"#,
                &r,
                &mut vec![],
            )
        }
    }
    for (w, h) in [(80, 24), (100, 30)] {
        for supported in [false, true] {
            let root = tempfile::Builder::new()
                .prefix("tinkery-TEST-choice-")
                .tempdir()
                .unwrap();
            std::fs::create_dir(root.path().join(".git")).unwrap();
            let store = root.path().join("thoughts");
            let host = Arc::new(Host::default());
            let mut app = BrainDump::with_host(host.clone())
                .with_shift_enter(supported)
                .with_thoughts(Ok(store.clone()), crate::repo::Repo::enclosing(root.path()));
            let initial = snapshot(w, h, &mut app, true).unwrap();
            assert!(
                initial.lines().last().unwrap().contains(if supported {
                    "shift+enter newline"
                } else {
                    "^J newline"
                }),
                "{initial}"
            );
            let words = "  TEST café 👩‍💻\r\nKeep\tthese.  ";
            app.paste(words);
            app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
            let exact = format!("{words}\n\n");
            key(&mut app, KeyCode::Enter);
            let choice = snapshot(w, h, &mut app, true).unwrap();
            assert!(
                choice.contains("1  Clarify into a goal · a model reads your words"),
                "{choice}"
            );
            assert!(
                choice.contains("2  Save as a thought · nothing reads them"),
                "{choice}"
            );
            assert!(!app.running(), "Fresh Enter started a request");
            assert!(app.conversation().sources().is_empty());
            for event in [
                KeyEvent::from(KeyCode::Enter),
                KeyEvent::from(KeyCode::Enter),
                KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Repeat),
                KeyEvent::new_with_kind(
                    KeyCode::Char('1'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                ),
                KeyEvent::new(KeyCode::Char('1'), KeyModifiers::ALT),
                KeyEvent::from(KeyCode::F(2)),
                KeyEvent::from(KeyCode::F(1)),
                KeyEvent::from(KeyCode::Char('x')),
            ] {
                app.handle_key(event);
            }
            app.paste("1\n2\r\n");
            super::tests::settle(&mut app);
            assert_eq!(
                host.0.load(Ordering::SeqCst),
                0,
                "Enter inside the choice started a model call"
            );
            assert!(
                snapshot(w, h, &mut app, true)
                    .unwrap()
                    .contains("1  Clarify into a goal")
            );
            key(&mut app, KeyCode::Esc);
            assert!(
                !snapshot(w, h, &mut app, true)
                    .unwrap()
                    .contains("1  Clarify into a goal")
            );
            app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
            app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
            assert_eq!(app.take_copy_request().as_deref(), Some(exact.as_str()));
            assert_eq!(host.0.load(Ordering::SeqCst), 0);
            key(&mut app, KeyCode::Enter);
            key(&mut app, KeyCode::Char('2'));
            assert_eq!(host.0.load(Ordering::SeqCst), 0);
            assert_eq!(app.exit(), &Exit::Stay);
            assert_eq!(app.take_thought_saved(), Some((exact.clone(), false)));
            let screen = snapshot(w, h, &mut app, true).unwrap();
            assert!(
                screen.contains("Thought saved · find it in Tend"),
                "{screen}"
            );
            assert!(!screen.contains("1  Clarify into a goal"), "{screen}");
            let saved: serde_json::Value = serde_json::from_slice(
                &std::fs::read(
                    std::fs::read_dir(store)
                        .unwrap()
                        .next()
                        .unwrap()
                        .unwrap()
                        .path(),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(saved["words"], exact);

            let mut app = BrainDump::with_host(host.clone());
            snapshot(w, h, &mut app, true).unwrap();
            app.paste(words);
            key(&mut app, KeyCode::Enter);
            let screen = snapshot(w, h, &mut app, true).unwrap();
            assert!(
                screen.contains("Unavailable: Open --home to save a thought in Tend."),
                "{screen}"
            );
            key(&mut app, KeyCode::Char('2'));
            assert_eq!(app.exit(), &Exit::Stay);
            assert_eq!(host.0.load(Ordering::SeqCst), 0);
            key(&mut app, KeyCode::Char('1'));
            super::tests::settle(&mut app);
            assert_eq!(host.0.load(Ordering::SeqCst), 1);
            assert_eq!(app.conversation().sources()[0].text, words);
            app.paste("TEST answer");
            key(&mut app, KeyCode::Enter);
            super::tests::settle(&mut app);
            assert_eq!(host.0.load(Ordering::SeqCst), 2);
            assert!(
                !snapshot(w, h, &mut app, true)
                    .unwrap()
                    .contains("1  Clarify into a goal")
            );
        }
    }
}

#[test]
fn a_save_result_leaves_when_new_words_arrive_but_not_on_navigation() {
    let root = tempfile::Builder::new()
        .prefix("tinkery-TEST-thought-")
        .tempdir()
        .unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let mut app = BrainDump::default().with_thoughts(
        Ok(root.path().join("thoughts")),
        crate::repo::Repo::enclosing(root.path()),
    );
    for new_words in ["typed", "pasted"] {
        app.paste(&format!("TEST saved words before {new_words}"));
        snapshot(100, 30, &mut app, true).unwrap();
        key(&mut app, KeyCode::Enter);
        key(&mut app, KeyCode::Char('2'));
        let shown = snapshot(100, 30, &mut app, true).unwrap();
        assert!(shown.contains("Thought saved · find it in Tend"), "{shown}");
        key(&mut app, KeyCode::Left);
        key(&mut app, KeyCode::Up);
        let shown = snapshot(100, 30, &mut app, true).unwrap();
        assert!(shown.contains("Thought saved · find it in Tend"), "{shown}");
        if new_words == "typed" {
            key(&mut app, KeyCode::Char('x'));
        } else {
            app.paste("TEST unsaved words");
        }
        let shown = snapshot(100, 30, &mut app, true).unwrap();
        assert!(!shown.contains("Thought saved"), "{new_words}: {shown}");
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        key(&mut app, KeyCode::Backspace);
    }
}
