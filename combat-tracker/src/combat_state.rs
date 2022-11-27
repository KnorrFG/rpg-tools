use anyhow::{anyhow, ensure, Context, Result};
use derive_new::new;
use persistent_structs::PersistentStruct;
use std::fmt;

use crate::utils;

#[derive(PersistentStruct, Default, Clone, new)]
pub struct CombatState {
    pub current_round: usize,
    pub current_idx: usize,
    pub participants: Vec<Participant>,
}

#[derive(PersistentStruct, Clone, Copy, Default, PartialEq, Eq, PartialOrd)]
pub struct TimeVec {
    pub round: usize,
    pub sub_round_time: SubRoundTime,
}

#[derive(PersistentStruct, Clone)]
pub struct Participant {
    pub name: String,
    pub hp: u16,
    pub modifiers: Vec<Modifier>,
}

#[derive(PersistentStruct, Clone, new)]
pub struct Modifier {
    pub name: String,
    pub introduced_at: TimeVec,
    pub duration: Option<usize>,
}

#[derive(Clone, Copy, new, Eq, Default)]
pub struct SubRoundTime {
    nom: usize,
    denom: usize,
}

impl SubRoundTime {
    pub fn as_float(&self) -> f64 {
        self.nom as f64 / self.denom as f64
    }
}

impl PartialEq for SubRoundTime {
    fn eq(&self, other: &Self) -> bool {
        self.as_float() == other.as_float()
    }
}

impl PartialOrd for SubRoundTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_float().partial_cmp(&other.as_float())
    }
}

impl TimeVec {
    pub fn new(round: usize, sr_nom: usize, sr_denom: usize) -> TimeVec {
        TimeVec {
            round,
            sub_round_time: SubRoundTime::new(sr_nom, sr_denom),
        }
    }

    pub fn with_next_turn(self) -> TimeVec {
        let TimeVec {
            round,
            sub_round_time: SubRoundTime { nom, denom },
        } = self;
        assert!(denom > 0);
        if nom == denom - 1 {
            TimeVec::new(round + 1, 0, denom)
        } else {
            TimeVec::new(round, nom + 1, denom)
        }
    }
}

impl CombatState {
    pub fn now(&self) -> TimeVec {
        TimeVec {
            round: self.current_round,
            sub_round_time: SubRoundTime::new(self.current_idx, self.participants.len()),
        }
    }

    pub fn with_next_turn(self) -> CombatState {
        let mut next_state = if self.current_idx == self.participants.len() - 1 {
            self.update_current_round(|r| r + 1).with_current_idx(0)
        } else {
            self.update_current_idx(|i| i + 1)
        };
        let now = next_state.now();
        for p in &mut next_state.participants {
            p.modifiers.retain(|x| {
                if let Some(dur) = x.remaining_rounds(&now) {
                    dur > 0
                } else {
                    true
                }
            })
        }
        next_state
    }

    pub fn from_participants(participants: Vec<Participant>) -> CombatState {
        CombatState {
            participants,
            current_idx: 0,
            current_round: 0,
        }
    }
    pub fn with_nth_participant_popped(self, n: usize) -> (Self, Participant) {
        let (res, participants) = utils::with_popped_n(self.participants, n);
        (
            CombatState {
                participants,
                ..self
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

pub type ModifierFac = Box<dyn Fn(TimeVec) -> Modifier>;

impl Modifier {
    pub fn parse_factory(s: &str) -> Result<ModifierFac> {
        let elems: Vec<&str> = s.split(":").collect();
        ensure!(
            elems.len() >= 1,
            "Modifiers must have the following format: <Name>[:<Duration>]"
        );
        let name = elems[0].trim().to_string();
        match elems.len() {
            1 => Ok(Box::new(move |start| {
                Modifier::new(name.clone(), start, None)
            })),
            2 => {
                let dur: usize = elems[1]
                    .trim()
                    .parse()
                    .context("Parsing Modifier Duration")?;
                Ok(Box::new(move |start| {
                    Modifier::new(name.clone(), start, Some(dur))
                }))
            }
            _ => Err(anyhow!(
                "Modifiers must have the following format: <Name>[:<Duration>]"
            )),
        }
    }

    pub fn remaining_rounds(&self, now: &TimeVec) -> Option<i64> {
        if let Some(dur) = &self.duration {
            let start = self.introduced_at;
            let offset = if start.sub_round_time > now.sub_round_time {
                0
            } else {
                -1
            };
            Some((start.round + dur) as i64 - now.round as i64 + offset + 1)
        } else {
            None
        }
    }
}
