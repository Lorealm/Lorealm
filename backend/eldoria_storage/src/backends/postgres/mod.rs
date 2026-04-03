pub mod permissions;
pub mod sessions;
pub mod users;

use crate::StorageCore;
use crate::error::StorageError;
use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Clone)]
pub struct PostgresStorage {
    pub(crate) pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn connect(url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        tracing::info!("Running postgres migrations");
        sqlx::migrate!("src/migrations/postgres")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Database(format!("Migrations failed: {:?}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl StorageCore for PostgresStorage {
    async fn is_available(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        self.migrate().await
    }

    fn backend_type(&self) -> &str {
        "postgres"
    }
}
