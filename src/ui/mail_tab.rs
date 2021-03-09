use panorama_imap::response::Envelope;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::*,
};

use super::FrameType;

#[derive(Default)]
pub struct MailTabState {
    pub folders: Vec<String>,
    pub messages: Vec<Envelope>,
    pub message_list: TableState,
}

impl MailTabState {
    pub fn render(&mut self, f: &mut FrameType, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([Constraint::Length(20), Constraint::Max(5000)])
            .split(area);

        // folder list
        let items = self
            .folders
            .iter()
            .map(|s| ListItem::new(s.to_owned()))
            .collect::<Vec<_>>();

        let dirlist = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        // message list table
        let rows = self
            .messages
            .iter()
            .map(|s| Row::new(vec![s.subject.clone().unwrap_or_default()]))
            .collect::<Vec<_>>();

        let table = Table::new(rows)
            .style(Style::default().fg(Color::White))
            .widths(&[Constraint::Max(5000)])
            .header(Row::new(vec!["Subject"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::LightBlue));

        f.render_widget(dirlist, chunks[0]);
        f.render_stateful_widget(table, chunks[1], &mut self.message_list);
    }
}
