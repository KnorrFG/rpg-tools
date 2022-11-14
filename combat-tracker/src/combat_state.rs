use anyhow::{ensure, Context, Result};
use persistent_structs::PersistentStruct;
use std::fmt;

use crate::utils;

#[derive(PersistentStruct, Default, Clone)]
pub struct CombatState {
    pub now: TimeVec,
    pub participants: Vec<Participant>,
}

#[derive(PersistentStruct, Clone, Copy, Default)]
pub struct TimeVec {
    pub round: u8,
    pub in_round_pos: u8,
    pub round_len: usize,
}

#[derive(PersistentStruct, Clone)]
pub struct Participant {
    pub name: String,
    pub hp: u16,
    pub modifiers: Vec<Modifier>,
}

#[derive(PersistentStruct, Clone)]
pub struct Modifier {
    pub name: String,
    pub introduced_at: TimeVec,
    pub duration: TimeVec,
}

impl CombatState {
    pub fn from_participants(participants: Vec<Participant>) -> CombatState {
        CombatState {
            participants,
            now: TimeVec::default(),
        }
    }
    pub fn with_nth_participant_popped(self, n: usize) -> (Self, Participant) {
        let (res, participants) = utils::with_popped_n(self.participants, n);
        (
            CombatState {
                participants,
                now: self.now,
            },
            res,
        )
    }

    pub fn without_participant(self, n: usize) -> Self {
        self.update_participants(|mut ps| {
            ps.remove(n);
            ps
        })
    }
}

impl Participant {
    pub fn parse(s: &str) -> Result<Participant> {
        let splits: Vec<&str> = s.split(':').collect();
        Participant::parse_splits(splits)
    }

    pub fn parse_splits<'a>(s: impl IntoIterator<Item = &'a str>) -> Result<Participant> {
        let mut splits: Vec<&str> = s.into_iter().collect();

        ensure!(splits.len() > 1, "Didn't find a :");
        let hp_split = splits.pop().unwrap().trim();
        let hp = hp_split
            .parse()
            .context(format!("parsing {} as u8", hp_split))?;
        Ok(Participant {
            hp,
            name: splits.join(":"),
            modifiers: vec![],
        })
    }
}

impl fmt::Display for Participant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.hp)
    }
}
