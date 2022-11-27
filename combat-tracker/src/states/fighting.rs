use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use lazy_static::lazy_static;
use pad::PadStr;
use persistent_structs::PersistentStruct;
use std::{collections::HashMap, rc::Rc};
use tui::{
    layout::Constraint,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::{
    combat_state::{CombatState, Participant, SubRoundTime, TimeVec},
    states::{self, Boxable, State, StateBox},
    utils, view_utils as vu, Frame,
};

use super::AddingModifiers;

lazy_static! {
    static ref KEY_INFOS: Vec<KeyInfo> =
        to_key_infos("qweasdzxcrtyfghvbnuiojklm,.;p/QWEASDZXCRTYFGHVBNUIOJKLM<>P:?");
}

#[derive(Clone, PersistentStruct)]
pub struct Fighting {
    pub combat_state: CombatState,
    pub hp_mod_map: Rc<HashMap<char, HpCallbackBox>>,
    pub tag_add_map: Rc<HashMap<char, TagCallbackBox>>,
    pub key_infos: Vec<KeyInfo>,
}

pub type HpCallbackBox = Box<dyn Fn(CombatState) -> CombatState>;
pub type TagCallbackBox = Box<dyn Fn(Box<Fighting>) -> StateBox>;

#[derive(Clone)]
pub struct KeyInfo {
    pub decrement: char,
    pub increment: char,
    pub edit_modifiers: char,
}

impl Fighting {
    pub fn new(combat_state: CombatState) -> Fighting {
        let key_infos: Vec<KeyInfo> = KEY_INFOS
            .iter()
            .cloned()
            .take(combat_state.participants.len())
            .collect();
        // generate a map with closures that provide an accordingly updated participant
        // vector. As all those closures must be sure the participant vector they use
        // exists as long as they exist, the vector must be in an Rc
        let key_map_iter = key_infos
            .iter()
            .enumerate()
            .map(
                |(i, keys): (usize, &KeyInfo)| -> Vec<(char, HpCallbackBox)> {
                    vec![
                        (
                            keys.decrement,
                            Box::new(move |cs| {
                                cs.update_participants(|ps| {
                                    utils::update_nth(ps, i, |p| {
                                        p.clone().update_hp(|hp| if hp == 0 { 0 } else { hp - 1 })
                                    })
                                })
                            }),
                        ),
                        (
                            keys.increment,
                            Box::new(move |cs| {
                                cs.update_participants(|ps| {
                                    utils::update_nth(ps, i, |p| p.clone().update_hp(|hp| hp + 1))
                                })
                            }),
                        ),
                    ]
                },
            )
            .flatten();

        let tag_callback_map_iter =
            key_infos
                .iter()
                .enumerate()
                .map(|(i, key_infos)| -> (char, TagCallbackBox) {
                    (
                        key_infos.edit_modifiers,
                        Box::new(move |fighting| {
                            AddingModifiers::new(fighting, i, "".into()).boxed()
                        }),
                    )
                });
        Fighting {
            combat_state,
            hp_mod_map: Rc::new(HashMap::from_iter(key_map_iter)),
            tag_add_map: Rc::new(HashMap::from_iter(tag_callback_map_iter)),
            key_infos,
        }
    }
}

fn to_key_infos(s: &str) -> Vec<KeyInfo> {
    s.chars()
        .collect::<Vec<char>>()
        .chunks(3)
        .map(|chunk| {
            assert!(chunk.len() == 3);
            KeyInfo {
                decrement: chunk[0],
                increment: chunk[1],
                edit_modifiers: chunk[2],
            }
        })
        .collect()
}

impl State for Fighting {
    fn process(self: Box<Fighting>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Esc => Ok(states::Normal::from_combat_state(self.combat_state)?.boxed()),
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Ok(self
                    .update_combat_state(CombatState::with_next_turn)
                    .boxed()),
                KeyCode::Char(c) => {
                    if let Some(f) = self.hp_mod_map.clone().get(&c) {
                        Ok(self.update_combat_state(f).boxed())
                    } else if let Some(f) = self.tag_add_map.clone().get(&c) {
                        Ok(f(self))
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
        let info_text = Span::from(format!(
            "Fight - Esc: To normal; Current Round: {}",
            self.combat_state.current_round
        ));
        f.render_widget(Paragraph::new(info_text), chunks[0]);

        vu::render_fighting_mode_table(f, &self.combat_state, &self.key_infos, chunks[2]);
    }
}
