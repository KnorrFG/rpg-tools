use anyhow::{bail, Context, Result};
use argh::FromArgs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use std::{fs, io, path::PathBuf};
use tui::{backend::CrosstermBackend, Terminal};
// use unicode_width::UnicodeWidthStr;

mod combat_state;
mod states;
mod utils;
mod view_utils;

use states::{Boxable, StateBox};

pub type Frame<'a> = tui::Frame<'a, Backend>;
pub type Backend = CrosstermBackend<io::Stdout>;

#[derive(FromArgs)]
/// Pass a List of files to prepopulate the fight
struct Cli {
    #[argh(positional)]
    /// files to load
    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    // setup terminal
    let args: Cli = argh::from_env();
    let init_state = get_initial_state(&args.files).context("get initial state")?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it

    let res = run_app(init_state, &mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn get_initial_state(files: &Vec<PathBuf>) -> Result<StateBox> {
    if files.len() == 0 {
        Ok(states::Insert::default().boxed())
    } else {
        let mut content = String::new();
        for file in files {
            let file_contents = fs::read_to_string(file)?;
            content.push_str(&file_contents);
        }
        let lines: Vec<&str> = content.lines().collect();
        let mut participants = Vec::with_capacity(lines.len());
        let mut initiatives = Vec::with_capacity(lines.len());
        for line in lines {
            let (ini, p) = utils::parse_participant_with_ini(line).context("parse with ini")?;
            participants.push(p);
            initiatives.push(ini);
        }
        Ok(states::Normal::new(
            combat_state::CombatState::from_participants(participants),
            initiatives,
        )?
        .boxed())
    }
}

fn run_app(mut current_state: StateBox, terminal: &mut Terminal<Backend>) -> Result<()> {
    terminal.draw(|f| current_state.render(f))?;
    loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
            }
        }
        current_state = current_state.process(ev)?;
        terminal.draw(|f| current_state.render(f))?;
    }
}
