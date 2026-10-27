//! Think's help panel: bounded context, stable navigation and a scrolling body.
//! It borrows the existing help words; it never sends, saves or changes the goal.
use super::*;
use ratatui::{
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{Borders, Padding},
};

/// Handle help navigation, preferences and the numbered actions: a digit runs its row.
/// Hidden writing/goal actions stay inert; Ctrl-C remains owned by the caller.
pub(super) fn handle_key(app: &mut BrainDump, key: KeyEvent) -> bool {
    if !app.screen.help
        || app.screen.leave_prompt
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
    {
        return false;
    }
    if !app.screen.how
        && let Some(act) = crate::actions::pick(&app.rows(), key)
    {
        app.run(act);
        return true;
    }
    if !key.modifiers.is_empty()
        || (key.kind == KeyEventKind::Repeat
            && !matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
            ))
    {
        return true;
    }
    match key.code {
        KeyCode::F(1) => {
            app.screen.help = false;
            app.screen.how = false;
            app.screen.original_scroll = 0;
        }
        KeyCode::Char('h') => {
            app.screen.how = !app.screen.how;
            app.screen.original_scroll = 0;
        }
        KeyCode::Char('b') => app.screen.key_bar = !app.screen.key_bar,
        KeyCode::Esc if app.voice_active() => {
            app.voice_key(key);
        }
        KeyCode::Esc if app.screen.how => {
            app.screen.how = false;
            app.screen.original_scroll = 0;
        }
        KeyCode::Esc => app.screen.help = false,
        KeyCode::Down | KeyCode::PageDown => {
            app.screen.original_scroll = app.screen.original_scroll.saturating_add(5);
        }
        KeyCode::Up | KeyCode::PageUp => {
            app.screen.original_scroll = app.screen.original_scroll.saturating_sub(5);
        }
        _ => {}
    }
    true
}

/// Paint help over the reading side, preserving originals and the underlying state.
/// Long bodies scroll independently of the title and close/navigation hints.
pub(super) fn render(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    let area = frame.area();
    if !app.screen.help || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return;
    }
    let raw = if app.screen.how {
        layout::how_text(app)
    } else {
        layout::help_text(app)
    };
    let mut lines = raw.lines();
    let title = lines.next().unwrap_or("Help");
    let mut body: Vec<_> = lines.collect();
    if app.screen.how {
        body.remove(0);
    } else {
        body.pop();
    }
    let content = style_body(body.join("\n").trim(), app.screen.how, palette);
    let width = 58.min(area.width - area.width * 48 / 100 - 4);
    let content_width = width - 4;
    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
    let content_rows = paragraph.line_count(content_width);
    // Never cover the writing box (or its label): the operator's draft stays in view.
    let input = app.screen.input_area;
    let max_height =
        if input.y > 3 + 9 && input.height > 0 && input.right() > area.width - width - 2 {
            (input.y - 2 - 3).min(area.height - 6)
        } else {
            area.height - 6
        };
    let mut navigation = if app.screen.how {
        vec![("h / Esc", "Back to help"), ("F1", "Close help")]
    } else {
        vec![("h", "How it works"), ("F1 / Esc", "Close help")]
    };
    let cancel = if app.voice_active() {
        Some("Cancel voice")
    } else {
        None
    };
    if let Some(label) = cancel {
        if app.screen.how {
            navigation[0] = ("h", "Back to help");
        } else {
            navigation[1] = ("F1", "Close help");
        }
        navigation.push(("Esc", label));
    }
    // A short panel drops its blank rows and packs the keys onto as few lines as fit, so the
    // numbered actions keep the room and the writing box below stays in view.
    let tight = content_rows + navigation.len() + 5 > max_height as usize;
    if tight {
        navigation.push(("PgUp/PgDn", "Scroll"));
    }
    let footer_lines: Vec<Line> = if tight {
        packed(&navigation, width - 4, palette)
    } else {
        navigation
            .iter()
            .map(|(key, label)| action(key, label, false, palette))
            .collect()
    };
    let chrome = if tight { 2 } else { 5 };
    let height = (content_rows + footer_lines.len() + chrome).min(max_height as usize) as u16;
    let panel = Rect::new(area.width - width - 2, 3, width, height);
    let block = Block::default()
        .title(Span::styled(
            format!(" {title} "),
            palette.jade.remove_modifier(Modifier::DIM),
        ))
        .borders(Borders::ALL)
        .border_style(palette.muted)
        .padding(if tight {
            Padding::horizontal(1)
        } else {
            Padding::uniform(1)
        });
    let inner = block.inner(panel);
    let footer_height = footer_lines.len() as u16;
    let body_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height - footer_height - u16::from(!tight),
    );
    let footer = Rect::new(
        inner.x,
        inner.bottom() - footer_height,
        inner.width,
        footer_height,
    );
    let max_scroll = content_rows
        .saturating_sub(body_area.height as usize)
        .min(u16::MAX as usize) as u16;
    app.screen.original_scroll = app.screen.original_scroll.min(max_scroll);
    frame.render_widget(Clear, panel);
    frame.render_widget(block.style(palette.ink), panel);
    frame.render_widget(
        paragraph
            .style(palette.ink)
            .scroll((app.screen.original_scroll, 0)),
        body_area,
    );
    frame.render_widget(Paragraph::new(footer_lines), footer);
}

/// Key and label pairs side by side, as many to a line as fit in `width`.
fn packed(pairs: &[(&str, &str)], width: u16, palette: Palette) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = vec![];
    for (key, label) in pairs {
        let size = key.chars().count() + 1 + label.chars().count();
        let fits = lines
            .last()
            .is_some_and(|line| line.width() + 3 + size <= width as usize);
        let mut spans = vec![
            Span::styled(format!("{key} "), palette.ink.bold()),
            Span::styled(label.to_string(), palette.muted),
        ];
        match lines.last_mut() {
            Some(line) if fits => {
                line.spans.push(Span::raw("   "));
                line.spans.append(&mut spans);
            }
            _ => lines.push(Line::from(spans)),
        }
    }
    lines
}

fn action(key: &str, label: &str, consequence: bool, palette: Palette) -> Line<'static> {
    let mut spans = vec![];
    if consequence {
        spans.push(Span::styled("! ", palette.rust));
    }
    let column = if key.chars().all(|c| c.is_ascii_digit()) {
        3
    } else if consequence {
        10
    } else {
        12
    };
    let width = column.max(key.chars().count() + 1);
    spans.push(Span::styled(format!("{key:<width$}"), palette.ink.bold()));
    spans.push(Span::styled(label.to_owned(), palette.muted));
    Line::from(spans)
}

fn style_body(text: &str, technical: bool, palette: Palette) -> Text<'static> {
    let mut lines = vec![];
    for line in text.lines() {
        if matches!(
            line,
            "Write"
                | "Look"
                | "Finish"
                | "Leave"
                | "Words"
                | "Sending"
                | "Saving"
                | "After saving"
                | "Keys"
                | "Last result"
        ) {
            lines.push(Line::styled(line.to_owned(), palette.ink.bold()));
            continue;
        }
        if technical {
            lines.push(Line::styled(line.to_owned(), palette.ink));
            continue;
        }
        let (consequence, rest) = line
            .strip_prefix("! ")
            .map_or((false, line), |rest| (true, rest));
        for part in rest.split(" · ") {
            let key = part.split_whitespace().next().unwrap_or("");
            if matches!(
                key,
                "F1" | "F2"
                    | "1"
                    | "2"
                    | "3"
                    | "4"
                    | "5"
                    | "6"
                    | "7"
                    | "8"
                    | "9"
                    | "PgUp/PgDn"
                    | "Tab"
                    | "Esc"
                    | "c"
                    | "s"
                    | "b"
            ) {
                let (key, label) = (key, part[key.len()..].trim_start());
                lines.push(action(
                    &key.replace(" / ", "/"),
                    label,
                    consequence,
                    palette,
                ));
            } else if consequence {
                lines.push(Line::from(vec![
                    Span::styled("! ", palette.rust),
                    Span::styled(part.to_owned(), palette.ink),
                ]));
            } else if let Some((before, after)) = part.split_once("F2") {
                lines.push(Line::from(vec![
                    Span::styled(before.to_owned(), palette.muted),
                    Span::styled("F2", palette.ink.bold()),
                    Span::styled(after.to_owned(), palette.muted),
                ]));
            } else {
                lines.push(Line::styled(part.to_owned(), palette.muted));
            }
        }
    }
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technical_help_never_reformats_operator_words_as_action_rows() {
        let original = "F2 · keep this exact\n! original words\nc keep this too";
        let styled = style_body(original, true, Palette::new(false));
        let rendered = styled
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(rendered, original);
    }
}
