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

fn seed_read(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    use tinkery::seed_me::ledger;
    let refused = |reason: String| format!("Tinkery refused: {reason}");
    let value = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["status", "--json", directory] => {
            ledger::status(std::path::Path::new(directory)).map_err(refused)?
        }
        ["checks", directory] => {
            let directory = std::path::Path::new(directory);
            ledger::load(directory).map_err(refused)?;
            ledger::read_checks(directory).map_err(refused)?
        }
        _ => return Err("usage: tinkery seed status --json DIR | tinkery seed checks DIR".into()),
    };
    print!("{}", value.pretty().map_err(refused)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut no_color = std::env::var_os("NO_COLOR").is_some();
    let mut snapshot = false;
    let mut full_redraw = false;
    let mut shape_pi = false;
    let mut model = None;
    let mut thinking = None;
    let mut seed_session_root = None;
    let mut record_root = None;
    let mut link_root = None;
    let mut pi_command = None;
    let mut shape_budget = 0;
    let mut home_menu = false;
    let mut operator = None;
    let mut confirmation_file = None;
    let mut sdlc_root = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "seed" => return seed_read(args.collect()),
            "--no-color" => no_color = true,
            "--home" => home_menu = true,
            "--operator" => {
                if operator.is_some() {
                    return Err("--operator may be declared only once".into());
                }
                let value = args.next().ok_or("--operator needs human or agent")?;
                tinkery::think_record::GoalConfirmation::terminal(Some(&value))?;
                operator = Some(value);
            }
            "--agent-confirmation" => {
                if confirmation_file.is_some() {
                    return Err("--agent-confirmation may be supplied only once".into());
                }
                confirmation_file = Some(PathBuf::from(
                    args.next()
                        .ok_or("--agent-confirmation needs a JSON file")?,
                ));
            }
            "--snapshot" => snapshot = true,
            "--sdlc-root" => {
                sdlc_root = Some(PathBuf::from(
                    args.next()
                        .ok_or("--sdlc-root needs a tink-sdlc checkout path")?,
                ))
            }
            "--full-redraw" => full_redraw = true,
            "--shape-pi" => shape_pi = true,
            "--model" => model = Some(args.next().ok_or("--model needs provider/model-id")?),
            "--thinking" => thinking = Some(args.next().ok_or("--thinking needs a level")?),
            "--seed-session-root" => {
                seed_session_root = Some(PathBuf::from(
                    args.next()
                        .filter(|root| !root.is_empty())
                        .ok_or("--seed-session-root needs a location outside a repository")?,
                ));
            }
            "--link-root" => {
                link_root = Some(PathBuf::from(
                    args.next()
                        .filter(|root| !root.is_empty())
                        .ok_or("--link-root needs a folder")?,
                ));
            }
            "--record-root" => {
                record_root = Some(PathBuf::from(
                    args.next()
                        .filter(|root| !root.is_empty())
                        .ok_or("--record-root needs a folder")?,
                ));
            }
            "--shape-budget" => {
                shape_budget = args
                    .next()
                    .and_then(|calls| calls.parse::<usize>().ok())
                    .ok_or("--shape-budget needs a number of model calls")?;
            }
            "--pi-command" => {
                pi_command = Some(PathBuf::from(
                    args.next().ok_or("--pi-command needs an executable path")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "Tinkery: a calm terminal front door for agentic work.\n\nUsage: tinkery [--home] [--no-color] [--snapshot]\n       tinkery --shape-pi --model provider/model-id\n       tinkery seed status --json DIR\n       tinkery seed checks DIR\n\nWithout --shape-pi, think is practice: replies are simulated and nothing leaves your machine.\nWith --shape-pi, your words go to the model you chose and may cost money.\nNothing runs on its own. Only typing confirm saves a goal, and saving starts no work.\nInside, F1 shows what you can do now; F1 twice shows every key.\n\n--home       Open the home menu (the tinkery launcher passes it)\n--operator human|agent  Frozen launch declaration, not verified identity (default: not declared)\n--agent-confirmation FILE  Named agent and authorizing operator context (JSON)\n--shape-pi   Draft with a real model through Pi\n--model      provider/model-id to use (required with --shape-pi)\n--thinking   Reasoning level: off|minimal|low|medium|high|xhigh|max (default: low)\n--seed-session-root  Saved goals to list/save (default needs absolute HOME: ~/.local/share/tinkery/sessions)\n--record-root  Where Think history is saved/read (default: ~/.local/share/tinkery/think)\n--link-root PATH  Where tend's links go (default: ~/.local/share/tinkery/tend)\n--pi-command Pi executable (default: pi)\n--shape-budget CALLS  Model calls Shape may make this run (default: 0, none)\n--sdlc-root PATH  The tink-sdlc checkout watch reads with its own status --json (read-only; nothing is discovered)\n--full-redraw  Repaint every frame, for terminals that draw badly\n--no-color   Turn off colors (also respects NO_COLOR)\n--snapshot   Print a 100 x 30 view and exit; no terminal mode, no model requests\nTINKERY_NODE Node for local voice (otherwise found on PATH). Voice stays on your machine; installing it asks first."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}; use --help").into()),
        }
    }
    if !shape_pi
        && (model.is_some() || thinking.is_some() || pi_command.is_some() || shape_budget > 0)
    {
        return Err(
            "--model, --thinking, --pi-command and --shape-budget require --shape-pi".into(),
        );
    }
    let confirmation = match confirmation_file {
        Some(path) => {
            if operator.as_deref() != Some("agent") {
                return Err("--agent-confirmation requires --operator agent".into());
            }
            tinkery::think_record::GoalConfirmation::read_agent(&path)?
        }
        None => tinkery::think_record::GoalConfirmation::terminal(operator.as_deref())?,
    };
    let agent_confirmation = matches!(
        confirmation,
        tinkery::think_record::GoalConfirmation::Agent { .. }
    )
    .then(|| confirmation.clone());
    let record_root = match record_root {
        Some(root) if root.is_absolute() => Ok(root),
        Some(root) => Ok(std::env::current_dir()?.join(root)),
        None => tinkery::think_record::default_root(std::env::var_os("HOME").as_deref()),
    };
    if shape_pi {
        record_root.as_ref().map_err(Clone::clone)?;
    }
    let host = if shape_pi {
        Some(Arc::new(
            PiHost::new(
                pi_command.clone().unwrap_or_else(|| "pi".into()),
                model
                    .clone()
                    .ok_or("--shape-pi requires an explicit --model provider/model-id")?,
            )?
            .with_thinking(thinking.as_deref().unwrap_or("low"))?,
        ))
    } else {
        None
    };
    let default_root_requested = seed_session_root.is_none();
    let link_root = tinkery::tend::needs::resolve_root(link_root);
    let sessions_root = match seed_session_root {
        Some(root) => Ok(tinkery::seed_me::resolve_root(Some(root))?),
        None => tinkery::seed_me::default_root(std::env::var_os("HOME").as_deref()),
    };
    if shape_pi && !snapshot {
        sessions_root.as_ref().map_err(Clone::clone)?;
    }
    if !snapshot && (!io::stdin().is_terminal() || !io::stdout().is_terminal()) {
        return Err("interactive mode needs a terminal; use --snapshot or --help".into());
    }
    let migration = if default_root_requested {
        tinkery::seed_me::migration::migrate(std::env::var_os("HOME").as_deref())?
    } else {
        None
    };
    if snapshot && let Some(notice) = &migration {
        eprintln!("{notice}");
    }
    // Where Tinkery was started; found by looking for `.git`, never by running git.
    let repo = std::env::current_dir()
        .ok()
        .and_then(|dir| tinkery::repo::Repo::enclosing(&dir));
    // Shape's agent step reads files where the goal was made, else where Tinkery started.
    let shape_agent = match (shape_pi, &model) {
        (true, Some(model)) => Some(Arc::new(
            tinkery::shape::Agent::new(
                pi_command.clone().unwrap_or_else(|| "pi".into()),
                model.clone(),
                thinking.clone().unwrap_or_else(|| "low".into()),
                repo.as_ref()
                    .map(|r| r.root().to_path_buf())
                    .or_else(|| std::env::current_dir().ok()),
                tinkery::pi::CallBudget::new(shape_budget),
                record_root.clone().ok(),
            )?
            .with_confirmation(Some(confirmation.clone())),
        )),
        _ => None,
    };
    if home_menu {
        let make_dump: Box<dyn Fn() -> BrainDump> = if let Some(host) = host.clone() {
            let root = sessions_root.clone().ok();
            let records = record_root.clone()?;
            let repo = repo.clone();
            let confirmation = confirmation.clone();
            Box::new(move || {
                let mut dump = BrainDump::with_host(host.clone())
                    .with_record_root(records.clone())
                    .with_repo(repo.clone());
                dump.set_goal_confirmation(confirmation.clone())
                    .expect("validated launch declaration on fresh Think");
                match &root {
                    Some(root) => dump.with_sessions(Some(root.clone())),
                    None => dump,
                }
            })
        } else {
            let confirmation = confirmation.clone();
            Box::new(move || {
                let mut dump = BrainDump::default();
                dump.set_goal_confirmation(confirmation.clone())
                    .expect("validated launch declaration on fresh Think");
                dump
            })
        };
        let make_dump: Box<dyn Fn() -> BrainDump> = if snapshot {
            make_dump
        } else {
            Box::new(move || make_dump().with_voice(Box::new(tinkery::voice::ProcessHost::new())))
        };
        let mut menu = Home::new(make_dump, sessions_root, link_root.clone())
            .with_repo(repo)
            .with_shape_agent(shape_agent)
            .with_shape_held(
                operator.as_deref() == Some("agent") && (agent_confirmation.is_none() || !shape_pi),
            )
            .with_sdlc_root(match sdlc_root {
                Some(root) if root.is_relative() => Some(std::env::current_dir()?.join(root)),
                root => root,
            });
        if let Ok(root) = &record_root {
            menu = menu.with_record_root(root.clone());
        }
        if snapshot {
            menu.discover_for_snapshot(std::time::Duration::from_secs(5));
            println!("{}", menu::snapshot(100, 30, &mut menu, no_color)?);
            return Ok(());
        }
        let mut terminal = ratatui::try_init()?;
        let mut app = Intake::Home(Box::new(menu));
        let result = run_shape(
            &mut terminal,
            no_color,
            &mut app,
            full_redraw,
            link_root,
            None,
            migration,
        );
        ratatui::restore();
        if let Some(receipt) = result? {
            println!("{receipt}");
        }
        return Ok(());
    }
    let mut app = if let Some(host) = host {
        let dump = BrainDump::with_host(host)
            .with_record_root(record_root?)
            .with_repo(repo);
        Intake::Brain(Box::new(match sessions_root {
            Ok(root) => dump.with_sessions(Some(root)),
            Err(_) => dump,
        }))
    } else {
        Intake::Brain(Box::default())
    };
    if let Intake::Brain(dump) = &mut app {
        dump.set_goal_confirmation(confirmation)?;
    }
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
    app = match app {
        Intake::Brain(dump) => Intake::Brain(Box::new(
            (*dump).with_voice(Box::new(tinkery::voice::ProcessHost::new())),
        )),
        other => other,
    };
    let mut terminal = ratatui::try_init()?;
    let result = run_shape(
        &mut terminal,
        no_color,
        &mut app,
        full_redraw,
        link_root,
        shape_agent,
        migration,
    );
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
    fn tick(
        &mut self,
        link_root: &Result<PathBuf, String>,
        shape_agent: &Option<Arc<tinkery::shape::Agent>>,
    ) {
        match self {
            Self::Home(a) => a.tick(),
            Self::Brain(a) => a.tick(),
        }
        if let Self::Brain(dump) = self {
            let session = match dump.exit() {
                Exit::Stay => None,
                Exit::Menu => dump.saved().map(|r| (r.session.clone(), false)),
                Exit::Tend(path) => Some((path.clone(), true)),
            };
            if let Some((session, open_tend)) = session {
                let Self::Brain(dump) = std::mem::replace(self, Self::Brain(Box::default())) else {
                    unreachable!()
                };
                *self = Self::Home(Box::new(Home::from_receipt(
                    *dump,
                    &session,
                    open_tend,
                    link_root.clone(),
                    shape_agent.clone(),
                )));
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
    app: &mut Intake,
    full_redraw: bool,
    link_root: Result<PathBuf, String>,
    shape_agent: Option<Arc<tinkery::shape::Agent>>,
    mut migration: Option<String>,
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
        app.tick(&link_root, &shape_agent);
        execute!(io::stdout(), BeginSynchronizedUpdate)?;
        let drawn = (|| {
            terminal.draw(|frame| {
                if let Some(notice) = &migration {
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!(
                            "{notice}\n\nPress any key to continue."
                        ))
                        .wrap(ratatui::widgets::Wrap { trim: false }),
                        frame.area(),
                    );
                } else {
                    app.render(frame, Palette::new(no_color));
                }
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
    Ok(match app {
        Intake::Home(a) => a.finish(),
        Intake::Brain(a) => a.finish_handoff().map(|receipt| receipt.text()),
    })
}
