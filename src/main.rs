//! The `tinkery` command: reads options, picks the real or practice host, and runs the terminal
//! loop (or prints one `--snapshot`). Everything the screens do lives in the library; this file
//! owns only the terminal: raw mode, events, the clipboard, and restoring it on the way out.

use std::io::{self, IsTerminal};
use std::{path::PathBuf, sync::Arc};

use crossterm::{
    clipboard::CopyToClipboard,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyModifiers,
    },
    execute,
    style::Print,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use tinkery::{
    Palette,
    menu::{self, Home},
    think::{
        brain_dump::{self, BrainDump, Exit},
        drafting::PiHost,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut no_color = std::env::var_os("NO_COLOR").is_some();
    let mut snapshot = false;
    let mut full_redraw = false;
    let mut shape_pi = false;
    let mut model = None;
    let mut thinking = None;
    let mut seed_me = None;
    let mut seed_session_root = None;
    let mut record_root = None;
    let mut pi_command = None;
    let mut home_menu = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-color" => no_color = true,
            "--home" => home_menu = true,
            "--snapshot" => snapshot = true,
            "--full-redraw" => full_redraw = true,
            "--shape-pi" => shape_pi = true,
            "--model" => model = Some(args.next().ok_or("--model needs provider/model-id")?),
            "--thinking" => thinking = Some(args.next().ok_or("--thinking needs a level")?),
            "--seed-session-root" => {
                seed_session_root = Some(PathBuf::from(
                    args.next()
                        .ok_or("--seed-session-root needs a location outside a repository")?,
                ));
            }
            "--record-root" => {
                record_root = Some(PathBuf::from(
                    args.next().ok_or("--record-root needs a folder")?,
                ));
            }
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
                    "Tinkery: a calm, keyboard-first terminal front door for agentic work\n\nUsage: tinkery [--home] [--no-color] [--snapshot]\n       tinkery --shape-pi --model provider/model-id --seed-me PATH [--pi-command PATH]\n\n--home opens the home menu (think, shape, approve, watch, taste, measure); the `tinkery` launcher passes it.\nDefault: one blank brain-dump box; no guess until F2 submit. Simulated unless explicitly enabled. Edits remain unsaved until explicit goal affirmation.\nReal mode sends submitted dumps and answers to the chosen model.\nBorrowed Seed Me working-draft guidance. Explicit goal affirmation creates an active Seed Me session; no confirmed seed or factory execution.\nPi runs without tools, extensions, project context, or saved sessions.\n\nBrain dump: type directly; Enter adds a line. F1 help (twice for every key), F2 send, F3 review goal, F4 originals, F5 why, F6 local voice (Ctrl+Alt+Z), F8 skip (Ctrl-Z undoes), F9 copy, F10 back to the menu (guarded). Ctrl-C copies a selection or cancels sending; it never leaves. Shift-F2 / Ctrl-N adds a separate dump; its words remain unresolved until scope is answered. Aliases: Ctrl-O, Ctrl-D, Ctrl-G. Mac keyboards may need fn. The muted key bar shows only what's possible now; on by default, b in help toggles it for the session. A shaping call and a decision review precede display; nothing retries automatically. Review and typed confirm are separate; no seed or implementation approval. No browser is opened.\n\n--shape-pi   Enable real drafting through Pi; requests can incur provider charges\n--model      Explicit provider/model-id; no automatic model selection\n--thinking   Pi reasoning level (default: low); off|minimal|low|medium|high|xhigh|max\n--seed-me    Path to the actual Seed Me SKILL.md and its official helpers\n--seed-session-root Optional session location outside a repository (e.g. isolated tests)\n--record-root Folder for think records written on confirm (default: ~/.local/share/tinkery/think)\n--pi-command Pi executable (default: pi)\n--full-redraw Repaint every intake frame (host rendering workaround; more output)\n--no-color   Use terminal colors (also respects NO_COLOR)\n--snapshot   Print a 100 x 30 view without terminal mode or model requests\nTINKERY_NODE Optional Node executable for local voice; otherwise PATH/login-shell Node. Voice owns its local engine and settings; installation and model downloads require confirmation."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}; use --help").into()),
        }
    }
    if !shape_pi
        && (model.is_some()
            || thinking.is_some()
            || seed_me.is_some()
            || seed_session_root.is_some()
            || record_root.is_some()
            || pi_command.is_some())
    {
        return Err(
            "--model, --thinking, --seed-me, --record-root and --pi-command require --shape-pi"
                .into(),
        );
    }
    let record_root = match (record_root, shape_pi) {
        (Some(root), _) => root,
        (None, true) => tinkery::think::record::default_root(std::env::var_os("HOME").as_deref())?,
        // Only --shape-pi writes records.
        (None, false) => PathBuf::new(),
    };
    let goal_skill = seed_me.clone();
    if home_menu {
        let (default_root, substrate_dir) = Home::default_paths();
        let sessions_root = seed_session_root.clone().unwrap_or(default_root);
        let make_dump: Box<dyn Fn() -> BrainDump> = if shape_pi {
            let host = Arc::new(
                PiHost::new(
                    pi_command.clone().unwrap_or_else(|| "pi".into()),
                    model
                        .clone()
                        .ok_or("--shape-pi requires an explicit --model provider/model-id")?,
                    seed_me.clone().ok_or(
                        "--shape-pi requires --seed-me PATH to actual Seed Me instructions",
                    )?,
                )?
                .with_thinking(thinking.as_deref().unwrap_or("low"))?,
            );
            let skill = goal_skill.clone().unwrap();
            let root = seed_session_root.clone();
            let records = record_root.clone();
            Box::new(move || {
                BrainDump::with_host(host.clone())
                    .with_seed_me(skill.clone(), root.clone())
                    .with_record_root(records.clone())
            })
        } else {
            Box::new(BrainDump::default)
        };
        let make_dump: Box<dyn Fn() -> BrainDump> = if snapshot {
            make_dump
        } else {
            Box::new(move || make_dump().with_voice(Box::new(tinkery::voice::ProcessHost::new())))
        };
        let mut menu = Home::new(make_dump, sessions_root, substrate_dir);
        if let Some(skill) = goal_skill {
            menu = menu.with_seed_me(skill.canonicalize()?);
        }
        if snapshot {
            menu.discover_for_snapshot(std::time::Duration::from_secs(5));
            println!("{}", menu::snapshot(100, 30, &mut menu, no_color)?);
            return Ok(());
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err("interactive mode needs a terminal; use --snapshot or --help".into());
        }
        let mut terminal = ratatui::try_init()?;
        let result = run_shape(
            &mut terminal,
            no_color,
            Intake::Home(Box::new(menu)),
            full_redraw,
        );
        ratatui::restore();
        if let Some(receipt) = result? {
            println!("{receipt}");
        }
        return Ok(());
    }
    let mut app = if shape_pi {
        let host = Arc::new(
            PiHost::new(
                pi_command.unwrap_or_else(|| "pi".into()),
                model.ok_or("--shape-pi requires an explicit --model provider/model-id")?,
                seed_me
                    .ok_or("--shape-pi requires --seed-me PATH to actual Seed Me instructions")?,
            )?
            .with_thinking(thinking.as_deref().unwrap_or("low"))?,
        );
        Intake::Brain(Box::new(
            BrainDump::with_host(host)
                .with_seed_me(goal_skill.unwrap(), seed_session_root)
                .with_record_root(record_root),
        ))
    } else {
        Intake::Brain(Box::default())
    };
    if snapshot {
        println!(
            "{}",
            match &mut app {
                Intake::Home(app) => menu::snapshot(100, 30, app, no_color)?,
                Intake::Brain(app) => brain_dump::snapshot(100, 30, app, no_color)?,
            }
        );
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("interactive mode needs a terminal; use --snapshot or --help".into());
    }
    app = match app {
        Intake::Brain(dump) => Intake::Brain(Box::new(
            (*dump).with_voice(Box::new(tinkery::voice::ProcessHost::new())),
        )),
        other => other,
    };
    let mut terminal = ratatui::try_init()?;
    let result = run_shape(&mut terminal, no_color, app, full_redraw);
    ratatui::restore();
    if let Some(receipt) = result? {
        println!("{receipt}");
    }
    Ok(())
}

enum Intake {
    Home(Box<Home>),
    Brain(Box<BrainDump>),
}
impl Intake {
    fn quit(&self) -> bool {
        match self {
            Self::Home(a) => a.quit,
            Self::Brain(a) => *a.exit() == Exit::Menu && a.saved().is_none(),
        }
    }
    fn tick(&mut self) {
        match self {
            Self::Home(a) => a.tick(),
            Self::Brain(a) => a.tick(),
        }
        if let Self::Brain(dump) = self {
            let session = match dump.exit() {
                Exit::Stay => None,
                Exit::Menu => dump.saved().map(|r| (r.session.clone(), false)),
                Exit::Shape(path) => Some((path.clone(), true)),
            };
            if let Some((session, open_shape)) = session {
                let Self::Brain(dump) = std::mem::replace(self, Self::Brain(Box::default())) else {
                    unreachable!()
                };
                *self = Self::Home(Box::new(Home::from_receipt(*dump, &session, open_shape)));
            }
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame, p: Palette) {
        match self {
            Self::Home(a) => menu::render(f, a, p),
            Self::Brain(a) => brain_dump::render(f, a, p),
        }
    }
    fn handle_key(&mut self, k: crossterm::event::KeyEvent) {
        match self {
            Self::Home(a) => a.handle_key(k),
            Self::Brain(a) => a.handle_key(k),
        }
    }
    fn paste(&mut self, text: &str) {
        match self {
            Self::Home(a) => a.paste(text),
            Self::Brain(a) => a.paste(text),
        }
    }
    fn handle_mouse(&mut self, e: crossterm::event::MouseEvent, area: ratatui::layout::Rect) {
        match self {
            Self::Home(a) => a.handle_mouse(e, area),
            Self::Brain(a) => a.handle_mouse(e, area),
        }
    }
    fn take_copy_request(&mut self) -> Option<String> {
        match self {
            Self::Home(a) => a.take_copy_request(),
            Self::Brain(a) => a.take_copy_request(),
        }
    }
    fn copy_result(&mut self, sent: bool) {
        match self {
            Self::Home(a) => a.copy_result(sent),
            Self::Brain(a) => a.copy_result(sent),
        }
    }
}

struct InputGuard;

impl Drop for InputGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            EndSynchronizedUpdate,
            Print("\x1b[>0s"),
            DisableMouseCapture,
            DisableBracketedPaste
        );
    }
}

fn run_shape(
    terminal: &mut ratatui::DefaultTerminal,
    no_color: bool,
    mut app: Intake,
    full_redraw: bool,
) -> io::Result<Option<String>> {
    let _input_guard = InputGuard;
    execute!(
        io::stdout(),
        EnableBracketedPaste,
        EnableMouseCapture,
        Print("\x1b[>1s")
    )?;
    let mut repaint = full_redraw;
    while !app.quit() {
        app.tick();
        execute!(io::stdout(), BeginSynchronizedUpdate)?;
        let drawn = (|| {
            terminal.draw(|frame| {
                app.render(frame, Palette::new(no_color));
                if repaint || full_redraw {
                    for cell in &mut frame.buffer_mut().content {
                        cell.set_diff_option(ratatui::buffer::CellDiffOption::AlwaysUpdate);
                    }
                }
            })?;
            io::Result::Ok(())
        })();
        execute!(io::stdout(), EndSynchronizedUpdate)?;
        drawn?;
        repaint = false;
        if event::poll(std::time::Duration::from_millis(100))? {
            let area = terminal.size()?.into();
            for _ in 0..64 {
                match event::read()? {
                    Event::Key(key)
                        if key.code == KeyCode::Char('l')
                            && key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        repaint = true
                    }
                    Event::Key(key) => app.handle_key(key),
                    Event::Paste(text) => app.paste(&text),
                    Event::Mouse(event) => app.handle_mouse(event, area),
                    _ => {}
                }
                if app.quit() || !event::poll(std::time::Duration::ZERO)? {
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
    Ok(match &mut app {
        Intake::Home(a) => a.finish(),
        Intake::Brain(a) => a.finish_handoff(),
    })
}
