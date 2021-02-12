use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style, terminal,
};
use tokio::time;

use crate::ExitSender;

const FRAME: Duration = Duration::from_millis(33);

pub async fn run_ui(mut w: impl Write, exit: ExitSender) -> Result<()> {
    execute!(w, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    loop {
        execute!(w, cursor::MoveTo(0, 0))?;

        let now = Local::now();
        println!("time {}", now);

        // approx 60fps
        time::sleep(FRAME).await;

        if event::poll(FRAME)? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => break,
                _ => {}
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

    exit.send(()).expect("fake news?");
    Ok(())
}
