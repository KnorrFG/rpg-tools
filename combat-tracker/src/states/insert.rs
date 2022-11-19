use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use persistent_structs::PersistentStruct;
use tui::{
    text::Span,
    widgets::{Block, Borders, List, Paragraph},
};

use crate::{
    combat_state::{CombatState, Participant},
    states::{self, Boxable, State, StateBox},
    utils, view_utils as vu, Frame,
};

#[derive(Clone, Default, PersistentStruct)]
pub struct Insert {
    pub combat_state: CombatState,
    pub input_buffer: String,
    pub initiatives: Vec<Option<u8>>,
}

impl Insert {
    pub fn new(
        combat_state: CombatState,
        input_buffer: String,
        initiatives: impl IntoIterator<Item = Option<u8>>,
    ) -> Insert {
        Insert {
            combat_state,
            input_buffer,
            initiatives: Vec::from_iter(initiatives),
        }
    }
    pub fn with_char_push(self, c: char) -> StateBox {
        self.update_input_buffer(|mut b| {
            b.push(c);
            b
        })
        .boxed()
    }

    pub fn with_char_pop(self) -> StateBox {
        self.update_input_buffer(|mut b| {
            b.pop();
            b
        })
        .boxed()
    }

    pub fn with_new_participant(self, p: Participant, ini: Option<u8>) -> Self {
        self.update_combat_state(|cs| {
            cs.update_participants(|mut ps| {
                ps.push(p);
                ps
            })
        })
        .update_initiatives(|mut is| {
            is.push(ini);
            is
        })
    }
}

impl State for Insert {
    fn process(self: Box<Insert>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char(c) => Ok(self.with_char_push(c)),
                KeyCode::Backspace => Ok(self.with_char_pop()),
                KeyCode::Esc if self.combat_state.participants.len() > 0 => {
                    Ok(states::Normal::new(self.combat_state, self.initiatives)?.boxed())
                }
                KeyCode::Enter => {
                    let (ini, p) = utils::parse_participant_with_ini(&self.input_buffer)?;
                    Ok(self
                        .with_new_participant(p, ini)
                        .with_input_buffer("".into())
                        .boxed())
                }
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

        let list_lines =
            vu::participants_list_items(&self.combat_state.participants, &self.initiatives);

        let list =
            List::new(list_lines).block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(list, chunks[2]);
    }
}
