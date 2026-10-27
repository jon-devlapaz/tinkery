use super::*;
use crate::pi::{CallBudget, Request};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn setup(model: &str, thinking: &str, calls: usize) -> Setup {
    Setup::new(model.into(), thinking.into(), calls).unwrap()
}

fn press(controls: &mut Controls, code: KeyCode) {
    assert!(controls.key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn typed(controls: &mut Controls, text: &str) {
    for c in text.chars() {
        press(controls, KeyCode::Char(c));
    }
}

fn screen(controls: &Controls) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| controls.render(frame, crate::Palette::new(true)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..30)
        .map(|y| {
            (0..100)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn one_parser_refuses_what_neither_flags_nor_file_may_hold() {
    assert!(Setup::new("anthropic/model".into(), "low".into(), 10).is_ok());
    assert!(Setup::new(String::new(), "low".into(), 0).is_ok());
    for (model, thinking, calls, why) in [
        ("model", "low", 5, "provider/model-id"),
        ("a/b c", "low", 5, "provider/model-id"),
        ("a/b", "automatic", 5, "Use thinking off|minimal|low"),
        ("", "low", 5, "Choose a model"),
    ] {
        let refused = Setup::new(model.into(), thinking.into(), calls).unwrap_err();
        assert!(refused.contains(why), "{refused}");
    }
}

#[test]
fn launch_drafts_only_with_saved_calls_or_shape_pi_and_flags_win_for_one_launch() {
    let saved = setup("p/saved", "high", 10);
    let practice_saved = setup("p/saved", "high", 0);
    assert_eq!(Setup::launch(false, None, None, None, None), Ok(None));
    assert_eq!(
        Setup::launch(false, None, None, None, Some(&practice_saved)),
        Ok(None)
    );
    assert_eq!(
        Setup::launch(false, None, None, None, Some(&saved)),
        Ok(Some(saved.clone()))
    );
    assert_eq!(
        Setup::launch(
            true,
            Some("p/flag".into()),
            Some("max".into()),
            Some(2),
            Some(&saved)
        ),
        Ok(Some(setup("p/flag", "max", 2)))
    );
    assert_eq!(
        Setup::launch(true, None, None, None, Some(&practice_saved)),
        Ok(Some(practice_saved.clone())),
        "--shape-pi drafts with the saved model even with zero calls"
    );
    assert_eq!(
        Setup::launch(true, Some("p/flag".into()), None, None, None),
        Ok(Some(setup("p/flag", "low", 0))),
        "without a saved file, flags keep today's defaults"
    );
    assert!(
        Setup::launch(true, None, None, None, None)
            .unwrap_err()
            .contains("requires an explicit --model")
    );
    assert!(Setup::launch(true, Some("p/m".into()), Some("lots".into()), None, None).is_err());
}

#[test]
fn a_missing_file_is_practice_and_a_bad_one_is_refused_with_its_path() {
    let home = tempfile::tempdir().unwrap();
    let path = path(Some(home.path().as_os_str())).unwrap();
    assert_eq!(path, home.path().join(".config/tinkery/model.json"));
    assert_eq!(load(&path), Ok(None));
    assert_eq!(super::path(Some("relative".as_ref())), None);
    assert_eq!(super::path(None), None);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    for bad in [
        "not json",
        r#"{"model":"p/m","thinking":"low"}"#,
        r#"{"model":"p/m","thinking":"low","calls":5,"key":"x"}"#,
        r#"{"model":"p/m","thinking":"lots","calls":5}"#,
        r#"{"model":"","thinking":"low","calls":5}"#,
        r#"{"model":"p/m","thinking":"low","calls":-1}"#,
    ] {
        std::fs::write(&path, bad).unwrap();
        let refused = load(&path).unwrap_err();
        assert!(
            refused.contains(&path.display().to_string()) && refused.contains("using practice"),
            "{bad}: {refused}"
        );
    }
}

#[test]
fn only_s_writes_the_file_and_esc_leaves_it_alone() {
    let home = tempfile::tempdir().unwrap();
    let file = path(Some(home.path().as_os_str())).unwrap();
    let mut controls = Controls::new(Some(file.clone()), Ok(None), None);
    assert_eq!(controls.label(), "practice");
    assert!(!controls.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)));
    controls.open();
    assert!(screen(&controls).contains("Model     none"));
    press(&mut controls, KeyCode::Enter);
    typed(&mut controls, "anthropic/claude-test");
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Enter);
    let shown = screen(&controls);
    assert!(shown.contains("anthropic/claude-test"), "{shown}");
    assert!(
        shown.contains("medium") && shown.contains("10 each launch"),
        "{shown}"
    );
    press(&mut controls, KeyCode::Esc);
    assert!(!controls.is_open());
    assert!(!file.exists(), "esc wrote the file");

    controls.open();
    assert!(
        screen(&controls).contains("Model     none"),
        "esc kept the draft"
    );
    press(&mut controls, KeyCode::Enter);
    typed(&mut controls, "anthropic/claude-test");
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Char('s'));
    assert_eq!(
        load(&file),
        Ok(Some(setup("anthropic/claude-test", "low", 5)))
    );
    let shown = screen(&controls);
    assert!(
        shown.contains("Saved. A new launch drafts with it."),
        "{shown}"
    );
    assert_eq!(
        controls.label(),
        "practice",
        "saving changes later launches only"
    );
}

#[test]
fn saving_refuses_calls_without_a_model_and_typing_refuses_a_loose_model() {
    let home = tempfile::tempdir().unwrap();
    let file = path(Some(home.path().as_os_str())).unwrap();
    let mut controls = Controls::new(Some(file.clone()), Ok(None), None);
    controls.open();
    press(&mut controls, KeyCode::Enter);
    typed(&mut controls, "no-provider s");
    press(&mut controls, KeyCode::Enter);
    assert!(screen(&controls).contains("Use provider/model-id"));
    press(&mut controls, KeyCode::Esc);
    assert!(
        controls.is_open(),
        "esc while typing only undoes the typing"
    );
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Down);
    press(&mut controls, KeyCode::Enter);
    press(&mut controls, KeyCode::Char('s'));
    assert!(screen(&controls).contains("Choose a model before allowing calls."));
    assert!(!file.exists());
}

#[test]
fn an_invalid_file_is_explained_in_the_panel_and_saving_replaces_it() {
    let refused = "/TEST/model.json is not a model setup: bad. Save again".to_owned();
    let mut controls = Controls::new(None, Err(refused.clone()), None);
    assert_eq!(controls.label(), "practice");
    controls.open();
    assert!(screen(&controls).contains("/TEST/model.json is not a model setup"));
    press(&mut controls, KeyCode::Char('s'));
    assert!(screen(&controls).contains("Saving needs an absolute HOME"));
}

#[test]
fn adding_calls_raises_this_launch_only_and_a_launch_never_exceeds_its_cap() {
    let budget = CallBudget::new(1);
    assert!(budget.reserve(Request::Shaping).is_ok());
    let refused = budget.reserve(Request::Shaping).unwrap_err();
    assert!(refused.contains("Add calls to keep going."), "{refused}");
    let home = tempfile::tempdir().unwrap();
    let file = path(Some(home.path().as_os_str())).unwrap();
    let mut controls = Controls::new(Some(file.clone()), Ok(None), Some(budget.clone()));
    assert_eq!(controls.label(), "0 calls left");
    controls.open();
    for _ in 0..3 {
        press(&mut controls, KeyCode::Down);
    }
    press(&mut controls, KeyCode::Enter);
    assert_eq!(budget.remaining(), 5);
    assert_eq!(controls.label(), "5 calls left");
    assert!(screen(&controls).contains("5 left · enter adds 5"));
    press(&mut controls, KeyCode::Esc);
    assert!(!file.exists(), "adding calls saved nothing");
    for _ in 0..5 {
        assert!(budget.reserve(Request::SeedStep).is_ok());
    }
    assert!(budget.reserve(Request::SeedStep).is_err(), "cap exceeded");
    assert_eq!(budget.used(), 6);
}

#[test]
fn an_agent_sees_the_setup_but_cannot_save_it_or_add_calls() {
    let home = tempfile::tempdir().unwrap();
    let file = path(Some(home.path().as_os_str())).unwrap();
    let budget = CallBudget::new(0);
    let mut controls =
        Controls::new(Some(file.clone()), Ok(None), Some(budget.clone())).for_agent();
    controls.open();
    press(&mut controls, KeyCode::Char('s'));
    assert!(screen(&controls).contains("An agent can't save a setup or add calls"));
    for _ in 0..3 {
        press(&mut controls, KeyCode::Down);
    }
    press(&mut controls, KeyCode::Enter);
    assert_eq!(budget.limit(), 0);
    assert!(!file.exists());
}

#[test]
fn practice_has_no_now_row_and_clicking_the_label_opens_the_panel() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mut controls = Controls::new(None, Ok(None), None);
    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    let at = |column| MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    let line = screen(&controls).lines().nth(1).unwrap().to_owned();
    let start = line.find("practice").unwrap() as u16;
    assert!(!controls.click(at(start - 1), area));
    assert!(controls.click(at(start), area));
    assert!(controls.is_open());
    assert!(!screen(&controls).contains("Now "));
    for _ in 0..3 {
        press(&mut controls, KeyCode::Down);
    }
    assert!(screen(&controls).contains("› Model"), "three rows wrap");
}
