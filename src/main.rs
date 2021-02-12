#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate crossterm;
#[macro_use]
extern crate log;
#[macro_use]
extern crate pin_project;

mod app;
mod config;
mod event;
mod mailapp;
mod panorama;
mod ui;

use std::io;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::Result;
use tokio::runtime::Runtime;

use crate::panorama::Panorama;
use crate::config::watch_config;
use crate::ui::Ui;

fn main() -> Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("output.log")?)
        .apply()?;

    let runtime = Runtime::new()?;
    thread::spawn(move || {
        let panorama = Panorama::new().unwrap();
        runtime.block_on(panorama.run());
    });

    let stdout = io::stdout();
    let (evts_tx, evts_rx) = channel();

    // spawn a thread for listening to configuration changes
    thread::spawn(move || {
        watch_config();
    });
    info!("poggers");

    // run the ui on the main thread
    let ui = Ui::init(stdout, evts_rx)?;
    ui.run()?;

    Ok(())
}
