#[macro_use]
extern crate tracing;

use std::path::PathBuf;

use anyhow::Result;
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
    // use panorama::config::*;
    // let c = ImapConfig{
    //     server:String::from("ouais"),
    //     port: 1,
    //     tls: TlsMethod::Starttls,
    //     auth: ImapAuth::Plain{username:String::from("osu"), password:String::from("game")},
    // };
    // let s = toml::to_string(&c)?;
    // println!("{}", s);
    // panic!();

    // parse command line arguments into options struct
    let _opt = Opt::from_args();

    // print logs to file as directed by command line options
    use tracing_subscriber::filter::LevelFilter;

    let file = tracing_appender::rolling::daily("public", "lol");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_writer(non_blocking)
        .with_thread_ids(true)
        .init();
    debug!("shiet");

    // TODO: debug
    let x = span!(tracing::Level::WARN, "ouais");
    let _y = x.enter();

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
