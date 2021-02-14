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
use std::path::PathBuf;

use anyhow::Result;
use futures::future::TryFutureExt;
use structopt::StructOpt;
use tokio::sync::{mpsc, oneshot};
use xdg::BaseDirectories;

use crate::config::{spawn_config_watcher, MailConfig};

type ExitSender = oneshot::Sender<()>;

#[derive(Debug, StructOpt)]
#[structopt(author, about)]
struct Opt {
    /// Config file
    #[structopt(long = "config-file", short = "c")]
    config_path: Option<PathBuf>,

    /// The path to the log file. By default, does not log.
    #[structopt(long = "log-file")]
    log_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments into options struct
    let opt = Opt::from_args();

    // print logs to file as directed by command line options
    setup_logger(&opt)?;

    let xdg = BaseDirectories::new()?;
    let config_update = spawn_config_watcher()?;

    let config = MailConfig::default();
    // let config: MailConfig = {
    //     let config_path = opt
    //         .config_path
    //         .clone()
    //         .unwrap_or_else(|| "config.toml".into());
    //     let mut config_file = File::open(config_path)?;
    //     let mut contents = Vec::new();
    //     config_file.read_to_end(&mut contents)?;
    //     toml::from_slice(&contents)?
    // };

    // used to notify the runtime that the process should exit
    let (exit_tx, exit_rx) = oneshot::channel::<()>();

    // used to send commands to the mail service
    let (mail_tx, mail_rx) = mpsc::unbounded_channel();

    tokio::spawn(mail::run_mail(config_update.clone(), mail_rx).unwrap_or_else(report_err));
    let stdout = std::io::stdout();
    tokio::spawn(ui::run_ui(stdout, exit_tx).unwrap_or_else(report_err));

    exit_rx.await?;
    Ok(())
}

fn report_err(err: anyhow::Error) {
    error!("error: {:?}", err);
}

fn setup_logger(opt: &Opt) -> Result<()> {
    let mut fern = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug);

    if let Some(path) = &opt.log_file {
        fern = fern.chain(fern::log_file(path)?);
    }

    fern.apply()?;
    Ok(())
}
