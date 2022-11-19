use anyhow::{ensure, Result};
use crossterm::event::{Event, KeyCode};
use persistent_structs::PersistentStruct;
use tui::{
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    combat_state::CombatState,
    states::{self, Boxable, State, StateBox},
    utils, view_utils as vu, Frame,
};

#[derive(Clone, PersistentStruct)]
pub struct Normal {
    pub combat_state: CombatState,
    pub initiatives: Vec<Option<u8>>,
    pub current_selection: usize,
}

impl Normal {
    pub fn new(combat_state: CombatState, initiatives: Vec<Option<u8>>) -> Result<Normal> {
        ensure!(
            combat_state.participants.len() > 0,
            "Normal mode can only be used with at least one participant"
        );
        Ok(Normal {
            combat_state,
            initiatives,
            current_selection: 0,
        })
    }

    fn increment_selection(self) -> Normal {
        let len = self.combat_state.participants.len() - 1;
        self.update_current_selection(|s| if s == len { 0 } else { s + 1 })
    }

    fn decrement_selection(self) -> Normal {
        let len = self.combat_state.participants.len() - 1;
        self.update_current_selection(|s| if s == 0 { len - 1 } else { s - 1 })
    }

    fn change_selection(self) -> StateBox {
        let idx = self.current_selection;
        let (combat_state, editee) = self.combat_state.with_nth_participant_popped(idx);
        let (editee_ini, initiatives) = utils::with_popped_n(self.initiatives, idx);

        states::Insert::new(
            combat_state,
            format!(
                "{}{}",
                editee,
                if let Some(ini) = editee_ini {
                    format!(":{}", ini)
                } else {
                    "".to_string()
                }
            ),
            initiatives,
        )
        .boxed()
    }

    fn delete_selection(self) -> Normal {
        let idx = self.current_selection;
        let res = self
            .update_combat_state(|s| s.without_participant(idx))
            .update_initiatives(|mut is| {
                is.remove(idx);
                is
            });
        let new_index = if idx == res.combat_state.participants.len() {
            idx - 1
        } else {
            idx
        };
        res.with_current_selection(new_index)
    }

    pub fn from_combat_state(cs: CombatState) -> Result<Self> {
        ensure!(
            cs.participants.len() > 0,
            "Normal mode must always have at least one entry"
        );
        let initiatives = vec![None; cs.participants.len()];
        Normal::new(cs, initiatives)
    }

    pub fn roll_initiatives(self) -> Normal {
        let mut res = self.update_initiatives(|inis| {
            inis.into_iter()
                .map(|ini| match ini {
                    None => Some(utils::roll(2, 6)),
                    ini => ini,
                })
                .collect()
        });
        // even though it is mut, it will not be mutated, according to the docs
        let mut sorter = permutation::sort_by(&res.initiatives, |a, b| b.unwrap().cmp(&a.unwrap()));
        sorter.apply_slice_in_place(&mut res.initiatives);
        sorter.apply_slice_in_place(&mut res.combat_state.participants);
        res
    }

    pub fn move_selected_down(self) -> Normal {
        let a = self.current_selection;
        self.increment_selection().swap_current_selection_with(a)
    }

    pub fn move_selected_up(self) -> Normal {
        let a = self.current_selection;
        self.decrement_selection().swap_current_selection_with(a)
    }

    fn swap_current_selection_with(self, swap_pos: usize) -> Normal {
        let sel = self.current_selection;
        self.update_combat_state(|cs| {
            cs.update_participants(|mut ps| {
                ps.swap(sel, swap_pos);
                ps
            })
        })
    }
}

impl State for Normal {
    fn process(self: Box<Normal>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char('j') => Ok(self.increment_selection().boxed()),
                KeyCode::Char('k') => Ok(self.decrement_selection().boxed()),
                KeyCode::Char('J') => Ok(self.move_selected_down().boxed()),
                KeyCode::Char('K') => Ok(self.move_selected_up().boxed()),
                KeyCode::Char('c') => Ok(self.change_selection()),
                KeyCode::Char('d') => Ok(self.delete_selection().boxed()),
                KeyCode::Char('r') => Ok(self.roll_initiatives().boxed()),
                KeyCode::Char('i') => {
                    Ok(
                        states::Insert::new(self.combat_state, "".to_string(), self.initiatives)
                            .boxed(),
                    )
                }
                KeyCode::Enter => Ok(states::Fighting::new(self.combat_state).boxed()),
                _ => Ok(self),
            }
        } else {
            Ok(self)
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = vu::select_layout(f.size());
        let info_text = Span::from(
            "Normal - c: change; d: delete; j & k: navigate; r: roll ini; enter: start fight",
        );
        f.render_widget(Paragraph::new(info_text), chunks[0]);

        let list_lines: Vec<ListItem> =
            vu::participants_list_items(&self.combat_state.participants, &self.initiatives);
        let list = List::new(list_lines)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut list_state = ListState::default();
        list_state.select(Some(self.current_selection));
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }
}
