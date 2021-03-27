//! Module for managing the offline storage of emails

use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Local};
use futures::{
    future::{self, FutureExt, TryFutureExt},
    stream::{StreamExt, TryStreamExt},
};
use indexmap::IndexMap;
use panorama_imap::response::AttributeValue;
use sha2::{Digest, Sha256};
use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    sqlite::{Sqlite, SqlitePool},
    Error as SqlxError, Row,
};
use tokio::{
    fs,
    sync::{broadcast, watch, RwLock},
    task::JoinHandle,
};

use crate::config::{Config, ConfigWatcher};

use super::{EmailMetadata, MailEvent};

static MIGRATOR: Migrator = sqlx::migrate!();

/// Manages email storage on disk, for both database and caches
///
/// This struct is clone-safe: cloning it will just return a reference to the same data structure
#[derive(Clone, Debug)]
pub struct MailStore {
    config: Arc<RwLock<Option<Config>>>,
    inner: Arc<RwLock<Option<MailStoreInner>>>,
    handle: Arc<JoinHandle<()>>,
    store_out_tx: Arc<watch::Sender<Option<MailStoreUpdate>>>,

    /// A receiver for listening to updates to the mail store
    pub store_out_rx: watch::Receiver<Option<MailStoreUpdate>>,
}

#[derive(Debug)]
/// This is associated with a particular config. When the config is updated, this gets replaced
struct MailStoreInner {
    pool: SqlitePool,
    mail_dir: PathBuf,
    accounts: IndexMap<String, Arc<AccountRef>>,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// Probably an event about new emails? i forgot
pub enum MailStoreUpdate {
    /// The list of accounts has been updated (probably as a result of a config update)
    AccountListUpdate(()),
}

impl MailStore {
    /// Creates a new MailStore
    pub fn new(mut config_watcher: ConfigWatcher) -> Self {
        let config = Arc::new(RwLock::new(None));
        let config2 = config.clone();

        let inner = Arc::new(RwLock::new(None));
        let inner2 = inner.clone();

        let (store_out_tx, store_out_rx) = watch::channel(None);
        let store_out_tx = Arc::new(store_out_tx);
        let store_out_tx2 = store_out_tx.clone();

        let listener = async move {
            while let Ok(()) = config_watcher.changed().await {
                let new_config = config_watcher.borrow().clone();

                let fut = future::try_join(
                    async {
                        let mut write = config2.write().await;
                        write.replace(new_config.clone());
                        Ok::<_, Error>(())
                    },
                    async {
                        let new_inner =
                            MailStoreInner::init_with_config(new_config.clone()).await?;
                        let mut write = inner2.write().await;
                        write.replace(new_inner);
                        Ok(())
                    },
                );

                match fut.await {
                    Ok(_) => store_out_tx2.send(Some(MailStoreUpdate::AccountListUpdate(()))),
                    Err(e) => {
                        error!("during mail loop: {}", e);
                        panic!();
                    }
                };
            }
        };
        let handle = tokio::spawn(listener);

        MailStore {
            config,
            inner,
            handle: Arc::new(handle),
            store_out_tx,
            store_out_rx,
        }
    }

    /// Nuke all messages with an invalid UIDVALIDITY
    pub async fn nuke_old_uidvalidity(&self, current: usize) {}

    /// Given a UID and optional message-id try to identify a particular message
    pub async fn try_identify_email(
        &self,
        acct: impl AsRef<str>,
        folder: impl AsRef<str>,
        uid: u32,
        uidvalidity: u32,
        message_id: Option<&str>,
    ) -> Result<Option<u32>> {
        let read = self.inner.read().await;
        let inner = match &*read {
            Some(v) => v,
            None => return Ok(None),
        };
        let existing: Option<(u32,)> = into_opt(
            sqlx::query_as(
                r#"
            SELECT rowid FROM "mail"
            WHERE account = ? AND folder = ?
                AND uid = ? AND uidvalidity = ?
            "#,
            )
            .bind(acct.as_ref())
            .bind(folder.as_ref())
            .bind(uid)
            .bind(uidvalidity)
            .fetch_one(&inner.pool)
            .await,
        )?;
        mem::drop(read);

        if let Some(existing) = existing {
            let rowid = existing.0;
            return Ok(Some(rowid));
        }

        Ok(None)
    }

    /// Stores the given email
    pub async fn store_email(
        &self,
        acct: impl AsRef<str>,
        folder: impl AsRef<str>,
        uid: u32,
        uidvalidity: u32,
        attrs: Vec<AttributeValue>,
    ) -> Result<()> {
        let mut body = None;
        let mut internaldate = None;
        for attr in attrs {
            match attr {
                AttributeValue::BodySection(body_attr) => body = body_attr.data,
                AttributeValue::InternalDate(date) => internaldate = Some(date),
                _ => {}
            }
        }

        let body = match body {
            Some(v) => v,
            None => return Ok(()),
        };
        let internaldate = match internaldate {
            Some(v) => v,
            None => return Ok(()),
        };

        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        let hash = hasher.finalize();
        let filename = format!("{}.mail", hex::encode(hash));
        let path = {
            match &*self.inner.read().await {
                Some(inner) => inner.mail_dir.join(&filename),
                None => return Ok(()),
            }
        };
        fs::write(path, &body)
            .await
            .context("error writing email to file")?;

        // parse email
        let mut message_id = None;
        let mut subject = None;
        let mail = mailparse::parse_mail(body.as_bytes())
            .with_context(|| format!("error parsing email with uid {}", uid))?;
        for header in mail.headers.iter() {
            let key = header.get_key_ref();
            let value = header.get_value();
            match key.to_ascii_lowercase().as_str() {
                "message-id" => message_id = Some(value),
                "subject" => subject = Some(value),
                _ => {}
            }
        }

        debug!("message-id: {:?}", message_id);

        let read = self.inner.read().await;
        let inner = match &*read {
            Some(v) => v,
            None => return Ok(()),
        };
        let existing = into_opt(
            sqlx::query(
                r#"
            SELECT * FROM "mail"
            WHERE account = ? AND folder = ?
                AND uid = ? AND uidvalidity = ?
            "#,
            )
            .bind(acct.as_ref())
            .bind(folder.as_ref())
            .bind(uid)
            .bind(uidvalidity)
            .fetch_one(&inner.pool)
            .await,
        )?;

        if existing.is_none() {
            let id = sqlx::query(
                r#"
                INSERT INTO "mail" (
                    account, subject, message_id, folder, uid, uidvalidity,
                    filename, internaldate
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(acct.as_ref())
            .bind(subject)
            .bind(message_id)
            .bind(folder.as_ref())
            .bind(uid)
            .bind(uidvalidity)
            .bind(filename)
            .bind(internaldate.to_rfc3339())
            .execute(&inner.pool)
            .await
            .context("error inserting email into db")?
            .last_insert_rowid();
        }
        mem::drop(read);

        // self.email_events
        //     .send(EmailUpdateInfo {})
        //     .context("error sending email update info to the broadcast channel")?;

        Ok(())
    }

    /// Event handerl
    pub async fn handle_mail_event(&self, evt: MailEvent) -> Result<()> {
        debug!("TODO: handle {:?}", evt);
        match evt {
            MailEvent::FolderList(acct, folders) => {
                let inner = self.inner.write().await;
                let acct_ref = match inner.as_ref().and_then(|inner| inner.accounts.get(&acct)) {
                    Some(inner) => inner.clone(),
                    None => return Ok(()),
                };
                mem::drop(inner);
                acct_ref.set_folders(folders).await;
            }
            _ => {}
        }
        Ok(())
    }

    /// Return a map of the accounts that are currently being tracked as well as a reference to the
    /// account handles themselves
    pub async fn list_accounts(&self) -> IndexMap<String, Arc<AccountRef>> {
        let read = self.inner.read().await;
        let inner = match read.as_ref() {
            Some(v) => v,
            None => return IndexMap::new(),
        };

        inner.accounts.clone()
    }
}

impl MailStoreInner {
    async fn init_with_config(config: Config) -> Result<Self> {
        let data_dir = config.data_dir.to_string_lossy();
        let data_dir = PathBuf::from(shellexpand::tilde(data_dir.as_ref()).as_ref());

        let mail_dir = data_dir.join("mail");
        if !mail_dir.exists() {
            fs::create_dir_all(&mail_dir).await?;
        }
        info!("using mail dir: {:?}", mail_dir);

        // create database parent
        let db_path = data_dir.join("panorama.db");
        let db_parent = db_path.parent();
        if let Some(path) = db_parent {
            fs::create_dir_all(path).await?;
        }

        let db_path_str = db_path.to_string_lossy();
        let db_path = format!("sqlite:{}", db_path_str);
        info!("using database path: {}", db_path_str);

        // create the database file if it doesn't already exist -_ -
        if !Sqlite::database_exists(&db_path_str).await? {
            Sqlite::create_database(&db_path_str).await?;
        }

        let pool = SqlitePool::connect(&db_path_str).await?;
        MIGRATOR.run(&pool).await?;
        debug!("run migrations : {:?}", MIGRATOR);

        let accounts = config
            .mail_accounts
            .keys()
            .map(|acct| {
                let folders = RwLock::new(Vec::new());
                (
                    acct.to_owned(),
                    Arc::new(AccountRef {
                        folders,
                        pool: pool.clone(),
                    }),
                )
            })
            .collect();

        Ok(MailStoreInner {
            mail_dir,
            pool,
            accounts,
        })
    }
}

#[derive(Debug)]
/// Holds a reference to an account
pub struct AccountRef {
    folders: RwLock<Vec<String>>,
    pool: SqlitePool,
}

impl AccountRef {
    /// Gets the folders on this account
    pub async fn get_folders(&self) -> Vec<String> {
        self.folders.read().await.clone()
    }

    /// Sets the folders on this account
    pub async fn set_folders(&self, folders: Vec<String>) {
        *self.folders.write().await = folders;
    }

    /// Gets the n latest messages in the given folder
    pub async fn get_newest_n_messages(
        &self,
        folder: impl AsRef<str>,
        n: usize,
    ) -> Result<Vec<EmailMetadata>> {
        let folder = folder.as_ref();
        let messages: Vec<EmailMetadata> = sqlx::query_as(
            r#"
            SELECT internaldate, subject FROM mail
            WHERE folder = ?
            ORDER BY internaldate DESC
        "#,
        )
        .bind(folder)
        .fetch(&self.pool)
        .map_ok(|(date, subject): (String, String)| EmailMetadata {
            date: Some(
                DateTime::parse_from_rfc3339(&date)
                    .unwrap()
                    .with_timezone(&Local),
            ),
            subject,
            ..EmailMetadata::default()
        })
        .try_collect()
        .await?;
        debug!("found {} messages", messages.len());
        Ok(messages)
    }
}

fn into_opt<T>(res: Result<T, SqlxError>) -> Result<Option<T>> {
    match res {
        Ok(v) => Ok(Some(v)),
        Err(SqlxError::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
