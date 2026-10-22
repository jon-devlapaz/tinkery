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

fn screen(view: &mut Measure, width: u16, height: u16, no_color: bool) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| view.render(f, f.area(), crate::Palette::new(no_color), false))
        .unwrap();
    terminal
        .backend()
        .buffer()
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
    assert!(screen(&mut v, 80, 24, true).contains("Partial results: first 100 PRs"));
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
fn titles_cannot_inject_terminal_controls() {
    assert_eq!(visible("hello\x1b[31m\nworld"), "hello [31m world");
}
