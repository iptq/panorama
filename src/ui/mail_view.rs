use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicI8, AtomicU32, Ordering},
    Arc,
};

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local};
use chrono_humanize::HumanTime;
use panorama_imap::response::Envelope;
use panorama_tui::{
    crossterm::event::{KeyCode, KeyEvent},
    tui::{
        buffer::Buffer,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Span, Spans},
        widgets::*,
    },
};
use tokio::{sync::RwLock, task::JoinHandle};

use crate::mail::{
    store::{AccountRef, MailStoreUpdate},
    EmailMetadata,
};

use super::{FrameType, HandlesInput, InputResult, MailStore, TermType, Window, UI};

#[derive(Debug)]
/// A singular UI view of a list of mail
pub struct MailView {
    pub mail_store: MailStore,
    pub message_list: TableState,
    pub selected: Arc<AtomicU32>,
    pub change: Arc<AtomicI8>,
    current: Arc<RwLock<Option<Current>>>,
    mail_store_listener: JoinHandle<()>,
}

#[derive(Debug)]
struct Current {
    account: Arc<AccountRef>,
    folder: Option<String>,
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

#[async_trait(?Send)]
impl Window for MailView {
    fn name(&self) -> String {
        String::from("email")
    }

    async fn draw(&self, f: &mut FrameType<'_, '_>, area: Rect, ui: &UI) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([Constraint::Length(20), Constraint::Max(5000)])
            .split(area);

        let accts = self.mail_store.list_accounts().await;

        // folder list
        let mut items = vec![];
        for (acct_name, acct_ref) in accts.iter() {
            let folders = acct_ref.get_folders().await;
            items.push(ListItem::new(acct_name.to_owned()));
            for folder in folders {
                items.push(ListItem::new(format!(" {}", folder)));
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

        let mut rows = vec![];
        if let Some(current) = self.current.read().await.as_ref() {
            let messages = current
                .account
                .get_newest_n_messages("INBOX", chunks[1].height as usize)
                .await?;
            for meta in messages.iter() {
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

        Ok(())
    }

    async fn update(&mut self) {
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
        let current = Arc::new(RwLock::new(None));
        let current2 = current.clone();

        let mut listener = mail_store.store_out_rx.clone();
        let mail_store2 = mail_store.clone();
        let mail_store_listener = tokio::spawn(async move {
            while let Ok(()) = listener.changed().await {
                let updated = listener.borrow().clone();
                debug!("new update from mail store: {:?}", updated);

                // TODO: maybe do the processing of updates somewhere else?
                // in case events get missed
                match updated {
                    Some(MailStoreUpdate::AccountListUpdate(_)) => {
                        // TODO: maybe have a default account?
                        let accounts = mail_store2.list_accounts().await;
                        if let Some((acct_name, acct_ref)) = accounts.iter().next() {
                            let mut write = current2.write().await;
                            *write = Some(Current {
                                account: acct_ref.clone(),
                                folder: None,
                            })
                        }
                    }
                    _ => {}
                }
            }
        });

        MailView {
            mail_store,
            current,
            message_list: TableState::default(),
            selected: Arc::new(AtomicU32::default()),
            change: Arc::new(AtomicI8::default()),
            mail_store_listener,
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
}
