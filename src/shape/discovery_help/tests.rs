//! Isolated Shape F1 contracts; no prompt, provider or Seed Me helper is involved.

use super::*;
use ratatui::{Terminal, backend::TestBackend};

fn failures() -> Vec<ReadFailure> {
    (0..MAX_READ_FAILURES + 3)
        .map(|index| {
            ReadFailure::new(
                format!("/TEST-sessions/TEST-session-{index}/ledger.json").into(),
                format!(
                    "Invalid JSON: TEST reason {index} {}",
                    "long reason ".repeat(60)
                ),
            )
        })
        .collect()
}

#[test]
fn help_caps_samples_and_discloses_the_full_remaining_count() {
    let mut shape = Shape::list();
    shape.offer(Ok(&[]), MAX_READ_FAILURES + 3);
    shape.offer_failures(&failures());
    let help = shape.help_text();
    assert_eq!(shape.failures.len(), MAX_READ_FAILURES);
    assert_eq!(help.matches("ledger.json").count(), MAX_READ_FAILURES);
    assert!(
        help.contains("3 more unreadable records not listed."),
        "{help}"
    );
    assert!(!help.contains("Tend shows"));
    assert!(
        shape
            .failures
            .iter()
            .all(|failure| failure.reason.chars().count() <= 513)
    );
    shape.offer(Ok(&[]), MAX_READ_FAILURES + 1);
    shape.offer_failures(&failures());
    assert!(
        shape
            .help_text()
            .contains("1 more unreadable record not listed.")
    );
}

#[test]
fn fresh_and_failed_reads_clear_old_diagnostics() {
    let mut shape = Shape::list();
    shape.offer(Ok(&[]), 11);
    shape.offer_failures(&failures());
    shape.offer(Err("TEST root unavailable".into()), 11);
    assert!(shape.failures.is_empty());
    assert!(!shape.help_text().contains("ledger.json"));
    shape.offer_failures(&failures());
    assert!(
        shape.failures.is_empty(),
        "failed roots accept no stale per-file samples"
    );
    shape.offer(Ok(&[]), 0);
    shape.offer_failures(&failures());
    assert!(shape.failures.is_empty());
    assert!(!shape.help_text().contains("Unreadable session records"));
}

#[test]
fn paths_and_reasons_escape_controls_only_in_the_display_copy() {
    let failure = ReadFailure::new(
        "/TEST/line\n\x1b[31m\u{202e}/ledger.json".into(),
        "TEST error\r\x1b[2J",
    );
    let original = failure.clone();
    let mut shape = Shape::list();
    shape.offer(Ok(&[]), 1);
    shape.offer_failures(&[failure]);
    let help = shape.help_text();
    assert!(help.contains("line\\n\\u{1b}"), "{help:?}");
    assert!(help.contains("TEST error\\r\\u{1b}"), "{help:?}");
    assert!(!help.contains('\x1b'));
    assert!(!help.contains('\u{202e}'));
    assert_eq!(shape.failures[0], original);
}

#[test]
fn capped_help_can_scroll_to_omission_notice_with_fixed_exit_keys() {
    for (width, height) in [(80, 24), (100, 30)] {
        let mut shape = Shape::list();
        shape.offer(Ok(&[]), 11);
        shape.offer_failures(&failures());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        shape.scroll_help(KeyCode::End);
        terminal
            .draw(|frame| shape.render_help(frame, frame.area(), Palette::new(true)))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            text.contains("3 more unreadable records not listed."),
            "{text}"
        );
        assert!(
            text.contains("PgUp/PgDn scroll · F1 close · Esc back"),
            "{text}"
        );
        assert!(shape.help_scroll > 0);
        shape.reset_help();
        assert_eq!(shape.help_scroll, 0);
    }
}
