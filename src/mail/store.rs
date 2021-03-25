//! Package for managing the offline storage of emails

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use panorama_imap::response::AttributeValue;
use sha2::{Digest, Sha256};
use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    sqlite::{Sqlite, SqlitePool},
    Error as SqlxError, Row,
};
use tokio::{fs, sync::broadcast};

use crate::config::Config;

static MIGRATOR: Migrator = sqlx::migrate!();

/// Manages email storage on disk, for both database and caches
///
/// This struct is clone-safe: cloning it will just return a reference to the same data structure
#[derive(Clone)]
pub struct MailStore {
    config: Config,
    mail_dir: PathBuf,
    pool: SqlitePool,
    // email_events: broadcast::Sender<EmailUpdateInfo>,
}

#[derive(Clone, Debug)]
pub struct EmailUpdateInfo {}

impl MailStore {
    /// Creates a new MailStore
    pub async fn new(config: Config) -> Result<Self> {
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

        // let (new_email_tx, new_email_rx) = broadcast::channel(100);

        Ok(MailStore {
            config,
            mail_dir,
            pool,
            // email_events: new_email_tx,
        })
    }

    // /// Subscribes to the email updates
    // pub fn subscribe(&self) -> broadcast::Receiver<EmailUpdateInfo> {
    //     self.email_events.subscribe()
    // }

    /// Given a UID and optional message-id try to identify a particular message
    pub async fn try_identify_email(
        &self,
        acct: impl AsRef<str>,
        folder: impl AsRef<str>,
        uid: u32,
        uidvalidity: u32,
        message_id: Option<&str>,
    ) -> Result<Option<u32>> {
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
            .fetch_one(&self.pool)
            .await,
        )?;

        if let Some(existing) = existing {
            let rowid = existing.0;
            debug!(
                "folder: {:?} uid: {:?} rowid: {:?}",
                folder.as_ref(),
                uid,
                rowid,
            );
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
        for attr in attrs {
            if let AttributeValue::BodySection(body_attr) = attr {
                body = body_attr.data;
            }
        }

        let body = match body {
            Some(v) => v,
            None => return Ok(()),
        };

        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        let hash = hasher.finalize();
        let filename = format!("{}.mail", hex::encode(hash));
        let path = self.mail_dir.join(&filename);
        fs::write(path, &body)
            .await
            .context("error writing email to file")?;

        // parse email
        let mut message_id = None;
        let mail = mailparse::parse_mail(body.as_bytes())
            .with_context(|| format!("error parsing email with uid {}", uid))?;
        for header in mail.headers.iter() {
            let key = header.get_key_ref();
            let key = key.to_ascii_lowercase();
            let value = header.get_value();
            if key == "message-id" {
                message_id = Some(value);
            }
        }

        debug!("message-id: {:?}", message_id);

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
            .fetch_one(&self.pool)
            .await,
        )?;

        if existing.is_none() {
            let id = sqlx::query(
                r#"
                INSERT INTO "mail" (account, message_id, folder, uid, uidvalidity, filename)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(acct.as_ref())
            .bind(message_id)
            .bind(folder.as_ref())
            .bind(uid)
            .bind(uidvalidity)
            .bind(filename)
            .execute(&self.pool)
            .await
            .context("error inserting email into db")?
            .last_insert_rowid();
        }

        // self.email_events
        //     .send(EmailUpdateInfo {})
        //     .context("error sending email update info to the broadcast channel")?;

        Ok(())
    }
}

fn into_opt<T>(res: Result<T, SqlxError>) -> Result<Option<T>> {
    match res {
        Ok(v) => Ok(Some(v)),
        Err(SqlxError::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
