use std::path::{Path, PathBuf};
use std::thread;

use anyhow::Result;
use fern::colors::{Color, ColoredLevelConfig};
use futures::future::TryFutureExt;
use panorama::{
    config::{spawn_config_watcher_system, ConfigWatcher},
    mail::{self, MailEvent, MailStore},
    report_err,
    ui::{self, UiParams},
};
use structopt::StructOpt;
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime},
    sync::mpsc,
    task::LocalSet,
};
use xdg::BaseDirectories;

#[derive(Debug, StructOpt)]
#[structopt(author, about)]
struct Opt {
    /// The path to the log file. By default, does not log.
    #[structopt(long = "log-file")]
    log_file: Option<PathBuf>,

    /// Run this application headlessly
    #[structopt(long = "headless")]
    headless: bool,

    /// Don't watch the config file for changes. (NYI)
    // TODO: implement this or decide if it's useless
    #[structopt(long = "no-watch-config")]
    _no_watch_config: bool,
}

fn main() -> Result<()> {
    // parse command line arguments into options struct
    let opt = Opt::from_args();
    setup_logger(opt.log_file.as_ref())?;

    let rt = Runtime::new().unwrap();
    rt.block_on(run(opt)).unwrap();

    Ok(())
}

// #[tokio::main(flavor = "multi_thread")]
async fn run(opt: Opt) -> Result<()> {
    let _xdg = BaseDirectories::new()?;
    let (_config_thread, config_update) = spawn_config_watcher_system()?;
    let mail_store = MailStore::new(config_update.clone());

    // used to notify the runtime that the process should exit
    let (exit_tx, mut exit_rx) = mpsc::channel::<()>(1);

    // send messages from the UI thread to the mail thread
    let (_ui2mail_tx, ui2mail_rx) = mpsc::unbounded_channel();

    // send messages from the mail thread to the UI thread
    let (mail2ui_tx, mail2ui_rx) = mpsc::unbounded_channel();

    // send messages from the UI thread to the vm thread
    let (ui2vm_tx, _ui2vm_rx) = mpsc::unbounded_channel();

    let config_update2 = config_update.clone();
    let mail_store2 = mail_store.clone();
    tokio::spawn(async move {
        mail::run_mail(mail_store2, config_update2, ui2mail_rx, mail2ui_tx)
            .unwrap_or_else(report_err)
            .await;
    });

    if !opt.headless {
        let config_update2 = config_update.clone();
        run_ui(config_update2, mail_store.clone(), exit_tx, mail2ui_rx, ui2vm_tx);
    }

    exit_rx.recv().await;

    // TODO: graceful shutdown
    // yada yada create a background process and pass off the connections so they can be safely
    // shutdown
    std::process::exit(0);
    // Ok(())
}

// Spawns the entire UI in a different thread, since it must be thread-local
fn run_ui(
    config_update: ConfigWatcher,
    mail_store: MailStore,
    exit_tx: mpsc::Sender<()>,
    mail2ui_rx: mpsc::UnboundedReceiver<MailEvent>,
    _ui2vm_tx: mpsc::UnboundedSender<()>,
) {
    let stdout = std::io::stdout();

    let rt = RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    thread::spawn(move || {
        let localset = LocalSet::new();
        let params = UiParams {
            config_update,
            mail_store,
            stdout,
            exit_tx,
            mail2ui_rx,
        };

        localset.spawn_local(async {
            ui::run_ui2(params).unwrap_or_else(report_err).await;
        });

        rt.block_on(localset);
    });
}

fn setup_logger(log_file: Option<impl AsRef<Path>>) -> Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Blue)
        .debug(Color::BrightBlack)
        .warn(Color::Yellow)
        .error(Color::Red);
    let mut logger = fern::Dispatch::new()
        .filter(|meta| {
            meta.target() != "tokio_util::codec::framed_impl"
                && !meta.target().starts_with("rustls::client")
                && !meta.target().starts_with("sqlx::query")
        })
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Trace);
    if let Some(log_file) = log_file {
        logger = logger.chain(fern::log_file(log_file)?);
    }
    logger.apply()?;

    Ok(())
}
