//! Package for managing the offline storage of emails

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

/// SQLite email manager
#[derive(Clone)]
pub struct MailStore {
    pool: SqlitePool,
}

impl MailStore {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePool::connect("hellosu.db").await?;

        let run = tokio::spawn(listen_loop(pool.clone()));

        Ok(MailStore { pool })
    }
}

async fn listen_loop(pool: SqlitePool) {
}
