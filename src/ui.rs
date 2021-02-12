use std::io::Write;
use std::sync::mpsc::Receiver;

use anyhow::Result;
use crossterm::{
    event,
    terminal::{self, EnterAlternateScreen},
};

use crate::event::Event;

pub struct Ui<S: Write> {
    screen: S,
    evts: Receiver<Event>,
}

impl<S: Write> Ui<S> {
    pub fn init(mut screen: S, evts: Receiver<Event>) -> Result<Self> {
        execute!(screen, EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;

        Ok(Ui { screen, evts })
    }

    pub fn run(mut self) -> Result<()> {
        use crossterm::event::{Event, KeyCode, KeyEvent};

        loop {
            // check for new events
            use std::sync::mpsc::TryRecvError;
            match self.evts.try_recv() {
                Ok(evt) => {}
                Err(TryRecvError::Empty) => {} // skip
                Err(TryRecvError::Disconnected) => todo!("impossible?"),
            }

            // read events from the terminal
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => {
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl<S: Write> Drop for Ui<S> {
    fn drop(&mut self) {
        use crossterm::{cursor::Show, style::ResetColor, terminal::LeaveAlternateScreen};

        execute!(self.screen, ResetColor, Show, LeaveAlternateScreen,).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}
