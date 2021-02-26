//! Mail

use anyhow::Result;
use futures::{future::FutureExt, stream::StreamExt};
use panorama_imap::{
    client::{
        auth::{self, Auth},
        ClientBuilder, ClientConfig,
    },
    command::Command as ImapCommand,
};
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tokio_stream::wrappers::WatchStream;

use crate::config::{Config, ConfigWatcher, ImapAuth, MailAccountConfig, TlsMethod};

/// Command sent to the mail thread by something else (i.e. UI)
pub enum MailCommand {
    /// Refresh the list
    Refresh,

    /// Send a raw command
    Raw(ImapCommand),
}

/// Main entrypoint for the mail listener.
pub async fn run_mail(
    mut config_watcher: ConfigWatcher,
    _cmd_in: UnboundedReceiver<MailCommand>,
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
            let handle = tokio::spawn(async move {
                debug!("opening imap connection for {:?}", acct);
                loop {
                    match imap_main(acct.clone()).await {
                        Ok(_) => {}
                        Err(err) => {
                            error!("IMAP Error: {}", err);
                        }
                    }

                    warn!("connection dropped, retrying");
                }
            });
            curr_conn.push(handle);
        }
    }

    Ok(())
}

/// The main sequence of steps for the IMAP thread to follow
async fn imap_main(acct: MailAccountConfig) -> Result<()> {
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
        let mut authed = match acct.imap.auth {
            ImapAuth::Plain { username, password } => {
                let auth = auth::Plain { username, password };
                auth.perform_auth(unauth).await?
            }
        };

        debug!("authentication successful!");

        // let's just select INBOX for now, maybe have a config for default mailbox later?
        debug!("selecting the INBOX mailbox");
        authed.select("INBOX").await?;

        loop {
            debug!("listing all emails...");
            let folder_tree = authed.list().await?;

            let mut idle_stream = authed.idle();

            loop {
                idle_stream.next().await;
                debug!("got an event");
            }
        }
    }
}
