use super::*;
use ratatui::style::Modifier;

pub(super) fn render(frame: &mut Frame, app: &mut BrainDump, palette: Palette) {
    let area = frame.area();
    app.area = area;
    frame.render_widget(Block::default().style(palette.ink), area);
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
        "Continue with Seed Me · Ctrl-C leave"
    } else if app.handoff_job.is_some() {
        "Wait for saving · Ctrl-C leave after saving"
    } else if app.goal_review.is_some() {
        "Type confirm · Enter affirm · Esc go back"
    } else if app.running() {
        "Keep your draft · Esc cancel sending"
    } else if app.sources.is_empty() {
        "Type anything · F2 send"
    } else if app.ready.is_some() {
        "Keep your draft · F2 when ready"
    } else if app
        .scope_pending
        .as_ref()
        .is_some_and(|(_, q)| app.available(q))
    {
        "Added words stay unresolved · answer their scope question · F2 send"
    } else if app.last_failure.is_some() {
        "Reading failed · F2 tries again · F5 why · Shift-F2 add clarification"
    } else if app.add_more {
        "Type a new dump · F2 send"
    } else if app.focused_question().is_none() {
        "No question active · F5 why · F3 review a reading"
    } else if app.board_focus {
        "F6 / Tab write · F5 why · F3 review goal"
    } else {
        "Type your answer · F2 send · F3 review goal"
    };
    let send = if app.real {
        "! F2 sends your words to the model; may cost extra calls."
    } else {
        "F2 gives a practice reading; no model call or save."
    };
    let finish = if app.receipt.is_some() {
        "Goal saved. Continue with Seed Me in any harness."
    } else if !app.real || app.goal_config.is_none() {
        "Goal confirmation needs real mode and Seed Me."
    } else {
        "! F3 / Ctrl-G reviews, without saving; clicking Review goal also works. Type confirm + Enter to create a Seed Me session; this goal can't be edited after."
    };
    let leave = if app.receipt.is_some() {
        "Ctrl-C exits; your confirmed goal stays saved."
    } else if app
        .goal_config
        .as_ref()
        .and_then(handoff::Config::recovery_path)
        .is_some()
    {
        "! Ctrl-C exits; unconfirmed work is lost. The recovery session remains."
    } else {
        "! Ctrl-C exits; nothing is saved yet. You'll be asked before losing work."
    };
    format!(
        "Right now\n{now}\n\nHelp — actions after returning\nF1 return · PgUp/PgDn scroll\nh How it works\n! means a consequential action\n\nWrite\nType; Enter starts a new line.\n! Shift-F2 / Ctrl-N switches between a new dump and an answer.\n\nSend\n{send}\n\nLook\nF6 / Tab switches writing / board controls.\nF7 / board [ / ] changes reading; F9 / board y copies.\nF5 / Ctrl-D why · F4 / Ctrl-O originals\nPgUp/PgDn scrolls the reading or review.\nWheel scrolls source; Ctrl-PgUp/PgDn changes original.\nDrag or Shift+arrows selects exact text; Ctrl-E extracts.\ne shows cards; Enter returns to source.\n! F8 / board s skips the question; board u undoes the last skip.\nCtrl-L repaints.\nMac keyboards may need fn, or the standard function keys setting.\nb toggles the muted key bar (this run only; off by default).\n\nFinish\n{finish}\n\nLeave\n! F10 is an alternative to Ctrl-C.\n{leave}\nn or Esc keeps your work; y or a second Ctrl-C leaves.\n\nItalic is my guess, not your words. Highlighted words support it. Underlined words are still unclear."
    )
}
pub(super) fn how_text(app: &BrainDump) -> String {
    format!(
        "How it works\nh or Esc returns to Help · PgUp/PgDn scroll\n\nSending and checks\n{}\nReal requests may incur charges. One shaping call is on the display path. After valid output is displayed, one meaning/continuity audit runs in the background: at most two provider calls per send, no automatic retry. Meaning checks are log-only: no flags shown, no rejection, repair, question withholding or confirmation barrier. Exact spans/schema/history remain local blocking checks. Audit failure leaves the reading untouched. Calls remain bounded and cancellable; a newer send cancels the owned older audit. Logs retain decisions, reasons, responses, coverage locations and elapsed milliseconds. A model judgment is not proof of understanding.\n\nGoal and authority\nOnly explicit confirmation creates an active Seed Me session and pins its origin. Answers, skips, empty question queues and review opening confirm nothing. Goal is not seed or implementation approval; outcome/options remain proposals. No seed confirmation or intake readiness.\n\nOriginals and viewing\nUnmarked words are neutral; original dumps/answers stay intact. Review opens the current original at its beginning; wheel scrolls the left source and Ctrl-PgUp/PgDn changes original. Questions stay open, nonblocking.\n\nAfter saving\nTinkery is read-only. Continue the active session in Seed Me. No browser opens. Only Tinkery's owned viewer stops at exit; the session stays active. Saving already affirmed durable writes cannot be undone; Ctrl-C waits for read-back before exit.\n\nLeaving\nUnconfirmed work is not saved. Ctrl-C (or board q) asks before discarding originals, replies or a reading; n/Esc keeps everything. y or a second Ctrl-C leaves. Paste is not an exit authorization. After confirmation, the goal stays saved, so exit is immediate.\n\n{}\n\n{}\n\nLast result\n{}",
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
pub(super) fn render_key_bar(frame: &mut Frame, app: &BrainDump, palette: Palette) {
    let area = frame.area();
    if !app.key_bar || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return;
    }
    let labels = if area.width >= 110 {
        [
            "help",
            "send",
            "review",
            "originals",
            "why",
            "switch",
            "reading",
            "skip",
            "copy",
            "exit",
        ]
    } else {
        [
            "help", "send", "goal", "src", "why", "mode", "read", "skip", "copy", "exit",
        ]
    };
    let mut spans = vec![];
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(ratatui::text::Span::raw(" "));
        }
        spans.push(ratatui::text::Span::styled(
            (i + 1).to_string(),
            palette.ink,
        ));
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
        Paragraph::new("Leave and lose this? y / n").style(palette.jade),
        rect,
    );
}
