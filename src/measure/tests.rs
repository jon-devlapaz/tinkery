//! Failure facts, safe disclosure and complete wrapped navigation; no real ledger reads.

use super::*;
use crossterm::event::KeyModifiers;
use std::process::{Command, Stdio};

fn output(stderr: &[u8], success: bool) -> Output {
    let mut out = Command::new("/bin/sh")
        .args(["-c", if success { "exit 0" } else { "exit 7" }])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    out.stderr = stderr.to_vec();
    out
}

fn failure(reading: &Reading) -> &Failure {
    let Report::Failed(failure) = &reading.report else {
        panic!("expected failure")
    };
    failure
}

fn press(reading: &mut Reading, help: &mut bool, code: KeyCode) -> bool {
    reading.key(KeyEvent::new(code, KeyModifiers::NONE), help)
}

fn snapshot(reading: &mut Reading, width: u16, height: u16, no_color: bool, help: bool) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
    loop {
        terminal
            .draw(|frame| reading.render(frame, frame.area(), Palette::new(no_color), help))
            .unwrap();
        if !reading.local_help() || reading.viewport.ready() {
            break;
        }
    }
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn unavailable_directory_is_unknown_and_the_launch_error_is_complete() {
    let tmp = tempfile::tempdir().unwrap();
    for directory in [tmp.path().join("missing"), tmp.path().join("file")] {
        std::fs::write(tmp.path().join("file"), "not a directory").unwrap();
        let reading = Reading::new(&directory);
        assert_eq!(failure(&reading).cause, Cause::DirectoryUnavailable);
        assert!(failure(&reading).details.contains("Launch error:"));
        assert!(
            failure(&reading)
                .details
                .contains(directory.to_str().unwrap())
        );
        assert!(!directory.join("ledger.json").exists());
    }
}

#[test]
fn launch_causes_are_conservative_and_success_bytes_are_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    for (kind, expected) in [
        (io::ErrorKind::NotFound, Cause::PythonMissing),
        (io::ErrorKind::PermissionDenied, Cause::Launch),
        (io::ErrorKind::Other, Cause::Launch),
    ] {
        let reading =
            Reading::from_result(tmp.path(), Err(io::Error::new(kind, "first\nlast café")));
        assert_eq!(failure(&reading).cause, expected);
        assert!(failure(&reading).details.contains("first\nlast café"));
    }
    let bytes = b"Report\r\nOriginal spacing:  \n\xff";
    let mut out = output(b"ignored warning", true);
    out.stdout = bytes.to_vec();
    let reading = Reading::from_result(tmp.path(), Ok(out));
    let Report::Complete(body) = reading.report else {
        panic!("not complete")
    };
    assert_eq!(body, String::from_utf8_lossy(bytes));
}

#[test]
fn only_the_exact_missing_target_module_diagnostic_is_classified() {
    for diagnostic in [
        "/usr/bin/python3: No module named tink_substrate\n",
        "/usr/bin/python3.13: No module named tink_substrate\n",
    ] {
        let reading =
            Reading::from_result(Path::new("/TEST"), Ok(output(diagnostic.as_bytes(), false)));
        assert_eq!(failure(&reading).cause, Cause::ModuleUnavailable);
    }
    for diagnostic in [
        "/usr/bin/python3: No module named dependency\n",
        "warning: No module named tink_substrate\n",
        "Traceback\n/usr/bin/python3: No module named tink_substrate\n",
        "ModuleNotFoundError: No module named 'dependency'\n",
        "",
        "first\n\nlast\n",
    ] {
        let reading =
            Reading::from_result(Path::new("/TEST"), Ok(output(diagnostic.as_bytes(), false)));
        let failure = failure(&reading);
        assert_eq!(failure.cause, Cause::Exit, "{diagnostic}");
        assert!(failure.details.contains("Exit status: 7"));
        assert!(failure.details.ends_with(if diagnostic.is_empty() {
            "No error details were returned."
        } else {
            diagnostic
        }));
    }
}

#[test]
fn diagnostics_escape_controls_invalid_bytes_and_bidi_without_executing_or_losing_them() {
    assert_eq!(
        display_bytes(b"caf\xc3\xa9\n\x1b[2J\r\t\x07\xff"),
        "café\n\\u{1b}[2J\\r\\t\\u{7}\\xff"
    );
    assert_eq!(
        display_bytes("界👩‍💻\u{202e}hidden\u{2066}".as_bytes()),
        "界👩‍💻\\u{202e}hidden\\u{2066}"
    );
    for control in [
        '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
        '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ] {
        assert_eq!(
            display_bytes(format!("ع{control}界👩‍💻").as_bytes()),
            format!("ع{}界👩‍💻", control.escape_default()),
            "unescaped bidi control {control:?}"
        );
    }
}

#[test]
fn all_wrapped_diagnostics_are_reachable_and_navigation_never_scrolls_under_help() {
    let tmp = tempfile::tempdir().unwrap();
    let directory = tmp.path().join("long-café-path-".repeat(20));
    let tokens = (0..400)
        .map(|i| format!("T{i:03}café界👩‍💻"))
        .collect::<Vec<_>>();
    let stderr = format!("FIRST\n{}\nLAST", tokens.join(" "));
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for no_color in [false, true] {
            let mut reading =
                Reading::from_result(&directory, Ok(output(stderr.as_bytes(), false)));
            let mut help = false;
            let summary = snapshot(&mut reading, width, height, no_color, help);
            assert!(summary.contains("Ledger report unavailable") && summary.contains(UNKNOWN));
            assert!(summary.contains("esc back   ? help"));
            assert!(!summary.contains("FIRST") && !summary.contains("Working directory:"));
            press(&mut reading, &mut help, KeyCode::F(1));
            let guidance = snapshot(&mut reading, width, height, no_color, help);
            assert!(guidance.contains("Right now") && guidance.contains("h how it works"));
            assert!(!guidance.contains("q quits") && !guidance.contains("Working directory:"));
            press(&mut reading, &mut help, KeyCode::Char('h'));
            let mut seen = String::new();
            for _ in 0..200 {
                let text = snapshot(&mut reading, width, height, no_color, help);
                assert!(text.contains("esc back   h help") && text.contains("q menu"));
                seen.push_str(&text.split_whitespace().collect::<String>());
                if text.contains("LAST") {
                    break;
                }
                press(&mut reading, &mut help, KeyCode::PageDown);
            }
            assert!(seen.contains("FIRST") && seen.contains("LAST"));
            // TestBackend includes padding cells for wide glyphs; ignore those display spaces.
            for token in &tokens {
                assert!(seen.contains(token), "missing {token} at {width}x{height}");
            }
            assert!(
                seen.contains(
                    &directory
                        .to_str()
                        .unwrap()
                        .split_whitespace()
                        .collect::<String>()
                )
            );
            assert_eq!(reading.scroll, 0, "help scrolled the hidden summary");
            press(&mut reading, &mut help, KeyCode::Esc);
            assert!(help && !reading.how);
            press(&mut reading, &mut help, KeyCode::Esc);
            assert!(!help);
            assert_eq!(
                snapshot(&mut reading, width, height, no_color, help),
                summary
            );
            assert!(press(&mut reading, &mut help, KeyCode::Esc));
            for code in [KeyCode::Char('?'), KeyCode::F(1)] {
                press(&mut reading, &mut help, code);
                press(&mut reading, &mut help, KeyCode::Char('h'));
                press(&mut reading, &mut help, code);
                assert!(!help && !reading.how);
            }
        }
    }
}

#[test]
fn diagnostics_beyond_u16_rows_reach_the_tail_and_scroll_back() {
    let stderr = (0..70_000)
        .map(|i| format!("row-{i:05} café\n"))
        .collect::<String>()
        + "LAST-DIAGNOSTIC-ROW";
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for no_color in [false, true] {
            let mut reading =
                Reading::from_result(Path::new("/TEST"), Ok(output(stderr.as_bytes(), false)));
            let mut help = false;
            press(&mut reading, &mut help, KeyCode::F(1));
            press(&mut reading, &mut help, KeyCode::Char('h'));
            for _ in 0..7_100 {
                press(&mut reading, &mut help, KeyCode::PageDown);
            }
            let tail = snapshot(&mut reading, width, height, no_color, help);
            assert!(tail.contains("LAST-DIAGNOSTIC-ROW"), "{tail}");
            assert!(tail.contains("row-69999 café"));
            assert!(reading.help_scroll > usize::from(u16::MAX));
            assert!(tail.contains("esc back   h help") && tail.contains("q menu"));
            let end = reading.help_scroll;
            for key in [KeyCode::Up, KeyCode::Char('k')] {
                press(&mut reading, &mut help, key);
            }
            assert_eq!(reading.help_scroll, end - 2);
            for key in [KeyCode::Down, KeyCode::Char('j')] {
                press(&mut reading, &mut help, key);
            }
            assert_eq!(reading.help_scroll, end);
            // A larger viewport clamps at the real end, not at a 16-bit offset.
            let resized = snapshot(&mut reading, 320, 40, no_color, help);
            assert!(resized.contains("LAST-DIAGNOSTIC-ROW"));
            for _ in 0..7_100 {
                press(&mut reading, &mut help, KeyCode::PageUp);
            }
            assert_eq!(reading.help_scroll, 0);
            let start = snapshot(&mut reading, width, height, no_color, help);
            assert!(start.contains("How it works") && start.contains("row-00000 café"));
            assert!(!press(&mut reading, &mut help, KeyCode::Esc));
            assert!(snapshot(&mut reading, width, height, no_color, help).contains("Right now"));
            assert!(!press(&mut reading, &mut help, KeyCode::Esc));
            assert!(
                snapshot(&mut reading, width, height, no_color, help)
                    .contains("Ledger report unavailable")
            );
            assert!(press(&mut reading, &mut help, KeyCode::Char('q')));
        }
    }
}

#[test]
fn a_single_unicode_line_can_wrap_beyond_u16_rows_without_losing_segment_boundaries() {
    use super::viewport::Viewport;
    let mut viewport = Viewport::default();
    let text = (0..70_000)
        .map(|i| match i {
            65_534 => "甲",
            65_535 => "乙",
            65_536 => "丙",
            65_537 => "丁",
            _ => "👩‍💻",
        })
        .collect::<String>()
        + "終";
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(2, 4)).unwrap();
    let mut offset = usize::MAX;
    terminal
        .draw(|frame| {
            viewport.render(
                frame,
                frame.area(),
                Palette::new(true),
                text.clone(),
                &mut offset,
            )
        })
        .unwrap();
    settle_viewport(
        &mut terminal,
        &mut viewport,
        &text,
        &mut offset,
        Palette::new(true),
    );
    assert_eq!(offset, 70_001 - 4);
    assert_eq!(terminal.backend().buffer()[(0, 3)].symbol(), "終");
    offset = 65_534;
    terminal
        .draw(|frame| {
            viewport.render(
                frame,
                frame.area(),
                Palette::new(true),
                text.clone(),
                &mut offset,
            )
        })
        .unwrap();
    for (row, expected) in ["甲", "乙", "丙", "丁"].iter().enumerate() {
        assert_eq!(
            terminal.backend().buffer()[(0, row as u16)].symbol(),
            *expected
        );
    }
    terminal.backend_mut().resize(4, 4);
    offset = usize::MAX;
    terminal
        .draw(|frame| {
            viewport.render(
                frame,
                frame.area(),
                Palette::new(true),
                text.clone(),
                &mut offset,
            )
        })
        .unwrap();
    settle_viewport(
        &mut terminal,
        &mut viewport,
        &text,
        &mut offset,
        Palette::new(true),
    );
    assert_eq!(offset, 35_001 - 4);
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.symbol() == "終")
    );
}

fn settle_viewport(
    terminal: &mut ratatui::Terminal<ratatui::backend::TestBackend>,
    viewport: &mut Viewport,
    text: &str,
    offset: &mut usize,
    palette: Palette,
) {
    while !viewport.ready() {
        terminal
            .draw(|frame| viewport.render(frame, frame.area(), palette, text, offset))
            .unwrap();
    }
}

#[test]
fn segmented_viewport_preserves_normal_paragraph_wrapping_and_styles() {
    use super::viewport::Viewport;
    use ratatui::widgets::Wrap;
    let long = "Café 界 👩‍💻 words ".repeat(1_500);
    for text in [
        "Right now\n\n  leading spaces and a long Café 界 👩‍💻 reading with multiple words   \n\n\nlast line\n",
        long.as_str(),
    ] {
        for width in [2, 7, 40, 80, 100] {
            for no_color in [false, true] {
                let palette = Palette::new(no_color);
                let count = Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .line_count(width);
                let mut viewport = Viewport::default();
                let step = if count > 200 { count / 4 } else { 1 };
                for wanted in (0..count).step_by(step).chain([count.saturating_sub(1)]) {
                    let mut actual =
                        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 3))
                            .unwrap();
                    let mut expected =
                        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 3))
                            .unwrap();
                    let mut offset = wanted;
                    actual
                        .draw(|frame| {
                            viewport.render(frame, frame.area(), palette, text, &mut offset)
                        })
                        .unwrap();
                    settle_viewport(&mut actual, &mut viewport, text, &mut offset, palette);
                    let clamped = wanted.min(count.saturating_sub(3));
                    expected
                        .draw(|frame| {
                            frame.render_widget(
                                Paragraph::new(text)
                                    .style(palette.ink)
                                    .wrap(Wrap { trim: false })
                                    .scroll((clamped as u16, 0)),
                                frame.area(),
                            )
                        })
                        .unwrap();
                    assert_eq!(
                        actual.backend().buffer(),
                        expected.backend().buffer(),
                        "width={width}, offset={wanted}"
                    );
                }
            }
        }
    }
}

#[test]
fn summary_copy_keeps_implementation_details_in_technical_help() {
    let directory = tempfile::tempdir().unwrap();
    for (result, expected) in [
        (
            Err(io::Error::new(io::ErrorKind::NotFound, "missing python3")),
            "Tinkery couldn’t find what it needs to read the ledger.",
        ),
        (
            Ok(output(
                b"/usr/bin/python3: No module named tink_substrate\n",
                false,
            )),
            "Tinkery couldn’t load what it needs to read the ledger.",
        ),
    ] {
        let mut reading = Reading::from_result(directory.path(), result);
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                let summary = snapshot(&mut reading, width, height, no_color, false);
                assert!(summary.contains(expected), "{summary}");
                for internal in ["Python", "python3", "tink_substrate"] {
                    assert!(!summary.contains(internal), "{summary}");
                }
            }
        }
        let mut help = false;
        press(&mut reading, &mut help, KeyCode::F(1));
        press(&mut reading, &mut help, KeyCode::Char('h'));
        let details = snapshot(&mut reading, 80, 24, true, help);
        assert!(details.contains("python3") && details.contains("tink_substrate"));
    }
}

#[test]
fn help_instructions_are_shown_once_per_view() {
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for no_color in [false, true] {
            let mut reading =
                Reading::from_result(Path::new("/TEST"), Ok(output(b"diagnostic", false)));
            let mut help = false;
            press(&mut reading, &mut help, KeyCode::F(1));
            let guidance = snapshot(&mut reading, width, height, no_color, help);
            assert_eq!(guidance.matches("close").count(), 1, "{guidance}");
            assert_eq!(guidance.matches("menu").count(), 1, "{guidance}");
            assert_eq!(guidance.matches("scroll").count(), 1, "{guidance}");
            assert!(guidance.contains("esc/?/F1 close   h how it works   q menu"));
            assert!(guidance.contains("F10 or Ctrl-C quits Tinkery."));
            press(&mut reading, &mut help, KeyCode::Char('h'));
            let details = snapshot(&mut reading, width, height, no_color, help);
            assert_eq!(details.matches("help").count(), 1, "{details}");
            assert_eq!(details.matches("close").count(), 1, "{details}");
            assert_eq!(details.matches("menu").count(), 1, "{details}");
            assert_eq!(details.matches("scroll").count(), 1, "{details}");
            assert!(details.contains("esc back   h help   ?/F1 close   q menu"));
            assert!(details.contains("PgUp/PgDn or ↑↓ / j k scroll."));
            assert!(details.contains("F10 or Ctrl-C quits Tinkery."));
            assert!(!press(&mut reading, &mut help, KeyCode::Char('h')));
            assert_eq!(
                snapshot(&mut reading, width, height, no_color, help),
                guidance
            );
        }
    }
}

#[test]
fn incremental_segments_preserve_blank_rows_across_chunk_boundaries() {
    let text = "\n".repeat(150_000) + "END";
    let mut viewport = Viewport::default();
    let mut offset = usize::MAX;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 4)).unwrap();
    terminal
        .draw(|f| viewport.render(f, f.area(), Palette::new(true), text.as_str(), &mut offset))
        .unwrap();
    settle_viewport(
        &mut terminal,
        &mut viewport,
        &text,
        &mut offset,
        Palette::new(true),
    );
    assert_eq!(
        offset,
        Paragraph::new(text.as_str())
            .wrap(ratatui::widgets::Wrap { trim: false })
            .line_count(80)
            - 4
    );
    assert_eq!(terminal.backend().buffer()[(0, 3)].symbol(), "E");
}

#[test]
fn prepared_success_clipping_matches_paragraph_including_invisible_runs_and_large_valid_clusters() {
    let cluster = format!("é{}", "\u{0301}".repeat(2047));
    for body in [
        String::new(),
        "\n\n".into(),
        "café 界 👩‍💻 🇵🇭 العربية हिन्दी\nnext\nlast\n".into(),
        format!("{}tail\nnext", "\u{200b}".repeat(100_000)),
        format!("{}tail\nnext", cluster.repeat(100)),
        "🇵\u{200b}🇭 a\u{200b}\u{301}b".into(),
    ] {
        let mut out = output(b"", true);
        out.stdout = body.as_bytes().to_vec();
        let mut reading = Reading::from_result(Path::new("/TEST"), Ok(out));
        assert!(!reading.local_help());
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            for no_color in [false, true] {
                for offset in [0, 1, 3] {
                    reading.scroll = offset;
                    let mut actual =
                        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
                            .unwrap();
                    let mut expected =
                        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
                            .unwrap();
                    actual
                        .draw(|f| reading.render(f, f.area(), Palette::new(no_color), false))
                        .unwrap();
                    expected
                        .draw(|f| {
                            f.render_widget(
                                Paragraph::new(body.as_str())
                                    .style(Palette::new(no_color).ink)
                                    .scroll((offset as u16, 0)),
                                f.area(),
                            )
                        })
                        .unwrap();
                    assert_eq!(actual.backend().buffer(), expected.backend().buffer());
                }
            }
        }
    }
}

#[test]
fn grapheme_display_limit_checks_exact_valid_boundaries_on_both_paths() {
    for (size, cluster) in [
        (4095, format!("a{}", "\u{0301}".repeat(2047))),
        (4096, format!("é{}", "\u{0301}".repeat(2047))),
        (4097, format!("a{}", "\u{0301}".repeat(2048))),
    ] {
        assert_eq!(cluster.len(), size);
        assert_eq!(cluster.graphemes(true).count(), 1);
        for success in [false, true] {
            let mut out = output(if success { b"" } else { cluster.as_bytes() }, success);
            if success {
                out.stdout = cluster.as_bytes().to_vec();
            }
            let mut reading = Reading::from_result(Path::new("/TEST"), Ok(out));
            if size > CLUSTER_LIMIT {
                assert_eq!(reading.report_cause(), Some(&Cause::DisplayLimit));
                assert!(snapshot(&mut reading, 80, 24, true, false).contains(UNKNOWN));
                reading.how = true;
                let details = snapshot(&mut reading, 80, 24, true, true);
                assert!(details.contains("withheld") && details.contains("4096-byte"));
            } else if success {
                let Report::Complete(body) = &reading.report else {
                    panic!()
                };
                assert_eq!(body, &cluster);
            } else {
                assert_eq!(reading.report_cause(), Some(&Cause::Exit));
                assert!(failure(&reading).details.ends_with(&cluster));
            }
        }
    }
    let normal = "العربية हिन्दी café 界 👩‍💻 🇵🇭 👨‍👩‍👧‍👦\n";
    let mut out = output(b"", true);
    out.stdout = normal.as_bytes().to_vec();
    let reading = Reading::from_result(Path::new("/TEST"), Ok(out));
    assert!(matches!(reading.report, Report::Complete(body) if body == normal));
}

#[test]
#[ignore = "zero-call worst-case output frame measurement"]
fn measure_maximum_output_frames() {
    for (name, bytes) in [
        ("ascii", vec![b'x'; reader::LIMIT]),
        ("controls", vec![1; reader::LIMIT]),
        ("newlines", vec![b'\n'; reader::LIMIT]),
        (
            "one-grapheme",
            format!("a{}", "\u{0301}".repeat((reader::LIMIT - 1) / 2)).into_bytes(),
        ),
    ] {
        let started = std::time::Instant::now();
        let mut reading = Reading::from_result(Path::new("/TEST"), Ok(output(&bytes, false)));
        let preparation = started.elapsed();
        if name == "one-grapheme" {
            assert_eq!(reading.report_cause(), Some(&Cause::DisplayLimit));
        }
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let frame = std::time::Instant::now();
        terminal
            .draw(|f| reading.render(f, f.area(), Palette::new(true), true))
            .unwrap();
        reading.how = true;
        terminal
            .draw(|f| reading.render(f, f.area(), Palette::new(true), true))
            .unwrap();
        eprintln!(
            "{name}: prepare={preparation:?}, first-help-and-details-frames={:?}",
            frame.elapsed()
        );
    }
}

#[test]
#[ignore = "zero-call supported-limit steady frame measurement"]
fn measure_supported_limit_steady_frames() {
    let cluster = format!("é{}", "\u{0301}".repeat(2047));
    let body = cluster.repeat(reader::LIMIT / cluster.len());
    for success in [false, true] {
        let mut out = output(if success { b"" } else { body.as_bytes() }, success);
        if success {
            out.stdout = body.as_bytes().to_vec();
        }
        let prepare = std::time::Instant::now();
        let mut reading = Reading::from_result(Path::new("/TEST"), Ok(out));
        let preparation = prepare.elapsed();
        assert_ne!(reading.report_cause(), Some(&Cause::DisplayLimit));
        reading.how = true;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut maximum = std::time::Duration::ZERO;
        for _ in 0..32 {
            let frame = std::time::Instant::now();
            terminal
                .draw(|f| reading.render(f, f.area(), Palette::new(true), !success))
                .unwrap();
            maximum = maximum.max(frame.elapsed());
        }
        eprintln!(
            "16MiB of 4096-byte clusters: success={success}, prepare={preparation:?}, max-of-32-frames={maximum:?}"
        );
    }
}

#[test]
fn failure_summary_wraps_and_scroll_clamps_on_resize() {
    let tmp = tempfile::tempdir().unwrap();
    let mut reading = Reading::from_result(tmp.path(), Ok(output(b"first\nlast", false)));
    let mut help = false;
    // A compact viewport forces even the fixed-copy summary to wrap and scroll.
    let mut seen = String::new();
    for _ in 0..20 {
        let text = snapshot(&mut reading, 40, 8, true, help);
        assert!(text.contains("esc back   ? help"));
        seen.push_str(&text.split_whitespace().collect::<String>());
        press(&mut reading, &mut help, KeyCode::Down);
    }
    assert!(seen.contains("Recordedworkandattentionareunknown."));
    press(&mut reading, &mut help, KeyCode::Char('?'));
    press(&mut reading, &mut help, KeyCode::Char('h'));
    reading.help_scroll = usize::MAX;
    snapshot(&mut reading, 80, 24, true, help);
    snapshot(&mut reading, 320, 40, true, help);
    assert_eq!(reading.help_scroll, 0);
}
