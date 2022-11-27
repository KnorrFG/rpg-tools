use itertools::Itertools;
use pad::PadStr;
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, ListItem, Paragraph, Row, Table, TableState},
};

use std::iter;

use crate::{
    combat_state::{self as cs, CombatState, Participant, TimeVec},
    states::fighting::KeyInfo,
    Frame,
};

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

pub fn render_fighting_mode_table(
    f: &mut Frame,
    combat_state: &CombatState,
    key_infos: &Vec<KeyInfo>,
    target_rect: Rect,
) {
    let name_col_length = combat_state
        .participants
        .iter()
        .fold(0, |max, p| std::cmp::max(max, p.name.len()))
        + 1;

    let comma_span = Span::from(", ");
    let table_rows: Vec<Row> = combat_state
        .participants
        .iter()
        .zip(key_infos.iter())
        .map(|(p, key_info)| {
            let mods = render_modifiers(&p.modifiers, combat_state);
            let tags = mods.iter().intersperse(&comma_span);
            Row::new(vec![
                Text::from(
                    p.name
                        .pad_to_width_with_alignment(name_col_length, pad::Alignment::Right),
                ),
                Text::from(format!(
                    " <{}- HP: {} -{}> ",
                    key_info.decrement, p.hp, key_info.increment
                )),
                Text::from(Spans::from(
                    iter::once(Span::from(format!("Mods({}): [", key_info.edit_modifiers)))
                        .chain(tags.cloned())
                        .chain(iter::once(Span::from("]")))
                        .collect::<Vec<Span>>(),
                )),
            ])
        })
        .collect();
    let constraints = [
        Constraint::Length(name_col_length as u16),
        Constraint::Length(15),
        Constraint::Length(200),
    ];
    let table = Table::new(table_rows)
        .block(Block::default().borders(Borders::ALL).title("Participants"))
        .widths(&constraints)
        // ...and they can be separated by a fixed spacing.
        .column_spacing(2)
        // If you wish to highlight a row in any specific way when it is selected...
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol(">>");
    let mut table_state = TableState::default();
    table_state.select(Some(combat_state.current_idx));
    f.render_stateful_widget(table, target_rect, &mut table_state);
}

fn render_modifiers(mods: &Vec<cs::Modifier>, cs: &CombatState) -> Vec<Span<'static>> {
    let now = cs.now();
    let next = cs.clone().with_next_turn().now();
    mods.iter()
        .map(|modifier| {
            if let Some(dur) = modifier.remaining_rounds(&now) {
                let style = if modifier.remaining_rounds(&next).unwrap() == 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                };
                Span::styled(format!("{}:{}", modifier.name, dur), style)
            } else {
                Span::from(modifier.name.clone())
            }
        })
        .collect()
}
