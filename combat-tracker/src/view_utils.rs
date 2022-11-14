use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, ListItem, Paragraph},
};

use crate::{combat_state::Participant, Frame};

pub fn input_layout(r: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(r)
}

pub fn select_layout(r: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(r)
}

pub fn render_input_block(f: &mut Frame, title: &str, buffer: &str, chunk: Rect) {
    let input = Paragraph::new(buffer).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(input, chunk);
    f.set_cursor(chunk.x + buffer.len() as u16 + 1, chunk.y + 1);
}

pub fn participants_list_items(
    participants: &Vec<Participant>,
    inis: &Vec<Option<u8>>,
) -> Vec<ListItem<'static>> {
    participants
        .iter()
        .zip(inis)
        .map(|(p, ini)| {
            ListItem::new(format!(
                "{} - HP: {};{}",
                p.name,
                p.hp,
                if let Some(ini) = ini {
                    format!(" Ini: {}", ini)
                } else {
                    "".to_string()
                }
            ))
        })
        .collect()
}
