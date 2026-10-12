use super::goal_tests::{board, right_text};
use super::*;
fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn left(screen: &str, w: u16) -> Vec<String> {
    screen
        .lines()
        .map(|line| line.chars().take(usize::from(w * 48 / 100)).collect())
        .collect()
}
#[test]
fn one_help_control_and_stable_original_across_review_help_and_receipt() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let mut empty = BrainDump::default();
        let screen = snapshot(w, h, &mut empty, false).unwrap();
        assert_eq!(screen.lines().nth(1).unwrap().matches('?').count(), 1);
        assert!(!screen.contains("F2"));
        let mut a = board();
        let before = snapshot(w, h, &mut a, false).unwrap();
        let source_area = a.screen.source_area;
        a.screen.details = true;
        a.review_goal();
        assert!(!a.screen.details);
        let review = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(a.screen.source_area, source_area);
        // The key bar (last row) follows the state; everything above it stays put.
        let (mut l1, mut l2) = (left(&before, w), left(&review, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        let review_text = right_text(&review, w);
        assert!(!review_text.contains("Running out of questions"));
        assert!(review_text.contains("type confirm to save. You can't edit it after."));
        assert!(review_text.contains("type confirm"));
        key(&mut a, KeyCode::F(1));
        let help = snapshot(w, h, &mut a, false).unwrap();
        assert!(help.contains("Right now"));
        let (mut l1, mut l2) = (left(&before, w), left(&help, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        key(&mut a, KeyCode::Esc);
        a.handoff.review = None;
        a.handoff.receipt = Some(seed_me::Receipt::test(
            "/tmp/Tinkery-TEST/session-id".into(),
        ));
        let receipt = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(a.screen.source_area, source_area);
        let (mut l1, mut l2) = (left(&before, w), left(&receipt, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        assert_eq!(receipt.matches("Saved.").count(), 1);
        assert!(right_text(&receipt, w).contains("Next: shape it into a seed with Seed Me."));
        assert!(!receipt.contains("Tinkery-TEST/session-id"));
        assert!(!receipt.contains("seed not written yet"));
        for s in [&before, &review, &receipt] {
            for clutter in [
                "Ctrl-",
                "F2",
                "Tab",
                "real Pi",
                "/ provisional",
                "PROVISIONAL",
                "q exits",
            ] {
                assert!(!s.contains(clutter), "{clutter}");
            }
            assert_eq!(s.lines().nth(1).unwrap().matches('?').count(), 1);
        }
        key(&mut a, KeyCode::F(1));
        snapshot(w, h, &mut a, false).unwrap();
        assert!(a.screen.help);
        key(&mut a, KeyCode::Esc);
        assert!(!a.quitting() && !a.screen.help);
        assert!(a.handoff.receipt.is_some());
    }
}
#[test]
fn literal_question_mark_and_corner_help_preserve_local_typing_and_review() {
    let mut a = board();
    a.paste("A literal");
    key(&mut a, KeyCode::Char('?'));
    assert_eq!(a.input.text, "A literal?");
    assert!(!a.screen.help);
    let saved = (a.input.text.clone(), a.input.cursor);
    snapshot(100, 30, &mut a, false).unwrap();
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 97,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        a.screen.area,
    );
    assert!(a.screen.help);
    snapshot(100, 30, &mut a, false).unwrap();
    key(&mut a, KeyCode::Esc);
    assert_eq!(a.input.text, saved.0);
    assert_eq!(a.input.cursor, saved.1);
    assert!(!a.running());
}
#[test]
fn long_receipt_goal_is_scrollable_and_path_is_only_in_how_it_works() {
    let mut a = board();
    let path = format!(
        "/tmp/Tinkery-TEST/{}/END-SESSION-ID",
        "long-path-component/".repeat(35)
    );
    let mut receipt = seed_me::Receipt::test(path.clone().into());
    receipt.goal = format!("{}\nEND-GOAL", "Long confirmed goal line.\n".repeat(80));
    a.handoff.receipt = Some(receipt);
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        a.screen.paper_scroll = 0;
        let first = snapshot(w, h, &mut a, false).unwrap();
        assert!(right_text(&first, w).contains("Next: shape it into a seed with Seed Me."));
        assert!(!first.contains("Tinkery-TEST"));
        for _ in 0..50 {
            key(&mut a, KeyCode::PageDown);
        }
        let last = snapshot(w, h, &mut a, false).unwrap();
        assert!(right_text(&last, w).contains("END-GOAL"));
        assert!(right_text(&last, w).contains("Next: shape it into a seed with Seed Me."));
        assert!(!last.contains("END-SESSION-ID"));
        assert!(layout::how_text(&a).contains(&path));
        assert_eq!(
            a.handoff
                .receipt
                .as_ref()
                .unwrap()
                .session
                .to_string_lossy(),
            path
        );
    }
}
#[test]
fn full_failure_reason_is_accessible_and_never_promotes_a_rejected_reading() {
    let mut a = board();
    let old = a.conversation.board().cloned().unwrap();
    a.notice = format!(
        "Reshape failed: {} Or the final concrete referent was erased. Originals retained.",
        "source-grounded rejection explanation ".repeat(50)
    );
    a.conversation.set_applied(0);
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("Reading couldn't be updated."));
    assert_eq!(
        a.conversation.board().unwrap().framings[0].text,
        old.framings[0].text
    );
    key(&mut a, KeyCode::F(1));
    key(&mut a, KeyCode::Char('h'));
    for _ in 0..100 {
        key(&mut a, KeyCode::PageDown);
    }
    let last = snapshot(100, 30, &mut a, false).unwrap();
    assert!(
        right_text(&last, 100)
            .contains("Or the final concrete referent was erased. Originals retained.")
    );
}
#[cfg(unix)]
#[test]
fn one_shape_then_one_review_never_auto_repairs_or_withholds() {
    use std::os::unix::fs::PermissionsExt;
    for (mode, success) in [
        ("pass", true),
        ("syntax", false),
        ("invalid", false),
        ("transport", false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("SKILL.md");
        std::fs::write(
            &skill,
            "# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n",
        )
        .unwrap();
        let r = BoardRequest {
            ask_counts: Default::default(),
            sources: vec![Source {
                id: 1,
                text: "the restaurateur judges the result".into(),
                in_reply_to: None,
            }],
            fragments: vec![],
            previous: None,
            skipped: vec![],
            answered: vec![],
            settled: vec![],
            layout: vec![],
        };
        let program = dir.path().join("pi");
        std::fs::write(&program,format!(r#"#!/usr/bin/env python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
mode={mode:?}
kind=r.get('kind','shape')
assert '--thinking' not in sys.argv
log=root/'calls.jsonl'
previous=[json.loads(s) for s in log.read_text().splitlines()] if log.exists() else []
with log.open('a') as f:f.write(json.dumps(dict(kind=kind,request=r))+'\n')
if kind=='decision-review':
 assert sorted(r)==['answers','draft','kind','words'] and r['words']==['the restaurateur judges the result']
 p=sys.argv[sys.argv.index('--system-prompt')+1]
 assert 'how an action is carried out' in p and 'edge cases and overlaps' in p
 assert 'return none unless another decision is as costly' in p
 (root/'review-ok').write_text('1')
 print(json.dumps(dict(decisions=[])));sys.exit(0)
assert not previous
if mode=='transport':sys.exit(2)
if mode=='syntax':print('not JSON');sys.exit(0)
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the result.',supports=[dict(source=1,quote='invented' if mode=='invalid' else r['sources'][0]['text'],occurrence=0)])],outcome='A judged result.',misfits=[],questions=[],alternatives=[])))
"#)).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let pi = PiHost::new(program, "test/model".into(), skill)
            .unwrap()
            .with_default_thinking();
        let result = pi.reshape(r.clone(), &AtomicBool::new(false));
        assert_eq!(result.is_ok(), success, "{mode}: {result:?}");
        let calls = std::fs::read_to_string(dir.path().join("calls.jsonl")).unwrap();
        let reviews = calls
            .lines()
            .filter(|l| l.contains("\"decision-review\""))
            .count();
        assert_eq!(
            calls.lines().count() - reviews,
            1,
            "{mode}: one shaping call"
        );
        assert!(
            calls
                .lines()
                .all(|l| l.contains("\"shape\"") || l.contains("\"decision-review\"")),
            "{mode}: only shaping and review calls: {calls}"
        );
        // The decision review runs only on a readable board, once.
        let readable = !matches!(mode, "syntax" | "transport");
        assert_eq!(reviews, usize::from(readable), "{mode}: {calls}");
        assert_eq!(dir.path().join("review-ok").exists(), readable, "{mode}");
        for log in pi.diagnostics() {
            let d: checks::Decision = serde_json::from_str(&log).unwrap();
            assert!(!d.request_key.is_empty());
        }
        if mode == "invalid" {
            assert!(checks::why(&pi.diagnostics()).contains("Rejected attempt — not confirmable"));
        }
    }
}
#[cfg(unix)]
#[test]
fn a_send_costs_a_shaping_call_and_a_review_and_nothing_runs_after_display() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    std::fs::write(
        &skill,
        "# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n",
    )
    .unwrap();
    let program = dir.path().join("pi");
    std::fs::write(&program,r#"#!/usr/bin/env python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
kind=r.get('kind','shape')
with (root/'calls').open('a') as f:f.write(kind+'\n')
if kind=='decision-review':print(json.dumps(dict(decisions=[])));sys.exit(0)
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the result.',supports=[dict(source=1,quote=r['sources'][0]['text'],occurrence=0)])],outcome='A judged result.',misfits=[],questions=[],alternatives=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Arc::new(PiHost::new(program, "test/model".into(), skill).unwrap());
    let mut a = BrainDump::with_host(host);
    a.paste("the restaurateur judges the result");
    a.submit();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while a.running() && std::time::Instant::now() < deadline {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(a.conversation.board().is_some() && !a.running());
    // Nothing starts after the reading is shown: keep ticking, and no further call arrives.
    let settle = std::time::Instant::now() + std::time::Duration::from_millis(800);
    while std::time::Instant::now() < settle {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    drop(a);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("calls")).unwrap(),
        "shape\ndecision-review\n"
    );
}
