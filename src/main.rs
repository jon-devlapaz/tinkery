//! The `tinkery` command: reads options, picks the real or practice host, and runs the terminal
//! loop (or prints one `--snapshot`). Everything the screens do lives in the library; this file
//! owns only the terminal: raw mode, events, the clipboard, and restoring it on the way out.

// Tests and fixtures read files they made; operator paths go through `regular_file`.
#![cfg_attr(test, allow(clippy::disallowed_methods))]

#[path = "main/input.rs"]
mod input;

use std::io::{self, IsTerminal};

use crossterm::{
    clipboard::CopyToClipboard,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyModifiers, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    style::Print,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use tinkery::{
    Palette, ThemeSwitch,
    launch::{self, Launch},
    menu::{self, Home},
    model_setup::Controls,
};

fn seed_read(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    use tinkery::seed_me::ledger;
    let refused = |reason: String| format!("Tinkery refused: {reason}");
    let value = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["show", directory] => {
            let shown = tinkery::shape::command::show(std::path::Path::new(directory))?;
            println!("{}", serde_json::to_string_pretty(&shown)?);
            return Ok(());
        }
        ["status", "--json", directory] => {
            ledger::status(std::path::Path::new(directory)).map_err(refused)?
        }
        ["checks", directory] => {
            let directory = std::path::Path::new(directory);
            ledger::load(directory).map_err(refused)?;
            ledger::read_checks(directory).map_err(refused)?
        }
        _ => return Err(launch::SEED_USAGE.into()),
    };
    print!("{}", value.pretty().map_err(refused)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match launch::parse(std::env::args().skip(1))? {
        launch::Args::Help => {
            println!("{}", launch::help());
            return Ok(());
        }
        launch::Args::SeedRead(args) => return seed_read(args),
        launch::Args::Run(options) => options,
    };
    let (launch, controls) = Launch::resolve(&options)?;
    let snapshot = options.snapshot;
    if options.seed.is_none()
        && !snapshot
        && (!io::stdin().is_terminal() || !io::stdout().is_terminal())
    {
        return Err("interactive mode needs a terminal; use --snapshot or --help".into());
    }
    let migration = launch.migrate(!snapshot)?;
    if let Some(seed) = &options.seed {
        return launch.seed_write(seed);
    }
    let launch = if snapshot {
        launch
    } else {
        launch.at_terminal(crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false))
    };
    let mut app = if options.home {
        let mut menu = Home::new(launch);
        if snapshot {
            menu.discover_for_snapshot(std::time::Duration::from_secs(5));
        }
        menu
    } else {
        Home::alone(launch)
    };
    if snapshot {
        println!("{}", screen_text(&mut app, &controls, options.no_color)?);
        return Ok(());
    }
    let mut terminal = ratatui::try_init()?;
    let result = run_shape(
        &mut terminal,
        Header {
            theme: ThemeSwitch::new(options.no_color),
            model: controls,
        },
        &mut app,
        options.full_redraw,
        migration,
    );
    ratatui::restore();
    if let Some(receipt) = result? {
        println!("{receipt}");
    }
    Ok(())
}

// The header's two switches the loop owns over every screen: light or dark, and the model.
struct Header {
    theme: ThemeSwitch,
    model: Controls,
}

// The screen, then the model label and panel over it; the loop and --snapshot draw the same.
fn draw(frame: &mut ratatui::Frame, app: &mut Home, controls: &Controls, palette: Palette) {
    menu::render(frame, app, palette);
    controls.render(frame, palette);
}

// The 100 x 30 view as text: no terminal mode, no model requests. Home lines are trimmed.
fn screen_text(
    app: &mut Home,
    controls: &Controls,
    no_color: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let (width, height) = (100, 30);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))?;
    terminal.draw(|frame| draw(frame, app, controls, Palette::new(no_color)))?;
    let trim = !app.is_alone();
    let buffer = terminal.backend().buffer();
    Ok((0..height)
        .map(|y| {
            let line: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
            if trim {
                line.trim_end().to_owned()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

struct InputGuard;

// The renderer may resize before a Resize event or input-size check observes it.
fn frame_painted(
    area: ratatui::layout::Rect,
    previous: &mut Option<ratatui::layout::Rect>,
    discard_mouse: &mut bool,
) {
    if previous.replace(area).is_some_and(|old| old != area) {
        *discard_mouse = true;
    }
}

// Keys and paste survive repaint; mouse coordinates from the old layout do not.
fn input_for_frame(
    input: Event,
    drawn: ratatui::layout::Rect,
    current: ratatui::layout::Rect,
    pending: &mut Option<Event>,
    discard_mouse: &mut bool,
) -> Option<Event> {
    if matches!(input, Event::Resize(..)) || current != drawn {
        *discard_mouse = true;
        if matches!(input, Event::Key(_) | Event::Paste(_)) {
            *pending = Some(input);
        }
        return None;
    }
    Some(input)
}

impl Drop for InputGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            EndSynchronizedUpdate,
            PopKeyboardEnhancementFlags,
            Print("\x1b[>0s"),
            DisableMouseCapture,
            DisableBracketedPaste
        );
    }
}

// F2 runs the named action, but hosts such as herdr or macOS may keep F2. Cmd+Enter (in
// terminals that report it) and Ctrl+Alt+S are the same key; every screen sees only F2.
fn send_alias(input: Event) -> Event {
    let Event::Key(key) = input else {
        return input;
    };
    let alias = match key.code {
        KeyCode::Enter => key.modifiers == KeyModifiers::SUPER,
        KeyCode::Char('s' | 'S') => key.modifiers == KeyModifiers::CONTROL | KeyModifiers::ALT,
        _ => false,
    };
    if !alias {
        return input;
    }
    Event::Key(KeyEvent {
        code: KeyCode::F(2),
        modifiers: KeyModifiers::NONE,
        ..key
    })
}

// A terminal receipt is truthful only when both frame drawing and ending synchronized
// output succeeded. Keep this single boundary shared by the real loop and failure tests.
fn finish_frame<T>(
    drawn: io::Result<T>,
    synchronized: io::Result<()>,
    acknowledge: impl FnOnce(),
) -> io::Result<T> {
    synchronized?;
    let drawn = drawn?;
    acknowledge();
    Ok(drawn)
}

fn run_shape(
    terminal: &mut ratatui::DefaultTerminal,
    header: Header,
    app: &mut Home,
    full_redraw: bool,
    mut migration: Option<String>,
) -> io::Result<Option<String>> {
    let _input_guard = InputGuard;
    execute!(
        io::stdout(),
        EnableBracketedPaste,
        EnableMouseCapture,
        Print("\x1b[>1s"),
        // Terminals that know this report Cmd+Enter; the rest ignore it.
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    let mut terminal_input = input::Input::new()?;
    let mut repaint = full_redraw;
    let mut pending_input = None;
    let mut previous_area = None;
    let Header {
        mut theme,
        model: mut controls,
    } = header;
    while !app.quit {
        app.tick();
        execute!(io::stdout(), BeginSynchronizedUpdate)?;
        let mut presentation = None;
        let drawn = (|| {
            let drawn = terminal.draw(|frame| {
                if let Some(notice) = &migration {
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!(
                            "{notice}\n\nPress any key to continue."
                        ))
                        .wrap(ratatui::widgets::Wrap { trim: false }),
                        frame.area(),
                    );
                } else {
                    draw(frame, app, &controls, theme.palette());
                    if !controls.is_open() {
                        presentation = app.presentation();
                    }
                }
                if repaint || full_redraw {
                    for cell in &mut frame.buffer_mut().content {
                        cell.set_diff_option(ratatui::buffer::CellDiffOption::AlwaysUpdate);
                    }
                }
            })?;
            io::Result::Ok(drawn.area)
        })();
        let drawn_area =
            finish_frame(drawn, execute!(io::stdout(), EndSynchronizedUpdate), || {
                if let Some(presentation) = presentation {
                    app.presented(presentation);
                }
            })?;
        let mut resized = false;
        frame_painted(drawn_area, &mut previous_area, &mut resized);
        if resized {
            terminal_input.invalidate_mouse();
        }
        repaint = false;
        let ready = pending_input.is_some() || terminal_input.poll();
        if !ready {
            std::thread::park_timeout(std::time::Duration::from_millis(100));
        }
        if ready || terminal_input.poll() {
            for _ in 0..64 {
                let input = match pending_input.take() {
                    Some(input) => input,
                    None => terminal_input.read()?,
                };
                let area = terminal.size()?.into();
                let Some(input) =
                    input_for_frame(input, drawn_area, area, &mut pending_input, &mut resized)
                else {
                    terminal_input.invalidate_mouse();
                    break;
                };
                match send_alias(input) {
                    // Only the source can acknowledge that old-layout input has drained.
                    Event::Mouse(_) if !terminal_input.mouse_ready() => {}
                    Event::Key(_) if migration.is_some() => {
                        migration = None;
                    }
                    Event::Paste(_) | Event::Mouse(_) if migration.is_some() => {}
                    Event::Key(key)
                        if key.code == KeyCode::Char('l')
                            && key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        repaint = true
                    }
                    Event::Key(key) if theme.key(key) => {}
                    Event::Mouse(event) if theme.click(event, area) => {}
                    Event::Key(key) if controls.key(key) => {}
                    Event::Paste(text) if controls.paste(&text) => {}
                    Event::Mouse(event) if controls.click(event, area) => {}
                    Event::Key(key) => app.handle_key(key),
                    Event::Paste(text) => app.paste(&text),
                    Event::Mouse(event) => app.handle_mouse(event, area),
                    _ => {}
                }
                if app.take_model_request() {
                    controls.open();
                }
                if app.quit || !terminal_input.poll() {
                    break;
                }
            }
            if let Some(markdown) = app.take_copy_request() {
                app.copy_result(
                    execute!(io::stdout(), CopyToClipboard::to_clipboard_from(markdown)).is_ok(),
                );
            }
        }
    }
    Ok(app.finish())
}

#[cfg(test)]
mod tests {
    #[test]
    fn presentation_receipt_requires_successful_draw_and_synchronized_output() {
        for (draw_ok, sync_ok) in [(true, true), (false, true), (true, false), (false, false)] {
            let mut acknowledged = false;
            let drawn = if draw_ok {
                Ok(())
            } else {
                Err(std::io::Error::other("TEST draw failed"))
            };
            let sync = if sync_ok {
                Ok(())
            } else {
                Err(std::io::Error::other("TEST synchronization failed"))
            };
            let result = super::finish_frame(drawn, sync, || acknowledged = true);
            assert_eq!(result.is_ok(), draw_ok && sync_ok);
            assert_eq!(acknowledged, draw_ok && sync_ok);
        }
    }

    use super::*;
    use crossterm::event::KeyEvent;
    use ratatui::layout::Rect;

    #[test]
    fn cmd_enter_and_ctrl_alt_s_are_f2_and_nothing_else_is() {
        let key = |code, modifiers| Event::Key(KeyEvent::new(code, modifiers));
        let f2 = key(KeyCode::F(2), KeyModifiers::NONE);
        let ctrl_alt = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert_eq!(send_alias(key(KeyCode::Enter, KeyModifiers::SUPER)), f2);
        assert_eq!(send_alias(key(KeyCode::Char('s'), ctrl_alt)), f2);
        for kept in [
            key(KeyCode::Enter, KeyModifiers::NONE),
            key(KeyCode::Enter, KeyModifiers::SHIFT),
            key(KeyCode::Enter, KeyModifiers::SUPER | KeyModifiers::SHIFT),
            key(KeyCode::Char('s'), KeyModifiers::NONE),
            key(KeyCode::Char('s'), KeyModifiers::CONTROL),
            key(KeyCode::Char('s'), KeyModifiers::ALT),
            Event::Paste("s".into()),
        ] {
            assert_eq!(send_alias(kept.clone()), kept);
        }
    }

    #[test]
    fn resized_input_waits_for_paint_and_is_delivered_once_unchanged() {
        let large = Rect::new(0, 0, 100, 30);
        let small = Rect::new(0, 0, 79, 24);
        for (drawn, current) in [(large, small), (small, large)] {
            for input in [
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                Event::Paste("confirm café 👩‍💻".into()),
            ] {
                let mut pending = None;
                let mut discard_mouse = false;
                // Size changed before any Resize event arrived.
                assert!(
                    input_for_frame(
                        input.clone(),
                        drawn,
                        current,
                        &mut pending,
                        &mut discard_mouse
                    )
                    .is_none()
                );
                let retained = pending.take().expect("input lost during resize");
                assert_eq!(
                    input_for_frame(retained, current, current, &mut pending, &mut discard_mouse),
                    Some(input)
                );
                assert!(pending.is_none());
            }
        }
    }

    #[test]
    fn resize_requests_a_paint_without_creating_input() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let mut pending = None;
        let mut discard_mouse = false;
        assert!(
            input_for_frame(
                Event::Resize(80, 24),
                area,
                area,
                &mut pending,
                &mut discard_mouse
            )
            .is_none()
        );
        assert!(pending.is_none());
    }

    #[test]
    fn resize_marks_queued_mouse_coordinates_stale() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let large = Rect::new(0, 0, 100, 30);
        let small = Rect::new(0, 0, 80, 24);
        // Outside Back in the old review, directly on Back in the new review.
        let click = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 40,
            row: 21,
            modifiers: KeyModifiers::NONE,
        });
        for resize_first in [false, true] {
            let mut pending = None;
            let mut discard_mouse = false;
            let painted = if resize_first {
                assert!(
                    input_for_frame(
                        Event::Resize(80, 24),
                        large,
                        small,
                        &mut pending,
                        &mut discard_mouse
                    )
                    .is_none()
                );
                small
            } else {
                large
            };
            let delivered = input_for_frame(
                click.clone(),
                painted,
                small,
                &mut pending,
                &mut discard_mouse,
            );
            assert!(discard_mouse);
            // The dispatch match ignores mouse while draining the old layout's queue.
            assert!(delivered.is_none() || matches!(delivered, Some(Event::Mouse(_))));
            assert!(pending.is_none(), "old-layout click was retained");
        }
    }

    #[test]
    fn automatic_draw_resize_marks_mouse_stale_without_a_resize_event() {
        let large = Rect::new(0, 0, 100, 30);
        let small = Rect::new(0, 0, 80, 24);
        let mut previous = None;
        let mut discard_mouse = false;
        frame_painted(large, &mut previous, &mut discard_mouse);
        assert!(!discard_mouse);
        frame_painted(small, &mut previous, &mut discard_mouse);
        // The next input-size check sees equal sizes, so paint must notice the change.
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let mut pending = None;
        assert_eq!(
            input_for_frame(
                enter.clone(),
                small,
                small,
                &mut pending,
                &mut discard_mouse
            ),
            Some(enter)
        );
        assert!(
            discard_mouse,
            "auto-resize admitted old-layout mouse coordinates"
        );
        frame_painted(small, &mut previous, &mut discard_mouse);
        assert!(
            discard_mouse,
            "a later paint cleared protection before input drained"
        );
    }
}
