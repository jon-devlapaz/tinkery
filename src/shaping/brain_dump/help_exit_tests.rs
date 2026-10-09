use super::goal_tests::board;
use super::tests::amend_board;
use super::*;
fn key(a: &mut BrainDump, k: KeyCode) {
    a.handle_key(KeyEvent::new(k, KeyModifiers::NONE));
}
fn ctrl_c(a: &mut BrainDump) {
    a.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
}
#[test]
fn help_leads_with_context_groups_actions_and_keeps_internals_on_second_page() {
    let mut a = BrainDump::default();
    assert!(yohaku::help_text(&a).starts_with("Right now\nType anything · F2 send"));
    assert!(yohaku::help_text(&a).contains("practice reading; no model call"));
    a = board();
    let text = yohaku::help_text(&a);
    assert!(text.starts_with("Right now\nType your answer"));
    for heading in ["Write", "Send", "Look", "Finish", "Leave"] {
        assert!(text.contains(&format!("\n{heading}\n")));
    }
    for consequence in [
        "! F2 sends your words",
        "! Shift-F2 / Ctrl-N switches",
        "! F8 / board s skips",
        "! F3 / Ctrl-G reviews",
        "! Ctrl-C exits",
    ] {
        assert!(text.contains(consequence));
    }
    for internal in [
        "shape/audit",
        "answer-continuity",
        "viewer",
        "pins its origin",
        "durable writes",
    ] {
        assert!(!text.contains(internal));
    }
    assert_eq!(text.matches("Italic is my guess, not your words. Highlighted words support it. Underlined words are still unclear.").count(),1);
    assert!(text.contains("nothing is saved yet"));
    a.board_focus = true;
    assert!(yohaku::help_text(&a).starts_with("Right now\nF6 / Tab write"));
    a.board_focus = false;
    a.add_more = true;
    assert!(yohaku::help_text(&a).starts_with("Right now\nType a new dump"));
    a.add_more = false;
    a.review_goal();
    assert!(yohaku::help_text(&a).starts_with("Right now\nType confirm"));
    let frozen = a.goal_review.as_ref().unwrap().affirmation.goal.clone();
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::Char('h'));
    assert!(a.how && a.help);
    let how = yohaku::how_text(&a);
    for s in [
        "at most two provider calls per send",
        "no automatic retry",
        "outcome/options remain proposals",
        "session stays active",
    ] {
        assert!(how.contains(s));
    }
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("How it works"));
    a.paste("confirm");
    assert!(a.goal_review.as_ref().unwrap().input.is_empty());
    key(&mut a, KeyCode::Esc);
    assert!(a.help && !a.how);
    key(&mut a, KeyCode::Esc);
    assert!(!a.help);
    a.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    let details = snapshot(100, 30, &mut a, false).unwrap();
    assert!(details.contains("h How it works"));
    key(&mut a, KeyCode::Char('h'));
    assert!(a.how && a.help && !a.details);
    key(&mut a, KeyCode::F(1));
    assert_eq!(a.goal_review.as_ref().unwrap().affirmation.goal, frozen);
    a.goal_review = None;
    a.receipt = Some(handoff::Receipt::test("/tmp/Tinkery-TEST/session".into()));
    let text = yohaku::help_text(&a);
    assert!(text.starts_with("Right now\nContinue with Seed Me"));
    assert!(!text.contains("nothing is saved"));
}
#[test]
fn exit_guard_keeps_unsent_original_reply_and_frozen_review_until_explicit_leave() {
    for state in 0..5 {
        let mut a = if state == 0 {
            BrainDump::default()
        } else {
            board()
        };
        if state == 0 || state == 2 {
            a.paste("unsent words?");
        }
        if state == 3 {
            a.review_goal();
            a.paste("conf");
        }
        if state == 4 {
            a.guess = None;
        }
        let source = a.sources.clone();
        let input = a.input.text.clone();
        let review = a.goal_review.as_ref().map(|r| r.input.clone());
        ctrl_c(&mut a);
        assert!(a.leave_prompt && !a.quit);
        let screen = snapshot(100, 30, &mut a, false).unwrap();
        assert_eq!(screen.matches("Leave and lose this? y / n").count(), 1);
        assert!(screen.lines().nth(1).unwrap().contains("tinkery"));
        a.paste("y");
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::F(2));
        let mut repeated = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        repeated.kind = KeyEventKind::Repeat;
        a.handle_key(repeated);
        a.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::ALT));
        assert!(!a.quit);
        key(&mut a, KeyCode::Char('n'));
        assert!(!a.leave_prompt && !a.quit);
        assert_eq!(
            serde_json::to_value(&a.sources).unwrap(),
            serde_json::to_value(&source).unwrap()
        );
        assert_eq!(a.input.text, input);
        assert_eq!(a.goal_review.as_ref().map(|r| r.input.clone()), review);
        assert!(!a.running());
        ctrl_c(&mut a);
        key(&mut a, KeyCode::Esc);
        assert!(!a.leave_prompt);
        ctrl_c(&mut a);
        if state % 2 == 0 {
            key(&mut a, KeyCode::Char('y'));
        } else {
            ctrl_c(&mut a);
        }
        assert!(a.quit);
        assert!(a.receipt.is_none());
    }
}
#[test]
fn empty_and_confirmed_exit_immediately_but_pending_durable_handoff_waits() {
    let mut a = BrainDump::default();
    ctrl_c(&mut a);
    assert!(a.quit && !a.leave_prompt);
    let mut a = board();
    a.receipt = Some(handoff::Receipt::test("/tmp/Tinkery-TEST/session".into()));
    ctrl_c(&mut a);
    assert!(a.quit && !a.leave_prompt && a.receipt.is_some());
    let mut a = board();
    a.review_goal();
    let (tx, rx) = std::sync::mpsc::channel();
    a.handoff_job = Some(rx);
    ctrl_c(&mut a);
    assert!(!a.quit && a.exit_after_handoff && !a.leave_prompt);
    assert!(a.notice.contains("durable goal read-back"));
    tx.send(Err("TEST missing helper".into())).unwrap();
    a.tick();
    assert!(a.quit && a.notice.contains("TEST missing helper"));
}
#[cfg(unix)]
#[test]
fn concern_loss_is_logged_not_silently_repaired_or_declared_faithful() {
    use std::os::unix::fs::PermissionsExt;
    for faithful in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("SKILL.md");
        std::fs::write(
            &skill,
            "# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n",
        )
        .unwrap();
        let program = dir.path().join("pi");
        std::fs::write(&program,format!(r#"#!/usr/bin/env python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
p=sys.argv[sys.argv.index('--system-prompt')+1]
worry='i am worried that this thing has been overengineered'
if r.get('kind')=='meaning-preservation':
 assert 'Concerns and negative judgments are concrete content too' in p
 assert r['board']['framings']==r['reading'] and r['mode']=='log-only'
 print(json.dumps(dict(missing=[] if {faithful} else [dict(source=1,quote=worry,occurrence=0)],false_choice=False)));sys.exit(0)
assert worry==r['sources'][0]['text']
assert 'Worries stay worries, not diagnoses, praise or aspirations' in p
log=root/'shapes'
n=int(log.read_text()) if log.exists() else 0
assert n==0
log.write_text(str(n+1))
text='You worry this thing has been overengineered.' if {faithful} else 'Learn this system better, durable and robust.'
print(json.dumps(dict(uncertain=False,framings=[dict(text=text,supports=[dict(source=1,quote=worry,occurrence=0)])],outcome=text,misfits=[],questions=[],alternatives=[])))
"#,faithful=if faithful{"True"}else{"False"})).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let host = Arc::new(PiHost::new(program, "test/model".into(), skill).unwrap());
        let mut a = BrainDump::with_host(host.clone());
        a.paste("i am worried that this thing has been overengineered");
        a.submit();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while (a.running() || a.audit_job.is_some()) && std::time::Instant::now() < deadline {
            a.tick();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!a.running() && a.audit_job.is_none());
        assert!(a.guess.is_some());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("shapes")).unwrap(),
            "1"
        );
        assert_eq!(
            a.sources[0].text,
            "i am worried that this thing has been overengineered"
        );
        let d: checks::Decision = serde_json::from_str(host.diagnostics().last().unwrap()).unwrap();
        assert_eq!(d.mode, "log-only");
        assert_eq!(d.decision, if faithful { "pass" } else { "flag" });
        assert!(
            !snapshot(100, 30, &mut a, false)
                .unwrap()
                .contains("Possible whole-board loss")
        );
        assert!(a.receipt.is_none() && a.handoff_job.is_none());
    }
}
#[test]
fn source_wrap_positions_header_and_scroll_survive_review_and_resize() {
    let text = format!(
        "i am worried that this thing has been overengineered\n{}\nfinal concern remains open",
        "A second exact line with café 👩‍💻, 中, and whitespace.\n".repeat(45)
    );
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut a = board();
        a.sources[0].text = text.clone();
        amend_board(&mut a, |g| g.framings[0].supports.clear());
        a.source_scroll = 7;
        snapshot(w, h, &mut a, false).unwrap();
        a.review_goal();
        assert_eq!(a.source_scroll, 0);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        for step in 0..5 {
            if step == 1 {
                a.handle_mouse(
                    MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column: 3,
                        row: 5,
                        modifiers: KeyModifiers::NONE,
                    },
                    a.area,
                );
                assert!(a.source_scroll > 0);
            }
            if step == 2 {
                key(&mut a, KeyCode::F(1));
            }
            if step == 3 {
                key(&mut a, KeyCode::Char('h'));
            }
            if step == 4 {
                ctrl_c(&mut a);
            }
            terminal
                .draw(|f| render(f, &mut a, Palette::new(false)))
                .unwrap();
            let b = terminal.backend().buffer();
            assert_eq!(b[(2, 1)].symbol(), "t");
            assert_eq!(b[(w - 3, 1)].symbol(), "?");
            let wrapped = Note::new(&text).wrap(a.source_area.width);
            for (index, row, col) in &wrapped.positions {
                let Some(row) = row.checked_sub(usize::from(a.source_scroll)) else {
                    continue;
                };
                if row >= usize::from(a.source_area.height)
                    || *col >= usize::from(a.source_area.width)
                {
                    continue;
                }
                let Some(g) = text[*index..].graphemes(true).next() else {
                    continue;
                };
                if g == "\n" {
                    continue;
                }
                assert_eq!(
                    b[(a.source_area.x + *col as u16, a.source_area.y + row as u16)].symbol(),
                    g,
                    "{w} step {step} byte {index}"
                );
            }
        }
        assert_eq!(a.sources[0].text, text);
        assert!(!a.quit);
    }
}
