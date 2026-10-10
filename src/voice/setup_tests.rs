use super::*;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, Mutex},
};
#[derive(Default)]
struct Fake {
    session: Arc<Mutex<Consent>>,
    events: VecDeque<Event>,
    commands: Vec<Command>,
    restarted: usize,
}
impl VoiceHost for Rc<RefCell<Fake>> {
    fn setup_session(&self) -> Arc<Mutex<Consent>> {
        self.borrow().session.clone()
    }
    fn send(&mut self, command: Command) -> Result<(), Failure> {
        self.borrow_mut().commands.push(command);
        Ok(())
    }
    fn poll(&mut self) -> Option<Event> {
        self.borrow_mut().events.pop_front()
    }
    fn shutdown(&mut self) {}
    fn restart(&mut self) {
        self.borrow_mut().restarted += 1;
    }
}
fn requirement() -> Event {
    Event::NeedsInstall {
        size_mb: 20,
        message: "Voice needs a one-time local install (about 20 MB). Install now? y / n".into(),
    }
}
fn fixture() -> (Voice, Rc<RefCell<Fake>>, Instant) {
    let fake = Rc::new(RefCell::new(Fake::default()));
    fake.borrow_mut().events.push_back(requirement());
    let mut voice = Voice::new(Box::new(fake.clone()));
    let now = Instant::now();
    voice.tick(now);
    (voice, fake, now)
}
#[test]
fn setup_never_installs_until_key_then_explicit_yes_and_success_never_records() {
    let (mut voice, fake, now) = fixture();
    assert!(voice.notice().is_empty());
    assert!(!voice.setup_prompt());
    assert!(!voice.active());
    voice.toggle(now);
    assert!(voice.setup_prompt());
    assert!(voice.notice().ends_with("y / n"));
    assert!(fake.borrow().commands.is_empty());
    voice.answer_setup(true, now);
    assert_eq!(fake.borrow().commands, [Command::Install]);
    assert_eq!(voice.notice(), "installing voice…");
    voice.tick(now + Duration::from_secs(31));
    assert!(voice.failure().is_none());
    fake.borrow_mut()
        .events
        .extend([Event::Loading, Event::Ready { version: 1 }]);
    voice.tick(now + Duration::from_secs(32));
    assert!(voice.available());
    assert!(!voice.active());
    assert!(voice.notice().is_empty());
    voice.toggle(now);
    assert_eq!(
        fake.borrow().commands.last(),
        Some(&Command::Start { take: 1 })
    );
}
#[test]
fn declined_install_is_not_asked_again_even_after_a_new_think_visit() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.answer_setup(false, now);
    for _ in 0..3 {
        voice.toggle(now);
        assert!(!voice.setup_prompt());
        assert!(voice.notice().is_empty());
    }
    assert!(fake.borrow().commands.is_empty());
    voice.close();
    fake.borrow_mut().events.push_back(requirement());
    let mut next = Voice::new(Box::new(fake.clone()));
    next.tick(now);
    next.toggle(now);
    assert!(!next.setup_prompt());
    assert!(fake.borrow().commands.is_empty());
}
#[test]
fn failure_requires_an_explicit_retry_key_but_not_repeating_unchanged_consent() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.answer_setup(true, now);
    fake.borrow_mut().events.push_back(
        Failure::new(
            "install",
            "Voice install failed: registry unavailable. Press the voice key to retry.",
        )
        .event(),
    );
    voice.tick(now);
    assert_eq!(fake.borrow().commands, [Command::Install]);
    voice.tick(now);
    assert_eq!(fake.borrow().restarted, 0);
    voice.toggle(now);
    assert_eq!(fake.borrow().restarted, 1);
    fake.borrow_mut().events.push_back(requirement());
    voice.tick(now);
    assert!(!voice.setup_prompt());
    assert_eq!(fake.borrow().commands, [Command::Install, Command::Install]);
}
#[test]
fn changed_node_major_requires_fresh_reinstall_consent() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.answer_setup(true, now);
    voice.close();
    voice.toggle(now);
    fake.borrow_mut().events.push_back(Event::NeedsInstall {
        size_mb: 20,
        message: "Voice used Node 26; found 28. Reinstall (~20 MB)? y / n".into(),
    });
    voice.tick(now);
    assert!(voice.setup_prompt());
    assert_eq!(fake.borrow().commands, [Command::Install]);
    voice.answer_setup(true, now);
    assert_eq!(fake.borrow().commands, [Command::Install, Command::Install]);
}
#[test]
fn model_download_consent_is_separate_and_declined_model_does_not_download() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.answer_setup(true, now);
    fake.borrow_mut().events.push_back(Event::NeedsModel {
        size_mb: 732,
        message: "Voice model is missing (732 MB). Download now? y / n".into(),
    });
    voice.tick(now);
    assert!(voice.setup_prompt());
    assert!(voice.notice().contains("732 MB"));
    voice.answer_setup(false, now);
    voice.toggle(now);
    assert!(!voice.setup_prompt());
    assert_eq!(fake.borrow().commands, [Command::Install]);
    let (mut other, other_fake, now) = fixture();
    other_fake.borrow_mut().events.push_back(Event::NeedsModel {
        size_mb: 732,
        message: "Download now? y / n".into(),
    });
    other.tick(now);
    other.toggle(now);
    other.answer_setup(true, now);
    assert_eq!(other_fake.borrow().commands, [Command::Download]);
}
#[test]
fn install_deadline_closes_work_and_stale_ready_cannot_restart_it() {
    let (mut voice, fake, now) = fixture();
    voice.toggle(now);
    voice.answer_setup(true, now);
    voice.tick(now + Duration::from_secs(600));
    assert_eq!(voice.failure().unwrap().kind, "timeout");
    fake.borrow_mut()
        .events
        .push_back(Event::Ready { version: 1 });
    voice.tick(now);
    assert!(!voice.available());
}
#[test]
fn key_label_matches_platform_and_setup_protocol_stays_typed() {
    assert_eq!(key_label("macos"), "⌃⌥Z");
    assert_eq!(key_label("linux"), "6");
    assert_eq!(key_label("windows"), "6");
    assert_eq!(
        serde_json::to_string(&Command::Install).unwrap(),
        r#"{"command":"install"}"#
    );
    assert!(matches!(
        Event::parse(br#"{"event":"needsmodel","size_mb":732,"message":"Download?"}"#).unwrap(),
        Event::NeedsModel { .. }
    ));
    assert!(Event::parse(br#"{"event":"needsinstall","message":"bad"}"#).is_err());
}
