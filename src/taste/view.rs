//! Taste's visual hierarchy: compact result counts, exact source words in headed detail,
//! and separately requested evidence. Presentation never changes source facts or copy payloads.

use super::{
    goal,
    next::Pause,
    read::{Item, ResultRow},
    safe,
};
use crate::{Palette, sdlc::Goal};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Passed,
    Failed,
    Stale,
    Waiting,
    Unknown,
}

fn state(item: &Item) -> State {
    match item.state.as_str() {
        "pass" | "pass (may need recheck)" => State::Passed,
        "fail" => State::Failed,
        "stale" => State::Stale,
        s if s.starts_with("pending") => State::Waiting,
        _ => State::Unknown,
    }
}

fn style(state: State, p: Palette) -> Style {
    match state {
        State::Failed => p.rust,
        State::Passed => p.ink,
        _ => p.muted,
    }
}

fn status(item: &Item) -> String {
    match state(item) {
        State::Passed if item.state == "pass" => "✓ passed".into(),
        State::Passed => "✓ passed; may need recheck".into(),
        State::Failed => "! failed".into(),
        State::Stale => "… stale".into(),
        State::Unknown if item.state == "unknown (none reported)" => {
            "? unknown: no CI reported".into()
        }
        State::Unknown => "? unknown".into(),
        State::Waiting => format!(
            "… waiting: {}",
            match item.state.as_str() {
                "pending (matching evidence missing)"
                | "pending (attestation evidence missing)" => "no record found",
                "pending (inspect externally)" => "review externally",
                "pending (not activated)" => "not activated",
                "pending (result revision unknown)" => "revision unknown",
                "pending (failure revision unknown or different)" =>
                    "failure revision differs or is unknown",
                "pending (evidence unavailable)" => "evidence unavailable",
                _ => "not finished",
            }
        ),
    }
}

fn owed_item(row: &ResultRow, item: &Item) -> bool {
    item.kind == "human attestation"
        && row.gates.iter().any(|g| {
            g == &item.label || g.strip_prefix("may need recheck: ") == Some(item.label.as_str())
        })
}

fn preview(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    let mut result = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        if used + g.width() + 1 > width {
            break;
        }
        result.push_str(g);
        used += g.width();
    }
    if width > 0 {
        result.push('…');
    }
    result
}

/// One compact block per result, with jade selection and no check wording or technical IDs.
pub(super) fn list(
    rows: &[ResultRow],
    selected: usize,
    width: u16,
    p: Palette,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let suffix = if row.merged {
            " · merged"
        } else if row.head.is_some() {
            " · open"
        } else {
            " · result unknown"
        };
        let title = safe(goal(&row.goal).split('\n').next().unwrap_or(""));
        lines.push(Line::from(vec![
            Span::styled(if index == selected { "› " } else { "  " }, p.jade),
            Span::styled(
                preview(
                    &title,
                    usize::from(width).saturating_sub(2 + suffix.width()),
                ),
                p.ink,
            ),
            Span::styled(suffix, p.muted),
        ]));
        let mut counts = vec![Span::raw("  ")];
        for (s, glyph, word) in [
            (State::Passed, "✓", "passed"),
            (State::Waiting, "…", "waiting"),
            (State::Stale, "…", "stale"),
            (State::Failed, "!", "failed"),
            (State::Unknown, "?", "unknown"),
        ] {
            let count = row.items.iter().filter(|item| state(item) == s).count();
            if count > 0 {
                counts.push(Span::styled(
                    format!("{glyph} {count} {word}  "),
                    style(s, p),
                ));
            }
        }
        if row.items.is_empty() {
            counts.push(Span::styled("? no checks reported  ", p.muted));
        }
        if !matches!(row.goal, Goal::Matched(_)) {
            counts.push(Span::styled("? goal unknown  ", p.muted));
        }
        if !row.gates.is_empty() {
            counts.push(Span::styled(format!("You owe {}", row.gates.len()), p.jade));
        }
        lines.push(Line::from(counts));
        lines.push(Line::default());
    }
    lines
}

// Preserve every displayed grapheme, including spaces and explicit newlines. Only layout wraps.
fn chunks(text: &str, width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    for logical in text.split('\n') {
        let display = safe(logical);
        let mut rest = display.as_str();
        if rest.is_empty() {
            rows.push(String::new());
        }
        while !rest.is_empty() {
            let mut end = 0;
            let mut used = 0;
            let mut space = None;
            for (at, g) in rest.grapheme_indices(true) {
                if used + g.width() > width.max(1) && end > 0 {
                    break;
                }
                end = at + g.len();
                used += g.width();
                if g.chars().all(char::is_whitespace) {
                    space = Some(end);
                }
            }
            if end < rest.len()
                && let Some(boundary) = space
            {
                end = boundary;
            }
            rows.push(rest[..end].into());
            rest = &rest[end..];
        }
    }
    rows
}

fn block(
    lines: &mut Vec<Line<'static>>,
    prefix: Span<'static>,
    text: &str,
    width: u16,
    ink: Style,
) {
    let indent = prefix.width();
    for (index, chunk) in chunks(text, usize::from(width).saturating_sub(indent))
        .into_iter()
        .enumerate()
    {
        lines.push(Line::from(vec![
            if index == 0 {
                prefix.clone()
            } else {
                Span::raw(" ".repeat(indent))
            },
            Span::styled(chunk, ink),
        ]));
    }
}

fn heading(lines: &mut Vec<Line<'static>>, name: &str, style: Style) {
    if !lines.is_empty() {
        lines.push(Line::default());
    }
    lines.push(Line::styled(
        name.to_owned(),
        style.add_modifier(Modifier::BOLD),
    ));
}

fn check(lines: &mut Vec<Line<'static>>, item: &Item, width: u16, p: Palette) {
    block(
        lines,
        Span::styled(format!("{}  ", status(item)), style(state(item), p)),
        &item.label,
        width,
        p.ink,
    );
}

/// All source wording remains reachable; evidence is a separate deliberate view, never a read.
pub(super) fn detail(
    row: &ResultRow,
    pause: Option<&Pause>,
    at: Option<Instant>,
    width: u16,
    p: Palette,
    evidence: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if evidence {
        heading(&mut lines, "Evidence", p.ink);
        for item in &row.items {
            lines.push(Line::default());
            check(&mut lines, item, width, p);
            block(
                &mut lines,
                Span::raw("  "),
                &format!("Source: {}\nReported state: {}", item.kind, item.state),
                width,
                p.muted,
            );
            let receipt = if item.kind == "automated" {
                item.evidence
                    .replace("; argv:", ";\nargv:")
                    .replace("; cwd:", ";\ncwd:")
                    .replace("; receipt log SHA-256:", ";\nreceipt log SHA-256:")
            } else {
                item.evidence.clone()
            };
            block(&mut lines, Span::raw("  "), &receipt, width, p.ink);
        }
        if !row.gaps.is_empty() {
            heading(&mut lines, "Source notes", p.ink);
            for gap in &row.gaps {
                lines.push(Line::default());
                block(&mut lines, Span::raw("  "), gap, width, p.ink);
            }
        }
        return lines;
    }
    if let Some(pause) = pause {
        heading(&mut lines, "Next", p.ink);
        let guess = p.ink.add_modifier(Modifier::ITALIC);
        block(
            &mut lines,
            Span::styled("  ", guess),
            &pause.next,
            width,
            guess,
        );
    }
    heading(&mut lines, "You owe", p.jade);
    if row.gates.is_empty() {
        lines.push(Line::styled("  No source gate owed.", p.muted));
    }
    for gate in &row.gates {
        block(&mut lines, Span::raw("  "), gate, width, p.jade);
    }
    heading(&mut lines, "Checks", p.ink);
    for item in row.items.iter().filter(|item| !owed_item(row, item)) {
        check(&mut lines, item, width, p);
    }
    heading(&mut lines, "Goal", p.ink);
    block(&mut lines, Span::raw("  "), goal(&row.goal), width, p.ink);
    heading(&mut lines, "Details", p.ink);
    for text in [
        format!(
            "PR: {}{}",
            row.url.as_deref().unwrap_or("unknown"),
            if row.merged { " · merged" } else { "" }
        ),
        format!("Commit: {}", row.head.as_deref().unwrap_or("unknown")),
        format!("Run ID: {}", row.id),
        format!("Run: {}", row.slug),
        format!("Seed digest: {}", row.seed),
        format!(
            "Read {} min ago (snapshot)",
            at.map_or(0, |at| at.elapsed().as_secs() / 60)
        ),
    ] {
        block(&mut lines, Span::raw("  "), &text, width, p.muted);
    }
    if !row.gaps.is_empty() {
        lines.push(Line::styled(
            format!("  {} source notes · e evidence", row.gaps.len()),
            p.muted,
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> ResultRow {
        let human =
            "Confirm the synthetic assumptions remain distinguishable from real observations.";
        ResultRow {
            id: "TEST-run-id".into(), slug: "TEST-run".into(),
            goal: Goal::Matched(format!("{}\nWhy: TEST reason.\nDone when: TEST result.", "TEST café 👩‍💻 goal ".repeat(12))),
            url: Some("https://github.com/TEST/example/pull/1".into()), head: Some("a".repeat(40)),
            merged: true, seed: "TEST-seed".into(), gates: vec![human.into()], gaps: vec![],
            items: vec![
                Item { label: "Every original check word stays intact while this long sentence wraps with a hanging indent and no abbreviation.".into(),
                    state: "fail".into(), kind: "automated".into(), evidence: "/TEST/log; argv: [TEST]".into() },
                Item { label: human.into(), state: "pending (matching evidence missing)".into(),
                    kind: "human attestation".into(), evidence: "No operator evidence recorded".into() },
            ],
        }
    }

    #[test]
    fn compact_list_has_counts_and_only_goal_preview_can_be_shortened() {
        for width in [76, 96] {
            let row = row();
            let lines = list(std::slice::from_ref(&row), 0, width, Palette::new(true));
            assert_eq!(lines.len(), 3);
            assert!(lines[0].to_string().contains('…'));
            assert!(lines[0].to_string().ends_with(" · merged"));
            assert!(lines[0].width() <= width as usize);
            let text = lines
                .iter()
                .map(Line::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                text.contains("! 1 failed")
                    && text.contains("… 1 waiting")
                    && text.contains("You owe 1")
            );
            assert!(!text.contains(&row.items[0].label) && !text.contains("Confirm the synthetic"));
        }
    }

    #[test]
    fn headings_palette_and_exact_owed_deduplication_do_not_change_sources() {
        let row = row();
        for mono in [false, true] {
            let p = Palette::new(mono);
            let lines = detail(&row, None, None, 96, p, false);
            let text = lines
                .iter()
                .map(Line::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.find("You owe").unwrap() < text.find("Checks").unwrap());
            assert!(text.find("Checks").unwrap() < text.find("Goal").unwrap());
            assert!(text.find("Goal").unwrap() < text.find("Details").unwrap());
            assert_eq!(text.matches("Confirm the synthetic").count(), 1);
            assert!(
                !text.contains("(automated)")
                    && !text.contains("argv:")
                    && !text.contains("/TEST/log")
            );
            let failed = lines
                .iter()
                .find(|line| line.to_string().starts_with("! failed"))
                .unwrap();
            assert_eq!(failed.spans[0].style, p.rust);
            assert_eq!(failed.spans[1].style, p.ink);
            let owed = lines
                .iter()
                .find(|line| line.to_string().starts_with("  Confirm the synthetic"))
                .unwrap();
            assert_eq!(owed.spans[1].style, p.jade);
            let failed_row = lines
                .iter()
                .position(|line| line.to_string().starts_with("! failed"))
                .unwrap();
            let owed_row = lines
                .iter()
                .position(|line| line.to_string().starts_with("  Confirm the synthetic"))
                .unwrap();
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(96, 40)).unwrap();
            terminal
                .draw(|frame| {
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(lines.clone()).style(p.ink),
                        frame.area(),
                    )
                })
                .unwrap();
            let cells = terminal.backend().buffer();
            assert_eq!(
                cells[(0, failed_row as u16)].fg,
                p.rust.fg.unwrap_or(ratatui::style::Color::Reset)
            );
            assert!(
                cells[(0, failed_row as u16)]
                    .modifier
                    .contains(Modifier::BOLD)
            );
            if mono {
                assert!(
                    cells[(0, failed_row as u16)]
                        .modifier
                        .contains(Modifier::UNDERLINED)
                );
            }
            assert_eq!(
                cells[(2, owed_row as u16)].fg,
                p.jade.fg.unwrap_or(ratatui::style::Color::Reset)
            );
            let evidence = detail(&row, None, None, 96, p, true)
                .iter()
                .map(Line::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                evidence.contains("/TEST/log")
                    && evidence.contains("argv:")
                    && evidence.contains("human attestation")
            );
        }
    }

    #[test]
    fn check_words_are_never_shortened_and_hanging_rows_rejoin_exactly() {
        let text = "Keep every word,  including repeated spaces and café 👩‍💻, while a long condition wraps without ellipsis.";
        for width in [76, 96] {
            let item = Item {
                label: text.into(),
                state: "pending (matching evidence missing)".into(),
                kind: "automated".into(),
                evidence: String::new(),
            };
            let mut lines = Vec::new();
            check(&mut lines, &item, width, Palette::new(true));
            let joined = lines
                .iter()
                .map(|line| line.spans[1].content.as_ref())
                .collect::<String>();
            assert_eq!(joined, text);
            assert!(lines.len() > 1);
            let indent = lines[0].spans[0].width();
            assert!(
                lines[1..]
                    .iter()
                    .all(|line| line.spans[0].width() == indent)
            );
        }
    }
}
