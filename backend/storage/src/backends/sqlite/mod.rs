pub mod blueprints;
pub mod characters;
pub mod connection;
pub mod conversations;
pub mod lorebook;
pub mod permissions;
pub mod persona;
pub mod plugins;
pub mod realms;
pub mod sessions;
pub mod state;
pub mod templates;
pub mod tokenizer;
pub mod tools;
pub mod users;
pub mod widgets;

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::StorageCore;
use crate::error::StorageError;
use async_trait::async_trait;
use sqlx::SqlitePool;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};

/// SQLite has no `now()`. This expression yields the same RFC3339 form sqlx
/// writes when it encodes a `DateTime<Utc>`.
pub(crate) const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%f+00:00','now')";

#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pub(crate) pool: SqlitePool,
}

impl SqliteStorage {
    /// Open (or create) the database at `path`.
    ///
    /// `max_connections` is the pool ceiling. SQLite serialises writers
    /// regardless, so the pool mainly buys concurrent reads; `busy_timeout`
    /// makes a blocked writer wait rather than fail outright.
    pub async fn new(
        path: impl AsRef<Path>,
        max_connections: u32,
        auto_create: bool,
    ) -> Result<Self, StorageError> {
        let path = path.as_ref();

        if auto_create {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(auto_create)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Open an in-memory database. Used by tests; the pool is capped at one
    /// connection because each new connection would otherwise get its own
    /// empty database.
    pub async fn in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from("data").join("lorealm.db")
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        tracing::info!("Running sqlite migrations");
        sqlx::migrate!("src/migrations/sqlite")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Migrations failed: {:?}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl StorageCore for SqliteStorage {
    async fn is_available(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        self.migrate().await
    }

    fn backend_type(&self) -> &str {
        "sqlite"
    }
}
