use anyhow::{anyhow, Result};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use tui::{
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{states, view_utils as vu, Boxable, CombatParticipant, Frame, State, StateBox};

#[derive(Clone)]
pub struct Normal {
    pub participants: Vec<CombatParticipant>,
    pub list_state: ListState,
}

impl Normal {
    fn increment_selection(mut self) -> Normal {
        if let Some(idx) = self.list_state.selected() {
            self.list_state
                .select(Some((idx + 1) % self.participants.len()));
            Self {
                list_state: self.list_state,
                ..self
            }
        } else {
            self
        }
    }

    fn decrement_selection(mut self) -> Normal {
        if let Some(idx) = self.list_state.selected() {
            self.list_state.select(Some(
                (idx + self.participants.len() - 1) % self.participants.len(),
            ));
            Self {
                list_state: self.list_state,
                ..self
            }
        } else {
            self
        }
    }

    fn change_selection(self) -> Result<StateBox> {
        let idx = self
            .list_state
            .selected()
            .ok_or(anyhow!("No State Selected"))?;
        let editor = self.participants[idx].clone();
        let participants = self
            .participants
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| if i != idx { Some(item) } else { None })
            .collect();

        Ok(states::Insert {
            participants,
            input_buffer: format!(
                "{}:{}{}",
                editor.name,
                editor.hp,
                if let Some(ini) = editor.initiative {
                    format!(":{}", ini)
                } else {
                    "".to_string()
                }
            ),
        }
        .boxed())
    }

    fn delete_selection(mut self) -> Result<Normal> {
        let idx = self
            .list_state
            .selected()
            .ok_or(anyhow!("No State Selected"))?;
        let participants: Vec<CombatParticipant> = self
            .participants
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| if i != idx { Some(item) } else { None })
            .collect();
        let new_index = if idx == participants.len() {
            idx - 1
        } else {
            idx
        };
        self.list_state.select(Some(new_index));
        Ok(Normal {
            participants,
            list_state: self.list_state,
        })
    }

    pub fn new(participants: Vec<CombatParticipant>) -> Self {
        assert!(participants.len() > 0);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Normal {
            participants,
            list_state,
        }
    }

    pub fn roll_initiatives(self) -> Normal {
        let participants = sort_with_ini(self.participants);
        Normal {
            participants,
            ..self
        }
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
        let Normal {
            mut participants,
            list_state,
        } = self;
        let current_selection = list_state.selected().unwrap();
        participants.swap(current_selection, swap_pos);

        // swap inis so inis and position match. Because we would need two mutable
        // references into the same vec this does not work without unsafe, and the
        // creation of two pointers, so i prefer this
        let tmp = participants[current_selection].initiative;
        participants[current_selection].initiative = participants[swap_pos].initiative;
        participants[swap_pos].initiative = tmp;

        Normal {
            participants,
            list_state,
        }
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
                KeyCode::Char('c') => self.change_selection(),
                KeyCode::Char('d') => Ok(self.delete_selection()?.boxed()),
                KeyCode::Char('r') => Ok(self.roll_initiatives().boxed()),
                KeyCode::Char('i') => Ok(states::Insert {
                    participants: self.participants,
                    input_buffer: "".to_string(),
                }
                .boxed()),
                KeyCode::Enter => {
                    let participants = sort_with_ini(self.participants);
                    Ok(states::Fighting::new(participants).boxed())
                }
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

        let list_lines: Vec<ListItem> = vu::participants_list_items(&self.participants);
        let list = List::new(list_lines)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(list, chunks[2], &mut self.list_state);
    }
}

fn sort_with_ini<T: IntoIterator<Item = CombatParticipant>>(
    participants: T,
) -> Vec<CombatParticipant> {
    let mut participants = participants
        .into_iter()
        .map(|p| {
            if p.initiative.is_none() {
                p.roll_initiative()
            } else {
                p
            }
        })
        .collect::<Vec<CombatParticipant>>();
    participants.sort_by(|a, b| b.initiative.unwrap().cmp(&a.initiative.unwrap()));
    participants
}
