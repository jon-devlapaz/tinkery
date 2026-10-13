//! Think help preserves the reader's context and authority while its own panel scrolls.
use super::goal_tests::board;
use super::*;

#[test]
fn escape_closes_help_before_cancelling_a_reading_and_keeps_words() {
    let mut app = BrainDump::with_host(Arc::new(Simulated));
    app.paste("These exact words stay mine.");
    app.submit();
    assert!(app.running());
    let original = app.conversation.sources()[0].text.clone();
    key(&mut app, KeyCode::F(1));
    assert!(
        snapshot(80, 24, &mut app, false)
            .unwrap()
            .contains("Close help")
    );
    key(&mut app, KeyCode::Esc);
    assert!(app.running() && !app.screen.help && !app.quitting());
    key(&mut app, KeyCode::Esc);
    assert!(!app.running() && !app.quitting());
    assert_eq!(app.conversation.sources()[0].text, original);
    assert!(app.notice.contains("Cancelled"));
}

#[test]
fn review_and_receipt_help_cannot_confirm_copy_or_open_shape() {
    let mut app = board();
    app.review_goal();
    app.paste("conf");
    let frozen = app
        .handoff
        .review
        .as_ref()
        .unwrap()
        .affirmation
        .goal
        .clone();
    key(&mut app, KeyCode::F(1));
    app.paste("irm");
    for code in [KeyCode::Enter, KeyCode::F(2), KeyCode::F(3), KeyCode::F(8)] {
        key(&mut app, code);
    }
    assert_eq!(app.handoff.review.as_ref().unwrap().input, "conf");
    assert_eq!(
        app.handoff.review.as_ref().unwrap().affirmation.goal,
        frozen
    );
    assert!(app.saved().is_none());
    key(&mut app, KeyCode::F(10));
    assert!(app.screen.leave_prompt && !app.quitting());
    key(&mut app, KeyCode::Char('n'));
    assert!(app.screen.help && !app.screen.leave_prompt);
    key(&mut app, KeyCode::Esc);
    app.handoff.review = None;
    app.handoff.receipt = Some(seed_me::Receipt::test("/tmp/tinkery-TEST-overlay".into()));
    let receipt = app.handoff.receipt.as_ref().unwrap().text();
    key(&mut app, KeyCode::F(1));
    for code in [
        KeyCode::Char('c'),
        KeyCode::Char('s'),
        KeyCode::F(3),
        KeyCode::Enter,
    ] {
        key(&mut app, code);
    }
    assert!(app.take_copy_request().is_none());
    assert_eq!(app.exit(), &Exit::Stay);
    assert_eq!(app.handoff.receipt.as_ref().unwrap().text(), receipt);
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('c'));
    assert!(app.take_copy_request().is_some());
}

fn key(app: &mut BrainDump, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn contextual_help_is_inset_and_keeps_the_original_and_draft() {
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for mono in [false, true] {
            let mut app = board();
            app.paste("my next answer stays mine");
            let sources = app.conversation.sources().to_vec();
            let input = app.input.text.clone();
            let before = snapshot(width, height, &mut app, mono).unwrap();
            let source_area = app.screen.source_area;
            key(&mut app, KeyCode::F(1));
            let help = snapshot(width, height, &mut app, mono).unwrap();
            assert!(help.contains("┌ Right now "));
            assert!(help.contains("Every key") && help.contains("Close help"));
            assert!(help.contains("my next answer stays mine"));
            assert_eq!(app.screen.source_area, source_area);
            assert_eq!(
                serde_json::to_value(app.conversation.sources()).unwrap(),
                serde_json::to_value(sources).unwrap()
            );
            assert_eq!(app.input.text, input);
            for (old, new) in before.lines().zip(help.lines()).take(height as usize - 1) {
                assert_eq!(
                    old.chars()
                        .take(width as usize * 48 / 100)
                        .collect::<String>(),
                    new.chars()
                        .take(width as usize * 48 / 100)
                        .collect::<String>()
                );
            }
            key(&mut app, KeyCode::Esc);
            assert!(!app.screen.help && !app.running() && !app.quitting());
        }
    }
}

#[test]
fn help_blocks_hidden_actions_and_typing_without_confirming_or_sending() {
    for event in [
        KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT),
        KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    ] {
        let mut app = board();
        app.paste("my answer");
        let input = app.input.text.clone();
        key(&mut app, KeyCode::F(1));
        app.handle_key(event);
        app.paste("confirm");
        assert!(app.screen.help);
        assert!(!app.screen.original && !app.screen.details && !app.add_more);
        assert!(!app.running() && app.handoff.review.is_none() && app.saved().is_none());
        assert_eq!(app.input.text, input);
        assert!(app.take_copy_request().is_none());
    }
}

#[test]
fn long_help_keeps_its_title_and_navigation_visible_while_scrolling_and_resizing() {
    let mut app = board();
    key(&mut app, KeyCode::F(1));
    key(&mut app, KeyCode::F(1));
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40), (80, 24)] {
        app.screen.original_scroll = 0;
        let first = snapshot(width, height, &mut app, true).unwrap();
        assert!(first.contains("┌ Every key "));
        assert!(first.contains("How it works") && first.contains("Scroll"));
        for _ in 0..100 {
            key(&mut app, KeyCode::PageDown);
        }
        let last = snapshot(width, height, &mut app, true).unwrap();
        assert!(last.contains("┌ Every key "));
        assert!(last.contains("Close help") && last.contains("Scroll"));
        assert!(last.contains("key bar"));
        assert!(app.screen.original_scroll > 0);
    }
    key(&mut app, KeyCode::Char('h'));
    let technical = snapshot(80, 24, &mut app, true).unwrap();
    assert!(technical.contains("┌ How it works ") && technical.contains("Back to help"));
    key(&mut app, KeyCode::Esc);
    assert!(app.screen.help && app.screen.help_all && !app.screen.how);
    key(&mut app, KeyCode::F(1));
    assert!(!app.screen.help && !app.running() && !app.quitting());
}

#[test]
fn scroll_hint_is_present_exactly_when_the_help_body_overflows() {
    for how in [false, true] {
        for height in [24, 30, 40, 60, 120] {
            for mono in [false, true] {
                let mut app = BrainDump::default();
                app.screen.help = true;
                app.screen.help_all = !how;
                app.screen.how = how;
                let first = snapshot(100, height, &mut app, mono).unwrap();
                for _ in 0..100 {
                    key(&mut app, KeyCode::PageDown);
                }
                let last = snapshot(100, height, &mut app, mono).unwrap();
                let overflow = app.screen.original_scroll > 0;
                assert_eq!(
                    first.contains("Scroll"),
                    overflow,
                    "how={how}, height={height}"
                );
                assert_eq!(last.contains("Scroll"), overflow);
                assert!(last.contains("Close help"));
                if !overflow {
                    assert_eq!(first, last);
                }
            }
        }
    }
}

#[test]
fn empty_technical_sections_leave_one_blank_row_before_last_result() {
    let app = BrainDump::default();
    let text = layout::how_text(&app);
    let before = text.split("Last result\n").next().unwrap();
    assert!(before.ends_with("\n\n"));
    assert!(!before.ends_with("\n\n\n"));
    assert!(text.contains("Mac F-keys may need fn.\n\nLast result"));
}

#[test]
fn help_palette_and_spacing_match_the_quiet_menu_hierarchy() {
    use ratatui::style::{Color, Modifier};
    for mono in [false, true] {
        let mut app = board();
        key(&mut app, KeyCode::F(1));
        let palette = Palette::new(mono);
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render(frame, &mut app, palette))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(52, 3)].symbol(), "R");
        assert_eq!(buffer[(52, 3)].fg, palette.jade.fg.unwrap_or(Color::Reset));
        assert!(buffer[(52, 3)].modifier.contains(Modifier::BOLD));
        assert!(!buffer[(52, 3)].modifier.contains(Modifier::DIM));
        assert_eq!(buffer[(52, 6)].symbol(), "F");
        assert_eq!(buffer[(52, 6)].fg, palette.ink.fg.unwrap_or(Color::Reset));
        assert!(buffer[(52, 6)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(64, 6)].fg, palette.muted.fg.unwrap_or(Color::Reset));
        assert!(!buffer[(64, 6)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(50, 6)].fg, palette.muted.fg.unwrap_or(Color::Reset));
        for y in [4, 9, 13, 15] {
            for x in 52..96 {
                assert_eq!(buffer[(x, y)].symbol(), " ");
            }
        }
        if mono {
            for y in 3..15 {
                for x in 50..98 {
                    assert_eq!(buffer[(x, y)].fg, Color::Reset);
                    assert_eq!(buffer[(x, y)].bg, Color::Reset);
                }
            }
        }
        let mut empty = BrainDump::default();
        key(&mut empty, KeyCode::F(1));
        terminal
            .draw(|frame| render(frame, &mut empty, palette))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(63, 5)].symbol(), "F");
        assert_eq!(buffer[(63, 5)].fg, palette.ink.fg.unwrap_or(Color::Reset));
        assert!(buffer[(63, 5)].modifier.contains(Modifier::BOLD));
        assert_eq!(
            layout::help_text(&empty).lines().nth(1),
            Some("Type, then F2 sends it.")
        );
    }
}

#[test]
fn how_it_works_says_what_leaves_the_machine_in_plain_words() {
    let mut app = BrainDump::default();
    assert!(layout::how_text(&app).contains("nothing leaves your machine"));
    app.real = true;
    let text = layout::how_text(&app);
    assert!(text.contains("your words go to the model you chose"));
    assert!(text.contains("may cost money"));
    for jargon in [
        "schema",
        "pins its origin",
        "intake readiness",
        "owned viewer",
        "nonblocking",
        "coverage locations",
    ] {
        assert!(!text.contains(jargon), "{jargon}");
    }
}
