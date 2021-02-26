use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;

use super::{Drawable, Rect, Screen};

pub struct Tabs {
    tabs: HashMap<String, Box<dyn Drawable>>,
}

impl Drawable for Tabs {
    fn draw(&self, w: &mut Screen, rect: Rect) -> Result<()> {
        Ok(())
    }

    fn invalidate(&mut self) {}
}

impl Tabs {
    pub fn new() -> Self {
        Tabs {
            tabs: HashMap::new(),
        }
    }
}
