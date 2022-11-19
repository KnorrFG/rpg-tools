use crate::combat_state::CombatState;
use crate::states::{self, Boxable, State, StateBox};
use anyhow::Result;
use derive_new::new;

#[derive(Clone, new)]
pub struct EditingModifiers {
    combat_state: CombatState,
    participant_pos: usize,
    input_buffer: String,
}

impl State for EditingModifiers {
    fn render(&mut self, f: &mut crate::Frame) {}

    fn process(self: Box<Self>, ev: crossterm::event::Event) -> StateBox {}
}
