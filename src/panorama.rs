use std::any::{Any, TypeId};
use std::collections::HashMap;

use anyhow::Result;

use crate::mailapp::MailApp;
use crate::app::{AnyApp, App};

pub struct Panorama {
    apps: Vec<Box<dyn AnyApp>>,
}

impl Panorama {
    pub fn new() -> Result<Panorama> {
        let mut apps = Vec::new();

        let mail = MailApp::new("mzhang.io")?;
        let mail = Box::new(App::new(mail)) as Box<dyn AnyApp>;
        apps.push(mail);

        Ok(Panorama {
            apps,
        })
    }

    pub async fn run(&self) -> Result<()> {
        debug!("starting all apps...");

        loop {
            self.apps.iter().map(|app| {
                app.say_hello();
            });
        }

        Ok(())
    }
}
