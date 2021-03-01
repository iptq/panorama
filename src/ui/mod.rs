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
use tokio::{sync::mpsc, time};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::*,
    Frame, Terminal,
};

use self::mail_tab::MailTab;

pub(crate) type FrameType<'a, 'b> = Frame<'a, CrosstermBackend<&'b mut Stdout>>;

const FRAME_DURATION: Duration = Duration::from_millis(17);

fn foo(f: &mut FrameType) {}

/// Main entrypoint for the UI
pub async fn run_ui(mut stdout: Stdout, exit_tx: mpsc::Sender<()>) -> Result<()> {
    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(&mut stdout);
    let mut term = Terminal::new(backend)?;

    let mut mail_tab = MailTab::new();

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

            mail_tab.draw(f, chunks[1]);
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

        // approx 60fps
        time::sleep(FRAME_DURATION).await;

        // if let Event::Input(input) = events.next()? {
        //     match input {
        //         Key::Char('q') => {
        //             break;
        //         }
        //         _ => {}
        //     }
        // }
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
