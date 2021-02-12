#[macro_use]
extern crate crossterm;

mod ui;

use anyhow::Result;
use lettre::SmtpClient;
use tokio::sync::oneshot;

type ExitSender = oneshot::Sender<()>;

#[tokio::main]
async fn main() -> Result<()> {
    SmtpClient::new_simple("");

    let (exit_tx, exit_rx) = oneshot::channel::<()>();

    let stdout = std::io::stdout();
    tokio::spawn(ui::run_ui(stdout, exit_tx));

    exit_rx.await?;
    Ok(())
}
