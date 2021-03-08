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

use self::mail_tab::render_mail_tab;

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

    let mut folders = Vec::<String>::new();
    let mut messages = Vec::<String>::new();

    loop {
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Length(1), Constraint::Max(5000)])
                .split(f.size());

            // this is the title bar
            let titles = vec!["hellosu"].into_iter().map(Spans::from).collect();
            let tabs = Tabs::new(titles);
            f.render_widget(tabs, chunks[0]);

            render_mail_tab(f, chunks[1], &folders, &messages);
        })?;

        let event = if event::poll(FRAME_DURATION)? {
            let event = event::read()?;
            // table.update(&event);

            if let Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) = event
            {
                break;
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
                        folders = new_folders;
                    }
                    MailEvent::MessageList(new_messages) => {
                        messages = new_messages;
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
