//! UI

mod table;

use std::fmt::Debug;
use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style::{self, Color},
    terminal,
};
use tokio::time;

use crate::ExitSender;

use self::table::Table;

const FRAME: Duration = Duration::from_millis(20);

/// X Y W H
#[derive(Copy, Clone)]
pub struct Rect(u16, u16, u16, u16);

/// UI entrypoint.
#[instrument(skip(w, exit))]
pub async fn run_ui(mut w: impl Write + Debug, exit: ExitSender) -> Result<()> {
    execute!(w, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let mut table = Table::default();
    table.push_row(vec!["ur mom Lol!".to_owned()]);
    table.push_row(vec!["hek".to_owned()]);

    loop {
        queue!(
            w,
            style::SetBackgroundColor(Color::Reset),
            style::SetForegroundColor(Color::Reset),
            // terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
        )?;

        let now = Local::now();
        println!("time {}", now);

        let (term_width, term_height) = terminal::size()?;
        let bounds = Rect(5, 5, term_width - 10, term_height - 10);
        table.draw(&mut w, bounds)?;
        w.flush()?;

        // approx 60fps
        time::sleep(FRAME).await;

        // check to see if there's even an event this frame. otherwise, just keep going
        if event::poll(FRAME)? {
            let event = event::read()?;
            table.update(&event);

            if let Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) = event
            {
                break;
            }
        }
    }

    execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;

    exit.send(()).await?;
    debug!("sent exit");
    Ok(())
}
