//! Garden geometry and ink contracts, using synthetic sessions and an injected time.

use super::*;
use crate::tend::{Session, Stage, timestamp};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};
use unicode_width::UnicodeWidthStr;

fn tend(count: usize) -> Tend {
    let mut tend = Tend::new(Err("TEST links unavailable".into()));
    tend.pending = false;
    tend.current = true;
    tend.sessions = (0..count)
        .map(|i| Session {
            id: Some(format!("session-{i}")),
            path: format!("/TEST/session-{i}").into(),
            goal: format!(
                "Goal {i}: Café 界 👩‍💻 {}\nWhy: never in garden",
                "long words ".repeat(10)
            ),
            stage: Stage::GoalSaved,
            updated: "2026-10-11T10:00:00Z".into(),
        })
        .collect();
    tend.all_sessions = tend.sessions.clone();
    tend
}
fn body(width: u16, height: u16) -> Rect {
    Rect::new(2, 3, width - 4, height - 4)
}
fn paint(tend: &mut Tend, width: u16, height: u16, no_color: bool) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            let palette = Palette::new(no_color);
            frame.render_widget(Paragraph::new("").style(palette.ink), frame.area());
            render(
                frame,
                tend,
                body(width, height),
                palette,
                timestamp("2026-10-11T12:00:00Z"),
            );
        })
        .unwrap();
    terminal.backend().buffer().clone()
}
fn line(buffer: &Buffer, row: u16) -> String {
    let mut text = String::new();
    let mut x = 2;
    while x < buffer.area.width - 2 {
        let symbol = buffer[(x, row)].symbol();
        text.push_str(symbol);
        x += symbol.width().max(1) as u16;
    }
    text
}

#[test]
fn needs_suffix_truncates_headline_and_refusals_stay_muted() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-ink-")
        .tempdir()
        .unwrap();
    let mut tend = tend(2);
    tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
    tend.needs.toggle("session-0", "session-1").unwrap();
    tend.sessions[1].goal = "Quiet garden".into();
    let mut sessions = tend.sessions.clone();
    sessions[1].stage = Stage::Completed;
    tend.apply(
        &Ok(crate::tend::Found {
            sessions,
            unreadable: 0,
            failures: Vec::new(),
        }),
        true,
        1,
    );
    for (width, height) in [(80, 24), (100, 30), (160, 40)] {
        for no_color in [false, true] {
            tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
            tend.links = tend.needs.read().into_links();
            tend.notice.clear();
            let buffer = paint(&mut tend, width, height, no_color);
            let selected = line(&buffer, height - 3);
            assert!(
                selected.ends_with("goal saved · 2h ago · needs Quiet garden · ready"),
                "{selected}"
            );
            assert!(
                selected.contains('…'),
                "headline truncates before needs: {selected}"
            );
            let palette = Palette::new(no_color);
            for x in width - 3 - "needs Quiet garden · ready".width() as u16..width - 2 {
                assert_eq!(
                    buffer[(x, height - 3)].fg,
                    palette.muted.fg.unwrap_or(Color::Reset)
                );
            }
            tend.needs = crate::tend::needs::Store::new(Err("TEST unavailable destination".into()));
            tend.start_link();
            let buffer = paint(&mut tend, width, height, no_color);
            assert_eq!(
                line(&buffer, height - 2).trim(),
                "Links couldn't be read, so nothing changed. F1 then h for details."
            );
            for x in 2..width - 2 {
                assert_eq!(
                    buffer[(x, height - 2)].fg,
                    palette.muted.fg.unwrap_or(Color::Reset)
                );
                assert_eq!(
                    buffer[(x, height - 2)].modifier.contains(Modifier::DIM),
                    no_color
                );
            }
        }
    }
}

#[test]
fn long_needs_keep_source_and_readiness_visible_with_full_help() {
    for ready in [false, true] {
        let tmp = tempfile::Builder::new()
            .prefix("tinkery-TEST-needs-footer-")
            .tempdir()
            .unwrap();
        let mut tend = tend(4);
        tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
        let mut sessions = tend.sessions.clone();
        sessions[0].goal = "Source goal\nWhy: intact".into();
        for (i, session) in sessions.iter_mut().enumerate().skip(1) {
            session.goal = format!("Dependency {i} {}", "界👩‍💻e\u{301} long ".repeat(25));
            session.stage = if ready {
                Stage::Completed
            } else {
                Stage::GoalSaved
            };
            tend.needs
                .set("session-0", &format!("session-{i}"), true)
                .unwrap();
        }
        let titles: Vec<_> = sessions.iter().skip(1).map(|s| s.goal.clone()).collect();
        tend.apply(
            &Ok(crate::tend::Found {
                sessions,
                unreadable: 0,
                failures: Vec::new(),
            }),
            true,
            1,
        );
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            for no_color in [false, true] {
                let buffer = paint(&mut tend, width, height, no_color);
                let footer = line(&buffer, height - 3);
                assert!(footer.contains("Source goal"), "{footer}");
                assert!(
                    footer.contains(if ready { " · ready" } else { " · not ready" }),
                    "{footer}"
                );
                assert!(footer.contains("F1 needs"), "{footer}");
                let (goal, status) = tend.selected_line((width - 4).into(), None);
                assert!(goal.width() + status.width() <= usize::from(width - 4));
                let help = tend.help_text();
                for title in &titles {
                    assert!(
                        help.contains(title),
                        "full dependency title must remain accessible"
                    );
                }
            }
        }
    }
}

#[test]
fn unreadable_link_refusal_and_note_say_it_once() {
    for at_commit in [false, true] {
        let tmp = tempfile::Builder::new()
            .prefix("tinkery-TEST-single-note-")
            .tempdir()
            .unwrap();
        let mut tend = tend(2);
        if at_commit {
            tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
            tend.start_link();
            tend.selected = 1;
            tend.needs = crate::tend::needs::Store::new(Err("TEST unreadable".into()));
            tend.commit_link();
        } else {
            tend.start_link();
        }
        for _ in 0..2 {
            let buffer = paint(&mut tend, 100, 30, true);
            let lines: Vec<_> = (0..30).map(|y| line(&buffer, y)).collect();
            assert_eq!(
                lines
                    .iter()
                    .filter(|s| s.contains("Links couldn't be read"))
                    .count(),
                1
            );
            assert_eq!(lines.iter().filter(|s| s.contains("F1 then h")).count(), 1);
            tend.step_selection(true);
        }
    }
}

#[test]
fn empty_goal_view_discloses_unreadable_links() {
    for (width, height) in [(40, 12), (80, 24), (100, 30)] {
        for ended in [false, true] {
            let tmp = tempfile::Builder::new()
                .prefix("tinkery-TEST-empty-note-")
                .tempdir()
                .unwrap();
            let path = tmp.path().join("needs.json");
            std::fs::write(&path, "{broken").unwrap();
            let mut tend = tend(0);
            tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().to_path_buf()));
            tend.apply(
                &Ok(crate::tend::Found {
                    sessions: vec![],
                    unreadable: 0,
                    failures: Vec::new(),
                }),
                true,
                1,
            );
            tend.ended = usize::from(ended);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| {
                    super::super::render(f, &mut tend, body(width, height), Palette::new(true))
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert!(
                line(buffer, height - 4).starts_with("Links couldn't be read."),
                "{buffer:?}"
            );
            assert!(line(buffer, 3).starts_with(if ended {
                "No active goals."
            } else {
                "No saved sessions yet."
            }));
            assert_eq!(std::fs::read_to_string(path).unwrap(), "{broken");
        }
    }
}

#[test]
fn unreadable_links_have_a_persistent_muted_note_with_other_notes() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-link-note-")
        .tempdir()
        .unwrap();
    let root = tmp.path().join("TEST-links");
    std::fs::create_dir_all(&root).unwrap();
    for bytes in [
        "{broken",
        "",
        r#"{"version":1,"needs":{}}"#,
        r#"{"version":2,"needs":[]}"#,
        r#"{"version":1,"needs":[{"goal":"a","needs":"b"},{"goal":"a","needs":"b"}]}"#,
        r#"{"version":1,"needs":[{"goal":"a","needs":"a"}]}"#,
    ] {
        std::fs::write(root.join("needs.json"), bytes).unwrap();
        let mut tend = tend(2);
        tend.needs = crate::tend::needs::Store::new(Ok(root.clone()));
        let mut sessions = tend.sessions.clone();
        sessions[1].stage = Stage::Completed;
        tend.apply(
            &Ok(crate::tend::Found {
                sessions,
                unreadable: 1,
                failures: Vec::new(),
            }),
            true,
            1,
        );
        for no_color in [false, true] {
            for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
                let buffer = paint(&mut tend, width, height, no_color);
                let note = line(&buffer, height - 4);
                assert!(
                    note.starts_with("Links couldn't be read. F1 then h for details."),
                    "{note}"
                );
                if width == 320 {
                    assert_eq!(
                        note.trim_end(),
                        "Links couldn't be read. F1 then h for details. · 1 ended not shown · 1 saved record couldn't be read; this view may be missing sessions."
                    );
                }
                assert!(line(&buffer, height - 2).trim().is_empty());
                let palette = Palette::new(no_color);
                for x in 2..width - 2 {
                    assert_eq!(
                        buffer[(x, height - 4)].fg,
                        palette.muted.fg.unwrap_or(Color::Reset)
                    );
                    assert_eq!(
                        buffer[(x, height - 4)].modifier.contains(Modifier::DIM),
                        no_color
                    );
                }
                tend.step_selection(true);
            }
        }
        tend.how = true;
        assert!(tend.help_text().contains("Cannot read links"));
        assert_eq!(
            std::fs::read(root.join("needs.json")).unwrap(),
            bytes.as_bytes()
        );
    }
}

#[test]
fn read_notes_share_the_muted_row_above_selection_without_changing_it() {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-read-notes-")
        .tempdir()
        .unwrap();
    let mut tend = tend(1);
    tend.needs = crate::tend::needs::Store::new(Ok(tmp.path().join("TEST-links")));
    let before = paint(&mut tend, 160, 40, true);
    assert!(line(&before, 36).trim().is_empty());
    for ended in [0, 1, 2] {
        for unreadable in [0, 1, 2] {
            let mut sessions = tend.sessions.clone();
            sessions.extend((0..ended).map(|i| Session {
                id: Some(format!("ended-{i}")),
                path: format!("/TEST/ended-{i}").into(),
                goal: "Ended goal must stay hidden".into(),
                stage: Stage::Completed,
                updated: String::new(),
            }));
            tend.apply(
                &Ok(super::super::Found {
                    sessions,
                    unreadable,
                    failures: Vec::new(),
                }),
                true,
                1,
            );
            let after = paint(&mut tend, 160, 40, true);
            assert_eq!(line(&before, 37), line(&after, 37));
            let mut expected = Vec::new();
            if ended > 0 {
                expected.push(format!("{ended} ended not shown"));
            }
            if unreadable > 0 {
                expected.push(format!(
                    "{} couldn't be read; this view may be missing sessions.",
                    records(unreadable)
                ));
            }
            assert_eq!(line(&after, 36).trim_end(), expected.join(" · "));
            for x in 2..158 {
                assert!(after[(x, 36)].modifier.contains(Modifier::DIM));
                assert!(!after[(x, 36)].modifier.contains(Modifier::BOLD));
            }
            assert!(line(&after, 38).trim().is_empty());
            assert!(!(3..36).any(|y| line(&after, y).contains("Ended goal")));
            assert_eq!(tend.sessions.len(), 1);
        }
    }
}
