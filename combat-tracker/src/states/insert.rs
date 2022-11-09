use std::iter;

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use tui::{
    text::Span,
    widgets::{Block, Borders, List, Paragraph},
};

use crate::{states, view_utils as vu, Boxable, CombatParticipant, Frame, State, StateBox};

#[derive(Clone)]
pub struct Insert {
    pub participants: Vec<CombatParticipant>,
    pub input_buffer: String,
}

impl Insert {
    pub fn with_char_push(&self, c: char) -> Box<Self> {
        let mut input_buffer = self.input_buffer.clone();
        input_buffer.push(c);
        Box::new(Insert {
            participants: self.participants.clone(),
            input_buffer,
        })
    }

    pub fn with_char_pop(&self) -> Box<Self> {
        let mut input_buffer = self.input_buffer.clone();
        input_buffer.pop();
        Box::new(Insert {
            participants: self.participants.clone(),
            input_buffer,
        })
    }
}

impl State for Insert {
    fn process(self: Box<Insert>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char(c) => Ok(self.with_char_push(c)),
                KeyCode::Backspace => Ok(self.with_char_pop()),
                KeyCode::Esc if self.participants.len() > 0 => {
                    Ok(states::Normal::new(self.participants).boxed())
                }
                KeyCode::Enter => match CombatParticipant::parse(&self.input_buffer) {
                    Ok(participant) => Ok(Insert {
                        participants: self
                            .participants
                            .into_iter()
                            .chain(iter::once(participant))
                            .collect(),
                        input_buffer: "".to_string(),
                    }
                    .boxed()),
                    Err(error) => Ok(states::Msg {
                        parent: self.clone().boxed(),
                        msg: format!("{:?}", error),
                    }
                    .boxed()),
                },
                _ => Ok(self),
            }
        } else {
            Ok(self)
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = vu::input_layout(f.size());
        let info_text =
            Span::from("Enter Participant syntax: \"Name: HP[: Inititive]\" (Esc: To Normal)");
        f.render_widget(Paragraph::new(info_text), chunks[0]);

        vu::render_input_block(f, "New Participant", &self.input_buffer, chunks[1]);

        let list_lines = vu::participants_list_items(&self.participants);

        let list =
            List::new(list_lines).block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(list, chunks[2]);
    }
}
