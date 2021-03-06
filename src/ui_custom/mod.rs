//! UI module

mod drawbuf;
mod table;
mod tabs;
mod widget;

use std::fmt::Debug;
use std::io::{Stdout, Write};
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent},
    style::{self, Color, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use tokio::time;

use crate::ExitSender;

use self::drawbuf::DrawBuf;
use self::table::Table;
use self::tabs::Tabs;
use self::widget::Widget;

const FRAME_DURATION: Duration = Duration::from_millis(20);

/// Type alias for the screen object we're drawing to
pub type Screen = Stdout;

/// Rectangle
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    /// Construct a new rectangle from (x, y) and (w, h)
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Rect { x, y, w, h }
    }
}

/// UI entrypoint.
pub async fn run_ui(mut w: Stdout, exit: ExitSender) -> Result<()> {
    execute!(w, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let (term_width, term_height) = terminal::size()?;
    let bounds = Rect::new(0, 0, term_width, term_height);
    let mut drawbuf = DrawBuf::new(bounds);

    let mut table = Table::default();
    table.push_row(vec!["ur mom Lol!".to_owned()]);
    table.push_row(vec!["hek".to_owned()]);

    let mut tabs = Tabs::new();
    tabs.add_tab("Mail", table);

    loop {
        queue!(
            w,
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset),
            Clear(ClearType::All),
            MoveTo(0, 0),
        )?;

        // let now = Local::now();
        // println!("time {}", now);

        let (term_width, term_height) = terminal::size()?;
        let bounds = Rect::new(5, 5, term_width - 10, term_height - 10);
        // table.draw(&mut w, bounds)?;
        // tabs.draw(&mut w, bounds)?;
        drawbuf.draw(&mut w)?;
        w.flush()?;

        // approx 60fps
        time::sleep(FRAME_DURATION).await;

        // check to see if there's even an event this frame. otherwise, just keep going
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

        tabs.update(event);
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
