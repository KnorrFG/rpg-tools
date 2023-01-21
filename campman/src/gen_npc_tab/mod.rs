use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::iter::once;
use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use derive_new::new;
use iced::alignment::Horizontal;
use iced::theme::Button as ButtonTheme;
use iced::widget::{column, row, Button, Column, Container, Row, Space, Text};
use iced::{Alignment, Color, Element, Length};
use iced_aw::TabLabel;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use toml::Value;

use super::{Message, Tab};
use macros::try_as;
mod npc_builder;
use npc_builder::{load_blueprints_from_table, NpcBlueprint, NpcBuilder, StringMap};

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
    Finalizing(Box<Blueprints>, StringMap),
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
        if let Err(e) = self.inner_update(message) {
            self.state = State::Error(format!("{}", e))
        }
    }

    pub fn inner_update(&mut self, message: GenNpcMessage) -> Result<()> {
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
            AttribSelected(s) => with_state! {&mut self.state,
                State::Building(blueprints, mut builder, mut bd) => {
                    let toggled = !bd.displayed_options.get(&s).unwrap();
                    bd.displayed_options.insert(s, toggled);
                    if bd.displayed_options.values().map(|x| if *x {1} else {0}).sum::<usize>() == bd.n {
                        let selections = bd.displayed_options
                            .into_iter()
                            .filter_map(|(name, selected)| if selected {Some(name)} else {None});
                            if let Some(npc) = builder.set_current_field_val(selections.collect())? {
                                State::Finalizing(blueprints, npc)
                            } else {
                                new_building_state(blueprints, builder)
                            }
                        } else {
                            State::Building(blueprints, builder, bd)
                        }
                }
            },
        }
        Ok(())
    }
}

fn new_building_state(bps: Box<Blueprints>, builder: NpcBuilder) -> State {
    let (field_name, opts, n) = builder.current_field_infos().unwrap();
    let rolled_options = roll_options(&opts, n);
    let displayed_opts = HashMap::from_iter(rolled_options);
    let bd = BuildingData::new(opts, displayed_opts, n, field_name);
    State::Building(bps, builder, bd)
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
            State::Finalizing(blueprints, npc) => render_finalizing(&npc),
            State::Initiated(blueprints) => render_initiated_screen(blueprints),
            State::Building(blueprints, builder, builder_data) => {
                render_building(blueprints, builder, builder_data).map(Message::GenNpcMsg)
            }
        }
    }
}

fn render_finalizing(npc: &StringMap) -> Element<'_, Message> {
    let col = Column::with_children(vec![render_npc(npc)]);
    col.push(
        row!(
            h_space(1),
            text_button("Add Tag", None).width(Length::FillPortion(1)),
            text_button("Add Description", None).width(Length::FillPortion(1)),
            h_space(1)
        )
        .spacing(10),
    )
    .spacing(10)
    .align_items(Alignment::Center)
    .into()
}

fn render_npc<'a, Message: 'a>(npc: &'a StringMap) -> Element<'a, Message> {
    Column::with_children(
        npc.iter()
            .map(|(key, vals)| {
                row!(
                    Text::new(format!("{}:", key.replace("-", " ").replace("_", " ")))
                        .size(24)
                        .width(Length::FillPortion(1))
                        .horizontal_alignment(Horizontal::Right),
                    Text::new(vals.join("\n"))
                        .size(24)
                        .width(Length::FillPortion(1))
                )
                .spacing(10)
                .into()
            })
            .collect(),
    )
    .into()
}

fn text_button<'a, Message>(
    s: impl Into<Cow<'a, str>>,
    msg: Option<Message>,
) -> Button<'a, Message> {
    let b = Button::new(Text::new(s).horizontal_alignment(Horizontal::Center));
    if let Some(msg) = msg {
        b.on_press(msg)
    } else {
        b
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
                                let b = Button::new(centered_text(name))
                                    .on_press(GenNpcMessage::AttribSelected(name.clone()))
                                    .width(Length::Fill);
                                if *selected {
                                    b.style(ButtonTheme::Positive)
                                } else {
                                    b
                                }
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

fn render_error(err: &str) -> Element<'static, Message> {
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
