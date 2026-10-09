use super::*;
use ratatui::style::Modifier;

pub(super) fn render(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    let area = frame.area();
    app.area = area;
    frame.render_widget(Block::default().style(palette.ink), area);
    frame.render_widget(
        Paragraph::new("tinkery").style(palette.muted),
        Rect::new(2, 1, 7, 1).intersection(area),
    );
    frame.render_widget(
        Paragraph::new("?").style(palette.muted),
        Rect::new(area.width.saturating_sub(3), 1, 1, 1).intersection(area),
    );
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(if app.help {
                "Help\nF1 or Esc returns. Ctrl-C exits.\nResize to 80×24. Words retained."
            } else {
                "Need 80×24. Words retained."
            })
            .style(palette.ink),
            Rect::new(0, 3, area.width, area.height.saturating_sub(3)).intersection(area),
        );
        return;
    }
    app.goal_button = Rect::default();
    if app.sources.is_empty() && !app.help {
        frame.render_widget(
            Paragraph::new("What's on your mind?").style(palette.muted),
            Rect::new(4, 5, area.width - 8, 1),
        );
        app.input_area = Rect::new(4, 8, area.width - 8, area.height - 12);
        render_input(frame, app, palette);
        return;
    }
    let left = Rect::new(2, 4, area.width * 48 / 100 - 4, area.height - 6);
    let right = Rect::new(
        area.width * 48 / 100 + 2,
        4,
        area.width - area.width * 48 / 100 - 4,
        area.height - 6,
    );
    app.agent_area = right;
    app.canvas.area = left;
    app.annotations_visible = (false, false);
    if app.show_extractions {
        draw_fragments(frame, app, left, palette);
    } else {
        intact_view::render_source(frame, app, left, palette);
    }
    if app.help || app.original || app.details {
        let text = if app.help {
            help_text(app)
        } else if app.original {
            app.originals()
        } else {
            format!(
                "{}\n\n{}\n\n{}",
                app.details_text(),
                app.notice,
                app.host.diagnostics().join("\n\n")
            )
        };
        let p = Paragraph::new(text)
            .style(palette.ink)
            .wrap(Wrap { trim: false });
        let max = p
            .line_count(right.width)
            .saturating_sub(right.height as usize) as u16;
        app.original_scroll = app.original_scroll.min(max);
        frame.render_widget(p.scroll((app.original_scroll, 0)), right);
        return;
    }
    if app.goal_review.is_some() || app.receipt.is_some() {
        goal_view::render_goal(frame, app, palette);
        return;
    }
    let rows = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(2),
    ])
    .split(right);
    app.agent_area = rows[0];
    let content = if let Some(g) = &app.guess {
        let reading = g
            .framings
            .iter()
            .map(|f| agent_text(&f.text))
            .collect::<Vec<_>>()
            .join("\n\n");
        app.update
            .as_ref()
            .map_or(reading.clone(), |u| format!("{u}\n\n{reading}"))
    } else if app.running() {
        "Thinking…".into()
    } else if app.ready.is_some() {
        "Reply ready.".into()
    } else if app.sources.len() != app.applied {
        "I couldn't form a faithful reading.".into()
    } else {
        String::new()
    };
    let mut text = ratatui::text::Text::from(content);
    if app.update.is_some() && app.guess.is_some() {
        text.lines[0].style = palette
            .jade
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::ITALIC);
    }
    let p = Paragraph::new(text)
        .style(palette.muted.add_modifier(Modifier::ITALIC))
        .wrap(Wrap { trim: false });
    let max = p
        .line_count(rows[0].width)
        .saturating_sub(rows[0].height as usize) as u16;
    app.paper_scroll = app.paper_scroll.min(max);
    frame.render_widget(p.scroll((app.paper_scroll, 0)), rows[0]);
    let q = app
        .focused_question()
        .map(|q| q.text.clone())
        .unwrap_or_else(|| {
            if app.guess.is_some() {
                "No unanswered question.".into()
            } else {
                String::new()
            }
        });
    frame.render_widget(
        Paragraph::new(q)
            .style(palette.jade)
            .wrap(Wrap { trim: false }),
        rows[1],
    );
    let input = Block::default()
        .title(if app.add_more {
            "Add more"
        } else {
            "Your reply"
        })
        .style(palette.ink);
    app.input_area = input.inner(rows[2]);
    frame.render_widget(input, rows[2]);
    render_input(frame, app, palette);
    if app.goal_config.is_some() && app.guess.is_some() {
        app.goal_button = Rect::new(rows[3].x, rows[3].y, 12, 1);
        frame.render_widget(
            Paragraph::new("Review goal").style(palette.jade),
            app.goal_button,
        );
    }
    if app.running() && app.guess.is_some() {
        frame.render_widget(
            Paragraph::new("Thinking…").style(palette.muted),
            Rect::new(rows[3].x, rows[3].y + 1, rows[3].width, 1),
        );
    }
    if !app.running() && app.applied != app.sources.len() && app.guess.is_some() {
        frame.render_widget(
            Paragraph::new(if app.ready.is_some() {
                "Reply ready."
            } else {
                "Reading couldn't be updated."
            })
            .style(palette.muted),
            Rect::new(rows[3].x, rows[3].y + 1, rows[3].width, 1),
        );
    }
}
fn help_text(app: &BrainDump) -> String {
    format!(
        "Help\n\nF1 or the corner ? opens help; ? stays literal while typing. Esc returns. PgUp/PgDn scroll.\nF2 submits; Enter adds a line. Tab switches input/board. Ctrl-N toggles a separate dump. Ctrl-C exits.\nCtrl-D details; Ctrl-O originals; Ctrl-G reviews the goal. In review, type confirm then Enter after reading all content. Questions do not block goal confirmation.\nBoard: s skips, y copies, [ / ] selects reading, Ctrl-E extracts selected text, e shows cards, Enter returns to source. Ctrl-PgUp/PgDn changes source; wheel scrolls; Ctrl-L repaints.\n\nMuted italic text is a provisional interpretation, not your words or an approval. Highlights support it; underlined words remain open; unmarked words are neutral. Settled means answered, not confirmed.\n{}\nReal requests may incur charges. Only a valid meaning rejection permits one automatic corrective attempt after F2. Syntax/transport/cancellation/helper failures never automatically retry. At most two shape/audit pairs plus one answer-continuity check; each provider call remains bounded and cancellable. Both attempts are retained below.\nGoal confirmation creates an active Seed Me session and pins its origin. No seed confirmation, intake readiness, or implementation approval. No browser opens. After handoff, Tinkery is read-only; q or Esc exits. Only the owned viewer stops; the session remains active.\n\n{}\n\n{}\n\nLast result\n{}",
        if app.real {
            "Mode: real provider."
        } else {
            "Mode: simulated practice; no real session."
        },
        app.host.diagnostics().join("\n\n"),
        app.receipt.as_ref().map_or(String::new(), |r| r.text()),
        app.notice
    )
}
