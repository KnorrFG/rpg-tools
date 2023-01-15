use iced::widget::Text;
use iced::Element;
use iced_aw::TabLabel;

use super::{Message, Tab};

pub struct ViewNpcTab;

#[derive(Debug, Clone)]
pub enum ViewNpcMessage {
    None,
}

impl ViewNpcTab {
    pub fn new() -> ViewNpcTab {
        ViewNpcTab
    }

    pub fn update(&mut self, message: ViewNpcMessage) {}
}

impl Tab for ViewNpcTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text("View NPC".into())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        Text::new("Under Construction").into()
    }
}
