use super::*;
use ratatui::style::Modifier;

pub(super) fn render(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    let area = frame.area();
    app.area = area;
    frame.render_widget(Block::default().style(palette.ink), area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(if app.help {
                "Help\nF1 or Esc returns. F10 back to the menu.\nResize to 80×24. Words retained."
            } else {
                "Need 80×24. Words retained."
            })
            .style(palette.ink),
            Rect::new(0, 3, area.width, area.height.saturating_sub(3)).intersection(area),
        );
        return;
    }
    app.goal_button = Rect::default();
    app.back_button = Rect::default();
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
        frame.render_widget(
            Paragraph::new(format!(
                "Words {} / {}",
                app.source_view + 1,
                app.sources.len()
            ))
            .style(palette.muted),
            Rect::new(left.x, 3, left.width, 1),
        );
        intact_view::render_source(frame, app, left, palette);
    }
    if app.help || app.original || app.details {
        let text = if app.help {
            if app.how {
                how_text(app)
            } else {
                help_text(app)
            }
        } else if app.original {
            app.originals()
        } else {
            format!(
                "Why\nh How it works\n\n{}\n\n{}\n\n{}",
                app.details_text(),
                app.notice,
                checks::why(&app.diagnostics())
            )
        };
        let text = if app.help {
            styled_help(text, palette)
        } else {
            ratatui::text::Text::styled(text, palette.ink)
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
    let failure = app.last_failure.as_ref().filter(|_| !app.running());
    let content = if let Some(error) = failure {
        let retained = app
            .guess
            .as_ref()
            .map(|g| {
                g.presented(app.reading)
                    .iter()
                    .map(|f| agent_text(&f.text))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default();
        format!(
            "Reading failed: {error}\n\nOriginals retained. Retry, inspect Why, or add clarification.\n\n{retained}"
        )
    } else if let Some(g) = &app.guess {
        let reading = g
            .presented(app.reading)
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
    if failure.is_some() {
        let retained_lines = app.guess.as_ref().map_or(0, |g| {
            g.presented(app.reading)
                .iter()
                .map(|f| agent_text(&f.text))
                .collect::<Vec<_>>()
                .join("\n\n")
                .lines()
                .count()
        });
        let known = text.lines.len().saturating_sub(retained_lines);
        for line in text.lines.iter_mut().take(known) {
            line.style = palette.ink.remove_modifier(Modifier::ITALIC);
        }
    }
    if app.update.is_some() && app.guess.is_some() && failure.is_none() {
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
        .title(format!(
            "{} · {}",
            if app.add_more {
                "Add more"
            } else {
                "Your reply"
            },
            if app.board_focus {
                "board controls"
            } else {
                "writing"
            }
        ))
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
    if [
        "Copy",
        "Clipboard",
        "Question skipped",
        "Skip undone",
        "Encoded",
        "Updated encoded",
        "Added words",
    ]
    .iter()
    .any(|p| app.notice.starts_with(p))
    {
        frame.render_widget(
            Paragraph::new(app.notice.as_str())
                .style(palette.muted)
                .wrap(Wrap { trim: false }),
            Rect::new(rows[3].x, rows[3].y + 1, rows[3].width, 1),
        );
    }
    if let Some(bytes) = app.encoded_bytes().filter(|b| *b >= 28 * 1024) {
        let offset = if app.goal_button.width > 0 { 14 } else { 0 };
        frame.render_widget(
            Paragraph::new(format!("{bytes} / 32768 bytes")).style(palette.muted),
            Rect::new(
                rows[3].x + offset,
                rows[3].y,
                rows[3].width.saturating_sub(offset),
                1,
            ),
        );
    }
    if app.running() && app.guess.is_some() {
        frame.render_widget(
            Paragraph::new("Thinking…").style(palette.muted),
            Rect::new(rows[3].x, rows[3].y + 1, rows[3].width, 1),
        );
    }
    if !app.running()
        && app.applied != app.sources.len()
        && app.guess.is_some()
        && app.scope_pending.is_none()
        && !["Copy", "Clipboard", "Question skipped", "Skip undone"]
            .iter()
            .any(|p| app.notice.starts_with(p))
    {
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
pub(super) fn help_text(app: &BrainDump) -> String {
    let now = if app.receipt.is_some() {
        "Your goal is saved. Continue it in Seed Me.\nF10 back to the menu."
    } else if app.handoff_job.is_some() {
        "Saving your goal. Wait a moment."
    } else if app.goal_review.is_some() {
        "Type confirm, then Enter, to save this goal.\nEsc goes back without saving."
    } else if app.running() {
        "Reading your words. Keep writing if you like.\nEsc cancels."
    } else if app.sources.is_empty() {
        "Type anything, in any order.\nF2 sends it."
    } else if app.ready.is_some() {
        "A new reading is ready.\nF2 when you want it."
    } else if app
        .scope_pending
        .as_ref()
        .is_some_and(|(_, q)| app.available(q))
    {
        "Answer the question about your added words.\nF2 sends."
    } else if app.last_failure.is_some() {
        "The reading failed; your words are safe.\nF2 tries again · F5 why."
    } else if app.add_more {
        "Type a new dump.\nF2 sends it."
    } else if app.focused_question().is_none() {
        "No question left.\nF3 reviews the goal."
    } else {
        "Type your answer.\nF2 sends · F8 skips · F3 reviews the goal."
    };
    if !app.help_all {
        return format!("Right now\n{now}\n\nF1 every key · Esc close");
    }
    let send = if app.real {
        "! F2   send (your words go to the model; small cost)"
    } else {
        "F2   send (practice reading; nothing leaves)"
    };
    let finish = if app.receipt.is_some() {
        "Goal saved. Continue with Seed Me in any harness."
    } else if !app.real || app.goal_config.is_none() {
        "Saving a goal needs real mode and Seed Me."
    } else {
        "! F3   review the goal; typing confirm saves it for good"
    };
    let leave = if app.receipt.is_some() {
        "F10  back to the menu; your goal stays saved"
    } else {
        "! F10  back to the menu; asks before losing your draft"
    };
    format!(
        "Every key\nF1 close · h how it works · PgUp/PgDn scroll\n\nWrite\nType; Enter starts a new line.\n{send}\n! Shift-F2  start a new dump instead of answering\n\nLook\nF7   the other reading\nF4   your originals\nF5   why (attempts and checks)\nF9   copy the reading\nDrag or Shift+arrows on your words selects; Ctrl-C copies\n! F8   skip the question (Ctrl-Z brings it back)\n\nFinish\n{finish}\n\nLeave\n{leave}\n\nWords\nreading   my guess at what you meant (grey)\noriginal  exactly what you typed (dark)\ngreen     words the reading is based on\nunderline words not placed yet\n!         loses something or can't be undone\nb         turns the key bar on or off"
    )
}
pub(super) fn how_text(app: &BrainDump) -> String {
    format!(
        "How it works\nh or Esc returns to Help · PgUp/PgDn scroll\n\nOther keys\nAliases: Ctrl-N new dump, Ctrl-G review, Ctrl-O originals, Ctrl-D why. Ctrl-E extracts a selection; e shows cards. Ctrl-L repaints. Mac keyboards may need fn, or the standard function keys setting.\n\nSending and checks\n{}\nReal requests may incur charges. One shaping call is on the display path. After valid output is displayed, one meaning/continuity audit runs in the background: at most two provider calls per send, no automatic retry. Meaning checks are log-only: no flags shown, no rejection, repair, question withholding or confirmation barrier. Exact spans/schema/history remain local blocking checks. Audit failure leaves the reading untouched. Calls remain bounded and cancellable; a newer send cancels the owned older audit. Logs retain decisions, reasons, responses, coverage locations and elapsed milliseconds. A model judgment is not proof of understanding.\n\nGoal and authority\nOnly explicit confirmation creates an active Seed Me session and pins its origin. Answers, skips, empty question queues and review opening confirm nothing. Goal is not seed or implementation approval; outcome/options remain proposals. No seed confirmation or intake readiness.\n\nOriginals and viewing\nUnmarked words are neutral; original dumps/answers stay intact. Review opens the current original at its beginning; wheel scrolls the left source and Ctrl-PgUp/PgDn changes original. Questions stay open, nonblocking.\n\nAfter saving\nTinkery is read-only. Continue the active session in Seed Me. No browser opens. Only Tinkery's owned viewer stops at exit; the session stays active. Saving already affirmed durable writes cannot be undone; F10 waits for read-back before returning.\n\nLeaving\nUnconfirmed work is not saved. F10 asks before discarding originals, replies or a reading; n/Esc keeps everything. y or a second F10 returns to the menu. Ctrl-C never leaves: it copies a selection or cancels sending. After confirmation, the goal stays saved, so returning is immediate.\n\n{}\n\n{}\n\nLast result\n{}",
        if app.real {
            "Real provider mode."
        } else {
            "Simulated practice; no real session."
        },
        app.diagnostics().join("\n\n"),
        app.receipt.as_ref().map_or(String::new(), |r| r.text()),
        app.notice
    )
}
/// Only what's possible right now (C1): key in ink, word muted.
pub(super) fn key_bar_items(app: &BrainDump) -> Vec<(&'static str, &'static str)> {
    if app.leave_prompt || app.handoff_job.is_some() {
        vec![]
    } else if app.receipt.is_some() {
        vec![("10", "menu")]
    } else if app.goal_review.is_some() {
        vec![("type confirm", "save"), ("esc", "back")]
    } else if app.running() {
        vec![("esc", "cancel"), ("10", "menu")]
    } else {
        let mut items = vec![("2", "send")];
        if app.guess.is_some() {
            items.push(("3", "review"));
        }
        if app.guess.as_ref().is_some_and(|g| g.framings.len() > 1) {
            items.push(("7", "other reading"));
        }
        if app.focused_question().is_some() {
            items.push(("8", "skip"));
        }
        items.push(("10", "menu"));
        items
    }
}
pub(super) fn render_key_bar(frame: &mut Frame, app: &BrainDump, palette: Palette) {
    let area = frame.area();
    if !app.key_bar || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return;
    }
    let mut spans = vec![];
    for (i, (key, label)) in key_bar_items(app).into_iter().enumerate() {
        if i > 0 {
            spans.push(ratatui::text::Span::raw("   "));
        }
        spans.push(ratatui::text::Span::styled(key, palette.ink));
        spans.push(ratatui::text::Span::styled(
            format!(" {label}"),
            palette.muted,
        ));
    }
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(spans)).style(palette.ink),
        Rect::new(2, area.height - 1, area.width - 4, 1),
    );
}
/// Help text with `!` in rust (never alone: the words say what's lost) and headings in bold.
pub(super) fn styled_help(text: String, palette: Palette) -> ratatui::text::Text<'static> {
    let mut prev_blank = true;
    let lines = text
        .lines()
        .map(|line| {
            let heading = prev_blank && !line.is_empty() && line.len() < 24 && !line.contains('·');
            prev_blank = line.is_empty();
            if let Some(rest) = line.strip_prefix("! ") {
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("! ", palette.rust),
                    ratatui::text::Span::styled(rest.to_owned(), palette.ink),
                ])
            } else if heading {
                ratatui::text::Line::styled(line.to_owned(), palette.ink.bold())
            } else {
                ratatui::text::Line::styled(line.to_owned(), palette.ink)
            }
        })
        .collect::<Vec<_>>();
    ratatui::text::Text::from(lines)
}
pub(super) fn render_header(frame: &mut Frame, palette: Palette) {
    let area = frame.area();
    frame.render_widget(
        Paragraph::new("tinkery").style(palette.muted),
        Rect::new(2, 1, 7, 1).intersection(area),
    );
    frame.render_widget(
        Paragraph::new("?").style(palette.muted),
        Rect::new(area.width.saturating_sub(3), 1, 1, 1).intersection(area),
    );
}
pub(super) fn render_leave_prompt(frame: &mut Frame, app: &BrainDump, palette: Palette) {
    if !app.leave_prompt {
        return;
    }
    let area = frame.area();
    let rect = Rect::new(
        2,
        area.height.saturating_sub(2),
        area.width.saturating_sub(4),
        1,
    )
    .intersection(area);
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled("! ", palette.rust),
            ratatui::text::Span::styled("Back to menu and lose this draft? y / n", palette.ink),
        ]))
        .style(palette.ink),
        rect,
    );
}
