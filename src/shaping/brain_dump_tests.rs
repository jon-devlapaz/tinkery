use super::*;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Default)]
struct Recording {
    requests: Mutex<Vec<BoardRequest>>,
    fail: AtomicBool,
}
fn guess(r: &BoardRequest) -> Guess {
    Guess {
        uncertain: false,
        framings: vec![Framing {
            text: format!(
                "Model reading {} originals: make reading comfortable.",
                r.sources.len()
            ),
            supports: r.fragments.iter().map(|f| f.id.clone()).collect(),
        }],
        outcome: "Read comfortably in the blog and RSS reader.".into(),
        misfits: vec![],
        questions: if r.answered.contains(&"reader".into())
            || r.skipped.iter().any(|q| q.id == "reader")
        {
            vec![Question {
                id: "rss-control".into(),
                text: "Who controls RSS appearance?".into(),
            }]
        } else {
            vec![
                Question {
                    id: "reader".into(),
                    text: "What is Hamster?".into(),
                },
                Question {
                    id: "audience".into(),
                    text: "Who needs comfortable reading?".into(),
                },
            ]
        },
        alternatives: [
            "A manual control / candidate",
            "Follow the system setting / candidate",
        ]
        .into_iter()
        .map(|label| Approach {
            label: label.into(),
            benefit: "Comfortable reading.".into(),
            cost: "Check the host.".into(),
            undo_cost: "Unverified.".into(),
        })
        .collect(),
    }
}
impl BoardHost for Recording {
    fn reshape(&self, r: BoardRequest, _: &AtomicBool) -> Result<Guess, String> {
        self.requests.lock().unwrap().push(r.clone());
        if self.fail.load(Ordering::Relaxed) {
            Err("injected failure".into())
        } else {
            Ok(guess(&r))
        }
    }
}
fn key(a: &mut BrainDump, code: KeyCode) {
    a.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn settle(a: &mut BrainDump) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while a.running() && Instant::now() < deadline {
        a.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!a.running());
}
#[test]
fn silent_entry_exact_fragments_and_sources_survive_answer_and_dragged_layout() {
    for (w, h) in [(80, 24), (100, 30), (160, 40), (320, 40)] {
        let host = Arc::new(Recording::default());
        let mut a = BrainDump::with_host(host.clone());
        let first = snapshot(w, h, &mut a, false).unwrap();
        assert!(first.contains("What's on your mind?"));
        assert!(!first.contains("I think this is about"));
        assert!(a.fragments.is_empty());
        let dump = "  I want a toggle.\nHamster is unfamiliar. Café 👩‍💻 matters!  ";
        a.paste(dump);
        a.tick();
        assert!(host.requests.lock().unwrap().is_empty());
        key(&mut a, KeyCode::F(2));
        settle(&mut a);
        assert_eq!(a.sources[0].text, dump);
        assert!(
            snapshot(w, h, &mut a, false)
                .unwrap()
                .contains("Model reading 1 originals")
        );
        a.guess.as_mut().unwrap().questions[0].text = "Should scratch thinking remain entirely separate unless someone explicitly chooses to carry it into the PR?".into();
        assert!(
            snapshot(w, h, &mut a, false).unwrap().contains("PR?"),
            "Focused live question clipped at supported size"
        );
        assert_eq!(a.fragments.len(), 3);
        for f in &a.fragments {
            assert_eq!(&dump[f.start..f.end], f.text);
        }
        assert!(a.paper().contains("Hamster"));
        assert!(
            a.paper()
                .contains("One question in focus\n\nShould scratch thinking")
        );
        if let CanvasNode::Text(n) = &mut a.canvas.state.data.nodes[0] {
            n.x = 1234.;
            n.y = -456.;
        }
        let old = a.layout();
        a.paste("Hamster is my RSS reader; following the system setting is fine.");
        key(&mut a, KeyCode::F(2));
        settle(&mut a);
        assert_eq!(&a.layout()[..old.len()], old.as_slice());
        assert_eq!(a.focused_question().unwrap().id, "rss-control");
        let updated = snapshot(w, h, &mut a, false).unwrap();
        assert!(
            updated.contains("Model reading 2 originals"),
            "Answer did not visibly update centre"
        );
        assert!(!updated.contains("Model reading 1 originals"));
        assert_eq!(a.sources[1].in_reply_to.as_deref(), Some("reader"));
        assert_eq!(a.sources[0].text, dump);
        assert!(a.paper().contains("Hamster is my RSS reader"));
        assert_eq!(host.requests.lock().unwrap().len(), 2);
        assert_eq!(host.requests.lock().unwrap()[1].layout[..old.len()], old);
        key(&mut a, KeyCode::Tab);
        key(&mut a, KeyCode::Char('y'));
        assert_eq!(a.take_copy_request(), Some(a.paper()));
        key(&mut a, KeyCode::Char('o'));
        assert!(
            snapshot(w, h, &mut a, false)
                .unwrap()
                .contains("Original 1")
        );
        key(&mut a, KeyCode::Esc);
        assert!(!a.original);
    }
}
#[test]
fn failure_retry_skip_no_implicit_confirmation_or_new_note_edit_trap() {
    let host = Arc::new(Recording::default());
    let mut a = BrainDump::with_host(host.clone());
    snapshot(100, 30, &mut a, false).unwrap();
    a.submit();
    assert!(host.requests.lock().unwrap().is_empty());
    key(&mut a, KeyCode::Char('n'));
    key(&mut a, KeyCode::Char('e'));
    assert_eq!(a.input.text, "ne");
    a.submit();
    settle(&mut a);
    let before = a.paper();
    key(&mut a, KeyCode::Tab);
    key(&mut a, KeyCode::Char('s'));
    assert_eq!(a.focused_question().unwrap().id, "audience");
    key(&mut a, KeyCode::Tab);
    host.fail.store(true, Ordering::Relaxed);
    a.paste("A second dump.");
    a.submit();
    settle(&mut a);
    assert!(a.notice.contains("injected failure"));
    assert_eq!(a.sources.len(), 2);
    assert_eq!(
        a.guess.as_ref().unwrap().outcome,
        "Read comfortably in the blog and RSS reader."
    );
    assert!(a.originals().contains("A second dump."));
    assert!(a.paper().contains("Nothing confirmed"));
    assert_ne!(a.paper(), before);
    host.fail.store(false, Ordering::Relaxed);
    a.submit();
    settle(&mut a);
    assert_eq!(a.sources.len(), 2);
    assert_eq!(a.focused_question().unwrap().id, "rss-control");
    assert!(
        host.requests
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .skipped
            .iter()
            .any(|q| q.id == "reader")
    );
}
#[test]
fn boundary_rejects_unsafe_unsupported_uncertain_and_skipped_guesses() {
    let host = Arc::new(Recording::default());
    let mut a = BrainDump::with_host(host.clone());
    snapshot(100, 30, &mut a, false).unwrap();
    a.paste("Exact words. More words.");
    a.submit();
    settle(&mut a);
    let r = host.requests.lock().unwrap()[0].clone();
    let g = guess(&r);
    let mut bad = g.clone();
    bad.framings[0].supports = vec!["invented".into()];
    assert!(bad.validate(&r).is_err());
    let mut bad = g.clone();
    bad.framings[0].supports.pop();
    assert!(bad.validate(&r).is_err());
    let mut bad = g.clone();
    bad.questions[0]
        .text
        .push_str(" The Ledger: link shows decisions.");
    assert!(bad.validate(&r).is_err());
    let mut bad = g.clone();
    bad.uncertain = true;
    assert!(bad.validate(&r).is_err());
    bad.framings.push(Framing {
        text: "Another possible meaning".into(),
        supports: r.fragments.iter().map(|f| f.id.clone()).collect(),
    });
    assert!(bad.validate(&r).is_ok());
    let mut bad = g.clone();
    bad.outcome = "\x1b]52;c;payload".into();
    assert!(bad.validate(&r).is_err());
    let mut bad = g.clone();
    bad.alternatives.pop();
    assert!(bad.validate(&r).is_err());
    let mut skipped = r.clone();
    skipped.skipped.push(g.questions[0].clone());
    assert!(g.validate(&skipped).is_err());
    let json = serde_json::to_string(&g).unwrap();
    assert!(serde_json::from_str::<Guess>(&(json.clone() + " extra")).is_err());
    assert!(
        serde_json::from_str::<Guess>(&json.replace(
            "\"uncertain\":false",
            "\"uncertain\":false,\"confirmed\":true"
        ))
        .is_err()
    );
}
#[test]
fn typing_during_request_is_never_replaced_by_the_result() {
    let host = Arc::new(Recording::default());
    let mut a = BrainDump::with_host(host);
    snapshot(100, 30, &mut a, false).unwrap();
    a.paste("First dump.");
    a.submit();
    a.paste("Local text while thinking.");
    settle(&mut a);
    assert_eq!(a.input.text, "Local text while thinking.");
    assert_eq!(a.sources.len(), 1);
    assert!(
        a.guess.is_none(),
        "Agent interrupted new typing with a framing"
    );
    assert!(a.ready.is_some());
    a.submit();
    settle(&mut a);
    assert!(a.guess.is_some());
    assert_eq!(a.sources.len(), 2);
}
#[test]
fn simulated_is_explicit_and_never_an_automatic_real_failure_fallback() {
    let mut a = BrainDump::default();
    snapshot(100, 30, &mut a, false).unwrap();
    a.paste("My own thought.");
    a.submit();
    settle(&mut a);
    assert!(a.guess.unwrap().framings[0].text.contains("Simulated"));
}

#[test]
fn actual_drag_keeps_exact_words_and_layout_on_the_next_submit() {
    let host = Arc::new(Recording::default());
    let mut a = BrainDump::with_host(host);
    snapshot(100, 30, &mut a, false).unwrap();
    a.paste("Read comfortably. Hamster is unfamiliar.");
    a.submit();
    settle(&mut a);
    snapshot(100, 30, &mut a, false).unwrap();
    let area = a.area;
    let rect = a.canvas.area;
    let n = &a.canvas.state.data.nodes[0];
    let (nx, ny) = n.pos();
    let x = ((nx - a.canvas.state.viewport_x) * a.canvas.state.zoom + f64::from(rect.width) / 2.0)
        as u16
        + rect.x
        + 4;
    let y = ((ny - a.canvas.state.viewport_y) * a.canvas.state.zoom + f64::from(rect.height) / 2.0)
        as u16
        + rect.y
        + 2;
    let before = a.layout();
    for (kind, dx, dy) in [
        (MouseEventKind::Down(MouseButton::Left), 0, 0),
        (MouseEventKind::Drag(MouseButton::Left), 4, 2),
        (MouseEventKind::Up(MouseButton::Left), 4, 2),
    ] {
        a.handle_mouse(
            MouseEvent {
                kind,
                column: x + dx,
                row: y + dy,
                modifiers: KeyModifiers::NONE,
            },
            area,
        );
    }
    assert_ne!(a.layout(), before, "Real mouse drag did not move a card");
    let moved = a.layout();
    assert!(a.canvas.state.floating_editor.is_none());
    for (n, f) in a.canvas.state.data.nodes.iter().zip(&a.fragments) {
        assert_eq!(n.text(), f.text);
    }
    a.paste("It is an RSS reader.");
    a.submit();
    settle(&mut a);
    assert_eq!(&a.layout()[..moved.len()], moved.as_slice());
}
#[cfg(unix)]
#[test]
fn board_process_uses_only_shaping_guidance_and_disabled_resources() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    std::fs::write(&skill,"# Seed Me\nFAKE_LEDGER_REPLY_RULE\n### Shape the working draft\nACTUAL_SHAPING_GUIDANCE\n### Size gate\nDO_NOT_RUN_LATER_GATE").unwrap();
    let host = Arc::new(Recording::default());
    let mut a = BrainDump::with_host(host.clone());
    snapshot(100, 30, &mut a, false).unwrap();
    a.paste("My exact words.");
    a.submit();
    settle(&mut a);
    let r = host.requests.lock().unwrap()[0].clone();
    let response = serde_json::to_string(&guess(&r)).unwrap();
    let program = dir.path().join("pi");
    std::fs::write(&program,format!("#!/usr/bin/env python3\nimport sys,json,os\np=sys.argv[sys.argv.index('--system-prompt')+1]\nassert 'ACTUAL_SHAPING_GUIDANCE' in p\nassert 'FAKE_LEDGER_REPLY_RULE' not in p and 'DO_NOT_RUN_LATER_GATE' not in p\nassert 'proposed mechanism EVEN WHEN EXPLICITLY REQUESTED' in p\nassert 'at least TWO credible' in p\nassert 'no appended status, ledger' in p\nassert all(x in sys.argv for x in ['--no-tools','--no-session','--no-extensions','--no-mcp','--no-context-files'])\nassert 'PI_SESSION_FILE' not in os.environ\nr=json.load(sys.stdin)\nassert r['sources'][0]['text']=='My exact words.'\nprint({response:?})\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pi = PiHost::new(program, "test/model".into(), skill).unwrap();
    let actual = pi.reshape(r, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        actual.outcome,
        guess(&host.requests.lock().unwrap()[0]).outcome
    );
}
