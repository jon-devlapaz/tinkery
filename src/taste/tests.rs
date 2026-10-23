use super::*;
use read::Item;

fn row() -> ResultRow {
    ResultRow {
        id: "TEST-run-id".into(),
        slug: "TEST-run".into(),
        goal: Goal::Matched(
            " TEST café  \nWhy: TEST reason.\nDone when: TEST check.\nMust not: TEST claims.  "
                .into(),
        ),
        url: Some("https://github.com/TEST/example/pull/1".into()),
        head: Some("a".repeat(40)),
        seed: "TEST-digest".into(),
        items: vec![Item {
            label: "TEST check".into(),
            state: "pending".into(),
            kind: "automated".into(),
            evidence: "TEST evidence".into(),
        }],
        gates: vec![
            "re-approve the plan".into(),
            "confirm no clinical claims".into(),
        ],
        gaps: Vec::new(),
    }
}

fn ready() -> Taste {
    let mut taste = Taste::open(Some(PathBuf::from("/TEST")));
    taste.rows = vec![row()];
    taste.at = Some(Instant::now());
    taste
}

fn screen(taste: &mut Taste, width: u16, height: u16) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| taste.render(f, f.area(), Palette::new(true)))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn exact_goal_survives_display_and_both_handoffs_without_a_verdict() {
    let row = row();
    let Goal::Matched(goal) = &row.goal else {
        panic!()
    };
    assert!(result_lines(&row).join("\n").contains(goal));
    for action in ['a', 'c'] {
        let copy = handoff(&row, action).unwrap();
        assert!(copy.contains(goal) && copy.contains(&"a".repeat(40)));
        assert!(copy.contains("Recheck current PR head, inputs and receipts"));
        assert!(
            copy.contains("re-approve the plan") && copy.contains("confirm no clinical claims")
        );
        assert!(copy.contains("No verdict recorded"));
        assert!(!copy.contains("gh pr review") && !copy.contains("gh pr merge"));
    }
}

#[test]
fn unknown_goal_or_result_refuses_copy_and_navigation_starts_nothing() {
    let mut taste = ready();
    for code in [
        KeyCode::Enter,
        KeyCode::Down,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Char('n'),
        KeyCode::Char('p'),
    ] {
        taste.key(KeyEvent::from(code));
        assert!(
            taste.reading.is_none()
                && taste.opening.is_none()
                && taste.take_copy_request().is_none()
        );
    }
    for goal in [Goal::Unknown, Goal::NoSeed] {
        taste.rows[0].goal = goal;
        taste.key(KeyEvent::from(KeyCode::Char('a')));
        assert!(taste.take_copy_request().is_none());
    }
    taste.rows[0] = row();
    taste.rows[0].head = None;
    assert!(handoff(&taste.rows[0], 'c').is_err());
    taste.rows[0].head = Some("a".repeat(40));
    taste.rows[0].url = None;
    assert!(handoff(&taste.rows[0], 'a').is_err());
}

#[test]
fn repeated_or_modified_keys_never_copy_open_or_read() {
    let mut taste = ready();
    for code in ['a', 'c', 'r', 'o'] {
        let mut key = KeyEvent::from(KeyCode::Char(code));
        key.kind = KeyEventKind::Repeat;
        taste.key(key);
        key.kind = KeyEventKind::Press;
        key.modifiers = KeyModifiers::ALT;
        taste.key(key);
        assert!(
            taste.reading.is_none()
                && taste.opening.is_none()
                && taste.take_copy_request().is_none()
        );
    }
    taste.key(KeyEvent::from(KeyCode::Char('a')));
    assert!(
        taste
            .take_copy_request()
            .unwrap()
            .contains("Acceptance review requested")
    );
    assert!(taste.take_copy_request().is_none());
    taste.copy_result(true);
    assert!(taste.notice.contains("No verdict recorded"));
    taste.copy_result(false);
    assert!(taste.notice.contains("Clipboard unavailable"));
}

#[test]
fn both_sizes_scroll_to_full_evidence_and_keep_navigation() {
    for (w, h) in [(100, 30), (80, 24)] {
        let mut taste = ready();
        taste.rows[0].items[0].evidence =
            format!("{} TEST-LAST-EVIDENCE", "TEST evidence ".repeat(500));
        taste.key(KeyEvent::from(KeyCode::Enter));
        let first = screen(&mut taste, w, h);
        assert!(first.contains("TEST café") && first.contains("copy correct"));
        assert!(!first.contains("TEST-LAST-EVIDENCE"));
        taste.key(KeyEvent::from(KeyCode::End));
        let tail = screen(&mut taste, w, h);
        assert!(tail.contains("TEST-LAST-EVIDENCE"));
        assert!(
            tail.contains(&"a".repeat(40)),
            "result identity scrolled away: {tail}"
        );
        assert_eq!(tail.matches("Commit:").count(), 1);
        taste.key(KeyEvent::from(KeyCode::Home));
        assert!(screen(&mut taste, w, h).contains("TEST café"));
        taste.key(KeyEvent::from(KeyCode::Esc));
        assert!(!taste.detail);
        assert!(taste.key(KeyEvent::from(KeyCode::Esc)));
    }
}

#[test]
fn selection_is_retained_by_run_and_pr_not_index_and_missing_selection_returns_to_list() {
    let mut taste = ready();
    taste.rows.push(ResultRow {
        id: "TEST-other".into(),
        ..row()
    });
    taste.selected = 1;
    taste.detail = true;
    let key = ("TEST-other".to_owned(), taste.rows[1].url.clone());
    taste.selection = Some(key);
    let mut rows = taste.rows.clone();
    rows.reverse();
    taste.reading = Some(Read::task(move |_| Ok(rows)));
    settle(&mut taste);
    assert_eq!(taste.rows[taste.selected].id, "TEST-other");
    taste.reading = Some(Read::task(|_| Ok(vec![row()])));
    settle(&mut taste);
    assert!(!taste.detail && taste.notice.contains("no longer"));
}

fn settle(taste: &mut Taste) {
    let until = Instant::now() + std::time::Duration::from_secs(5);
    while taste.reading.is_some() || taste.opening.is_some() {
        assert!(Instant::now() < until);
        taste.poll();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn unsafe_terminal_controls_are_only_escaped_for_display() {
    assert_eq!(safe("TEST\x1b[31m\u{202e}"), "TEST\\u{1b}[31m\\u{202e}");
    assert_eq!(safe(" TEST café  "), " TEST café  ");
}
