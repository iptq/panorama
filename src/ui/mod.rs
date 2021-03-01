//! UI library

mod events;

use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use termion::{event::Key, screen::AlternateScreen};
use tokio::sync::mpsc;
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Widget},
    Terminal,
};

use self::events::{Config, Event, Events};

/// Main entrypoint for the UI
pub async fn run_ui(stdout: Stdout, exit_tx: mpsc::Sender<()>) -> Result<()> {
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::with_config(Config {
        tick_rate: Duration::from_millis(17),
        ..Config::default()
    });

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(80),
                        Constraint::Percentage(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            let block = Block::default().title("Block").borders(Borders::ALL);
            f.render_widget(block, chunks[0]);
            let block = Block::default().title("Block 2").borders(Borders::ALL);
            f.render_widget(block, chunks[1]);
        })?;

        if let Event::Input(input) = events.next()? {
            match input {
                Key::Char('q') => {
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
