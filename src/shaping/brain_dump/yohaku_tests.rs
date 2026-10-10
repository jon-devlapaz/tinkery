use super::goal_tests::{board, right_text};
use super::*;
fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
#[test]
fn evidence_labels_are_plain_and_leave_source_bytes_untouched() {
    let mut source = Source {
        id: 2,
        text: "  both\n".into(),
        in_reply_to: Some("compounding-referent".into()),
    };
    assert_eq!(
        handoff::source_label(&source),
        "Tinkery original 2 / reply to compounding-referent"
    );
    assert_eq!(source.text, "  both\n");
    source.in_reply_to = None;
    assert_eq!(handoff::source_label(&source), "Tinkery original 2");
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
        let source_area = a.source_area;
        a.details = true;
        a.review_goal();
        assert!(!a.details);
        let review = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(a.source_area, source_area);
        // The key bar (last row) follows the state; everything above it stays put.
        let (mut l1, mut l2) = (left(&before, w), left(&review, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        let review_text = right_text(&review, w);
        assert!(review_text.contains("Running out of questions doesn't mean I understood you."));
        assert!(
            review_text.contains("Creates a Seed Me session; this goal can't be edited after.")
        );
        assert!(review_text.contains("type confirm"));
        key(&mut a, KeyCode::F(1));
        let help = snapshot(w, h, &mut a, false).unwrap();
        assert!(help.contains("Right now"));
        let (mut l1, mut l2) = (left(&before, w), left(&help, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        key(&mut a, KeyCode::Esc);
        a.goal_review = None;
        a.receipt = Some(handoff::Receipt::test(
            "/tmp/Tinkery-TEST/session-id".into(),
        ));
        let receipt = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(a.source_area, source_area);
        let (mut l1, mut l2) = (left(&before, w), left(&receipt, w));
        l1.pop();
        l2.pop();
        assert_eq!(l1, l2);
        assert_eq!(
            receipt
                .matches("Goal confirmed — seed not written yet")
                .count(),
            1
        );
        assert!(receipt.contains("Continue with Seed Me in any harness"));
        let next_row = receipt
            .lines()
            .position(|l| l.contains("Continue with Seed Me"))
            .unwrap();
        let path_row = receipt
            .lines()
            .position(|l| l.contains("Tinkery-TEST/session-id"))
            .unwrap();
        assert_eq!(
            next_row,
            path_row + 2,
            "Next action drifted into footer chrome"
        );
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
        assert!(a.help);
        key(&mut a, KeyCode::Esc);
        assert!(!a.quit && !a.help);
        assert!(a.receipt.is_some());
    }
}
#[test]
fn literal_question_mark_and_corner_help_preserve_local_typing_and_review() {
    let mut a = board();
    a.paste("A literal");
    key(&mut a, KeyCode::Char('?'));
    assert_eq!(a.input.text, "A literal?");
    assert!(!a.help);
    let saved = (a.input.text.clone(), a.input.cursor);
    snapshot(100, 30, &mut a, false).unwrap();
    a.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 97,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        a.area,
    );
    assert!(a.help);
    snapshot(100, 30, &mut a, false).unwrap();
    key(&mut a, KeyCode::Esc);
    assert_eq!(a.input.text, saved.0);
    assert_eq!(a.input.cursor, saved.1);
    assert!(!a.running());
}
#[test]
fn long_receipt_path_is_scrollable_without_hiding_the_next_action() {
    let mut a = board();
    let path = format!(
        "/tmp/Tinkery-TEST/{}/END-SESSION-ID",
        "long-path-component/".repeat(35)
    );
    a.receipt = Some(handoff::Receipt::test(path.clone().into()));
    for (w, h) in [(80, 24), (100, 30), (320, 40)] {
        a.paper_scroll = 0;
        let first = snapshot(w, h, &mut a, false).unwrap();
        assert!(first.contains("Continue with Seed Me in any harness"));
        for _ in 0..50 {
            key(&mut a, KeyCode::PageDown);
        }
        let last = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            right_text(&last, w)
                .replace(' ', "")
                .contains("END-SESSION-ID")
        );
        assert!(last.contains("Continue with Seed Me in any harness"));
        assert_eq!(a.receipt.as_ref().unwrap().session.to_string_lossy(), path);
    }
}
#[test]
fn full_failure_reason_is_accessible_and_never_promotes_a_rejected_reading() {
    let mut a = board();
    let old = a.guess.clone().unwrap();
    a.notice = format!(
        "Reshape failed: {} Or the final concrete referent was erased. Originals retained.",
        "source-grounded rejection explanation ".repeat(50)
    );
    a.applied = 0;
    let screen = snapshot(100, 30, &mut a, false).unwrap();
    assert!(screen.contains("Reading couldn't be updated."));
    assert_eq!(
        a.guess.as_ref().unwrap().framings[0].text,
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
fn background_advice_does_not_delay_display_help_or_local_typing_and_is_owned() {
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
import json,sys,pathlib,time
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
if r.get('kind')=='meaning-preservation':
 (root/'audit-active').write_text('ready')
 time.sleep(30)
 print(json.dumps(dict(missing=[],false_choice=True)));sys.exit(0)
log=root/'shapes'
n=int(log.read_text()) if log.exists() else 0
log.write_text(str(n+1))
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the result.',supports=[dict(source=1,quote=r['sources'][0]['text'],occurrence=0)])],outcome='A judged result.',misfits=[],questions=[dict(id='boundary',text='When should you taste the result?')] if not r['settled'] else [],alternatives=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Arc::new(PiHost::new(program, "test/model".into(), skill).unwrap());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("the restaurateur judges the result");
    a.submit();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !dir.path().join("audit-active").exists() && std::time::Instant::now() < deadline {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(dir.path().join("audit-active").exists());
    assert!(a.guess.is_some() && !a.running() && a.audit_job.is_some());
    a.paste("before final judgment");
    key(&mut a, KeyCode::F(1));
    let start = std::time::Instant::now();
    snapshot(100, 30, &mut a, false).unwrap();
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    key(&mut a, KeyCode::Esc);
    assert_eq!(a.input.text, "before final judgment");
    a.submit();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !host
        .diagnostics()
        .iter()
        .any(|s| s.contains("Request cancelled"))
        && std::time::Instant::now() < deadline
    {
        a.tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        host.diagnostics()
            .iter()
            .any(|s| s.contains("Request cancelled"))
    );
    assert_eq!(a.sources[0].text, "the restaurateur judges the result");
    assert_eq!(a.sources[1].text, "before final judgment");
    assert!(a.receipt.is_none());
    a.audit_job = None;
}
#[cfg(unix)]
#[test]
fn one_shape_then_one_log_only_audit_never_auto_repairs_or_withholds() {
    use std::os::unix::fs::PermissionsExt;
    for (mode, success) in [
        ("pass", true),
        ("flag", true),
        ("syntax", false),
        ("audit-syntax", true),
        ("audit-anchor", true),
        ("continuity", true),
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
if kind=='meaning-preservation':
 assert r['mode']=='log-only' and 'questions' in r['board'] and 'misfits' in r['board']
 if mode=='audit-syntax':print('not JSON');sys.exit(0)
 if mode=='audit-anchor':print(json.dumps(dict(missing=[dict(source=1,quote='invented',occurrence=0)],false_choice=False)));sys.exit(0)
 print(json.dumps(dict(missing=[dict(source=1,quote='restaurateur',occurrence=0)] if mode=='flag' else [],false_choice=False,repeated=['new'] if mode=='continuity' else [])));sys.exit(0)
assert not previous
if mode=='transport':sys.exit(2)
if mode=='syntax':print('not JSON');sys.exit(0)
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the result.',supports=[dict(source=1,quote='invented' if mode=='invalid' else r['sources'][0]['text'],occurrence=0)])],outcome='A judged result.',misfits=[],questions=[dict(id='new',text='Which boundary matters?')] if mode=='continuity' else [],alternatives=[])))
"#)).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let pi = PiHost::new(program, "test/model".into(), skill)
            .unwrap()
            .with_default_thinking();
        let result = pi.reshape(r.clone(), &AtomicBool::new(false));
        assert_eq!(result.is_ok(), success, "{mode}: {result:?}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("calls.jsonl"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        if let Ok(g) = result {
            let before = serde_json::to_value(&g).unwrap();
            let d = pi.advisory(r, g.clone(), &AtomicBool::new(false));
            assert_eq!(d.mode, "log-only");
            assert_eq!(
                d.decision,
                match mode {
                    "flag" | "continuity" => "flag",
                    "audit-syntax" | "audit-anchor" => "error",
                    _ => "pass",
                }
            );
            assert_eq!(before, serde_json::to_value(&g).unwrap());
            if mode == "continuity" {
                assert_eq!(g.questions.len(), 1);
            }
        }
        let calls = std::fs::read_to_string(dir.path().join("calls.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str::<serde_json::Value>(s).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(calls.iter().filter(|c| c["kind"] == "shape").count(), 1);
        assert_eq!(
            calls
                .iter()
                .filter(|c| c["kind"] == "meaning-preservation")
                .count(),
            usize::from(success)
        );
        for log in pi.diagnostics() {
            let d: checks::Decision = serde_json::from_str(&log).unwrap();
            assert!(!d.request_key.is_empty());
        }
        if mode == "invalid" {
            assert!(checks::why(&pi.diagnostics()).contains("Rejected attempt — not confirmable"));
        }
    }
}
