//! Mail

mod imap;
mod imap2;

use anyhow::Result;
use futures::stream::StreamExt;
use panorama_imap::{
    client::{ClientBuilder, ClientNotConnected},
    command::Command as ImapCommand,
};
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tokio_stream::wrappers::WatchStream;

use crate::config::{Config, ConfigWatcher, MailAccountConfig, TlsMethod};

use self::imap2::open_imap_connection;

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
    let mut curr_conn: Option<JoinHandle<_>> = None;

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
        if let Some(curr_conn) = curr_conn.take() {
            debug!("dropping connection...");
            curr_conn.abort();
        }

        let handle = tokio::spawn(async {
            for acct in config.mail_accounts.into_iter() {
                debug!("opening imap connection for {:?}", acct);
                match imap_main(acct).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("IMAP Error: {}", err);
                    }
                }
                // open_imap_connection(acct.imap).await.unwrap();
            }
        });

        curr_conn = Some(handle);
    }

    Ok(())
}

/// The main sequence of steps for the IMAP thread to follow
async fn imap_main(acct: MailAccountConfig) -> Result<()> {
    let builder: ClientNotConnected = ClientBuilder::default()
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

    // debug!("sending CAPABILITY");
    // let result = unauth.capabilities().await?;

    Ok(())
}
