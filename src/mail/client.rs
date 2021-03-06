use anyhow::{Context, Result};
use futures::{
    future::{FutureExt, TryFutureExt},
    stream::{self, Stream, StreamExt, TryStreamExt},
};
use notify_rust::{Notification, Timeout};
use panorama_imap::{
    client::{
        auth::{self, Auth},
        ClientBuilder, ClientConfig,
    },
    command::{Command as ImapCommand, FetchItems},
    response::{AttributeValue, Envelope, MailboxData, Response},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::config::{Config, ConfigWatcher, ImapAuth, MailAccountConfig, TlsMethod};

use super::{MailCommand, MailEvent, MailStore};

/// The main function for the IMAP syncing thread
pub async fn sync_main(
    config: Config,
    acct_name: impl AsRef<str>,
    acct: MailAccountConfig,
    mail2ui_tx: UnboundedSender<MailEvent>,
    mail_store: MailStore,
) -> Result<()> {
    let acct_name = acct_name.as_ref().to_owned();

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

        let folder_list = authed.list().await?;
        let _ = mail2ui_tx.send(MailEvent::FolderList(
            acct_name.clone(),
            folder_list.clone(),
        ));
        debug!("mailbox list: {:?}", folder_list);

        for folder in folder_list.iter() {
            debug!("folder: {}", folder);
            let select = authed.select(folder).await?;
            debug!("select response: {:?}", select);

            if let (Some(exists), Some(uidvalidity)) = (select.exists, select.uid_validity) {
                // figure out which uids don't exist locally yet
                let new_uids = stream::iter(1..exists).map(Ok).try_filter_map(|uid| {
                        mail_store.try_identify_email(&acct_name, &folder, uid, uidvalidity, None)
                            // invert the option to only select uids that haven't been downloaded
                            .map_ok(move |o| o.map_or_else(move || Some(uid), |v| None))
                            .map_err(|err| err.context("error checking if the email is already downloaded [try_identify_email]"))
                    }).try_collect::<Vec<_>>().await?;

                if !new_uids.is_empty() {
                    debug!("fetching uids {:?}", new_uids);
                    let fetched = authed
                        .uid_fetch(&new_uids, FetchItems::PanoramaAll)
                        .await
                        .context("error fetching uids")?;

                    fetched
                        .map(Ok)
                        .try_for_each_concurrent(None, |(uid, attrs)| {
                            mail_store.store_email(&acct_name, &folder, uid, uidvalidity, attrs)
                        })
                        .await
                        .context("error during fetch-store")?;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(50)).await;

        // TODO: remove this later
        continue;

        // let's just select INBOX for now, maybe have a config for default mailbox later?
        debug!("selecting the INBOX mailbox");
        let select = authed.select("INBOX").await?;
        debug!("select result: {:?}", select);

        loop {
            let message_uids = authed.uid_search().await?;
            let message_uids = message_uids.into_iter().take(30).collect::<Vec<_>>();
            let _ = mail2ui_tx.send(MailEvent::MessageUids(
                acct_name.clone(),
                message_uids.clone(),
            ));

            // TODO: make this happen concurrently with the main loop?
            let mut message_list = authed
                .uid_fetch(&message_uids, FetchItems::All)
                .await
                .unwrap();
            while let Some((uid, attrs)) = message_list.next().await {
                let evt = MailEvent::UpdateUid(acct_name.clone(), uid, attrs);
                // TODO: probably odn't care about this?
                let _ = mail2ui_tx.send(evt);
            }

            // check if IDLE is supported
            let supports_idle = authed.has_capability("IDLE").await?;
            if supports_idle {
                let mut idle_stream = authed.idle().await?;

                loop {
                    let evt = match idle_stream.next().await {
                        Some(v) => v,
                        None => break,
                    };
                    debug!("got an event: {:?}", evt);

                    match evt {
                        Response::MailboxData(MailboxData::Exists(uid)) => {
                            debug!("NEW MESSAGE WITH UID {:?}, droping everything", uid);
                            // send DONE to stop the idle
                            std::mem::drop(idle_stream);

                            let handle = Notification::new()
                                .summary("New Email")
                                .body("holy Shit,")
                                .icon("firefox")
                                .timeout(Timeout::Milliseconds(6000))
                                .show()?;

                            let message_uids = authed.uid_search().await?;
                            let message_uids =
                                message_uids.into_iter().take(20).collect::<Vec<_>>();
                            let _ = mail2ui_tx.send(MailEvent::MessageUids(
                                acct_name.clone(),
                                message_uids.clone(),
                            ));

                            // TODO: make this happen concurrently with the main loop?
                            let mut message_list = authed
                                .uid_fetch(&message_uids, FetchItems::All)
                                .await
                                .unwrap();
                            while let Some((uid, attrs)) = message_list.next().await {
                                let evt = MailEvent::UpdateUid(acct_name.clone(), uid, attrs);
                                // debug!("sent {:?}", evt);
                                mail2ui_tx.send(evt);
                            }

                            idle_stream = authed.idle().await?;
                        }
                        _ => {}
                    }
                }
            } else {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    debug!("heartbeat");
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
