use anyhow::Result;
use crossterm::event::Event;
use tui::style::Style;
use tui::text::Span;
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::{State, StateBox};
use crate::{combat_state::CombatState, view_utils as vu};

#[derive(Clone)]
pub struct EditingModifiers {
    combat_state: CombatState,
    participant_idx: usize,
    modifier_idx: usize,
    buffer: String,
}

impl State for EditingModifiers {
    fn process(self: Box<Self>, ev: Event) -> Result<StateBox> {
        Ok(self)
    }

    fn render(&mut self, f: &mut crate::Frame) {
        let participant = self.combat_state.participants[self.participant_idx];

        let chunks = vu::select_layout(f.size());
        let info_text = Span::from(
            "Editing Modifiers - enter: update, ctrl+j/k: move, ctrl+d: delete, esc: normal",
        );
        f.render_widget(Paragraph::new(info_text), chunks[0]);

        let list_lines: Vec<ListItem> =
            vu::participants_list_items(&self.combat_state.participants, &self.initiatives);
        let list = List::new(list_lines)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .highlight_style(Style::default().add_modifier(tui::style::Modifier::REVERSED));

        let mut list_state = ListState::default();
        list_state.select(Some(self.modifier_idx));
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }
}
