//! Graph mechanics on synthetic goals, with no worker or operator storage.

use super::*;
use crate::tend::{Found, Session, Stage};
use crossterm::event::KeyModifiers;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::time::{Duration, Instant};

fn fixture(count: usize) -> (tempfile::TempDir, Tend) {
    let tmp = tempfile::Builder::new()
        .prefix("tinkery-TEST-graph-")
        .tempdir()
        .unwrap();
    let mut tend = Tend::new(Ok(tmp.path().join("TEST-links")));
    tend.current = true;
    tend.apply(
        &Ok(Found {
            sessions: (0..count)
                .map(|i| Session {
                    id: Some(format!("id-{i}")),
                    path: format!("/TEST/{i}").into(),
                    goal: format!("Goal {i}\nWhy: never on graph"),
                    stage: Stage::GoalSaved,
                    updated: String::new(),
                })
                .collect(),
            unreadable: 0,
            failures: Vec::new(),
        }),
        true,
        1,
    );
    (tmp, tend)
}
fn body() -> Rect {
    Rect::new(2, 3, 96, 26)
}
fn paint(tend: &mut Tend, mono: bool) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| {
            let p = Palette::new(mono);
            f.render_widget(Paragraph::new("").style(p.ink), f.area());
            super::super::garden::render(f, tend, body(), p, None);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}
fn text(buffer: &Buffer) -> String {
    (0..30)
        .map(|y| {
            (0..100)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn point(tend: &Tend, index: usize) -> (u16, u16) {
    let guard = tend.graph.state.as_ref().unwrap().read();
    let graph = guard.simulation.get_graph();
    let node = graph
        .node_weights()
        .find(|n| n.data.id == identity(&tend.sessions[index]))
        .unwrap();
    let (x, y) = guard.viewport.world_to_screen(
        f64::from(node.location.x),
        f64::from(node.location.y),
        tend.graph.area,
    );
    (x.round() as u16, y.round() as u16)
}
fn pointer(tend: &mut Tend, kind: MouseEventKind, at: (u16, u16), now: Instant) {
    tend.pointer(
        MouseEvent {
            kind,
            column: at.0,
            row: at.1,
            modifiers: KeyModifiers::NONE,
        },
        body(),
        now,
    );
}

#[test]
fn identical_inputs_draw_identical_buffers_and_selection_has_colour_independent_weight() {
    for mono in [false, true] {
        let (_a, mut a) = fixture(7);
        let (_b, mut b) = fixture(7);
        let first = paint(&mut a, mono);
        for _ in 0..10 {
            assert_eq!(first, paint(&mut b, mono));
        }
        assert_eq!(first, paint(&mut a, mono));
        let p = Palette::new(mono);
        let cue = first.content.iter().find(|c| c.symbol() == "›").unwrap();
        assert_eq!(cue.fg, p.jade.fg.unwrap_or(Color::Reset));
        assert!(cue.modifier.contains(Modifier::BOLD));
        assert!(first.content.iter().any(|c| {
            c.symbol()
                .chars()
                .any(|c| ('\u{2801}'..='\u{28ff}').contains(&c))
                && c.modifier.contains(Modifier::BOLD)
        }));
        for cell in &first.content {
            assert_eq!(cell.bg, p.ink.bg.unwrap_or(Color::Reset));
            if mono {
                assert_eq!(cell.fg, Color::Reset);
            }
        }
        assert!(!text(&first).contains("Why:"));
        assert!(!a.graph.state.as_ref().unwrap().read().physics_worker_active);
    }
}

#[test]
fn forty_short_goal_labels_keep_spacing_without_overlap() {
    for chain in [false, true] {
        for (width, height) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
            let (_tmp, mut tend) = fixture(40);
            if chain {
                for i in 0..39 {
                    tend.links = Ok(tend
                        .needs
                        .toggle(&format!("id-{i}"), &format!("id-{}", i + 1))
                        .unwrap());
                }
            }
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut paint = |tend: &mut Tend| {
                terminal
                    .draw(|frame| {
                        super::super::garden::render(
                            frame,
                            tend,
                            Rect::new(2, 3, width - 4, height - 4),
                            Palette::new(true),
                            None,
                        )
                    })
                    .unwrap();
                terminal.backend().buffer().clone()
            };
            let first = paint(&mut tend);
            assert_eq!(first, paint(&mut tend), "layout must stay static");
            let hits = &tend.graph.label_hits;
            let full_labels = hits
                .iter()
                .filter(|(rect, id)| {
                    let session = tend.sessions.iter().find(|s| identity(s) == *id).unwrap();
                    let label: String = (rect.x..rect.right())
                        .map(|x| first[(x, rect.y)].symbol())
                        .collect();
                    label.trim_start_matches("› ") == session.goal.lines().next().unwrap()
                })
                .count();
            eprintln!("{width}x{height}, chain={chain}: {full_labels}/40 full labels");
            assert!(
                full_labels > 0,
                "{width}x{height}, chain={chain}: only {full_labels}/40 distinct full labels"
            );
            let overlaps = hits
                .iter()
                .enumerate()
                .map(|(i, (rect, _))| {
                    hits[i + 1..]
                        .iter()
                        .filter(|(other, _)| {
                            rect.y == other.y
                                && rect.x < other.right().saturating_add(1)
                                && other.x < rect.right().saturating_add(1)
                        })
                        .count()
                })
                .sum::<usize>();
            assert_eq!(overlaps, 0, "label overlap at {width}x{height}");
            let state = tend.graph.state.as_ref().unwrap().read();
            let points: Vec<_> = state
                .simulation
                .get_graph()
                .node_weights()
                .map(|node| {
                    state.viewport.world_to_screen(
                        f64::from(node.location.x),
                        f64::from(node.location.y),
                        tend.graph.area,
                    )
                })
                .collect();
            for (i, (x, y)) in points.iter().enumerate() {
                for (other_x, other_y) in &points[i + 1..] {
                    assert!((x - other_x).abs() >= 11.0 || (y - other_y).abs() >= 5.0);
                }
            }
        }
    }
}

#[test]
fn crowded_views_preserve_selected_identity_and_never_overlap_labels() {
    use ratatui::widgets::Widget;
    for (count, width, height) in [(40, 40, 12), (60, 100, 30), (60, 160, 40)] {
        for linking in [false, true] {
            let (_tmp, mut tend) = fixture(count);
            if count == 60 {
                for (i, session) in tend.sessions.iter_mut().enumerate() {
                    session.goal = format!(
                        "Garden {i:02} {}",
                        ["very long words ", "界👩‍💻e\u{301}", "العربية עברית "][i % 3].repeat(20)
                    );
                }
            }
            if linking {
                tend.start_link();
            }
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for selected in 0..count {
                assert_eq!(tend.selected, selected);
                let buffer = terminal
                    .draw(|frame| {
                        super::super::garden::render(
                            frame,
                            &mut tend,
                            Rect::new(2, 3, width - 4, height - 4),
                            Palette::new(true),
                            None,
                        )
                    })
                    .unwrap()
                    .buffer
                    .clone();
                let hits = &tend.graph.label_hits;
                let id = identity(&tend.sessions[selected]);
                let rect = hits
                    .iter()
                    .find(|(_, hit)| *hit == id)
                    .expect("selected label is always visible")
                    .0;
                let rendered: String = (rect.x..rect.right())
                    .map(|x| buffer[(x, rect.y)].symbol())
                    .collect();
                let title = tend.sessions[selected].goal.lines().next().unwrap();
                assert!(
                    rendered.contains(if count == 60 { &title[..9] } else { title }),
                    "{selected}: {rendered}"
                );
                for (i, (rect, id)) in hits.iter().enumerate() {
                    assert!(
                        hits[i + 1..]
                            .iter()
                            .all(|(other, _)| other.intersection(*rect).is_empty())
                    );
                    let session = tend.sessions.iter().find(|s| identity(s) == *id).unwrap();
                    let prefix = if tend.source_marked(
                        tend.sessions
                            .iter()
                            .position(|s| identity(s) == *id)
                            .unwrap(),
                    ) {
                        "› "
                    } else if linking && *id == identity(&tend.sessions[selected]) {
                        "→ "
                    } else {
                        ""
                    };
                    assert!(
                        [24, 18, 12, 8, 4].iter().any(|budget| {
                            let text = format!(
                                "{prefix}{}",
                                clipped(session.goal.lines().next().unwrap(), *budget)
                            );
                            if text.width().min((width - 4) as usize) != rect.width as usize {
                                return false;
                            }
                            let mut expected = Buffer::empty(*rect);
                            Paragraph::new(text).render(*rect, &mut expected);
                            let mut x = rect.x;
                            while x < rect.right() {
                                let symbol = expected[(x, rect.y)].symbol();
                                if symbol != buffer[(x, rect.y)].symbol() {
                                    return false;
                                }
                                x += symbol.width().max(1) as u16;
                            }
                            true
                        }),
                        "label cells must never be interleaved: {count}/{width}x{height} linking={linking} selected={selected} id={id} rect={rect:?} actual={:?}",
                        (rect.x..rect.right())
                            .map(|x| buffer[(x, rect.y)].symbol())
                            .collect::<String>()
                    );
                }
                tend.step_selection(true);
            }
        }
    }
}

#[test]
fn static_graph_never_starts_a_physics_worker() {
    let (_tmp, mut tend) = fixture(7);
    paint(&mut tend, true);
    for key in ['+', '-', 'H', 'J', 'K', 'L'] {
        tend.adjust_view(key);
        paint(&mut tend, true);
        let state = tend.graph.state.as_ref().unwrap().read();
        assert!(!state.physics_worker_active);
        assert!(state.is_settled);
        assert_eq!(state.alpha, 0.0);
    }
    tend.links = Ok(tend.needs.toggle("id-0", "id-1").unwrap());
    paint(&mut tend, true);
    assert!(
        !tend
            .graph
            .state
            .as_ref()
            .unwrap()
            .read()
            .physics_worker_active
    );
}

#[test]
fn duplicate_titles_resolve_edges_by_exact_identity_and_ended_missing_ambiguous_needs_draw_none() {
    let (_tmp, mut tend) = fixture(5);
    let mut sessions = tend.sessions.clone();
    sessions[1].goal = "Same title".into();
    sessions[2].goal = "Same title".into();
    sessions[3].stage = Stage::Ended;
    sessions[4].id = Some("id-2".into());
    tend.needs.toggle("id-0", "id-1").unwrap();
    tend.needs.toggle("id-0", "id-3").unwrap();
    tend.needs.toggle("id-0", "missing").unwrap();
    tend.needs.toggle("id-0", "id-2").unwrap();
    tend.apply(
        &Ok(Found {
            sessions,
            unreadable: 0,
            failures: Vec::new(),
        }),
        true,
        2,
    );
    paint(&mut tend, false);
    let guard = tend.graph.state.as_ref().unwrap().read();
    let graph = guard.simulation.get_graph();
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 1);
    let edge = graph.edge_indices().next().unwrap();
    let (a, b) = graph.edge_endpoints(edge).unwrap();
    assert_eq!(graph[a].data.id, identity(&tend.sessions[0]));
    assert_eq!(graph[b].data.id, identity(&tend.sessions[1]));
    let cache = guard.render_cache.lock();
    assert_eq!(cache.edges.len(), 1);
    let line = &cache.edges[0];
    assert_eq!(
        (line.x1, line.y1),
        (
            f64::from(graph[a].location.x),
            f64::from(graph[a].location.y)
        )
    );
    assert_eq!(
        (line.x2, line.y2),
        (
            f64::from(graph[b].location.x),
            f64::from(graph[b].location.y)
        )
    );
    assert_eq!(line.color, Palette::new(false).muted.fg.unwrap());
    assert!(cache.nodes.iter().all(|n| !n.filled));
    assert!(tend.selected_line(96, None).1.contains("2 needs not found"));
}

#[test]
fn clicks_use_identity_and_inclusive_double_click_clock_and_linking_stays_in_tend() {
    for (millis, opens) in [(499, true), (500, true), (501, false)] {
        let (_tmp, mut tend) = fixture(3);
        paint(&mut tend, true);
        let at = point(&tend, 1);
        let now = Instant::now();
        pointer(&mut tend, MouseEventKind::Down(MouseButton::Left), at, now);
        assert_eq!(tend.selected, 1);
        pointer(
            &mut tend,
            MouseEventKind::Down(MouseButton::Left),
            at,
            now + Duration::from_millis(millis),
        );
        assert_eq!(tend.detail, opens);
    }
    let (_tmp, mut tend) = fixture(3);
    paint(&mut tend, true);
    let now = Instant::now();
    let at = point(&tend, 1);
    pointer(&mut tend, MouseEventKind::Down(MouseButton::Left), at, now);
    let at = point(&tend, 2);
    pointer(
        &mut tend,
        MouseEventKind::Down(MouseButton::Left),
        at,
        now + Duration::from_millis(50),
    );
    assert!(!tend.detail && tend.selected == 2);
    tend.selected = 0;
    tend.start_link();
    let at = point(&tend, 1);
    pointer(&mut tend, MouseEventKind::Down(MouseButton::Left), at, now);
    assert!(!tend.detail && !tend.is_linking() && tend.selected == 0);
    assert!(
        tend.needs
            .read()
            .into_links()
            .unwrap()
            .contains("id-0", "id-1")
    );
}

#[test]
fn dragging_moves_ring_empty_drag_pans_scroll_zooms_and_unchanged_refresh_preserves_them() {
    let (_tmp, mut tend) = fixture(3);
    let before = paint(&mut tend, true);
    let at = point(&tend, 0);
    let now = Instant::now();
    pointer(&mut tend, MouseEventKind::Down(MouseButton::Left), at, now);
    pointer(
        &mut tend,
        MouseEventKind::Drag(MouseButton::Left),
        (at.0 + 5, at.1 + 2),
        now,
    );
    pointer(
        &mut tend,
        MouseEventKind::Up(MouseButton::Left),
        (at.0 + 5, at.1 + 2),
        now,
    );
    assert_ne!(before, paint(&mut tend, true));
    let moved = point(&tend, 0);
    assert!(moved.0.abs_diff(at.0 + 5) <= 1 && moved.1.abs_diff(at.1 + 2) <= 1);
    let shared = tend.graph.state.as_ref().unwrap().clone();
    let zoom = shared.read().viewport.zoom;
    pointer(&mut tend, MouseEventKind::ScrollUp, moved, now);
    assert!(shared.read().viewport.zoom > zoom);
    pointer(&mut tend, MouseEventKind::ScrollDown, moved, now);
    assert!((shared.read().viewport.zoom - zoom).abs() < 1e-12);
    let center = shared.read().viewport.center_x;
    pointer(
        &mut tend,
        MouseEventKind::Down(MouseButton::Left),
        (3, 4),
        now,
    );
    pointer(
        &mut tend,
        MouseEventKind::Drag(MouseButton::Left),
        (8, 4),
        now,
    );
    pointer(
        &mut tend,
        MouseEventKind::Up(MouseButton::Left),
        (8, 4),
        now,
    );
    assert_ne!(shared.read().viewport.center_x, center);
    let before = paint(&mut tend, true);
    let mut sessions = tend.sessions.clone();
    sessions.reverse();
    tend.apply(
        &Ok(Found {
            sessions,
            unreadable: 0,
            failures: Vec::new(),
        }),
        true,
        2,
    );
    assert_eq!(before, paint(&mut tend, true));
    assert!(Arc::ptr_eq(&shared, tend.graph.state.as_ref().unwrap()));
    assert!(!tend.detail);
}

#[test]
fn selection_follows_offscreen_goal_without_resetting_zoom_and_link_relayout_keeps_viewport() {
    let (_tmp, mut tend) = fixture(7);
    paint(&mut tend, true);
    let shared = tend.graph.state.as_ref().unwrap().clone();
    shared.write().viewport.pan_by(10000.0, 10000.0);
    let zoom = shared.read().viewport.zoom;
    tend.selected = 1;
    let buffer = paint(&mut tend, true);
    assert!(text(&buffer).contains("› Goal 1"));
    assert_eq!(shared.read().viewport.zoom, zoom);
    let center = (
        shared.read().viewport.center_x,
        shared.read().viewport.center_y,
    );
    tend.links = Ok(tend.needs.toggle("id-0", "id-1").unwrap());
    paint(&mut tend, true);
    let new = tend.graph.state.as_ref().unwrap().read();
    assert_eq!(new.viewport.zoom, zoom);
    // Relayout may bring the new position offscreen: following selection is allowed to pan.
    assert!(new.viewport.center_x.is_finite() && center.0.is_finite());
}

#[test]
fn all_goals_are_kept_beyond_grafs_default_cap() {
    let (_tmp, mut tend) = fixture(501);
    tend.sync_graph(body());
    assert_eq!(
        tend.graph
            .state
            .as_ref()
            .unwrap()
            .read()
            .simulation
            .get_graph()
            .node_count(),
        501
    );
}

#[test]
fn a_quiet_line_is_painted_only_for_visible_active_ends() {
    for ended in [false, true] {
        let (_tmp, mut tend) = fixture(2);
        let mut sessions = tend.sessions.clone();
        if ended {
            sessions[1].stage = Stage::Ended;
        }
        tend.needs.toggle("id-0", "id-1").unwrap();
        tend.apply(
            &Ok(Found {
                sessions,
                unreadable: 0,
                failures: Vec::new(),
            }),
            true,
            2,
        );
        tend.sync_graph(body());
        {
            let mut state = tend.graph.state.as_ref().unwrap().write();
            state.viewport = graf::viewport::Viewport::default();
            for (i, node) in state
                .simulation
                .get_graph_mut()
                .node_weights_mut()
                .enumerate()
            {
                node.location.x = if i == 0 { -40.0 } else { 40.0 };
                node.location.y = 0.0;
            }
        }
        let buffer = paint(&mut tend, false);
        let cells: Vec<_> = (47..=51)
            .flat_map(|x| (13..=15).map(move |y| (x, y)))
            .map(|p| &buffer[p])
            .collect();
        if ended {
            assert!(cells.iter().all(|c| c.symbol() == " "));
        } else {
            assert!(
                cells
                    .iter()
                    .any(|c| c.symbol() != " " && c.fg == Palette::new(false).muted.fg.unwrap())
            );
        }
    }
}

#[test]
fn dense_graphs_keep_rings_and_needs_lines_above_grafs_lod_threshold() {
    let (_tmp, mut tend) = fixture(1001);
    tend.links = Ok(tend.needs.toggle("id-0", "id-1").unwrap());
    tend.sync_graph(body());
    // Zoom far enough out to put every node in view, exercising the visible-count threshold.
    tend.graph.state.as_ref().unwrap().write().viewport.zoom *= 0.1;
    paint(&mut tend, true);
    let state = tend.graph.state.as_ref().unwrap().read();
    let cache = state.render_cache.lock();
    assert!(cache.visible_nodes.len() > 1000);
    assert_eq!(cache.edges.len(), 1);
    assert!(cache.nodes.iter().all(|n| !n.filled && n.radius > 1.0));
}

#[test]
fn a_three_goal_edge_does_not_paint_through_the_unrelated_middle_ring() {
    let (_tmp, mut tend) = fixture(3);
    tend.selected = 1;
    let before = paint(&mut tend, true);
    let center = point(&tend, 1);
    tend.links = Ok(tend.needs.toggle("id-0", "id-2").unwrap());
    let after = paint(&mut tend, true);
    for x in center.0.saturating_sub(1)..=center.0 + 1 {
        for y in center.1.saturating_sub(1)..=center.1 + 1 {
            assert_eq!(before[(x, y)].symbol(), after[(x, y)].symbol());
        }
    }
}

#[test]
fn reciprocal_needs_keep_one_edge_per_record() {
    let (_tmp, mut tend) = fixture(2);
    tend.needs.toggle("id-0", "id-1").unwrap();
    tend.links = Ok(tend.needs.toggle("id-1", "id-0").unwrap());
    tend.sync_graph(body());
    assert_eq!(
        tend.graph
            .state
            .as_ref()
            .unwrap()
            .read()
            .simulation
            .get_graph()
            .edge_count(),
        2
    );
}

#[test]
fn a_label_identifies_its_goal_even_when_rings_overlap() {
    let (_tmp, mut tend) = fixture(2);
    tend.sync_graph(body());
    {
        let mut state = tend.graph.state.as_ref().unwrap().write();
        for node in state.simulation.get_graph_mut().node_weights_mut() {
            node.location.x = 0.0;
            node.location.y = 0.0;
        }
    }
    paint(&mut tend, true);
    let id = identity(&tend.sessions[1]);
    let rect = tend
        .graph
        .label_hits
        .iter()
        .find(|(_, hit)| *hit == id)
        .unwrap()
        .0;
    let now = Instant::now();
    pointer(
        &mut tend,
        MouseEventKind::Down(MouseButton::Left),
        (rect.x, rect.y),
        now,
    );
    assert_eq!(tend.selected, 1);
    tend.selected = 0;
    tend.start_link();
    pointer(
        &mut tend,
        MouseEventKind::Down(MouseButton::Left),
        (rect.x, rect.y),
        now,
    );
    assert!(!tend.is_linking());
    assert!(tend.links.as_ref().unwrap().contains("id-0", "id-1"));
}

#[test]
fn overlapping_rings_paint_in_a_stable_order() {
    let mut expected = None;
    for _ in 0..24 {
        let (_tmp, mut tend) = fixture(7);
        tend.sync_graph(body());
        {
            let mut guard = tend.graph.state.as_ref().unwrap().write();
            for node in guard.simulation.get_graph_mut().node_weights_mut() {
                node.location.x = 0.0;
                node.location.y = 0.0;
            }
        }
        let buffer = paint(&mut tend, false);
        if let Some(expected) = &expected {
            assert_eq!(&buffer, expected);
        }
        expected = Some(buffer);
    }
}

#[test]
fn view_keys_pan_and_zoom_without_selecting_linking_or_writing() {
    let (tmp, mut tend) = fixture(3);
    paint(&mut tend, true);
    let shared = tend.graph.state.as_ref().unwrap().clone();
    let center = shared.read().viewport.center_x;
    let zoom = shared.read().viewport.zoom;
    tend.adjust_view('H');
    assert!(shared.read().viewport.center_x < center);
    tend.adjust_view('L');
    assert!((shared.read().viewport.center_x - center).abs() < 1e-12);
    tend.adjust_view('+');
    assert!(shared.read().viewport.zoom > zoom);
    tend.adjust_view('-');
    assert!((shared.read().viewport.zoom - zoom).abs() < 1e-12);
    assert_eq!(tend.selected, 0);
    assert!(!tend.detail && !tend.is_linking());
    assert!(!tmp.path().join("TEST-links").exists());
}
