use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use derive_new::new;
use tui::{
    layout::Margin,
    widgets::{Block, Borders, Paragraph},
};

use super::State;
use crate::{Frame, StateBox};

#[derive(Clone, new)]
pub struct Msg {
    pub parent: StateBox,
    pub msg: String,
}

impl State for Msg {
    fn process(self: Box<Msg>, ev: Event) -> Result<StateBox> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Enter => Ok(self.parent),
                _ => Ok(self),
            }
        } else {
            Ok(self)
        }
    }

    fn render(&mut self, f: &mut Frame) {
        // self.parent.render(f);
        let msg = Paragraph::new(&self.msg[..])
            .alignment(tui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Error"));
        let rect = f.size();
        f.render_widget(
            msg,
            rect.inner(&Margin {
                vertical: rect.height / 4,
                horizontal: rect.width / 4,
            }),
        );
    }
}
