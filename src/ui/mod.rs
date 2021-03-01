//! UI library

use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;
use tui::{
    text::Spans,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::*,
    Terminal,
};

/// Main entrypoint for the UI
pub async fn run_ui(stdout: Stdout, exit_tx: mpsc::Sender<()>) -> Result<()> {
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // let events = Events::with_config(Config {
    //     tick_rate: Duration::from_millis(17),
    //     ..Config::default()
    // });

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

            let titles = vec!["hellosu"].into_iter().map(Spans::from).collect();
            let tabs = Tabs::new(titles);
            f.render_widget(tabs, chunks[2]);
        })?;

        // if let Event::Input(input) = events.next()? {
        //     match input {
        //         Key::Char('q') => {
        //             break;
        //         }
        //         _ => {}
        //     }
        // }
    }

    Ok(())
}
