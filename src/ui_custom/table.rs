use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent},
    style::{self, Color},
};

use super::{DrawBuf, Rect, Screen, Widget};

#[derive(Default)]
pub struct Table {
    selected_row: Option<u16>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn push_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
        if self.selected_row.is_none() {
            self.selected_row = Some(0);
        }
    }
}

impl Widget for Table {
    fn update(&mut self, event: Option<Event>) {
        if let Some(Event::Key(KeyEvent { code, .. })) = event {
            match code {
                KeyCode::Char('j') => {
                    if let Some(selected_row) = &mut self.selected_row {
                        *selected_row = (self.rows.len() as u16 - 1).min(*selected_row + 1);
                    }
                }
                KeyCode::Char('k') => {
                    if let Some(selected_row) = &mut self.selected_row {
                        if *selected_row > 0 {
                            *selected_row -= 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn draw(&self, buf: &mut DrawBuf, rect: Rect) -> Result<()> {
        if !self.rows.is_empty() {
            let mut columns = Vec::new();
            for row in self.rows.iter() {
                for (i, cell) in row.iter().enumerate() {
                    if columns.is_empty() || columns.len() - 1 < i {
                        columns.push(0);
                    } else {
                        columns[i] = cell.len().max(columns[i]);
                    }
                }
            }

            for (i, row) in self.rows.iter().enumerate() {
                // queue!(w, cursor::MoveTo(rect.x, rect.y + i as u16))?;
                // if let Some(v) = self.selected_row {
                //     if v == i as u16 {
                //         // queue!(
                //         //     w,
                //         //     style::SetBackgroundColor(Color::White),
                //         //     style::SetForegroundColor(Color::Black)
                //         // )?;
                //     } else {
                //         // queue!(
                //         //     w,
                //         //     style::SetForegroundColor(Color::White),
                //         //     style::SetBackgroundColor(Color::Black)
                //         // )?;
                //     }
                // }
                let mut s = String::with_capacity(rect.w as usize);
                for (j, cell) in row.iter().enumerate() {
                    s += &cell;
                    for _ in 0..columns[j] + 1 {
                        s += " ";
                    }
                }
                for _ in 0..(rect.w - s.len() as u16) {
                    s += " ";
                }
                println!("{}", s);
            }

            let d = "\u{b7}".repeat(rect.w as usize);
            // queue!(
            //     w,
            //     style::SetBackgroundColor(Color::Black),
            //     style::SetForegroundColor(Color::White)
            // )?;
            for j in self.rows.len() as u16..rect.h {
                // queue!(w, cursor::MoveTo(rect.x, rect.y + j))?;
                println!("{}", d);
            }
        } else {
            let msg = "Nothing in this table!";
            let x = rect.x + (rect.w - msg.len() as u16) / 2;
            let y = rect.y + rect.h / 2;
            // queue!(w, cursor::MoveTo(x, y))?;
            println!("{}", msg);
        }
        Ok(())
    }

    fn invalidate(&mut self) {
        // TODO: do something
    }
}
