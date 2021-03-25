//! Package for managing the offline storage of emails

use std::path::{PathBuf, Path};

use anyhow::Result;
use panorama_imap::response::AttributeValue;
use sha2::{Digest, Sha256};
use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    sqlite::{Sqlite, SqlitePool},
    Error,
};
use tokio::fs;

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
}

impl MailStore {
    /// Creates a new MailStore
    pub async fn new(config: Config) -> Result<Self> {
        let mail_dir = config.mail_dir.to_string_lossy();
        let mail_dir_str = shellexpand::tilde(mail_dir.as_ref());
        let mail_dir = PathBuf::from(mail_dir_str.as_ref());
        if !mail_dir.exists() {
            fs::create_dir_all(&mail_dir).await?;
        }
        info!("using mail dir: {:?}", mail_dir);

        // create database parent
        let db_path = config.db_path.to_string_lossy();
        let db_path_str = shellexpand::tilde(db_path.as_ref());

        let db_path = PathBuf::from(db_path_str.as_ref());
        let db_parent = db_path.parent();
        if let Some(path) = db_parent {
            fs::create_dir_all(path).await?;
        }

        let db_path = format!("sqlite:{}", db_path_str);
        info!("using database path: {}", db_path_str);

        // create the database file if it doesn't already exist -_ -
        if !Sqlite::database_exists(&db_path_str).await? {
            Sqlite::create_database(&db_path_str).await?;
        }

        let pool = SqlitePool::connect(&db_path_str).await?;
        MIGRATOR.run(&pool).await?;
        debug!("run migrations : {:?}", MIGRATOR);

        Ok(MailStore { config, mail_dir, pool })
    }

    /// Gets the list of all the UIDs in the given folder that need to be updated
    pub fn get_new_uids(&self, exists: u32) {}

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
        fs::write(path, body).await?;

        let existing = sqlx::query(
            r#"
            SELECT FROM "mail"
            WHERE account = ? AND folder = ?
                AND uid = ? AND uidvalidity = ?
            "#,
        )
        .bind(acct.as_ref())
        .bind(folder.as_ref())
        .bind(uid)
        .bind(uidvalidity)
        .fetch_one(&self.pool)
        .await;

        let exists = match existing {
            Ok(_) => true,
            Err(Error::RowNotFound) => true,
            _ => false,
        };

        if !exists {
            let id = sqlx::query(
                r#"
                INSERT INTO "mail" (account, folder, uid, uidvalidity, filename)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(acct.as_ref())
            .bind(folder.as_ref())
            .bind(uid)
            .bind(uidvalidity)
            .bind(filename)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
        }

        Ok(())
    }
}
