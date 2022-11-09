use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use lazy_static::lazy_static;
use pad::PadStr;
use std::{collections::HashMap, iter::IntoIterator, rc::Rc};
use tui::{
    layout::{Alignment, Constraint},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState},
};

use crate::{states, view_utils as vu, Boxable, CombatParticipant, Frame, State, StateBox};

lazy_static! {
    static ref KEY_INFOS: Vec<KeyInfo> = to_key_infos("qwaszxerdfcvtyghbnuijkm,opl;./");
}

#[derive(Clone)]
pub struct Fighting {
    participants: Rc<Vec<(CombatParticipant, KeyInfo)>>,
    key_map: Rc<HashMap<char, CallbackBox>>,
    table_state: TableState,
}

pub type CallbackBox = Box<dyn Fn() -> Vec<CombatParticipant>>;

#[derive(Clone)]
pub struct KeyInfo {
    pub decrement: char,
    pub increment: char,
}

impl Fighting {
    pub fn new(participants: impl IntoIterator<Item = CombatParticipant>) -> Fighting {
        let participants = Rc::new(Vec::<(CombatParticipant, KeyInfo)>::from_iter(
            participants.into_iter().zip(KEY_INFOS.clone()),
        ));
        // generate a map with closures that provide an accordingly updated participant
        // vector. As all those closures must be sure the participant vector they use
        // exists as long as they exist, the vector must be in an Rc
        let key_map_iter = participants
            .iter()
            .enumerate()
            .map(|(i, (_, keys))| -> Vec<(char, CallbackBox)> {
                vec![
                    (
                        keys.decrement,
                        Box::new({
                            let participants = participants.clone();
                            move || update_vec_with(&participants, i, |p| p.clone().decrement_hp())
                        }),
                    ),
                    (
                        keys.increment,
                        Box::new({
                            let participants = participants.clone();
                            move || update_vec_with(&participants, i, |p| p.clone().increment_hp())
                        }),
                    ),
                ]
            })
            .flatten();
        Fighting {
            participants: participants.clone(),
            key_map: Rc::new(HashMap::from_iter(key_map_iter)),
            table_state: TableState::default(),
        }
    }
}

fn update_vec_with<F>(
    v: &Vec<(CombatParticipant, KeyInfo)>,
    idx: usize,
    f: F,
) -> Vec<CombatParticipant>
where
    F: FnOnce(&CombatParticipant) -> CombatParticipant,
{
    // this is safe because its only used in an iteration
    let target = v.get(idx).unwrap();
    let updated_target = f(&target.0);
    let mut participants: Vec<CombatParticipant> = v.iter().map(|x| x.0.clone()).collect();
    participants[idx] = updated_target;
    participants
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
                KeyCode::Esc => Ok(states::Normal::new(
                    self.participants.iter().cloned().map(|p| p.0).collect(),
                )
                .boxed()),
                KeyCode::Char(c) => {
                    if let Some(f) = self.key_map.get(&c) {
                        let new_vec = f();
                        Ok(Fighting::new(new_vec).boxed())
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
            .participants
            .iter()
            .fold(0, |max, (p, _)| std::cmp::max(max, p.name.len()))
            + 1;

        let table_rows: Vec<Row> = self
            .participants
            .iter()
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
