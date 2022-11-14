use crate::combat_state::Participant;
use anyhow::{Context, Result};
use rand::Rng;

pub fn parse_participant_with_ini(s: &str) -> Result<(Option<u8>, Participant)> {
    let mut splits: Vec<&str> = s.split(':').collect();
    let ini = if splits.len() > 2 {
        Some(splits.pop().unwrap().trim().parse()?)
    } else {
        None
    };
    Ok((
        ini,
        Participant::parse_splits(splits).context("Participant::parse_splits")?,
    ))
}

pub fn with_popped_n<T>(mut xs: Vec<T>, n: usize) -> (T, Vec<T>) {
    let elem = xs.remove(n);
    (elem, xs)
}
pub fn roll(n: u8, dice: u8) -> u8 {
    let mut rng = rand::thread_rng();
    let dist = rand::distributions::Uniform::new_inclusive(1, dice);
    (0..n).map(|_| rng.sample(dist) as u8).fold(0, |a, b| a + b)
}

pub fn update_nth<T, F>(mut xs: Vec<T>, n: usize, f: F) -> Vec<T>
where
    F: FnOnce(&T) -> T,
{
    if let Some(x) = xs.get(n) {
        xs[n] = f(x);
    }
    xs
}
