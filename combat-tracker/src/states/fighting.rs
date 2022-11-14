use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use lazy_static::lazy_static;
use pad::PadStr;
use std::{cmp::min, collections::HashMap, iter::IntoIterator, rc::Rc};
use tui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState},
};

use crate::{
    combat_state::{CombatState, Participant},
    states::{self, Boxable, State, StateBox},
    utils, view_utils as vu, Frame,
};

lazy_static! {
    static ref KEY_INFOS: Vec<KeyInfo> = to_key_infos("qwaszxerdfcvtyghbnuijkm,opl;./");
}

#[derive(Clone)]
pub struct Fighting {
    combat_state: Rc<CombatState>,
    key_map: Rc<HashMap<char, CallbackBox>>,
    table_state: TableState,
    key_infos: Vec<KeyInfo>,
}

pub type CallbackBox = Box<dyn Fn() -> CombatState>;

#[derive(Clone)]
pub struct KeyInfo {
    pub decrement: char,
    pub increment: char,
}

impl Fighting {
    pub fn new(combat_state: CombatState) -> Fighting {
        let combat_state = Rc::new(combat_state);
        let key_infos: Vec<KeyInfo> = KEY_INFOS
            .iter()
            .cloned()
            .take(combat_state.participants.len())
            .collect();
        // generate a map with closures that provide an accordingly updated participant
        // vector. As all those closures must be sure the participant vector they use
        // exists as long as they exist, the vector must be in an Rc
        let key_map_iter = combat_state
            .participants
            .iter()
            .zip(key_infos.iter())
            .enumerate()
            .map(
                |(i, (_, keys)): (usize, (&Participant, &KeyInfo))| -> Vec<(char, CallbackBox)> {
                    vec![
                        (
                            keys.decrement,
                            Box::new({
                                let cs = combat_state.clone();
                                move || {
                                    (*cs).clone().update_participants(|ps| {
                                        utils::update_nth(ps, i, |p| {
                                            p.clone()
                                                .update_hp(|hp| if hp == 0 { 0 } else { hp - 1 })
                                        })
                                    })
                                }
                            }),
                        ),
                        (
                            keys.increment,
                            Box::new({
                                let cs = combat_state.clone();
                                move || {
                                    (*cs).clone().update_participants(|ps| {
                                        utils::update_nth(ps, i, |p| {
                                            p.clone().update_hp(|hp| hp + 1)
                                        })
                                    })
                                }
                            }),
                        ),
                    ]
                },
            )
            .flatten();
        Fighting {
            combat_state: combat_state.clone(),
            key_map: Rc::new(HashMap::from_iter(key_map_iter)),
            table_state: TableState::default(),
            key_infos,
        }
    }
}

fn to_key_infos(s: &str) -> Vec<KeyInfo> {
    s.chars()
        .collect::<Vec<char>>()
        .chunks(2)
        .map(|chunk| {
            assert!(chunk.len() == 2);
            KeyInfo {
                decrement: chunk[0],
                increment: chunk[1],
            }
        })
        .collect()
}

impl State for Fighting {
    fn process(self: Box<Fighting>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Esc => {
                    Ok(states::Normal::from_combat_state((*self.combat_state).clone())?.boxed())
                }
                KeyCode::Char(c) => {
                    if let Some(f) = self.key_map.get(&c) {
                        let new_cs = f();
                        Ok(Fighting::new(new_cs).boxed())
                    } else {
                        Ok(self)
                    }
                }
                _ => Ok(self),
            }
        } else {
            Ok(self)
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = vu::select_layout(f.size());
        let info_text = Span::from("Fight - Esc: To normal");
        f.render_widget(Paragraph::new(info_text), chunks[0]);

        let name_col_length = self
            .combat_state
            .participants
            .iter()
            .fold(0, |max, p| std::cmp::max(max, p.name.len()))
            + 1;

        let table_rows: Vec<Row> = self
            .combat_state
            .participants
            .iter()
            .zip(self.key_infos.iter())
            .map(|(p, key_info)| {
                Row::new(vec![
                    p.name
                        .pad_to_width_with_alignment(name_col_length, pad::Alignment::Right),
                    format!(
                        " <{}- HP: {} -{}> ",
                        key_info.decrement, p.hp, key_info.increment
                    ),
                ])
            })
            .collect();
        let constraints = [
            Constraint::Length(name_col_length as u16),
            Constraint::Length(15),
        ];
        let table = Table::new(table_rows)
            .block(Block::default().borders(Borders::ALL).title("Participants"))
            .widths(&constraints)
            // ...and they can be separated by a fixed spacing.
            .column_spacing(2)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>");
        f.render_stateful_widget(table, chunks[2], &mut self.table_state);
    }
}
