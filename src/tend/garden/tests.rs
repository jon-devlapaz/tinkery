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
    sessions[1].stage = Stage::Ended;
    tend.apply(
        &Ok(crate::tend::Found {
            sessions,
            unreadable: 0,
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
fn read_notes_share_the_muted_row_above_selection_without_changing_it() {
    let mut tend = tend(1);
    let before = paint(&mut tend, 160, 40, true);
    assert!(line(&before, 36).trim().is_empty());
    for ended in [0, 1, 2] {
        for unreadable in [0, 1, 2] {
            let mut sessions = tend.sessions.clone();
            sessions.extend((0..ended).map(|i| Session {
                id: Some(format!("ended-{i}")),
                path: format!("/TEST/ended-{i}").into(),
                goal: "Ended goal must stay hidden".into(),
                stage: Stage::Ended,
                updated: String::new(),
            }));
            tend.apply(
                &Ok(super::super::Found {
                    sessions,
                    unreadable,
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
