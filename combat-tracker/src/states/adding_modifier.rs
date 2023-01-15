use crate::{
    combat_state::{CombatState, Modifier, ModifierFac},
    states, utils as ut, view_utils as vu,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use derive_new::new;
use persistent_structs::PersistentStruct;
use tui::text::Span;
use tui::widgets::Paragraph;

use super::{Boxable, Fighting, State, StateBox};

#[derive(Clone, new, PersistentStruct)]
pub struct AddingModifiers {
    parent_state: Box<Fighting>,
    target_participant: usize,
    input_buffer: String,
}

impl State for AddingModifiers {
    fn render(&mut self, f: &mut crate::Frame) {
        let chunks = vu::input_layout(f.size());
        let info_text = Span::from(format!(
            "Fight - Esc: To normal; Current Round: {}",
            self.parent_state.combat_state.current_round
        ));
        f.render_widget(Paragraph::new(info_text), chunks[0]);
        vu::render_input_block(f, "New Modifier", &self.input_buffer, chunks[1]);
        vu::render_fighting_mode_table(
            f,
            &self.parent_state.combat_state,
            &self.parent_state.key_infos,
            chunks[2],
        );
    }

    fn process(self: Box<Self>, ev: crossterm::event::Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Esc => Ok(self.parent_state),
                KeyCode::Enter => Ok(match Modifier::parse_factory(&self.input_buffer) {
                    Ok(mod_fac) => self.parent_with_modifier(mod_fac),
                    Err(e) => states::Msg::new(self, ut::err_to_string(&e)).boxed(),
                }),
                code => Ok(self
                    .update_input_buffer(|b| ut::update_buffer(b, code))
                    .boxed()),
            }
        } else {
            Ok(self)
        }
    }
}

impl AddingModifiers {
    pub fn parent_with_modifier(self, fac: ModifierFac) -> StateBox {
        let mut parent = self.parent_state;
        let new_mod = fac(parent.combat_state.now());
        parent.combat_state.participants[self.target_participant]
            .modifiers
            .push(new_mod);
        parent
    }
}
