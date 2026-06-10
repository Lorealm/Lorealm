use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Lorebook, LorebookEntry, LorebookMetadata, LorebookVersionInfo, Snowflake};
use crate::storage::LorebookStorage;

use super::PostgresStorage;

fn db_err(e: sqlx::Error) -> StorageError {
    StorageError::Database(e.to_string())
}

fn ser_err(e: serde_json::Error) -> StorageError {
    StorageError::InvalidInput(e.to_string())
}

#[async_trait]
impl LorebookStorage for PostgresStorage {
    async fn get_lorebook(&self, id: Snowflake) -> Result<Option<Lorebook>, StorageError> {
        let mut book = match sqlx::query_as::<_, Lorebook>(
            "SELECT id, owner_id, current_version_id, name, description, config_yaml,
                    schema_version, created_at, updated_at
             FROM lorebooks WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        {
            Some(b) => b,
            None => return Ok(None),
        };

        book.entries = sqlx::query_as::<_, LorebookEntry>(
            "SELECT id, lorebook_id, entry_id, source
             FROM lorebook_entries WHERE lorebook_id = $1 ORDER BY entry_id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(Some(book))
    }

    async fn get_lorebook_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Lorebook>, StorageError> {
        let snapshot: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT snapshot FROM lorebook_versions WHERE lorebook_id = $1 AND id = $2",
        )
        .bind(id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        match snapshot {
            None => Ok(None),
            Some(v) => {
                let mut book: Lorebook = serde_json::from_value(v).map_err(ser_err)?;
                book.current_version_id = version_id;
                Ok(Some(book))
            }
        }
    }

    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError> {
        let books = sqlx::query_as::<_, Lorebook>(
            "SELECT id, owner_id, current_version_id, name, description, config_yaml,
                    schema_version, created_at, updated_at
             FROM lorebooks WHERE owner_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(books.iter().map(LorebookMetadata::from).collect())
    }

    async fn save_lorebook(&self, lorebook: &Lorebook) -> Result<Snowflake, StorageError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(
            "INSERT INTO lorebooks
                (id, owner_id, current_version_id, name, description, config_yaml,
                 schema_version, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
             ON CONFLICT (id) DO UPDATE SET
                owner_id           = EXCLUDED.owner_id,
                current_version_id = EXCLUDED.current_version_id,
                name               = EXCLUDED.name,
                description        = EXCLUDED.description,
                config_yaml        = EXCLUDED.config_yaml,
                schema_version     = EXCLUDED.schema_version,
                updated_at         = now()",
        )
        .bind(lorebook.id)
        .bind(lorebook.owner_id)
        .bind(lorebook.current_version_id)
        .bind(&lorebook.name)
        .bind(&lorebook.description)
        .bind(&lorebook.config_yaml)
        .bind(lorebook.schema_version)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        sqlx::query("DELETE FROM lorebook_entries WHERE lorebook_id = $1")
            .bind(lorebook.id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for entry in &lorebook.entries {
            sqlx::query(
                "INSERT INTO lorebook_entries (id, lorebook_id, entry_id, source)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(entry.id)
            .bind(lorebook.id)
            .bind(&entry.entry_id)
            .bind(&entry.source)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(lorebook.current_version_id)
    }

    async fn delete_lorebook(&self, id: Snowflake) -> Result<bool, StorageError> {
        // Relies on ON DELETE CASCADE for entries, versions, and sessions.
        let res = sqlx::query("DELETE FROM lorebooks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_lorebook_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<LorebookVersionInfo>, StorageError> {
        let versions = sqlx::query_as::<_, LorebookVersionInfo>(
            "SELECT id, version_num, book_version, created_at
             FROM lorebook_versions WHERE lorebook_id = $1 ORDER BY version_num DESC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(versions)
    }

    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let mut restored = self
            .get_lorebook_at_version(id, version_id)
            .await?
            .ok_or_else(|| {
                StorageError::InvalidInput(format!(
                    "lorebook {id} has no version {version_id} to restore"
                ))
            })?;

        restored.current_version_id = new_version_id;
        self.save_lorebook(&restored).await
    }

    async fn list_enabled_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let ids: Vec<Snowflake> = sqlx::query_scalar(
            "SELECT lorebook_id FROM enabled_lorebooks WHERE user_id = $1 ORDER BY position",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(ids)
    }

    async fn set_enabled_lorebooks(
        &self,
        user_id: Snowflake,
        ordered: &[Snowflake],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query("DELETE FROM enabled_lorebooks WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for (position, lorebook_id) in ordered.iter().enumerate() {
            sqlx::query(
                "INSERT INTO enabled_lorebooks (user_id, lorebook_id, position) \
                 VALUES ($1, $2, $3)",
            )
            .bind(user_id)
            .bind(*lorebook_id)
            .bind(position as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
}
