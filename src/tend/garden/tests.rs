//! Garden geometry and ink contracts, using synthetic sessions and an injected time.

use super::*;
use crate::tend::{Session, Stage, row, timestamp};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Modifier};

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
fn deterministic_layout_keeps_cells_and_labels_apart_and_selected_on_every_page() {
    for (width, height, columns) in [(80, 24, 3), (100, 30, 4), (160, 40, 7), (320, 40, 15)] {
        let body = body(width, height);
        for selected in 0..237 {
            let layout = Layout::new(body, 237, selected);
            assert_eq!(layout, Layout::new(body, 237, selected));
            assert_eq!(layout.columns, columns);
            assert!(layout.cell_width >= 20);
            assert!(layout.start <= selected && selected < layout.end);
            assert_eq!(layout.page, selected / columns / layout.visible_rows);
            for index in layout.start..layout.end {
                let cell = layout.cell(index);
                assert_eq!(cell.intersection(layout.area), cell);
                assert_eq!(layout.hit(cell.x, cell.y), Some(index));
                assert_eq!(layout.hit(cell.right() - 1, cell.bottom() - 1), Some(index));
                assert_eq!(
                    layout.hit(cell.x, cell.bottom()),
                    None,
                    "blank rows aren't cells"
                );
                for other in index + 1..layout.end {
                    assert!(cell.intersection(layout.cell(other)).is_empty());
                }
            }
        }
        let layout = Layout::new(body, 237, 0);
        assert_eq!(
            layout.cell(columns).x - layout.cell(0).x,
            layout.cell_width / 2
        );
    }
}

#[test]
fn rings_keep_paper_and_hollow_centres_and_only_the_selected_goal_uses_jade() {
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for no_color in [false, true] {
            let palette = Palette::new(no_color);
            let mut tend = tend(7);
            tend.selected = 2;
            let buffer = paint(&mut tend, width, height, no_color);
            let layout = Layout::new(body(width, height), 7, 2);
            for index in 0..7 {
                let cell = layout.cell(index);
                let label = (cell.x..cell.right())
                    .find(|&x| buffer[(x, cell.y)].symbol() != " ")
                    .unwrap();
                let first = &buffer[(label, cell.y)];
                let style = if index == 2 {
                    palette.jade
                } else {
                    palette.muted
                };
                assert_eq!(first.fg, style.fg.unwrap_or(Color::Reset));
                assert_eq!(first.modifier.contains(Modifier::BOLD), index == 2);
                assert!(!first.modifier.contains(Modifier::ITALIC));
                assert_eq!(first.symbol() == "›", index == 2);
                let label_text: String = (cell.x..cell.right())
                    .map(|x| buffer[(x, cell.y)].symbol())
                    .collect();
                assert!(label_text.trim().ends_with('…'));
                let last = (cell.x..cell.right())
                    .rfind(|&x| buffer[(x, cell.y)].symbol() != " ")
                    .unwrap();
                assert!(last - label < cell.width - 2);
                let left = cell.x + cell.width / 2 - 2;
                let mut dot_count = 0;
                for y in 0..2 {
                    for x in 0..4 {
                        let pixel = &buffer[(left + x, cell.y + 1 + y)];
                        assert_eq!(pixel.bg, palette.ink.bg.unwrap_or(Color::Reset));
                        let bits = if pixel.symbol() == " " {
                            0
                        } else {
                            pixel.symbol().chars().next().unwrap() as u32 - 0x2800
                        };
                        dot_count += bits.count_ones();
                        assert_eq!(
                            pixel.fg,
                            if index == 2 {
                                palette.jade
                            } else {
                                palette.ink
                            }
                            .fg
                            .unwrap_or(Color::Reset)
                        );
                        if x == 1 || x == 2 {
                            // The centre dots are empty; the same character also carries the rim.
                            let interior = if y == 0 { 0b1110_0100 } else { 0b0001_1011 };
                            assert_eq!(bits & interior, 0, "a hollow ring never paints its centre");
                        }
                    }
                }
                assert!(dot_count > 0 && dot_count < 64);
            }
            for y in layout.area.y..layout.area.bottom() {
                let mut x = layout.area.x;
                while x < layout.area.right() {
                    let pixel = &buffer[(x, y)];
                    assert_eq!(pixel.bg, palette.ink.bg.unwrap_or(Color::Reset));
                    x += pixel.symbol().width().max(1) as u16;
                }
            }
            let (goal, status) = row(
                &tend.sessions[2],
                usize::from(width - 4),
                timestamp("2026-10-11T12:00:00Z"),
            );
            assert_eq!(line(&buffer, height - 3), format!("{goal}{status}"));
            assert_eq!(
                buffer[(2, height - 3)].fg,
                palette.ink.fg.unwrap_or(Color::Reset)
            );
            assert_eq!(
                buffer[(width - 3, height - 3)].fg,
                palette.muted.fg.unwrap_or(Color::Reset)
            );
            assert!(line(&buffer, height - 2).trim().is_empty());
        }
    }
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
fn ring_dots_are_mirror_symmetric_and_identical_in_every_cell() {
    let masks = [[1, 8], [2, 16], [4, 32], [64, 128]];
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let layout = Layout::new(body(width, height), 1000, 0);
        let capacity = layout.columns * layout.visible_rows;
        for no_color in [false, true] {
            for selected in [0, capacity] {
                let mut tend = tend(capacity + 7);
                tend.selected = selected;
                let buffer = paint(&mut tend, width, height, no_color);
                let layout = Layout::new(body(width, height), tend.sessions.len(), selected);
                let mut previous = None;
                for index in layout.start..layout.end {
                    let cell = layout.cell(index);
                    let left = cell.x + cell.width / 2 - 2;
                    let dots: [[bool; 8]; 8] = std::array::from_fn(|y| {
                        std::array::from_fn(|x| {
                            let symbol =
                                buffer[(left + x as u16 / 2, cell.y + 1 + y as u16 / 4)].symbol();
                            let bits = if symbol == " " {
                                0
                            } else {
                                symbol.chars().next().unwrap() as u32 - 0x2800
                            };
                            bits & masks[y % 4][x % 2] != 0
                        })
                    });
                    for y in 0..8 {
                        for x in 0..8 {
                            assert_eq!(dots[y][x], dots[y][7 - x], "left/right symmetry");
                            assert_eq!(dots[y][x], dots[7 - y][x], "top/bottom symmetry");
                        }
                    }
                    assert_eq!(dots.iter().flatten().filter(|&&dot| dot).count(), 16);
                    assert!(
                        dots[2..6]
                            .iter()
                            .all(|row| row[2..6].iter().all(|dot| !dot))
                    );
                    if let Some(previous) = previous {
                        assert_eq!(dots, previous);
                    }
                    previous = Some(dots);
                }
            }
        }
    }
}

#[test]
fn pages_follow_selection_and_only_multiple_pages_get_a_note() {
    for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let layout = Layout::new(body(width, height), 1000, 0);
        let capacity = layout.columns * layout.visible_rows;
        for count in [1, capacity, capacity + 1, capacity * 2 + 3] {
            let mut tend = tend(count);
            for selected in 0..count {
                tend.selected = selected;
                let buffer = paint(&mut tend, width, height, true);
                let layout = Layout::new(body(width, height), count, selected);
                let cell = layout.cell(selected);
                assert!(line(&buffer, cell.y).contains('›'));
                let selected_line = line(&buffer, height - 3);
                assert_eq!(selected_line.contains("page"), count > capacity);
                if count > capacity {
                    assert!(selected_line.ends_with(&format!(
                        " · page {} of {}",
                        layout.page + 1,
                        layout.pages
                    )));
                }
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
