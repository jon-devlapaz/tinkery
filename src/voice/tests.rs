use super::*;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Default)]
struct Fake {
    events: VecDeque<Event>,
    commands: Vec<Command>,
    closed: usize,
    restarted: usize,
    fail_send: bool,
}
impl VoiceHost for Rc<RefCell<Fake>> {
    fn send(&mut self, command: Command) -> Result<(), Failure> {
        let mut fake = self.borrow_mut();
        if fake.fail_send {
            return Err(Failure::new("protocol", "Send failed."));
        }
        fake.commands.push(command);
        Ok(())
    }
    fn poll(&mut self) -> Option<Event> {
        self.borrow_mut().events.pop_front()
    }
    fn shutdown(&mut self) {
        self.borrow_mut().closed += 1;
    }
    fn restart(&mut self) {
        self.borrow_mut().restarted += 1;
    }
}
fn fixture() -> (Voice, Rc<RefCell<Fake>>, Instant) {
    let fake = Rc::new(RefCell::new(Fake::default()));
    fake.borrow_mut()
        .events
        .push_back(Event::Ready { version: 1 });
    let mut voice = Voice::new(Box::new(fake.clone()));
    let now = Instant::now();
    voice.tick(now);
    (voice, fake, now)
}
#[test]
fn typed_protocol_tolerates_extra_and_future_fields_not_broken_required_fields() {
    assert_eq!(
        Event::parse(br#"{"event":"ready","version":1,"future":true}"#).unwrap(),
        Event::Ready { version: 1 }
    );
    assert_eq!(
        Event::parse(br#"{"event":"future","anything":true}"#).unwrap(),
        Event::Unknown
    );
    for bad in [
        br#"{"event":"partial","take":1}"#.as_slice(),
        br#"{"event":"ready"}"#,
        b"not json",
    ] {
        assert_eq!(Event::parse(bad).unwrap_err().kind, "protocol");
    }
    assert!(Event::parse(&vec![b'x'; protocol::MAX_LINE + 1]).is_err());
    assert_eq!(
        serde_json::to_string(&Command::Start { take: 3 }).unwrap(),
        r#"{"command":"start","take":3}"#
    );
}
#[test]
fn incompatible_version_disables_and_closes_helper() {
    let (mut voice, fake, now) = fixture();
    voice.close();
    voice.toggle(now);
    fake.borrow_mut()
        .events
        .push_back(Event::Ready { version: 2 });
    voice.tick(now);
    assert!(!voice.available());
    assert_eq!(voice.failure().unwrap().kind, "protocol");
    assert!(fake.borrow().closed >= 2);
}
#[test]
fn partial_replaces_and_final_is_never_an_implicit_send() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    fake.borrow_mut().events.extend([
        Event::Listening { take: 1 },
        Event::Partial {
            take: 1,
            text: "old".into(),
        },
        Event::Partial {
            take: 1,
            text: "new".into(),
        },
    ]);
    assert!(voice.tick(now).is_none());
    assert_eq!(voice.partial(), "new");
    fake.borrow_mut().events.push_back(Event::Final {
        take: 1,
        text: "final".into(),
    });
    let result = voice.tick(now).unwrap();
    assert_eq!(result.text, "final");
    assert!(!result.send);
    assert_eq!(voice.partial(), "");
    assert!(!voice.active());
}
#[test]
fn send_during_start_or_listening_waits_and_sends_exactly_once() {
    for listening in [false, true] {
        let (mut voice, fake, now) = fixture();
        voice.toggle(now);
        if listening {
            fake.borrow_mut()
                .events
                .push_back(Event::Listening { take: 1 });
            voice.tick(now);
        }
        voice.stop(true, now);
        voice.stop(true, now + Duration::from_secs(1));
        voice.toggle(now);
        assert_eq!(
            fake.borrow().commands,
            vec![Command::Start { take: 1 }, Command::Stop { take: 1 }]
        );
        fake.borrow_mut().events.extend([
            Event::Listening { take: 1 },
            Event::Final {
                take: 1,
                text: "words".into(),
            },
            Event::Final {
                take: 1,
                text: "duplicate".into(),
            },
        ]);
        assert!(voice.tick(now).unwrap().send);
        assert!(voice.tick(now).is_none());
    }
}
#[test]
fn cancel_discards_and_stale_events_cannot_touch_new_take() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.stop(true, now);
    voice.cancel();
    voice.toggle(now);
    fake.borrow_mut().events.extend([
        Event::Partial {
            take: 1,
            text: "stale".into(),
        },
        Event::Final {
            take: 1,
            text: "stale".into(),
        },
        Event::Error {
            take: Some(1),
            kind: "capture".into(),
            message: "stale".into(),
        },
        Event::Listening { take: 2 },
    ]);
    assert!(voice.tick(now).is_none());
    assert!(voice.active());
    assert_eq!(voice.partial(), "");
    assert!(voice.failure().is_none());
    fake.borrow_mut().events.push_back(Event::Final {
        take: 2,
        text: "new".into(),
    });
    assert!(!voice.tick(now).unwrap().send);
}
#[test]
fn finalization_deadline_does_not_extend_and_late_final_cannot_send() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.stop(true, now);
    voice.stop(true, now + Duration::from_secs(29));
    assert!(voice.tick(now + Duration::from_secs(30)).is_none());
    assert!(!voice.available());
    assert_eq!(voice.failure().unwrap().kind, "timeout");
    fake.borrow_mut().events.push_back(Event::Final {
        take: 1,
        text: "late".into(),
    });
    assert!(voice.tick(now + Duration::from_secs(31)).is_none());
    assert!(!voice.active());
    voice.toggle(now + Duration::from_secs(32));
    assert_eq!(fake.borrow().restarted, 1);
}
#[test]
fn loading_and_starting_are_bounded_but_listening_waits_for_explicit_stop() {
    let fake = Rc::new(RefCell::new(Fake::default()));
    let mut loading = Voice::new(Box::new(fake));
    loading.tick(Instant::now() + Duration::from_secs(31));
    assert_eq!(loading.failure().unwrap().kind, "timeout");
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    fake.borrow_mut()
        .events
        .push_back(Event::Listening { take: 1 });
    voice.tick(now);
    voice.tick(now + Duration::from_secs(60));
    assert!(voice.active());
}
#[test]
fn every_failure_preserves_specific_kind_and_cancels_pending_send() {
    for kind in [
        "no-node",
        "no-pi-voice",
        "no-model",
        "engine-load",
        "mic-permission",
        "mic-unavailable",
        "mic-busy",
        "capture",
        "protocol",
        "transcription",
    ] {
        let (mut voice, fake, now) = fixture();
        voice.toggle(now);
        voice.stop(true, now);
        fake.borrow_mut().events.push_back(Event::Error {
            take: Some(1),
            kind: kind.into(),
            message: "Action\n\u{1b}required".into(),
        });
        assert!(voice.tick(now).is_none());
        assert_eq!(voice.failure().unwrap().kind, kind);
        assert_eq!(voice.notice(), "Action required");
        fake.borrow_mut().events.push_back(Event::Final {
            take: 1,
            text: "late".into(),
        });
        assert!(voice.tick(now).is_none());
        assert!(!voice.active());
    }
}
#[test]
fn transport_failure_and_global_error_disable_voice() {
    let (mut voice, fake, now) = fixture();
    fake.borrow_mut().fail_send = true;
    voice.toggle(now);
    assert!(!voice.active());
    assert!(!voice.available());
    fake.borrow_mut()
        .events
        .push_back(Failure::new("engine-load", "Engine exited.").event());
    voice.tick(now);
    assert!(!voice.available());
    assert_eq!(voice.notice(), "Engine exited.");
}
#[test]
fn oversized_partial_cancels_without_silent_truncation() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.stop(true, now);
    fake.borrow_mut().events.push_back(Event::Partial {
        take: 1,
        text: "x".repeat(4097),
    });
    voice.tick(now);
    assert!(!voice.active());
    assert!(voice.partial().is_empty());
    assert_eq!(voice.failure().unwrap().kind, "capture");
}
#[test]
fn shutdown_drop_and_restart_do_not_record() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.close();
    assert_eq!(
        fake.borrow().commands,
        vec![Command::Start { take: 1 }, Command::Cancel { take: 1 }]
    );
    voice.toggle(now);
    assert_eq!(fake.borrow().restarted, 1);
    assert!(!voice.active());
    drop(voice);
    assert!(fake.borrow().closed >= 2);
}
#[test]
fn silence_fillers_are_exact_not_a_semantic_rewrite() {
    for text in ["", "   ", ".", "Thank you.", " YOU! "] {
        assert!(silence_filler(text));
    }
    for text in [
        "Thank you for fixing onboarding",
        "you can help",
        "Thank you,",
        "... actual words",
        "你好",
    ] {
        assert!(!silence_filler(text));
    }
}
