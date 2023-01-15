use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::iter::once;
use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use derive_new::new;
use iced::alignment::Horizontal;
use iced::widget::{column, row, Button, Column, Container, Row, Space, Text};
use iced::{Element, Length};
use iced_aw::TabLabel;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use toml::Value;

use super::{Message, Tab};

mod npc_builder;
use macros::try_as;
use npc_builder::{load_blueprints_from_table, NpcBlueprint, NpcBuilder};

/// enables creation of a new state by moving components of the old state.
/// first swaps the old state with a placeholder, then creates the new state
/// from the old state, and then swaps back.
macro_rules! with_state {
    ($state:expr, $pat:pat => $block:block) => {{
        let mut tmp_state = State::Error("Switching States".into());
        std::mem::swap($state, &mut tmp_state);
        let mut new_state = if let $pat = tmp_state {
            $block
        } else {
            State::Error(format!(
                "Unexpected State, expected: {}, got {:?} ",
                std::stringify!($pat),
                $state
            ))
        };
        std::mem::swap($state, &mut new_state);
    }};
}

type Blueprints = HashMap<String, NpcBlueprint>;
pub struct GenNpcTab {
    state: State,
}

#[derive(Debug)]
enum State {
    Error(String),
    Initiated(Box<Blueprints>),
    Building(Box<Blueprints>, NpcBuilder, BuildingData),
}

#[derive(Debug, new)]
struct BuildingData {
    all_options: Vec<String>,
    /// the bool implies whether the option is selected currently
    displayed_options: HashMap<String, bool>,
    n: usize,
    field_name: String,
}

#[derive(Debug, Clone)]
pub enum GenNpcMessage {
    ReInit,
    GenNpc(String),
    AttribSelected(String),
}

impl GenNpcTab {
    pub fn new() -> GenNpcTab {
        let attempt = || -> Result<GenNpcTab> {
            let conf_text = std::fs::read_to_string("/home/felix/.config/campman/npc_gen.toml")
                .context("Could not load npc_gen.toml")?;
            let t = conf_text.parse::<Value>()?;
            let t = load_blueprints_from_table(try_as!(t, table)?.clone())?;
            Ok(GenNpcTab {
                state: State::Initiated(Box::new(t)),
            })
        };
        attempt().unwrap_or_else(|err| GenNpcTab {
            state: State::Error(format!("{}", err)),
        })
    }

    pub fn update(&mut self, message: GenNpcMessage) {
        use GenNpcMessage::*;
        match message {
            ReInit => *self = Self::new(),
            GenNpc(name) => with_state! {&mut self.state,
                State::Initiated(bps) => {
                    let bp: NpcBlueprint = bps.get(&name).unwrap().clone();
                    let builder = NpcBuilder::new(bp);
                    let (field_name, opts, n) = builder.current_field_infos().unwrap();
                    let rolled_options = roll_options(&opts, n);
                    let displayed_opts = HashMap::from_iter(rolled_options);
                    let bd = BuildingData::new(opts, displayed_opts, n, field_name);
                    State::Building(bps, builder, bd)
                }
            },
            AttribSelected(_) => todo!(),
        }
    }
}

fn roll_options(xs: &Vec<String>, n: usize) -> HashMap<String, bool> {
    HashMap::from_iter(
        xs.into_iter()
            .choose_multiple(&mut rand::thread_rng(), n * 3)
            .into_iter()
            .map(|x| (x.clone(), false)),
    )
}

impl Tab for GenNpcTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text("Gen NPC".into())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        match &self.state {
            State::Error(e) => render_error(e),
            State::Initiated(blueprints) => render_initiated_screen(blueprints),
            State::Building(blueprints, builder, builder_data) => {
                render_building(blueprints, builder, builder_data).map(Message::GenNpcMsg)
            }
        }
    }
}

fn render_initiated_screen(bps: &Box<Blueprints>) -> Element<'_, Message> {
    let content: Element<'_, GenNpcMessage> = row!(
        Space::with_width(Length::FillPortion(1)),
        Container::new(
            column!(
                Text::new("What type of Npc do you want to generate?").size(24),
                Column::with_children(
                    bps.keys()
                        .map(|k| {
                            Button::new(
                                Text::new(k)
                                    .width(Length::Fill)
                                    .horizontal_alignment(Horizontal::Center),
                            )
                            .on_press(GenNpcMessage::GenNpc(k.clone()))
                            .width(Length::Fill)
                            .into()
                        })
                        .collect()
                )
                .spacing(10)
            )
            .spacing(10)
        )
        .width(Length::FillPortion(2))
        .padding(20),
        Space::with_width(Length::FillPortion(1))
    )
    .into();
    content.map(Message::GenNpcMsg)
}

fn h_space<T: 'static>(rel_width: u16) -> Element<'static, T> {
    Space::with_width(Length::FillPortion(rel_width)).into()
}

fn render_building<'a>(
    _bps: &'a Box<Blueprints>,
    _builder: &'a NpcBuilder,
    bd: &'a BuildingData,
) -> Element<'a, GenNpcMessage> {
    // theoretically, iced_lazy::responsive can be used to create a widget that knows its size,
    // but that doesn't compile currently, so this is a workaround for now
    println!("{:?}", bd);

    column!(
        centered_text(format!("Choose {} options for {}", bd.n, bd.field_name)).size(24),
        Row::with_children({
            let mut elems: Vec<Element<'_, _>> = (0..bd.n)
                .map(|idx| {
                    Column::with_children(
                        bd.displayed_options
                            .iter()
                            .dropping(idx * 3)
                            .take(3)
                            .map(|(name, selected)| {
                                Button::new(centered_text(name))
                                    .on_press(GenNpcMessage::AttribSelected(name.clone()))
                                    .width(Length::Fill)
                                    .into()
                            })
                            .collect(),
                    )
                    .spacing(10)
                    .width(Length::FillPortion(1))
                    .into()
                })
                .collect();

            // this is not efficient, but speed doesn't matter here, and it's the easiest
            // approach
            elems.push(h_space(1));
            elems.insert(0, h_space(1));
            elems
        })
        .spacing(10)
    )
    .spacing(10)
    .into()
}

fn centered_text<'a>(s: impl Into<Cow<'a, str>>) -> Text<'a> {
    Text::new(s)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
}

fn render_error(err: &str) -> Element<'_, Message> {
    let content: Element<'_, GenNpcMessage> = column!(
        Text::new(format!("An error Occured:\n{}", err)),
        Button::new("Try Again")
            .on_press(GenNpcMessage::ReInit)
            .padding(5)
    )
    .spacing(20)
    .into();
    content.map(Message::GenNpcMsg)
}
