#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate crossterm;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod config;
mod mail;
mod ui;

use std::fs::File;
use std::io::Read;

use anyhow::Result;
use futures::future::TryFutureExt;
use tokio::sync::{mpsc, oneshot};

use crate::config::Config;

type ExitSender = oneshot::Sender<()>;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;

    let config: Config = {
        let mut config_file = File::open("config.toml")?;
        let mut contents = Vec::new();
        config_file.read_to_end(&mut contents)?;
        toml::from_slice(&contents)?
    };

    let (exit_tx, exit_rx) = oneshot::channel::<()>();
    let (mail_tx, mail_rx) = mpsc::unbounded_channel();

    tokio::spawn(mail::run_mail(config.clone(), mail_rx).unwrap_or_else(report_err));
    let mut stdout = std::io::stdout();
    tokio::spawn(ui::run_ui(stdout, exit_tx).unwrap_or_else(report_err));

    exit_rx.await?;
    Ok(())
}

fn report_err(err: anyhow::Error) {
    error!("error: {:?}", err);
}

fn setup_logger() -> Result<()> {
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
    Ok(())
}
