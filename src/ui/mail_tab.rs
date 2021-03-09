use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Local};
use chrono_humanize::HumanTime;
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
    pub message_uids: Vec<u32>,
    pub message_map: HashMap<u32, EmailMetadata>,
    pub messages: Vec<Envelope>,
    pub message_list: TableState,
}

#[derive(Debug)]
pub struct EmailMetadata {
    pub date: DateTime<Local>,
    pub from: String,
    pub subject: String,
}

fn humanize_timestamp(date: DateTime<Local>) -> String {
    let now = Local::now();
    let diff = now - date;

    if diff < Duration::days(1) {
        HumanTime::from(date).to_string()
    } else if date.year() == now.year() {
        date.format("%b %e  %T").to_string()
    } else {
        date.to_rfc2822()
    }
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
            .message_uids
            .iter()
            .map(|id| {
                let meta = self.message_map.get(id);
                Row::new(vec![
                    "".to_owned(),
                    id.to_string(),
                    meta.map(|m| humanize_timestamp(m.date)).unwrap_or_default(),
                    meta.map(|m| m.from.clone()).unwrap_or_default(),
                    meta.map(|m| m.subject.clone()).unwrap_or_default(),
                ])
            })
            .collect::<Vec<_>>();

        let table = Table::new(rows)
            .style(Style::default().fg(Color::White))
            .widths(&[
                Constraint::Length(1),
                Constraint::Max(3),
                Constraint::Min(25),
                Constraint::Min(20),
                Constraint::Max(5000),
            ])
            .header(
                Row::new(vec!["", "UID", "Date", "From", "Subject"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(dirlist, chunks[0]);
        f.render_stateful_widget(table, chunks[1], &mut self.message_list);
    }
}
