//! Package for managing the offline storage of emails

use std::path::PathBuf;

use anyhow::Result;
use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    sqlite::{Sqlite, SqlitePool},
};
use tokio::fs;

static MIGRATOR: Migrator = sqlx::migrate!();

/// SQLite email manager
///
/// This struct is clone-safe: cloning it will just return a reference to the same data structure
#[derive(Clone)]
pub struct MailStore {
    mail_dir: PathBuf,
    pool: SqlitePool,
}

impl MailStore {
    /// Creates a new MailStore
    pub async fn new() -> Result<Self> {
        let db_path = "sqlite:hellosu.db";

        // create the database file if it doesn't already exist -_ -
        if !Sqlite::database_exists(db_path).await? {
            Sqlite::create_database(db_path).await?;
        }

        let pool = SqlitePool::connect(db_path).await?;
        MIGRATOR.run(&pool).await?;
        debug!("run migrations : {:?}", MIGRATOR);

        let mail_dir = PathBuf::from("hellosu/");
        if !mail_dir.exists() {
            fs::create_dir_all(&mail_dir).await?;
        }

        Ok(MailStore { mail_dir, pool })
    }

    /// Gets the list of all the UIDs in the given folder that need to be updated
    pub fn get_new_uids(&self, exists: u32) {

    }
}
