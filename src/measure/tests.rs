//! Synthetic PR evidence and rendering contracts; no commands or network.

use super::*;
use ratatui::{Terminal, backend::TestBackend};
use serde_json::json;

fn pr(number: u64, state: &str) -> serde_json::Value {
    json!({"number":number,"title":"An exact PR title","state":state,
        "url":format!("https://github.com/TEST/repo/pull/{number}"),"author":{"login":"tester"},
        "createdAt":"2026-01-01T00:00:00Z","updatedAt":"2026-01-02T00:00:00Z",
        "closedAt":if state == "OPEN" { None } else { Some("2026-01-02T00:00:00Z") },
        "mergedAt":if state == "MERGED" { Some("2026-01-02T00:00:00Z") } else { None },
        "headRefOid":"abc123","statusCheckRollup":null})
}

fn buffer(
    view: &mut Measure,
    width: u16,
    height: u16,
    no_color: bool,
    help: bool,
) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| view.render(f, f.area(), crate::Palette::new(no_color), help))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn screen(view: &mut Measure, width: u16, height: u16, no_color: bool) -> String {
    buffer(view, width, height, no_color, false)
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>()
}

fn view(root: &std::path::Path) -> Measure {
    std::fs::create_dir_all(root.join(".git")).unwrap();
    Measure::open(Repo::enclosing(root))
}

#[test]
fn states_remain_source_states_not_acceptance() {
    let report =
        parse(&json!([pr(1, "OPEN"), pr(2, "MERGED"), pr(3, "CLOSED")]).to_string()).unwrap();
    assert_eq!(
        report
            .prs
            .iter()
            .map(|p| p.state.label())
            .collect::<Vec<_>>(),
        ["open", "merged", "closed-unmerged"]
    );
    assert!(!report.partial);
}

#[test]
fn capped_results_are_never_complete() {
    let prs: Vec<_> = (1..=101).map(|n| pr(n, "OPEN")).collect();
    let report = parse(&serde_json::to_string(&prs).unwrap()).unwrap();
    assert!(report.partial);
    assert_eq!(report.prs.len(), 100);
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    v.since = "2026-01-01".into();
    v.report = Some(report);
    assert!(screen(&mut v, 80, 24, true).contains("1–7 of 100 PRs · more on GitHub"));
}

#[test]
fn malformed_unknown_duplicate_and_inconsistent_records_refuse() {
    for value in [
        json!({}),
        json!([pr(1, "ALIEN")]),
        json!([pr(1, "OPEN"), pr(1, "OPEN")]),
    ] {
        assert!(parse(&value.to_string()).is_err());
    }
    let mut value = pr(1, "MERGED");
    value["mergedAt"] = serde_json::Value::Null;
    assert!(parse(&json!([value]).to_string()).is_err());
    let mut value = pr(1, "CLOSED");
    value["closedAt"] = serde_json::Value::Null;
    assert!(parse(&json!([value]).to_string()).is_err());
    let mut value = pr(1, "OPEN");
    value["updatedAt"] = json!("not a date");
    assert!(parse(&json!([value]).to_string()).is_err());
    assert!(parse("[").is_err());
}

#[test]
fn opening_painting_and_navigation_never_start_a_read() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for nc in [true, false] {
            assert!(screen(&mut v, w, h, nc).contains("Nothing has been read yet"));
            for code in [KeyCode::Enter, KeyCode::Down, KeyCode::Up] {
                v.key(KeyEvent::new(code, KeyModifiers::NONE));
                assert!(v.read.is_none());
            }
        }
    }
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        v.key(KeyEvent::new(KeyCode::Char('r'), modifiers));
    }
    let mut repeat = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    v.key(repeat);
    assert!(v.read.is_none());
}

#[test]
fn details_keep_checks_unknown_and_escape_returns_to_selection() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    v.report = Some(parse(&json!([pr(1, "OPEN"), pr(2, "MERGED")]).to_string()).unwrap());
    v.since = "2026-01-01".into();
    v.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    v.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        for nc in [true, false] {
            let text = screen(&mut v, w, h, nc);
            assert!(text.contains("#2 · merged"));
            assert!(text.contains("Checks unavailable or not recorded"));
            assert!(text.contains("Not merge-commit checks or operator acceptance."));
        }
    }
    assert!(!v.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    assert!(!v.detail && v.selected == 1 && v.read.is_none());
    assert!(v.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
}

#[test]
fn empty_results_are_distinct_from_unread_results() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    assert!(!screen(&mut v, 80, 24, true).contains("No PRs found"));
    v.since = "2026-01-01".into();
    v.report = Some(parse("[]").unwrap());
    assert!(screen(&mut v, 80, 24, true).contains("No PRs found"));
}

#[test]
fn long_details_scroll_to_the_last_reported_check() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    let mut value = pr(1, "OPEN");
    value["title"] = json!("A long title that wraps on a narrow terminal. ".repeat(5));
    value["statusCheckRollup"] = json!(
        (0..40)
            .map(|i| json!({"name":format!("check-{i}"),"state":"PENDING"}))
            .collect::<Vec<_>>()
    );
    v.report = Some(parse(&json!([value]).to_string()).unwrap());
    v.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for _ in 0..100 {
        v.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    let text = screen(&mut v, 80, 24, true);
    assert!(text.contains("check-39: PENDING"), "{text}");
    assert!(text.contains("esc list"));
}

#[test]
fn list_stays_short_with_aligned_muted_metadata_and_reachable_last_row() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    v.report = Some(
        parse(
            &json!(
                (1..=100)
                    .map(|n| pr(n, if n % 2 == 0 { "MERGED" } else { "OPEN" }))
                    .collect::<Vec<_>>()
            )
            .to_string(),
        )
        .unwrap(),
    );
    v.since = "2026-01-01".into();
    for (w, h) in [(70, 49), (80, 24), (100, 30), (160, 40), (320, 40)] {
        for nc in [false, true] {
            v.selected = 0;
            let b = buffer(&mut v, w, h, nc, false);
            let rows: Vec<String> = b
                .content
                .chunks(w as usize)
                .map(|row| row.iter().map(|c| c.symbol()).collect())
                .collect();
            assert_eq!(
                rows.iter()
                    .filter(|line| line.contains("An exact PR title"))
                    .count(),
                7
            );
            assert!(
                rows[9..(h as usize - 2)]
                    .iter()
                    .all(|line| line.trim().is_empty())
            );
            assert!(rows[0].contains("1–7 of 100 PRs"));
            assert!(rows[2].find("An exact PR title") == rows[3].find("An exact PR title"));
            if nc {
                assert!(b[(2, 2)].modifier.contains(ratatui::style::Modifier::DIM));
                assert!(b[(16, 2)].modifier.contains(ratatui::style::Modifier::BOLD));
            } else {
                assert_ne!(b[(2, 2)].fg, b[(16, 2)].fg);
            }
            for _ in 0..110 {
                v.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
            }
            let text = screen(&mut v, w, h, nc);
            assert!(text.contains("94–100 of 100 PRs") && text.contains("> #100"));
            assert_eq!(v.report.as_ref().unwrap().prs.len(), 100);
            assert!(v.read.is_none());
        }
    }
}

#[test]
fn selected_title_wraps_and_pages_without_losing_selection_or_original() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    let original = format!(
        "{} LAST-TITLE-WORD",
        "Unicode 漢字 e\u{301} words remain intact. ".repeat(80)
    );
    let mut item = pr(1, "OPEN");
    item["title"] = json!(original);
    v.report = Some(parse(&json!([item, pr(2, "CLOSED")]).to_string()).unwrap());
    for (w, h) in [(70, 49), (80, 24), (100, 30), (160, 40), (320, 40)] {
        for nc in [false, true] {
            v.scroll = 0;
            let text = screen(&mut v, w, h, nc);
            assert!(text.contains("pg↑↓ title"), "{text}");
            assert!(!text.contains("LAST-TITLE-WORD"));
            for _ in 0..100 {
                v.key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
            }
            assert!(screen(&mut v, w, h, nc).contains("LAST-TITLE-WORD"));
            assert_eq!(v.selected, 0);
            assert_eq!(v.report.as_ref().unwrap().prs[0].title, original);
            assert!(v.read.is_none());
        }
    }
    v.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(v.scroll, 0);
    assert!(!screen(&mut v, 80, 24, true).contains("pg↑↓ title"));
}

#[test]
fn read_provenance_moves_to_help_and_details_not_the_list() {
    let root = tempfile::tempdir().unwrap();
    let mut v = view(root.path());
    v.since = "2026-01-01".into();
    v.read_at = "2026-01-31 12:34 UTC".into();
    v.report = Some(parse(&json!([pr(1, "OPEN")]).to_string()).unwrap());
    assert!(!screen(&mut v, 80, 24, true).contains("12:34"));
    let help = buffer(&mut v, 80, 20, true, true)
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(
        help.contains("Window: updated since 2026-01-01 UTC"),
        "{help}"
    );
    assert!(help.contains("2026-01-31 12:34 UTC"), "{help}");
    v.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    v.scroll = u16::MAX;
    assert!(screen(&mut v, 80, 24, true).contains("2026-01-31 12:34 UTC"));
}

#[test]
fn unselected_clipping_is_explicit_and_keeps_graphemes_whole() {
    assert_eq!(clipped("漢e\u{301}abc", 4), "漢e\u{301}…");
    assert_eq!(clipped("short", 5), "short");
    assert_eq!(clipped("long", 0), "");
}

#[test]
fn titles_cannot_inject_terminal_controls() {
    assert_eq!(visible("hello\x1b[31m\nworld"), "hello [31m world");
}
