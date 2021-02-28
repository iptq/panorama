use std::ops::Range;

use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    style::{Color, SetBackgroundColor, SetForegroundColor},
};

use super::{Rect, Screen};

pub struct DrawBuf {
    rect: Rect,
    buffer: Vec<Cell>,
    dirty: Vec<(u16, Range<u16>)>,
}

#[derive(Clone, Copy)]
struct Cell {
    sym: char,
    fg: Color,
    bg: Color,
}

impl DrawBuf {
    pub fn new(rect: Rect) -> Self {
        DrawBuf {
            rect,
            buffer: vec![
                Cell {
                    sym: ' ',
                    fg: Color::Reset,
                    bg: Color::Reset
                };
                (rect.w * rect.h) as usize
            ],
            dirty: (0..rect.h).map(|row| (row, 0..rect.w)).collect(),
        }
    }

    pub fn draw(&mut self, w: &mut Screen) -> Result<()> {
        for (row, range) in self.dirty.drain(..) {
            queue!(w, MoveTo(row, range.start))?;
            for i in range {
                let idx = row * self.rect.w + i;
                let cell = &self.buffer[idx as usize];
                queue!(w, SetForegroundColor(cell.fg), SetBackgroundColor(cell.bg),)?;
                println!("{}", cell.sym);
            }
        }

        Ok(())
    }
}
