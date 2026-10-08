use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tinkery::{
    MIN_HEIGHT, MIN_WIDTH,
    shaping::{
        drafting::PiHost,
        scratchpad::{self, Scratchpad},
    },
};

fn key(app: &mut Scratchpad, code: KeyCode, area: Rect) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), area);
}
fn settle(app: &mut Scratchpad, area: Rect) {
    let deadline = Instant::now() + Duration::from_secs(100);
    while app.request_running() && Instant::now() < deadline {
        app.tick(Duration::ZERO, area);
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!app.request_running(), "Request did not settle");
    let visible = scratchpad::snapshot(area.width, area.height, app, false).unwrap();
    assert!(
        visible.contains("Working paper / model"),
        "No valid model draft:\n{visible}"
    );
    assert!(
        !visible.contains("Draft failed:"),
        "Model request failed:\n{visible}"
    );
}
fn save(app: &mut Scratchpad, directory: &std::path::Path, name: &str, area: Rect) {
    std::fs::write(directory.join(format!("{name}.md")), app.paper()).unwrap();
    let rendered = scratchpad::snapshot(area.width, area.height, app, false).unwrap();
    let view = serde_json::json!({
        "width": area.width,
        "height": area.height,
        "lines": rendered.lines().collect::<Vec<_>>(),
    });
    std::fs::write(
        directory.join(format!("{name}-{}x{}.json", area.width, area.height)),
        serde_json::to_string_pretty(&view).unwrap() + "\n",
    )
    .unwrap();
}
fn main() {
    assert_eq!((MIN_WIDTH, MIN_HEIGHT), (80, 24));
    let directory = PathBuf::from(std::env::args().nth(1).expect("Evidence directory"));
    std::fs::create_dir_all(&directory).unwrap();
    assert!(
        std::fs::read_dir(&directory).unwrap().next().is_none(),
        "Evidence directory must be empty; do not overwrite a prior run"
    );
    let model = std::env::var("TINKERY_LIVE_MODEL").expect("Explicit model");
    let skill = std::env::var("TINKERY_SEED_ME").expect("Actual instruction path");
    for (name, input, width, height) in [
        (
            "01-problem",
            "I keep losing track of why we made certain choices.\n\nI want those reasons near the work.",
            100,
            30,
        ),
        (
            "02-plan",
            "I want a graph connecting decisions to work.\n\nSo I can find why we made those choices.",
            80,
            24,
        ),
    ] {
        let area = Rect::new(0, 0, width, height);
        let host = PiHost::new("pi".into(), model.clone(), skill.clone().into()).unwrap();
        let mut app = Scratchpad::with_host(Arc::new(host));
        scratchpad::snapshot(width, height, &mut app, false).unwrap();
        key(&mut app, KeyCode::Enter, area);
        app.paste(input, area);
        key(&mut app, KeyCode::F(2), area);
        assert!(app.request_running());
        settle(&mut app, area);
        assert_eq!(app.sent_note(), Some(input));
        assert_eq!(app.notes()[0].text(), input);
        save(&mut app, &directory, name, area);
        println!("PASS: {name}, actual {model}, {width}x{height}; complete working paper captured");
        if name == "02-plan" && std::env::var_os("TINKERY_LIVE_SKIP_FEEDBACK").is_none() {
            key(&mut app, KeyCode::Char('r'), area);
            app.paste("Keep finding reasons near the work. Cut the graph as the leading option. Reshape the goal around being able to revisit a choice without losing its context.", area);
            key(&mut app, KeyCode::F(2), area);
            settle(&mut app, area);
            assert_eq!(app.sent_note(), Some(input));
            assert_eq!(app.notes()[0].text(), input);
            save(&mut app, &directory, "03-feedback", area);
            println!(
                "PASS: feedback revision, actual {model}, {width}x{height}; originals unchanged"
            );
        }
    }
    println!("Evidence: {}", directory.display());
}
