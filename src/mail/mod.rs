//! Mail

mod client;
mod metadata;

use anyhow::Result;
use futures::{
    future::FutureExt,
    stream::{Stream, StreamExt},
};
use notify_rust::{Notification, Timeout};
use panorama_imap::{
    client::{
        auth::{self, Auth},
        ClientBuilder, ClientConfig,
    },
    command::Command as ImapCommand,
    response::{AttributeValue, Envelope, MailboxData, Response},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::wrappers::WatchStream;

use crate::config::{Config, ConfigWatcher, ImapAuth, MailAccountConfig, TlsMethod};

pub use self::metadata::EmailMetadata;

/// Command sent to the mail thread by something else (i.e. UI)
#[derive(Debug)]
#[non_exhaustive]
pub enum MailCommand {
    /// Refresh the list
    Refresh,

    /// Send a raw command
    Raw(ImapCommand),
}

/// Possible events returned from the server that should be sent to the UI
#[derive(Debug)]
#[non_exhaustive]
pub enum MailEvent {
    /// Got the list of folders
    FolderList(Vec<String>),

    /// Got the current list of messages
    MessageList(Vec<Envelope>),

    /// A list of the UIDs in the current mail view
    MessageUids(Vec<u32>),

    /// Update the given UID with the given attribute list
    UpdateUid(u32, Vec<AttributeValue>),

    /// New message came in with given UID
    NewUid(u32),
}

/// Main entrypoint for the mail listener.
pub async fn run_mail(
    mut config_watcher: ConfigWatcher,
    ui2mail_rx: UnboundedReceiver<MailCommand>,
    mail2ui_tx: UnboundedSender<MailEvent>,
) -> Result<()> {
    let mut curr_conn: Vec<JoinHandle<_>> = Vec::new();

    // let mut config_watcher = WatchStream::new(config_watcher);
    loop {
        debug!("listening for configs");
        let config: Config = match config_watcher.changed().await {
            Ok(_) => config_watcher.borrow().clone(),
            _ => break,
        };
        debug!("got");

        // TODO: gracefully shut down connection
        // just gonna drop the connection for now
        // FUTURE TODO: possible to hash the connections and only reconn the ones that changed
        debug!("dropping all connections...");
        for conn in curr_conn.drain(0..) {
            conn.abort();
        }

        for (acct_name, acct) in config.mail_accounts.into_iter() {
            let mail2ui_tx = mail2ui_tx.clone();
            let handle = tokio::spawn(async move {
                // debug!("opening imap connection for {:?}", acct);

                // this loop is to make sure accounts are restarted on error
                loop {
                    match client::imap_main(acct.clone(), mail2ui_tx.clone()).await {
                        Ok(_) => {}
                        Err(err) => {
                            error!("IMAP Error: {}", err);
                        }
                    }

                    warn!("connection dropped, retrying");

                    // wait a bit so we're not hitting the server really fast if the fail happens
                    // early on
                    //
                    // TODO: some kind of smart exponential backoff that considers some time
                    // threshold to be a failing case?
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
            curr_conn.push(handle);
        }
    }

    Ok(())
}
