#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate crossterm;
#[macro_use]
extern crate log;

mod mail;
mod ui;

use anyhow::Result;
use futures::future::TryFutureExt;
use tokio::sync::oneshot;

type ExitSender = oneshot::Sender<()>;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;

    let (exit_tx, exit_rx) = oneshot::channel::<()>();

    tokio::spawn(mail::run_mail("mzhang.io", 143).unwrap_or_else(report_err));

    let stdout = std::io::stdout();
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
