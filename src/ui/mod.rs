//! UI library

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
    Terminal,
};

const FRAME_DURATION: Duration = Duration::from_millis(17);

/// Main entrypoint for the UI
pub async fn run_ui(mut stdout: Stdout, exit_tx: mpsc::Sender<()>) -> Result<()> {
    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Max(5000),
                    // Constraint::Percentage(10),
                    // Constraint::Percentage(80),
                    // Constraint::Percentage(10),
                ])
                .split(f.size());

            let chunks2 = Layout::default()
                .direction(Direction::Horizontal)
                .margin(0)
                .constraints([
                    Constraint::Length(20),
                    Constraint::Max(5000),
                    //
                ])
                .split(chunks[1]);

            // this is the title bar
            let titles = vec!["hellosu"].into_iter().map(Spans::from).collect();
            let tabs = Tabs::new(titles);
            f.render_widget(tabs, chunks[0]);

            // TODO: check active tab
            let items = [
                ListItem::new("Osu"),
                ListItem::new("Game").style(Style::default().add_modifier(Modifier::BOLD)),
            ];
            let dirlist = List::new(items)
                .block(Block::default().title("List").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>");
            f.render_widget(dirlist, chunks2[0]);

            let block = Block::default().title("Block").borders(Borders::ALL);
            f.render_widget(block, chunks2[1]);

            // let block = Block::default().title("Block 2").borders(Borders::ALL);
            // f.render_widget(block, chunks[1]);
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

    mem::drop(terminal);

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
