//! UI library

mod colon_prompt;
mod input;
mod keybinds;
mod mail_tab;
mod messages;
mod windows;

use std::any::Any;
use std::collections::HashMap;
use std::io::Stdout;
use std::mem;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Result;
use chrono::{Local, TimeZone};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style, terminal,
};
use downcast_rs::Downcast;
use futures::{future::FutureExt, select, stream::StreamExt};
use panorama_imap::response::{AttributeValue, Envelope};
use tokio::{sync::mpsc, time};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::*,
    Frame, Terminal,
};

use crate::mail::{EmailMetadata, MailEvent};

use self::colon_prompt::ColonPrompt;
use self::input::{BaseInputHandler, HandlesInput, InputResult};
use self::mail_tab::MailTabState;
pub(crate) use self::messages::*;
use self::windows::*;

pub(crate) type FrameType<'a, 'b> = Frame<'a, CrosstermBackend<&'b mut Stdout>>;
pub(crate) type TermType<'a, 'b> = &'b mut Terminal<CrosstermBackend<&'a mut Stdout>>;

const FRAME_DURATION: Duration = Duration::from_millis(17);

/// Main entrypoint for the UI
pub async fn run_ui(
    mut stdout: Stdout,
    exit_tx: mpsc::Sender<()>,
    mut mail2ui_rx: mpsc::UnboundedReceiver<MailEvent>,
) -> Result<()> {
    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(&mut stdout);
    let mut term = Terminal::new(backend)?;
    let mut mail_tab = MailTabState::default();

    // state stack for handling inputs
    let should_exit = Arc::new(AtomicBool::new(false));
    let mut input_states: Vec<Box<dyn HandlesInput>> = vec![Box::new(BaseInputHandler(
        should_exit.clone(),
        mail_tab.change.clone(),
    ))];

    let mut window_layout = WindowLayout::default();
    let mut page_names = HashMap::new();

    // TODO: have this be configured thru the settings?
    let (mail_id, mail_page) = window_layout.new_page();
    page_names.insert(mail_page, "Email");

    while !should_exit.load(Ordering::Relaxed) {
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Max(5000),
                    Constraint::Length(1),
                ])
                .split(f.size());

            // this is the title bar
            // let titles = vec!["email"].into_iter().map(Spans::from).collect();
            let titles = window_layout
                .list_pages()
                .iter()
                .filter_map(|id| page_names.get(id))
                .map(|s| Spans::from(*s))
                .collect();
            let tabs = Tabs::new(titles);
            f.render_widget(tabs, chunks[0]);

            // this is the main mail tab
            mail_tab.render(f, chunks[1]);

            // this is the status bar
            if let Some(last_state) = input_states.last() {
                let downcasted = last_state.downcast_ref::<ColonPrompt>();
                match downcasted {
                    Some(colon_prompt) => {
                        let status = Block::default().title(vec![
                            Span::styled(":", Style::default().fg(Color::Gray)),
                            Span::raw(&colon_prompt.value),
                        ]);
                        f.render_widget(status, chunks[2]);
                        f.set_cursor(colon_prompt.value.len() as u16 + 1, chunks[2].y);
                    }
                    None => {
                        let status = Paragraph::new("hellosu");
                        f.render_widget(status, chunks[2]);
                    }
                };
            }
        })?;

        let event = if event::poll(FRAME_DURATION)? {
            let event = event::read()?;
            // table.update(&event);

            if let Event::Key(evt) = event {
                // handle states in the state stack
                // although this is written in a for loop, every case except one should break
                let mut should_pop = false;
                for input_state in input_states.iter_mut().rev() {
                    match input_state.handle_key(&mut term, evt)? {
                        InputResult::Ok => break,
                        InputResult::Push(state) => {
                            input_states.push(state);
                            break;
                        }
                        InputResult::Pop => {
                            should_pop = true;
                            break;
                        }
                    }
                }

                if should_pop {
                    input_states.pop();
                }
            }

            Some(event)
        } else {
            None
        };

        select! {
            mail_evt = mail2ui_rx.recv().fuse() => {
                debug!("received mail event: {:?}", mail_evt);
                // TODO: handle case that channel is closed later
                let mail_evt = mail_evt.unwrap();

                match mail_evt {
                    MailEvent::FolderList(new_folders) => mail_tab.folders = new_folders,
                    MailEvent::MessageList(new_messages) => mail_tab.messages = new_messages,
                    MailEvent::MessageUids(new_uids) => mail_tab.message_uids = new_uids,

                    MailEvent::UpdateUid(uid, attrs) => {
                        let meta = EmailMetadata::from_attrs(attrs);
                        let uid = meta.uid.unwrap_or(uid);
                        mail_tab.message_map.insert(uid, meta);
                    }
                    MailEvent::NewUid(uid) => {
                        debug!("new msg!");
                        mail_tab.message_uids.push(uid);
                    }
                    _ => {}
                }
            }

            // approx 60fps
            _ = time::sleep(FRAME_DURATION).fuse() => {}
        }
    }

    mem::drop(term);

    execute!(
        stdout,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;

    exit_tx.send(()).await?;
    debug!("sent exit");
    Ok(())
}
