use anyhow::{anyhow, ensure, Result};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use persistent_structs::PersistentStruct;
use tui::{
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    combat_state::{CombatState, Participant},
    states::{self, Boxable, State, StateBox},
    utils, view_utils as vu, Frame,
};

#[derive(Clone, PersistentStruct)]
pub struct Normal {
    pub combat_state: CombatState,
    pub initiatives: Vec<Option<u8>>,
    pub list_state: ListState,
}

impl Normal {
    pub fn new(combat_state: CombatState, initiatives: Vec<Option<u8>>) -> Result<Normal> {
        ensure!(
            combat_state.participants.len() > 0,
            "Normal mode can only be used with at least one participant"
        );
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Ok(Normal {
            combat_state,
            initiatives,
            list_state,
        })
    }

    fn increment_selection(self) -> Normal {
        let len = self.combat_state.participants.len();
        self.update_list_state(|mut ls| {
            let idx = ls.selected().expect("List has no selection");
            ls.select(Some((idx + 1) % len));
            ls
        })
    }

    fn decrement_selection(self) -> Normal {
        let len = self.combat_state.participants.len();
        self.update_list_state(|mut ls| {
            let idx = ls.selected().expect("List has no selection");
            ls.select(Some((idx + len - 1) % len));
            ls
        })
    }

    fn change_selection(self) -> StateBox {
        let idx = self.list_state.selected().expect("No List selection");
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
        let idx = self.list_state.selected().expect("No State Selected");
        let mut res = self
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
        res.list_state.select(Some(new_index));
        res
    }

    pub fn from_combat_state(cs: CombatState) -> Result<Self> {
        ensure!(
            cs.participants.len() > 0,
            "Normal mode must always have at least one entry"
        );
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let initiatives = vec![None; cs.participants.len()];
        Ok(Normal {
            combat_state: cs,
            initiatives,
            list_state,
        })
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
        let a = self.list_state.selected().unwrap();
        self.increment_selection().swap_current_selection_with(a)
    }

    pub fn move_selected_up(self) -> Normal {
        let a = self.list_state.selected().unwrap();
        self.decrement_selection().swap_current_selection_with(a)
    }

    fn swap_current_selection_with(self, swap_pos: usize) -> Normal {
        let current_selection = self.list_state.selected().unwrap();
        self.update_combat_state(|cs| {
            cs.update_participants(|mut ps| {
                ps.swap(current_selection, swap_pos);
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

        f.render_stateful_widget(list, chunks[2], &mut self.list_state);
    }
}
