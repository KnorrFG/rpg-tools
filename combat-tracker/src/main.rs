use anyhow::{bail, Result};
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

mod view_utils;

mod states;

pub trait Boxable {
    fn boxed(self) -> StateBox;
    fn box_clone(&self) -> StateBox;
}

impl<T> Boxable for T
where
    T: 'static + State + Clone,
{
    fn boxed(self) -> StateBox {
        Box::new(self)
    }

    fn box_clone(&self) -> StateBox {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn State> {
    fn clone(&self) -> Box<dyn State> {
        self.box_clone()
    }
}

pub trait State: Boxable {
    fn process(self: Box<Self>, ev: Event) -> Result<StateBox>;
    fn render(&mut self, f: &mut Frame);
}

type StateBox = Box<dyn State>;
type Frame<'a> = tui::Frame<'a, Backend>;
type Backend = CrosstermBackend<io::Stdout>;

#[derive(Clone)]
pub struct CombatParticipant {
    pub name: String,
    pub hp: u16,
    pub initiative: Option<u8>,
    pub infos: Vec<String>,
}

impl CombatParticipant {
    fn parse(s: &str) -> Result<CombatParticipant> {
        let elems: Vec<&str> = s.rsplit(":").collect();
        match elems.len() {
            (3..) => {
                let (hp, ini, name) = match (
                    elems[0].trim().parse::<u16>(),
                    elems[1].trim().parse::<u16>(),
                ) {
                    (Ok(ini), Ok(hp)) => (hp, Some(ini as u8), elems[2..].join(":")),
                    (Ok(hp), Err(_)) => (hp, None, elems[1..].join(":")),
                    (Err(_), _) => bail!("Can't parse this"),
                };
                Ok(CombatParticipant {
                    name,
                    hp,
                    initiative: ini,
                    infos: vec![],
                })
            }
            2 => {
                let hp: u16 = elems[0].trim().parse()?;
                let name = elems[1].to_string();
                Ok(CombatParticipant {
                    name,
                    hp,
                    initiative: None,
                    infos: vec![],
                })
            }
            _ => bail!("No Colon found"),
        }
    }

    pub fn roll_initiative(self) -> Self {
        CombatParticipant {
            initiative: Some(roll(2, 6)),
            ..self
        }
    }

    pub fn increment_hp(self) -> Self {
        if self.hp < u16::MAX {
            CombatParticipant {
                hp: self.hp + 1,
                ..self
            }
        } else {
            self
        }
    }

    pub fn decrement_hp(self) -> Self {
        if self.hp > 0 {
            CombatParticipant {
                hp: self.hp - 1,
                ..self
            }
        } else {
            self
        }
    }
}

fn roll(n: u8, dice: u8) -> u8 {
    let mut rng = rand::thread_rng();
    let dist = rand::distributions::Uniform::new_inclusive(1, dice);
    (0..n).map(|_| rng.sample(dist) as u8).fold(0, |a, b| a + b)
}

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
    let initial_participants = load_initial_participants(&args.files)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it

    let init_state = if initial_participants.len() == 0 {
        states::Insert {
            participants: initial_participants,
            input_buffer: "".to_string(),
        }
        .boxed()
    } else {
        states::Normal::new(initial_participants).boxed()
    };
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

fn load_initial_participants(files: &[PathBuf]) -> Result<Vec<CombatParticipant>> {
    let mut res = vec![];
    for file in files {
        let content = fs::read_to_string(file)?;
        for line in content.lines() {
            res.push(CombatParticipant::parse(&line)?);
        }
    }
    Ok(res)
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
