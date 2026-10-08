use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use tinkery::shaping::{Note, Shape, snapshot};

fn key(app: &mut Shape, code: KeyCode) {
    app.handle_key(
        KeyEvent::new(code, KeyModifiers::NONE),
        Rect::new(0, 0, 100, 30),
    );
}

fn finish(app: &mut Shape) {
    for _ in 0..5 {
        app.tick(Duration::from_secs(1), Rect::new(0, 0, 100, 30));
    }
}

#[test]
fn example_becomes_one_tentative_goal_outcome_and_open_question() {
    let mut app = Shape::default();
    let original = app.note.text.clone();
    app.send();
    finish(&mut app);
    let paper = app.paper();
    for heading in [
        "## Tentative goal",
        "## Possible outcome",
        "## One open question",
    ] {
        assert_eq!(paper.matches(heading).count(), 1);
    }
    assert_eq!(paper.matches('?').count(), 1);
    assert!(paper.contains("Keep the reasons behind important decisions"));
    assert!(paper.contains("Open a work item and see what was chosen"));
    assert!(paper.contains("Which decisions are worth keeping"));
    assert!(!paper.contains(&original));
    assert_eq!(app.note.text, original);
    assert_eq!(app.sent_note(), Some(original.as_str()));
    let screen = snapshot(100, 30, &mut app, false).unwrap();
    assert!(screen.contains("Working paper / sample"));
    assert!(screen.contains("One open question"));
    assert!(screen.contains("Which decisions are worth keeping close to the work?"));
    assert!(!screen.contains("more below"));
    assert!(screen.contains("simulated / unsaved"));
    assert!(screen.contains("Draft / unconfirmed"));
}

#[test]
fn other_notes_are_honestly_labeled_templates_not_the_authored_example() {
    let mut app = Shape::default();
    app.note = Note::new("I'd like a tiny garden planner.");
    app.send();
    finish(&mut app);
    let paper = app.paper();
    assert!(paper.contains("I'd like a tiny garden planner."));
    assert!(!paper.contains("reasons behind important decisions"));
    assert!(!paper.contains("which evidence informed it"));
    assert_eq!(paper.matches('?').count(), 1);
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("Working paper / template")
    );
}

#[test]
fn overflow_cues_track_top_middle_bottom_and_clear_when_content_fits() {
    let mut app = Shape::default();
    app.note = Note::new(&"A long unfinished thought.\n".repeat(45));
    app.send();
    finish(&mut app);
    let top = snapshot(100, 30, &mut app, false).unwrap();
    assert!(top.contains("more below / Tab to read"));
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::PageDown);
    let middle = snapshot(100, 30, &mut app, false).unwrap();
    assert!(middle.contains("more above / below / j/k scroll"));
    key(&mut app, KeyCode::End);
    let bottom = snapshot(100, 30, &mut app, false).unwrap();
    assert!(bottom.contains("more above / j/k scroll"));
    assert!(!bottom.contains("more above / below / j/k scroll"));
    assert!(bottom.contains("One open question"));
    key(&mut app, KeyCode::Home);
    assert!(
        snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("more below / j/k scroll")
    );
    // Replacing the long draft with the short sample removes stale overflow cues.
    app.note = Shape::default().note;
    app.send();
    finish(&mut app);
    assert!(
        !snapshot(100, 30, &mut app, false)
            .unwrap()
            .contains("more below")
    );
}

#[test]
fn card_overflow_and_unsent_changes_remain_visible_without_explanatory_chrome() {
    let mut app = Shape::default();
    let arrival = snapshot(100, 30, &mut app, false).unwrap();
    for removed in [
        "Only the card is editable",
        "Nothing is saved",
        "Example note / edit freely",
        "Sample writer / ready",
    ] {
        assert!(!arrival.contains(removed));
    }
    app.send();
    key(&mut app, KeyCode::Enter);
    app.paste(&"\nAnother thought.".repeat(15), Rect::new(0, 0, 100, 30));
    let bottom = snapshot(100, 30, &mut app, false).unwrap();
    assert!(bottom.contains("Unsent changes"));
    assert!(bottom.lines().nth(19).unwrap().contains("more above"));
    app.note.cursor = 0;
    let top = snapshot(100, 30, &mut app, false).unwrap();
    assert!(top.lines().nth(19).unwrap().contains("more below"));
    for (width, height) in [(80, 24), (100, 30), (180, 60)] {
        let screen = snapshot(width, height, &mut app, true).unwrap();
        assert!(screen.contains("simulated / unsaved"));
        assert!(screen.contains("F2 send"));
    }
}
