//! Mail

mod imap;

use anyhow::Result;
use futures::stream::StreamExt;
use panorama_imap::builders::command::Command as ImapCommand;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tokio_stream::wrappers::WatchStream;

use crate::config::{Config, ConfigWatcher};

use self::imap::open_imap_connection;

/// Command sent to the mail thread by something else (i.e. UI)
pub enum MailCommand {
    /// Refresh the list
    Refresh,

    /// Send a raw command
    Raw(ImapCommand),
}

/// Main entrypoint for the mail listener.
pub async fn run_mail(
    config_watcher: ConfigWatcher,
    _cmd_in: UnboundedReceiver<MailCommand>,
) -> Result<()> {
    let mut curr_conn: Option<JoinHandle<_>> = None;

    let mut config_watcher = WatchStream::new(config_watcher);
    loop {
        debug!("listening for configs");
        let a = config_watcher.next().await;
        debug!("got config {:?}", a);
        let config: Config = match a {
            Some(Some(v)) => v,
            _ => break,
        };

        // TODO: gracefully shut down connection
        // just gonna drop the connection for now
        if let Some(curr_conn) = curr_conn.take() {
            debug!("dropping connection...");
            curr_conn.abort();
        }

        let handle = tokio::spawn(async {
            for acct in config.mail_accounts.into_iter() {
                open_imap_connection(acct.imap).await.unwrap();
            }
        });
        curr_conn = Some(handle);
    }

    Ok(())
}
