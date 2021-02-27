use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;
use crossterm::{cursor::MoveTo, event::Event};

use super::{Rect, Screen, Widget};

pub struct Tabs {
    id_incr: usize,
    active_id: usize,
    names: Vec<(usize, String)>,
    contents: HashMap<usize, Box<dyn Widget>>,
}

impl Tabs {
    pub fn new() -> Self {
        Tabs {
            id_incr: 0,
            active_id: 0,
            names: Vec::new(),
            contents: HashMap::new(),
        }
    }

    pub fn add_tab(&mut self, name: impl AsRef<str>, drawable: impl Widget + 'static) {
        let id = self.id_incr;
        self.id_incr += 1;

        self.names.push((id, name.as_ref().to_owned()));
        self.contents.insert(id, Box::new(drawable));
    }
}

impl Widget for Tabs {
    fn update(&mut self, event: Option<Event>) {}

    fn draw(&self, w: &mut Screen, rect: Rect) -> Result<()> {
        queue!(w, MoveTo(rect.x, rect.y))?;
        for (id, name) in self.names.iter() {
            println!(" {} ", name);
        }

        let new_rect = Rect::new(rect.x, rect.y + 1, rect.w, rect.h - 1);
        if let Some(widget) = self.contents.get(&self.active_id) {
            widget.draw(w, new_rect)?;
        }

        Ok(())
    }

    fn invalidate(&mut self) {}
}
