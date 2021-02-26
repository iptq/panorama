use std::io::Write;

use anyhow::Result;

use super::{Rect, Screen};

pub trait Drawable {
    /// Draws this UI element to the screen
    fn draw(&self, w: &mut Screen, rect: Rect) -> Result<()>;

    /// Invalidates this UI element, queueing it for redraw
    fn invalidate(&mut self);
}
