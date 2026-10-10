use super::*;
use crate::voice::{Command, Event, Failure, VoiceHost};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Default)]
struct Fake {
    events: VecDeque<Event>,
    commands: Vec<Command>,
    closed: usize,
    restarted: usize,
}
impl VoiceHost for Rc<RefCell<Fake>> {
    fn send(&mut self, command: Command) -> Result<(), Failure> {
        self.borrow_mut().commands.push(command);
        Ok(())
    }
    fn poll(&mut self) -> Option<Event> {
        self.borrow_mut().events.pop_front()
    }
    fn shutdown(&mut self) {
        self.borrow_mut().closed += 1;
    }
    fn restart(&mut self) {
        self.borrow_mut().restarted += 1;
    }
}
fn attach(a: BrainDump) -> (BrainDump, Rc<RefCell<Fake>>) {
    let fake = Rc::new(RefCell::new(Fake::default()));
    fake.borrow_mut()
        .events
        .push_back(Event::Ready { version: 1 });
    let mut a = a.with_voice(Box::new(fake.clone()));
    a.tick();
    (a, fake)
}
fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn event(a: &mut BrainDump, fake: &Rc<RefCell<Fake>>, event: Event) {
    fake.borrow_mut().events.push_back(event);
    a.tick();
}
fn start(a: &mut BrainDump, fake: &Rc<RefCell<Fake>>) {
    key(a, KeyCode::F(6));
    event(a, fake, Event::Listening { take: 1 });
}

#[test]
fn partial_and_final_follow_current_cursor_not_a_captured_anchor() {
    let (mut a, fake) = attach(BrainDump::default());
    a.input = Note::new("A🙂Z");
    a.input.cursor = 1;
    start(&mut a, &fake);
    event(
        &mut a,
        &fake,
        Event::Partial {
            take: 1,
            text: "ghost".into(),
        },
    );
    assert_eq!(a.input.text, "A🙂Z");
    key(&mut a, KeyCode::Right);
    a.paste("Typed ");
    assert!(
        snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("Typed ghostZ")
    );
    let (_, preview) = voice_input::input_text(&a, Palette::new(false));
    assert_eq!(preview.to_string(), "A🙂Typed ghostZ");
    assert!(!a.paper().contains("ghost"));
    assert!(a.sources.is_empty());
    assert!(a.fragments.is_empty());
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "said".into(),
        },
    );
    assert_eq!(a.input.text, "A🙂Typed saidZ");
    assert_eq!(a.input.cursor, "A🙂Typed said".len());
    assert!(a.sources.is_empty());
    assert!(a.guess.is_none());
    assert!(!a.running());
}
#[test]
fn edit_delete_paste_and_replaced_partial_preserve_ordinary_text() {
    let (mut a, fake) = attach(BrainDump::default());
    a.input = Note::new("left right");
    a.input.cursor = 5;
    start(&mut a, &fake);
    event(
        &mut a,
        &fake,
        Event::Partial {
            take: 1,
            text: "old".into(),
        },
    );
    key(&mut a, KeyCode::Delete);
    key(&mut a, KeyCode::Backspace);
    a.paste(" ↔ ");
    event(
        &mut a,
        &fake,
        Event::Partial {
            take: 1,
            text: "new".into(),
        },
    );
    let text = snapshot(100, 30, &mut a, false).unwrap();
    assert!(text.contains("left ↔ newight"));
    assert!(!text.contains("old"));
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "final".into(),
        },
    );
    assert_eq!(a.input.text, "left ↔ finalight");
}
#[test]
fn actual_partial_cells_are_muted_and_final_cells_are_ink_in_color_and_no_color() {
    for no_color in [false, true] {
        for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let (mut a, fake) = attach(BrainDump::default());
            a.input = Note::new("ABCD");
            a.input.cursor = 2;
            start(&mut a, &fake);
            event(
                &mut a,
                &fake,
                Event::Partial {
                    take: 1,
                    text: "ghost".into(),
                },
            );
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            let p = Palette::new(no_color);
            terminal.draw(|f| render(f, &mut a, p)).unwrap();
            let b = terminal.backend().buffer();
            for x in 6..11 {
                assert_eq!(
                    b[(x, 8)].symbol(),
                    &"ghost"[(x - 6) as usize..(x - 5) as usize]
                );
                if no_color {
                    assert!(b[(x, 8)].modifier.contains(ratatui::style::Modifier::DIM));
                } else {
                    assert_eq!(b[(x, 8)].fg, p.muted.fg.unwrap());
                }
            }
            assert!(!b[(4, 8)].modifier.contains(ratatui::style::Modifier::DIM));
            assert!(!b[(11, 8)].modifier.contains(ratatui::style::Modifier::DIM));
            event(
                &mut a,
                &fake,
                Event::Final {
                    take: 1,
                    text: "ghost".into(),
                },
            );
            terminal.draw(|f| render(f, &mut a, p)).unwrap();
            for x in 6..11 {
                let cell = &terminal.backend().buffer()[(x, 8)];
                assert!(!cell.modifier.contains(ratatui::style::Modifier::DIM));
                if !no_color {
                    assert_eq!(cell.fg, p.ink.fg.unwrap());
                }
            }
        }
    }
}
#[test]
fn unicode_combining_partial_and_wrapping_do_not_modify_or_split_input() {
    let (mut a, fake) = attach(BrainDump::default());
    a.input = Note::new("e tail 👩‍💻");
    a.input.cursor = 1;
    start(&mut a, &fake);
    for partial in ["\u{301}", "\n你好 👩‍💻", "word ".repeat(100).as_str()] {
        event(
            &mut a,
            &fake,
            Event::Partial {
                take: 1,
                text: partial.into(),
            },
        );
        snapshot(80, 24, &mut a, true).unwrap();
        assert_eq!(a.input.text, "e tail 👩‍💻");
    }
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "\u{301}".into(),
        },
    );
    assert_eq!(a.input.text, "e\u{301} tail 👩‍💻");
    assert_eq!(a.input.cursor, 3);
}
#[test]
fn aliases_never_undo_skip_or_insert_omega_and_repeats_do_not_toggle() {
    for (code, mods) in [
        (
            KeyCode::Char('z'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
        (KeyCode::Char('Ω'), KeyModifiers::NONE),
        (
            KeyCode::Char('Ω'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
    ] {
        let (mut a, fake) = attach(BrainDump::default());
        a.skipped.push(Question {
            target: None,
            id: "keep".into(),
            text: "Keep this skipped?".into(),
        });
        a.handle_key(KeyEvent::new(code, mods));
        assert_eq!(a.skipped.len(), 1);
        assert!(a.input.text.is_empty());
        assert_eq!(fake.borrow().commands, vec![Command::Start { take: 1 }]);
        let mut repeat = KeyEvent::new(code, mods);
        repeat.kind = KeyEventKind::Repeat;
        a.handle_key(repeat);
        assert_eq!(fake.borrow().commands.len(), 1);
        a.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(a.skipped.is_empty());
        assert!(a.voice_active());
    }
}
#[test]
fn esc_cancels_voice_before_shaping_and_drops_all_late_callbacks() {
    let (mut a, fake) = attach(BrainDump::default());
    a.paste("typed");
    start(&mut a, &fake);
    key(&mut a, KeyCode::F(2));
    key(&mut a, KeyCode::Esc);
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "late".into(),
        },
    );
    assert_eq!(a.input.text, "typed");
    assert!(a.sources.is_empty());
    assert!(!a.running());
    assert_eq!(
        fake.borrow().commands,
        vec![
            Command::Start { take: 1 },
            Command::Stop { take: 1 },
            Command::Cancel { take: 1 }
        ]
    );
}
#[test]
fn f2_waits_for_final_then_submits_exactly_once_with_intact_source() {
    let (mut a, fake) = attach(BrainDump::default());
    a.paste("typed ");
    key(&mut a, KeyCode::F(6));
    key(&mut a, KeyCode::F(2));
    key(&mut a, KeyCode::F(2));
    assert!(a.sources.is_empty());
    assert!(!a.running());
    event(&mut a, &fake, Event::Listening { take: 1 });
    event(
        &mut a,
        &fake,
        Event::Partial {
            take: 1,
            text: "temporary".into(),
        },
    );
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "spoken".into(),
        },
    );
    assert_eq!(a.sources.len(), 1);
    assert_eq!(a.sources[0].id, 1);
    assert_eq!(a.sources[0].text, "typed spoken");
    assert!(a.sources[0].in_reply_to.is_none());
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "duplicate".into(),
        },
    );
    assert_eq!(a.sources.len(), 1);
    assert!(a.input.text.is_empty());
    assert_eq!(
        fake.borrow().commands,
        vec![Command::Start { take: 1 }, Command::Stop { take: 1 }]
    );
    assert!(a.receipt.is_none());
    assert!(a.goal_review.is_none());
}
#[test]
fn direct_submit_uses_the_same_wait_for_final_contract() {
    let (mut a, fake) = attach(BrainDump::default());
    start(&mut a, &fake);
    a.submit();
    assert!(a.sources.is_empty());
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "spoken".into(),
        },
    );
    assert_eq!(a.sources[0].text, "spoken");
}
#[test]
fn every_failure_is_one_muted_actionable_line_and_does_not_send_typed_words() {
    for kind in [
        "no-node",
        "install",
        "download",
        "config",
        "no-model",
        "engine-load",
        "mic-permission",
        "mic-unavailable",
        "mic-busy",
    ] {
        let (mut a, fake) = attach(BrainDump::default());
        a.paste("typed");
        start(&mut a, &fake);
        key(&mut a, KeyCode::F(2));
        event(
            &mut a,
            &fake,
            Event::Error {
                take: Some(1),
                kind: kind.into(),
                message: "Action required.".into(),
            },
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|f| render(f, &mut a, Palette::new(true)))
            .unwrap();
        let b = terminal.backend().buffer();
        assert_eq!(b[(2, 28)].symbol(), "A");
        assert!(b[(2, 28)].modifier.contains(ratatui::style::Modifier::DIM));
        assert_eq!(a.voice.as_ref().unwrap().failure().unwrap().kind, kind);
        assert_eq!(a.input.text, "typed");
        assert!(a.sources.is_empty());
        event(
            &mut a,
            &fake,
            Event::Final {
                take: 1,
                text: "late".into(),
            },
        );
        assert_eq!(a.input.text, "typed");
        assert!(a.sources.is_empty());
    }
}
#[test]
fn empty_fillers_and_insert_overflow_never_send_or_overwrite() {
    for final_text in [
        "",
        "thank you",
        "YOU.",
        ".",
        "\u{1b}\r",
        "x".repeat(4097).as_str(),
    ] {
        let (mut a, fake) = attach(BrainDump::default());
        a.paste("typed");
        start(&mut a, &fake);
        key(&mut a, KeyCode::F(2));
        event(
            &mut a,
            &fake,
            Event::Final {
                take: 1,
                text: final_text.into(),
            },
        );
        assert_eq!(a.input.text, "typed");
        assert!(a.sources.is_empty());
        assert!(!a.running());
        let notice = a.voice.as_ref().unwrap().notice();
        assert!(notice == "Didn't catch that." || notice.contains("4096"));
    }
}
#[test]
fn final_uses_existing_paste_sanitation_and_does_not_grant_confirmation() {
    let (mut a, fake) = attach(BrainDump::default());
    start(&mut a, &fake);
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "a\r\nb\u{0}confirm\u{1b}".into(),
        },
    );
    assert_eq!(a.input.text, "a\nbconfirm");
    assert!(a.sources.is_empty());
    assert!(a.receipt.is_none());
    assert!(a.goal_review.is_none());
}
#[test]
fn f10_closes_helper_preserves_unsaved_guard_and_can_reload_after_n() {
    let (mut a, fake) = attach(BrainDump::default());
    a.paste("typed");
    start(&mut a, &fake);
    key(&mut a, KeyCode::F(10));
    assert!(a.leave_prompt);
    assert!(!a.quit);
    assert!(fake.borrow().closed > 0);
    assert!(!a.voice_active());
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "late".into(),
        },
    );
    key(&mut a, KeyCode::Char('n'));
    assert_eq!(a.input.text, "typed");
    key(&mut a, KeyCode::F(6));
    assert_eq!(fake.borrow().restarted, 1);
    assert!(!a.voice_active());
    event(&mut a, &fake, Event::Ready { version: 1 });
    key(&mut a, KeyCode::F(6));
    assert!(a.voice_active());
    drop(a);
    assert!(fake.borrow().closed >= 2);
}
#[test]
fn review_and_receipt_disable_voice_and_cannot_receive_final_or_confirm() {
    let (mut a, fake) = attach(super::c16_tests::app(
        serde_json::json!({"parts": super::c16_tests::parts()}),
    ));
    a.review_goal();
    assert!(a.goal_review.is_some());
    for code in [KeyCode::F(6), KeyCode::Char('Ω')] {
        key(&mut a, code);
    }
    event(
        &mut a,
        &fake,
        Event::Final {
            take: 1,
            text: "confirm".into(),
        },
    );
    assert!(fake.borrow().commands.is_empty());
    assert!(a.goal_review.as_ref().unwrap().input.is_empty());
    assert!(a.handoff_job.is_none());
    assert!(a.input.text.is_empty());
    assert!(!a.voice_available());
    a.goal_review = None;
    a.receipt = Some(handoff::Receipt::test("/TEST/voice-receipt".into()));
    key(&mut a, KeyCode::F(6));
    assert!(fake.borrow().commands.is_empty());
    assert!(!yohaku::help_text(&a).contains("speak"));
    assert_eq!(yohaku::key_bar_items(&a), vec![("10", "menu")]);
}
#[test]
fn active_voice_blocks_readiness_and_review_even_without_typed_text() {
    let (mut a, fake) = attach(super::c16_tests::app(
        serde_json::json!({"parts": super::c16_tests::parts()}),
    ));
    assert!(a.goal_ready());
    start(&mut a, &fake);
    assert!(!a.goal_ready());
    a.review_goal();
    assert!(a.goal_review.is_none());
    assert!(a.notice.contains("Stop or cancel voice"));
    key(&mut a, KeyCode::Esc);
    assert!(a.goal_ready());
}
#[test]
fn conditional_help_and_key_bar_and_no_voice_in_library_snapshots() {
    let mut a = BrainDump {
        help_all: true,
        ..Default::default()
    };
    assert!(!yohaku::help_text(&a).contains("speak"));
    assert!(
        !snapshot(100, 30, &mut a, false)
            .unwrap()
            .contains("6 speak")
    );
    let (mut a, fake) = attach(a);
    assert!(yohaku::help_text(&a).contains("F6 / Ctrl+Alt+Z (⌃⌥Z)   speak (local)"));
    assert!(
        yohaku::key_bar_items(&a)
            .contains(&(crate::voice::key_label(std::env::consts::OS), "speak"))
    );
    start(&mut a, &fake);
    assert_eq!(
        yohaku::key_bar_items(&a),
        vec![
            (crate::voice::key_label(std::env::consts::OS), "stop"),
            ("esc", "cancel")
        ]
    );
    event(
        &mut a,
        &fake,
        Event::Error {
            take: None,
            kind: "no-node".into(),
            message: "Set TINKERY_NODE.".into(),
        },
    );
    assert!(yohaku::help_text(&a).contains("speak"));
}
#[test]
fn shaping_results_defer_while_voice_is_active_even_with_empty_input() {
    let (mut a, fake) = attach(BrainDump::default());
    a.paste("ordinary original");
    a.submit();
    start(&mut a, &fake);
    for _ in 0..100 {
        a.tick();
        if a.ready.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(a.ready.is_some());
    assert!(a.guess.is_none());
    assert_eq!(a.sources[0].text, "ordinary original");
    assert!(!a.goal_ready());
}

#[test]
fn first_use_key_bar_and_muted_prompt_preserve_input_and_require_explicit_yes() {
    for no_color in [false, true] {
        let fake = Rc::new(RefCell::new(Fake::default()));
        let mut a = BrainDump::default().with_voice(Box::new(fake.clone()));
        a.paste("typed");
        assert!(
            yohaku::key_bar_items(&a)
                .contains(&(crate::voice::key_label(std::env::consts::OS), "speak"))
        );
        event(
            &mut a,
            &fake,
            Event::NeedsInstall {
                size_mb: 20,
                message: "Voice needs a one-time local install (about 20 MB). Install now? y / n"
                    .into(),
            },
        );
        assert!(a.voice.as_ref().unwrap().notice().is_empty());
        key(&mut a, KeyCode::F(6));
        assert!(a.voice.as_ref().unwrap().setup_prompt());
        let palette = Palette::new(no_color);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| render(f, &mut a, palette)).unwrap();
        let cell = &terminal.backend().buffer()[(2, 28)];
        assert_eq!(cell.symbol(), "V");
        if no_color {
            assert!(cell.modifier.contains(ratatui::style::Modifier::DIM));
        } else {
            assert_eq!(cell.fg, palette.muted.fg.unwrap());
        }
        key(&mut a, KeyCode::Char('y'));
        assert_eq!(a.input.text, "typed");
        assert_eq!(fake.borrow().commands, [Command::Install]);
        assert_eq!(a.voice.as_ref().unwrap().notice(), "installing voice…");
        event(&mut a, &fake, Event::Loading);
        event(&mut a, &fake, Event::Ready { version: 1 });
        assert!(!a.voice_active());
        assert!(a.sources.is_empty());
        assert_eq!(a.input.text, "typed");
    }
}

#[test]
fn decline_is_consumed_without_typing_or_repeated_prompt_and_plain_messages_are_muted() {
    let fake = Rc::new(RefCell::new(Fake::default()));
    let mut a = BrainDump::default().with_voice(Box::new(fake.clone()));
    a.paste("typed");
    event(
        &mut a,
        &fake,
        Event::NeedsInstall {
            size_mb: 20,
            message: "Install now? y / n".into(),
        },
    );
    key(&mut a, KeyCode::F(6));
    key(&mut a, KeyCode::Char('n'));
    key(&mut a, KeyCode::F(6));
    assert!(!a.voice.as_ref().unwrap().setup_prompt());
    assert!(fake.borrow().commands.is_empty());
    assert_eq!(a.input.text, "typed");
    for (kind, message) in [
        (
            "mic-permission",
            "Allow microphone access for your terminal in System Settings → Privacy & Security → Microphone.",
        ),
        ("mic-busy", "Microphone is in use by another app."),
        ("mic-unavailable", "No microphone found."),
        (
            "install",
            "Voice install failed: registry unavailable. Press the voice key to retry.",
        ),
    ] {
        let (mut a, fake) = attach(BrainDump::default());
        a.paste("typed");
        event(
            &mut a,
            &fake,
            Event::Error {
                take: None,
                kind: kind.into(),
                message: message.into(),
            },
        );
        assert_eq!(a.voice.as_ref().unwrap().notice(), message);
        assert_eq!(a.input.text, "typed");
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|f| render(f, &mut a, Palette::new(true)))
            .unwrap();
        assert!(
            terminal.backend().buffer()[(2, 28)]
                .modifier
                .contains(ratatui::style::Modifier::DIM)
        );
    }
}
