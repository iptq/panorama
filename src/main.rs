#[macro_use]
extern crate log;

use std::fs::OpenOptions;
use std::path::PathBuf;

use anyhow::Result;
use fern::colors::{Color, ColoredLevelConfig};
use futures::future::TryFutureExt;
use panorama::{config::spawn_config_watcher_system, mail, report_err, ui};
use structopt::StructOpt;
use tokio::sync::mpsc;
use xdg::BaseDirectories;

#[derive(Debug, StructOpt)]
#[structopt(author, about)]
struct Opt {
    /// The path to the log file. By default, does not log.
    #[structopt(long = "log-file")]
    log_file: Option<PathBuf>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // parse command line arguments into options struct
    let opt = Opt::from_args();

    let colors = ColoredLevelConfig::new()
        .info(Color::Blue)
        .debug(Color::BrightBlack)
        .warn(Color::Yellow)
        .error(Color::Red);
    let mut logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Debug);
    if let Some(log_file) = opt.log_file {
        logger = logger.chain(fern::log_file(log_file)?);
    }
    logger.apply()?;

    let _xdg = BaseDirectories::new()?;
    let (_config_thread, config_update) = spawn_config_watcher_system()?;

    // used to notify the runtime that the process should exit
    let (exit_tx, mut exit_rx) = mpsc::channel::<()>(1);

    // used to send commands to the mail service
    let (_mail_tx, mail_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let config_update = config_update.clone();
        mail::run_mail(config_update, mail_rx)
            .unwrap_or_else(report_err)
            .await;
    });

    let stdout = std::io::stdout();
    tokio::spawn(ui::run_ui(stdout, exit_tx).unwrap_or_else(report_err));

    exit_rx.recv().await;

    // TODO: graceful shutdown
    // yada yada create a background process and pass off the connections so they can be safely
    // shutdown
    std::process::exit(0);
    // Ok(())
}
