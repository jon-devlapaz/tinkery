use std::io::{self, IsTerminal};
use std::{path::PathBuf, sync::Arc, time::Instant};

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
};
use tinkery::{
    App, Palette,
    shaping::{
        drafting::PiHost,
        scratchpad::{self, Scratchpad},
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut no_color = std::env::var_os("NO_COLOR").is_some();
    let mut snapshot = false;
    let mut workbench = false;
    let mut shape_pi = false;
    let mut model = None;
    let mut seed_me = None;
    let mut pi_command = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-color" => no_color = true,
            "--snapshot" => snapshot = true,
            "--workbench" => workbench = true,
            "--shape-pi" => shape_pi = true,
            "--model" => model = Some(args.next().ok_or("--model needs provider/model-id")?),
            "--seed-me" => {
                seed_me = Some(PathBuf::from(
                    args.next().ok_or("--seed-me needs a SKILL.md path")?,
                ))
            }
            "--pi-command" => {
                pi_command = Some(PathBuf::from(
                    args.next().ok_or("--pi-command needs an executable path")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "Tinkery / seed shaping prototype\n\nUsage: tinkery [--workbench] [--no-color] [--snapshot]\n       tinkery --shape-pi --model provider/model-id --seed-me PATH [--pi-command PATH]\n\nDefault: simulated, full-width sticky-note scratchpad. All edits are unsaved.\nReal drafting is opt-in: selected written notes and feedback go to the chosen model.\nOnly Seed Me's working-draft step runs. No seed confirmation or factory execution.\nPi runs without tools, extensions, project context, or saved sessions.\n\nn / Ctrl-N adds a note. e / Enter edits. F2 shapes selected written notes.\nBlank notes are skipped; an all-blank selection retains the paper.\nr opens model feedback: keep, cut, or reshape; F2 revises; Esc cancels.\nPgUp / PgDn reads the paper while you write feedback.\np toggles the paper inspector. Tab switches focus. ? helps.\nClick to select; double-click to edit. Drag notes or empty canvas.\nWheel zooms the canvas and scrolls the paper. Ctrl-F fits all; s resizes.\nDel deletes a note; Ctrl-Z / Ctrl-Y undo / redo.\nq quits outside editing; Ctrl-C quits anytime.\n\n--shape-pi   Enable real drafting through Pi; requests can incur provider charges\n--model      Explicit provider/model-id; no automatic model selection\n--seed-me    Path to the actual Seed Me SKILL.md\n--pi-command Pi executable (default: pi)\n--workbench  Open the earlier read-only demo\n--no-color   Use terminal colors (also respects NO_COLOR)\n--snapshot   Print a 100 x 30 view without terminal mode or model requests"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}; use --help").into()),
        }
    }
    if shape_pi && workbench {
        return Err("--shape-pi is for the scratchpad, not --workbench".into());
    }
    if !shape_pi && (model.is_some() || seed_me.is_some() || pi_command.is_some()) {
        return Err("--model, --seed-me and --pi-command require --shape-pi".into());
    }
    let mut app = if shape_pi {
        Scratchpad::with_host(Arc::new(PiHost::new(
            pi_command.unwrap_or_else(|| "pi".into()),
            model.ok_or("--shape-pi requires an explicit --model provider/model-id")?,
            seed_me.ok_or("--shape-pi requires --seed-me PATH to actual Seed Me instructions")?,
        )?))
    } else {
        Scratchpad::default()
    };
    if snapshot {
        println!(
            "{}",
            if workbench {
                tinkery::snapshot(100, 30, &mut App::default(), no_color)?
            } else {
                scratchpad::snapshot(100, 30, &mut app, no_color)?
            }
        );
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("interactive mode needs a terminal; use --snapshot or --help".into());
    }
    let mut terminal = ratatui::try_init()?;
    let result = if workbench {
        run_workbench(&mut terminal, no_color)
    } else {
        run_shape(&mut terminal, no_color, app)
    };
    ratatui::restore();
    result?;
    Ok(())
}

fn run_workbench(terminal: &mut ratatui::DefaultTerminal, no_color: bool) -> io::Result<()> {
    let mut app = App::default();
    while !app.quit {
        terminal.draw(|frame| tinkery::render(frame, &mut app, Palette::new(no_color)))?;
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key, terminal.size()?.into());
        }
    }
    Ok(())
}

struct InputGuard;

impl Drop for InputGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    }
}

fn run_shape(
    terminal: &mut ratatui::DefaultTerminal,
    no_color: bool,
    mut app: Scratchpad,
) -> io::Result<()> {
    let _input_guard = InputGuard;
    execute!(io::stdout(), EnableBracketedPaste, EnableMouseCapture)?;
    let mut last = Instant::now();
    while !app.quit {
        let now = Instant::now();
        app.tick(now.duration_since(last), terminal.size()?.into());
        last = now;
        terminal.draw(|frame| scratchpad::render(frame, &mut app, Palette::new(no_color)))?;
        if event::poll(std::time::Duration::from_millis(100))? {
            let area = terminal.size()?.into();
            match event::read()? {
                Event::Key(key) => app.handle_key(key, area),
                Event::Paste(text) => app.paste(&text, area),
                Event::Mouse(event) => app.handle_mouse(event, area),
                _ => {}
            }
        }
    }
    Ok(())
}
