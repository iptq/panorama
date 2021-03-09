//! Mail

use anyhow::Result;
use futures::{
    future::FutureExt,
    stream::{Stream, StreamExt},
};
use panorama_imap::{
    client::{
        auth::{self, Auth},
        ClientBuilder, ClientConfig,
    },
    command::Command as ImapCommand,
    response::Envelope,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::wrappers::WatchStream;

use crate::config::{Config, ConfigWatcher, ImapAuth, MailAccountConfig, TlsMethod};

/// Command sent to the mail thread by something else (i.e. UI)
#[derive(Debug)]
pub enum MailCommand {
    /// Refresh the list
    Refresh,

    /// Send a raw command
    Raw(ImapCommand),
}

/// Possible events returned from the server that should be sent to the UI
#[derive(Debug)]
pub enum MailEvent {
    /// Got the list of folders
    FolderList(Vec<String>),

    /// Got the current list of messages
    MessageList(Vec<Envelope>),
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

        for acct in config.mail_accounts.into_iter() {
            let mail2ui_tx = mail2ui_tx.clone();
            let handle = tokio::spawn(async move {
                // debug!("opening imap connection for {:?}", acct);

                // this loop is to make sure accounts are restarted on error
                loop {
                    match imap_main(acct.clone(), mail2ui_tx.clone()).await {
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

/// The main sequence of steps for the IMAP thread to follow
async fn imap_main(acct: MailAccountConfig, mail2ui_tx: UnboundedSender<MailEvent>) -> Result<()> {
    // loop ensures that the connection is retried after it dies
    loop {
        let builder: ClientConfig = ClientBuilder::default()
            .hostname(acct.imap.server.clone())
            .port(acct.imap.port)
            .tls(matches!(acct.imap.tls, TlsMethod::On))
            .build()
            .map_err(|err| anyhow!("err: {}", err))?;

        debug!("connecting to {}:{}", &acct.imap.server, acct.imap.port);
        let unauth = builder.open().await?;

        let unauth = if matches!(acct.imap.tls, TlsMethod::Starttls) {
            debug!("attempting to upgrade");
            let client = unauth.upgrade().await?;
            debug!("upgrade successful");
            client
        } else {
            unauth
        };

        debug!("preparing to auth");
        // check if the authentication method is supported
        let mut authed = match &acct.imap.auth {
            ImapAuth::Plain { username, password } => {
                let auth = auth::Plain {
                    username: username.clone(),
                    password: password.clone(),
                };
                auth.perform_auth(unauth).await?
            }
        };

        debug!("authentication successful!");

        // let's just select INBOX for now, maybe have a config for default mailbox later?
        debug!("selecting the INBOX mailbox");
        authed.select("INBOX").await?;

        loop {
            let folder_list = authed.list().await?;
            debug!("mailbox list: {:?}", folder_list);
            let _ = mail2ui_tx.send(MailEvent::FolderList(folder_list));

            let message_uids = authed.uid_search().await?;
            let message_uids = message_uids.into_iter().take(20).collect::<Vec<_>>();
            let message_list = authed.uid_fetch(&message_uids).await?;
            let _ = mail2ui_tx.send(MailEvent::MessageList(message_list));

            let mut idle_stream = authed.idle().await?;

            loop {
                let evt = idle_stream.next().await;
                debug!("got an event: {:?}", evt);

                if false {
                    break;
                }
            }

            if false {
                break;
            }
        }

        // wait a bit so we're not hitting the server really fast if the fail happens
        // early on
        //
        // TODO: some kind of smart exponential backoff that considers some time
        // threshold to be a failing case?
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
