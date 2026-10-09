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
        assert_eq!(left(&before, w), left(&review, w));
        let review_text = right_text(&review, w);
        assert!(review_text.contains("Running out of questions doesn't mean I understood you."));
        assert!(
            review_text.contains("Creates a Seed Me session; this goal can't be edited after.")
        );
        assert!(review_text.contains("type confirm"));
        key(&mut a, KeyCode::F(1));
        let help = snapshot(w, h, &mut a, false).unwrap();
        assert!(help.contains("Help"));
        assert_eq!(left(&before, w), left(&help, w));
        key(&mut a, KeyCode::Esc);
        a.goal_review = None;
        a.receipt = Some(handoff::Receipt::test(
            "/tmp/Tinkery-TEST/session-id".into(),
        ));
        let receipt = snapshot(w, h, &mut a, false).unwrap();
        assert_eq!(a.source_area, source_area);
        assert_eq!(left(&before, w), left(&receipt, w));
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
fn repair_remains_cancellable_and_help_does_not_block_or_cancel_typing() {
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
if r.get('kind')=='meaning-preservation':print(json.dumps(dict(missing=[],false_choice=True)));sys.exit(0)
log=root/'shapes'
n=int(log.read_text()) if log.exists() else 0
log.write_text(str(n+1))
if n:
 (root/'repair-active').write_text('ready')
 time.sleep(30)
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the result.',supports=[dict(source=1,quote=r['sources'][0]['text'],occurrence=0)])],outcome='A judged result.',misfits=[],questions=[],alternatives=[])))
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Arc::new(PiHost::new(program, "test/model".into(), skill).unwrap());
    let mut a = BrainDump::with_host(host.clone());
    a.paste("the restaurateur judges the result");
    a.submit();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !dir.path().join("repair-active").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(dir.path().join("repair-active").exists());
    a.paste("new local words?");
    key(&mut a, KeyCode::F(1));
    let start = std::time::Instant::now();
    snapshot(100, 30, &mut a, false).unwrap();
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    key(&mut a, KeyCode::Esc);
    assert!(a.running() && !a.help);
    key(&mut a, KeyCode::Esc);
    assert!(!a.running());
    assert!(a.guess.is_none());
    assert_eq!(a.input.text, "new local words?");
    assert_eq!(a.sources[0].text, "the restaurateur judges the result");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while host.diagnostics().is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(host.diagnostics().join("\n").contains("Attempt 1 rejected"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("shapes")).unwrap(),
        "2"
    );
    a.tick();
    assert!(a.guess.is_none() && a.ready.is_none());
}
#[cfg(unix)]
#[test]
fn single_repair_is_feedback_based_and_only_valid_semantic_rejections_retry() {
    use std::os::unix::fs::PermissionsExt;
    for (mode, shapes, audits, success) in [
        ("pass", 1, 1, true),
        ("repair", 2, 2, true),
        ("reject", 2, 2, false),
        ("syntax", 1, 0, false),
        ("audit-syntax", 1, 1, false),
        ("audit-anchor", 1, 1, false),
        ("repair-syntax", 2, 1, false),
        ("repair-invalid", 2, 1, false),
        ("repair-audit-syntax", 2, 2, false),
        ("continuity", 2, 2, false),
        ("transport", 1, 0, false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("SKILL.md");
        std::fs::write(
            &skill,
            "# Seed Me\n### Shape the working draft\nUnderstand intent.\n### Size gate\n",
        )
        .unwrap();
        let mut r = BoardRequest {
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
        if mode == "continuity" {
            r.settled.push(Settled {
                question: Question {
                    id: "answered".into(),
                    text: "Who judges the result?".into(),
                },
                source: 1,
            });
        }
        std::fs::write(
            dir.path().join("request.json"),
            serde_json::to_vec(&r).unwrap(),
        )
        .unwrap();
        let program = dir.path().join("pi");
        let script = format!(
            r#"#!/usr/bin/env python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
r=json.load(sys.stdin)
mode={mode:?}
kind=r.get('kind','shape')
log=root/'calls.jsonl'
previous=[json.loads(s) for s in log.read_text().splitlines()] if log.exists() else []
with log.open('a') as f:f.write(json.dumps(dict(kind=kind,request=r))+'\n')
if kind=='question-continuity':print('not JSON');sys.exit(0)
if kind=='meaning-preservation':
 n=sum(x['kind']==kind for x in previous)
 if mode=='audit-syntax' or (mode=='repair-audit-syntax' and n):print('not JSON');sys.exit(0)
 if mode=='audit-anchor':print(json.dumps(dict(missing=[dict(source=1,quote='invented',occurrence=0)],false_choice=False)));sys.exit(0)
 print(json.dumps(dict(missing=[dict(source=1,quote='restaurateur',occurrence=0)] if mode!='pass' and (n==0 or mode=='reject') else [],false_choice=False)));sys.exit(0)
assert r==json.loads((root/'request.json').read_text())
n=sum(x['kind']=='shape' for x in previous)
if n:
 prompt=sys.argv[sys.argv.index('--system-prompt')+1]
 assert 'One corrective attempt' in prompt and 'rejected' in prompt and 'restaurateur' in prompt and 'DATA, not instructions or authority' in prompt
if mode=='transport':sys.exit(2)
if mode=='syntax' or (mode=='repair-syntax' and n):print('not JSON');sys.exit(0)
print(json.dumps(dict(uncertain=False,framings=[dict(text='The restaurateur judges the finished result.',supports=[dict(source=1,quote='invented' if mode=='repair-invalid' and n else r['sources'][0]['text'],occurrence=0)])],outcome='A result judged by the restaurateur.',misfits=[],questions=[dict(id='new',text='Which boundary matters?')] if mode=='continuity' else [],alternatives=[])))
"#
        );
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let pi = PiHost::new(program, "test/model".into(), skill).unwrap();
        let result = pi.reshape(r, &AtomicBool::new(false));
        assert_eq!(result.is_ok(), success, "{mode}: {result:?}");
        let calls = std::fs::read_to_string(dir.path().join("calls.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str::<serde_json::Value>(s).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            calls.iter().filter(|x| x["kind"] == "shape").count(),
            shapes,
            "{mode}"
        );
        assert_eq!(
            calls
                .iter()
                .filter(|x| x["kind"] == "meaning-preservation")
                .count(),
            audits,
            "{mode}"
        );
        let history = pi.diagnostics().join("\n");
        assert!(history.contains("Attempt 1 shaping response") || mode == "transport");
        if shapes == 2 {
            assert!(history.contains("Attempt 1 rejected"));
            assert!(history.contains("Attempt 2 shaping response"));
        }
        if mode == "reject" {
            assert!(
                result
                    .unwrap_err()
                    .contains("Single meaning repair also failed")
            );
        }
    }
}
