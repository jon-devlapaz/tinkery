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
            if app.how {
                how_text(app)
            } else {
                help_text(app)
            }
        } else if app.original {
            app.originals()
        } else {
            format!(
                "Details\nh How it works\n\n{}\n\n{}\n\n{}",
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
    } else if app.add_more {
        "Type a new dump · F2 send"
    } else if app.board_focus {
        "Tab write · Ctrl-D look closer · Ctrl-G review goal"
    } else {
        "Type your answer · F2 send · Ctrl-G review goal"
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
        "! Ctrl-G reviews, without saving. Type confirm + Enter to create a Seed Me session; this goal can't be edited after."
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
        "Right now\n{now}\n\nHelp — actions after returning\nF1 return · PgUp/PgDn scroll\nh How it works\n! means a consequential action\n\nWrite\nType; Enter starts a new line.\n! Ctrl-N switches between a new dump and an answer.\n\nSend\n{send}\n\nLook\nTab switches writing / board controls.\nOn the board: [ / ] changes reading; y copies.\nCtrl-D details · Ctrl-O originals\nPgUp/PgDn scrolls the reading or review.\nWheel scrolls source; Ctrl-PgUp/PgDn changes original.\nDrag or Shift+arrows selects exact text; Ctrl-E extracts.\ne shows cards; Enter returns to source.\n! s skips the question for good in this run.\nCtrl-L repaints.\n\nFinish\n{finish}\n\nLeave\n{leave}\nn or Esc keeps your work; y or a second Ctrl-C leaves.\n\nItalic is my guess, not your words. Highlighted words support it. Underlined words are still unclear."
    )
}
pub(super) fn how_text(app: &BrainDump) -> String {
    format!(
        "How it works\nh or Esc returns to Help · PgUp/PgDn scroll\n\nSending and repair\n{}\nReal requests may incur charges. Only a valid meaning rejection permits one automatic corrective attempt after F2. Syntax/transport/cancellation/helper failures never automatically retry. At most two shape/audit pairs plus one answer-continuity check. A successful send uses two to five provider calls; failures may stop earlier. Each call remains bounded and cancellable. Both shaping/audit attempts are retained below. A model judgment is not proof of understanding.\n\nGoal and authority\nOnly explicit confirmation creates an active Seed Me session and pins its origin. Answers, skips, empty question queues and review opening confirm nothing. Goal is not seed or implementation approval; outcome/options remain proposals. No seed confirmation or intake readiness.\n\nOriginals and viewing\nUnmarked words are neutral; original dumps/answers stay intact. Review opens the current original at its beginning; wheel scrolls the left source and Ctrl-PgUp/PgDn changes original. Questions stay open, nonblocking.\n\nAfter saving\nTinkery is read-only. Continue the active session in Seed Me. No browser opens. Only Tinkery's owned viewer stops at exit; the session stays active. Saving already affirmed durable writes cannot be undone; Ctrl-C waits for read-back before exit.\n\nLeaving\nUnconfirmed work is not saved. Ctrl-C (or board q) asks before discarding originals, replies or a reading; n/Esc keeps everything. y or a second Ctrl-C leaves. Paste is not an exit authorization. After confirmation, the goal stays saved, so exit is immediate.\n\n{}\n\n{}\n\nLast result\n{}",
        if app.real {
            "Real provider mode."
        } else {
            "Simulated practice; no real session."
        },
        app.host.diagnostics().join("\n\n"),
        app.receipt.as_ref().map_or(String::new(), |r| r.text()),
        app.notice
    )
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
