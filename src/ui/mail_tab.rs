use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::*,
};

use super::FrameType;

pub fn render_mail_tab(f: &mut FrameType, area: Rect, folders: &[String], messages: &[String]) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Length(20), Constraint::Max(5000)])
        .split(area);

    let items = folders
        .iter()
        .map(|s| ListItem::new(s.to_owned()))
        .collect::<Vec<_>>();

    let dirlist = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");

    let rows = messages
        .iter()
        .map(|s| Row::new(vec![s.as_ref()]))
        .collect::<Vec<_>>();
    let table = Table::new(rows)
        .style(Style::default().fg(Color::White))
        .widths(&[Constraint::Max(5000)])
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(dirlist, chunks[0]);
    f.render_widget(table, chunks[1]);
}
