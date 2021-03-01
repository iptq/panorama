use std::io::Write;

use anyhow::Result;
use crossterm::event::Event;

use super::{DrawBuf, Rect, Screen};

pub trait Widget {
    /// Updates the widget given an event
    fn update(&mut self, event: Option<Event>);

    /// Draws this UI element to the screen
    fn draw(&self, buf: &mut DrawBuf, rect: Rect) -> Result<()>;

    /// Invalidates this UI element, queueing it for redraw
    fn invalidate(&mut self);
}
