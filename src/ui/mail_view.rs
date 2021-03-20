use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicI8, AtomicU32, Ordering},
    Arc,
};

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local};
use chrono_humanize::HumanTime;
use crossterm::event::{KeyCode, KeyEvent};
use panorama_imap::response::Envelope;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::*,
};

use crate::mail::EmailMetadata;

use super::{FrameType, HandlesInput, InputResult, MailStore, TermType, Window, UI};

#[derive(Debug)]
pub struct MailView {
    pub mail_store: MailStore,
    pub message_list: TableState,
    pub selected: Arc<AtomicU32>,
    pub change: Arc<AtomicI8>,
}

impl HandlesInput for MailView {
    fn handle_key(&mut self, term: TermType, evt: KeyEvent) -> Result<InputResult> {
        let KeyEvent { code, .. } = evt;
        match code {
            // KeyCode::Char('q') => self.0.store(true, Ordering::Relaxed),
            // KeyCode::Char('j') => self.1.store(1, Ordering::Relaxed),
            // KeyCode::Char('k') => self.1.store(-1, Ordering::Relaxed),
            KeyCode::Char(':') => {
                // let colon_prompt = Box::new(ColonPrompt::init(term));
                // return Ok(InputResult::Push(colon_prompt));
            }
            _ => {}
        }

        Ok(InputResult::Ok)
    }
}

impl Window for MailView {
    fn name(&self) -> String {
        String::from("email")
    }

    fn draw(&self, f: FrameType, area: Rect, ui: &UI) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([Constraint::Length(20), Constraint::Max(5000)])
            .split(area);

        let accts = self.mail_store.iter_accts();

        // folder list
        let mut items = vec![];
        for acct in accts.iter() {
            let result = self.mail_store.folders_of(acct);
            if let Some(folders) = result {
                items.push(ListItem::new(acct.to_owned()));
                for folder in folders {
                    items.push(ListItem::new(format!(" {}", folder)));
                }
            }
        }

        let dirlist = List::new(items)
            .block(Block::default().borders(Borders::NONE).title(Span::styled(
                "hellosu",
                Style::default().add_modifier(Modifier::BOLD),
            )))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        // message list table
        // let mut metas = self
        //     .message_uids
        //     .iter()
        //     .filter_map(|id| self.message_map.get(id))
        //     .collect::<Vec<_>>();
        // metas.sort_by_key(|m| m.date);
        // let rows = metas
        //     .iter()
        //     .rev()
        //     .map(|meta| {
        //         let mut row = Row::new(vec![
        //             String::from(if meta.unread { "\u{2b24}" } else { "" }),
        //             meta.uid.map(|u| u.to_string()).unwrap_or_default(),
        //             meta.date.map(|d| humanize_timestamp(d)).unwrap_or_default(),
        //             meta.from.clone(),
        //             meta.subject.clone(),
        //         ]);
        //         if meta.unread {
        //             row = row.style(
        //                 Style::default()
        //                     .fg(Color::LightCyan)
        //                     .add_modifier(Modifier::BOLD),
        //             );
        //         }
        //         row
        //     })
        //     .collect::<Vec<_>>();

        let mut rows = vec![];
        for acct in accts.iter() {
            let result = self.mail_store.messages_of(acct);
            if let Some(messages) = result {
                for meta in messages {
                    let mut row = Row::new(vec![
                        String::from(if meta.unread { "\u{2b24}" } else { "" }),
                        meta.uid.map(|u| u.to_string()).unwrap_or_default(),
                        meta.date.map(|d| humanize_timestamp(d)).unwrap_or_default(),
                        meta.from.clone(),
                        meta.subject.clone(),
                    ]);
                    if meta.unread {
                        row = row.style(
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                    rows.push(row);
                }
            }
        }
        let table = Table::new(rows)
            .style(Style::default().fg(Color::White))
            .widths(&[
                Constraint::Length(1),
                Constraint::Max(3),
                Constraint::Min(20),
                Constraint::Min(35),
                Constraint::Max(5000),
            ])
            .header(
                Row::new(vec!["", "UID", "Date", "From", "Subject"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(dirlist, chunks[0]);
        f.render_widget(table, chunks[1]);
    }
}

/// Turn a timestamp into a format that a human might read when viewing it in a table.
///
/// This means, for dates within the past 24 hours, report it in a relative format.
///
/// For dates sent this year, omit the year entirely.
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

impl MailView {
    pub fn new(mail_store: MailStore) -> Self {
        MailView {
            mail_store,
            message_list: TableState::default(),
            selected: Arc::new(AtomicU32::default()),
            change: Arc::new(AtomicI8::default()),
        }
    }

    pub fn move_down(&mut self) {
        // if self.message_uids.is_empty() {
        //     return;
        // }
        // let len = self.message_uids.len();
        // if let Some(selected) = self.message_list.selected() {
        //     if selected + 1 < len {
        //         self.message_list.select(Some(selected + 1));
        //     }
        // } else {
        //     self.message_list.select(Some(0));
        // }
    }

    pub fn move_up(&mut self) {
        // if self.message_uids.is_empty() {
        //     return;
        // }
        // let len = self.message_uids.len();
        // if let Some(selected) = self.message_list.selected() {
        //     if selected >= 1 {
        //         self.message_list.select(Some(selected - 1));
        //     }
        // } else {
        //     self.message_list.select(Some(len - 1));
        // }
    }

    pub fn update(&mut self) {
        // make the change
        if self
            .change
            .compare_exchange(-1, 0, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.move_up();
        }
        if self
            .change
            .compare_exchange(1, 0, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.move_down();
        }
    }
}
