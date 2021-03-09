//! UI library

mod mail_tab;

use std::io::Stdout;
use std::mem;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style, terminal,
};
use futures::{future::FutureExt, select, stream::StreamExt};
use tokio::{sync::mpsc, time};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::*,
    Frame, Terminal,
};

use crate::mail::MailEvent;

use self::mail_tab::MailTabState;

pub(crate) type FrameType<'a, 'b> = Frame<'a, CrosstermBackend<&'b mut Stdout>>;

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

    loop {
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
            let titles = vec!["OSU mail"].into_iter().map(Spans::from).collect();
            let tabs = Tabs::new(titles);
            f.render_widget(tabs, chunks[0]);

            mail_tab.render(f, chunks[1]);

            let status = Paragraph::new("hellosu");
            f.render_widget(status, chunks[2]);
        })?;

        let event = if event::poll(FRAME_DURATION)? {
            let event = event::read()?;
            // table.update(&event);

            if let Event::Key(KeyEvent { code, .. }) = event {
                let selected = mail_tab.message_list.selected();
                let len = mail_tab.messages.len();
                let seln = selected
                    .map(|x| if x < len - 1 { x + 1 } else { x })
                    .unwrap_or(0);
                let selp = selected.map(|x| if x > 0 { x - 1 } else { 0 }).unwrap_or(0);
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') => mail_tab.message_list.select(Some(seln)),
                    KeyCode::Char('k') => mail_tab.message_list.select(Some(selp)),
                    _ => {}
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
                    MailEvent::FolderList(new_folders) => {
                        mail_tab.folders = new_folders;
                    }
                    MailEvent::MessageList(new_messages) => {
                        mail_tab.messages = new_messages;
                    }
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
