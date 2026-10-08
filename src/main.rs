use std::io::{self, IsTerminal};
use std::time::Instant;

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
};
use tinkery::{
    App, Palette,
    shaping::scratchpad::{self, Scratchpad},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut no_color = std::env::var_os("NO_COLOR").is_some();
    let mut snapshot = false;
    let mut workbench = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-color" => no_color = true,
            "--snapshot" => snapshot = true,
            "--workbench" => workbench = true,
            "--help" | "-h" => {
                println!(
                    "Tinkery / seed shaping prototype\n\nUsage: tinkery [--workbench] [--no-color] [--snapshot]\n\nDefault: full-width Pinstar sticky-note scratchpad with an on-demand paper inspector.\nNo connected agents, files, sessions, or approvals. All edits are unsaved.\n\nn / Ctrl-N adds a note. e / Enter edits. F2 shapes one selected note.\np toggles the paper inspector. Tab switches focus. ? helps.\nClick to select; double-click to edit. Drag notes or empty canvas.\nWheel zooms the canvas and scrolls the paper. Ctrl-F fits all; s resizes.\nDel deletes a note; Ctrl-Z / Ctrl-Y undo / redo.\nq quits outside editing; Ctrl-C quits anytime.\n\n--workbench  Open the earlier read-only demo\n--no-color   Use terminal colors (also respects NO_COLOR)\n--snapshot   Print a 100 x 30 view without terminal mode"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}; use --help").into()),
        }
    }
    if snapshot {
        println!(
            "{}",
            if workbench {
                tinkery::snapshot(100, 30, &mut App::default(), no_color)?
            } else {
                scratchpad::snapshot(100, 30, &mut Scratchpad::default(), no_color)?
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
        run_shape(&mut terminal, no_color)
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

fn run_shape(terminal: &mut ratatui::DefaultTerminal, no_color: bool) -> io::Result<()> {
    let _input_guard = InputGuard;
    execute!(io::stdout(), EnableBracketedPaste, EnableMouseCapture)?;
    let mut app = Scratchpad::default();
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
