use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{Column, Container, Text},
    Element, Font, Length, Sandbox, Settings,
};
use iced_aw::{style::TabBarStyles, TabLabel, Tabs};

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;

mod gen_npc_tab;
use gen_npc_tab::{GenNpcMessage, GenNpcTab};

mod view_npc_tab;
use view_npc_tab::{ViewNpcMessage, ViewNpcTab};

mod iced_utils;

static CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();

fn main() -> Result<()> {
    init()?;
    Ok(CampMan::run(Settings::default())?)
}

struct CampMan {
    active_tab: usize,
    gen_npc_tab: GenNpcTab,
    view_npc_tab: ViewNpcTab,
}

#[derive(Clone, Debug)]
enum Message {
    TabSelected(usize),
    GenNpcMsg(GenNpcMessage),
    ViewNpcMsg(ViewNpcMessage),
}

impl Sandbox for CampMan {
    type Message = Message;

    fn new() -> Self {
        CampMan {
            active_tab: 0,
            gen_npc_tab: GenNpcTab::new(),
            view_npc_tab: ViewNpcTab::new(),
        }
    }

    fn title(&self) -> String {
        String::from("Campaign Manager")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::TabSelected(selected) => self.active_tab = selected,
            Message::GenNpcMsg(message) => self.gen_npc_tab.update(message),
            Message::ViewNpcMsg(message) => self.view_npc_tab.update(message),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Tabs::new(self.active_tab, Message::TabSelected)
            .push(self.gen_npc_tab.tab_label(), self.gen_npc_tab.view())
            .push(self.view_npc_tab.tab_label(), self.view_npc_tab.view())
            .tab_bar_style(TabBarStyles::default())
            //.icon_font(ICON_FONT)
            //.tab_bar_position(TabBarPosition::Top)
            .into()
    }
}

trait Tab {
    type Message;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message> {
        Container::new(self.content())
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(TAB_PADDING)
            .into()
    }

    fn content(&self) -> Element<'_, Self::Message>;
}

fn init() -> Result<()> {
    CONFIG_PATH
        .set(
            dirs::config_dir()
                .ok_or(anyhow!("Couldn't find config dir"))?
                .join("campman/config.toml"),
        )
        .map_err(|_| anyhow!("init was called twice"))?;
    Ok(())
}

fn conf_dir() -> &'static Path {
    CONFIG_PATH.get().unwrap().parent().unwrap()
}
