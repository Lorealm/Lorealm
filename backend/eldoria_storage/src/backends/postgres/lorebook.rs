//! Postgres implementation of `LorebookStorage` and `LorebookSessionStorage`.
//!
//! Uses sqlx's runtime query API (not the compile-checked `query!` macros) so
//! it builds without a live `DATABASE_URL`. Swap to the macros later if you
//! want compile-time verification.
//!
//! Assumptions (see the chat review):
//!   - `book_version` is derived from the config YAML via `extract_book_version`.
//!   - `lorebook_versions` has a `snapshot JSONB` column holding the serialized
//!     aggregate; `lorebook_entries` and `lorebook_versions` cascade on
//!     `DELETE FROM lorebooks`.
//!   - `Snowflake` implements sqlx `Type`/`Encode`/`Decode` for Postgres BIGINT.
//!
//! Adjust the `use` paths to match your module layout.

use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{
    Lorebook, LorebookEntry, LorebookMetadata, LorebookSession, LorebookVersionInfo, Snowflake,
};
use crate::storage::{LorebookSessionStorage, LorebookStorage};

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
                // Pinned read: head reflects the requested version, not the entity head.
                book.current_version_id = version_id;
                Ok(Some(book))
            }
        }
    }

    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError> {
        // Heads only — entries are left empty to keep list payloads light.
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

        /* let next_num: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_num), 0) + 1 FROM lorebook_versions WHERE lorebook_id = $1",
        )
        .bind(lorebook.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?; */

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

        // Children are replaced wholesale — the aggregate is the source of truth.
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
        // TODO: Re-review and add a debounce-based system for saving lorebooks.
        // let snapshot = serde_json::to_value(lorebook).map_err(ser_err)?;
        // let book_version = extract_book_version(&lorebook.config_yaml);

        // sqlx::query(
        //     "INSERT INTO lorebook_versions
        //         (id, lorebook_id, version_num, book_version, snapshot, created_at)
        //      VALUES ($1, $2, $3, $4, $5, now())",
        // )
        // .bind(lorebook.current_version_id)
        // .bind(lorebook.id)
        // .bind(next_num as i32)
        // .bind(&book_version)
        // .bind(&snapshot)
        // .execute(&mut *tx)
        // .await
        // .map_err(db_err)?;

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

        // Re-save as a fresh version that mirrors the old snapshot.
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
        // Replace-the-whole-list, atomically: positions are rewritten 0..n so there
        // is no gap management and the writer fully owns ordering.
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

#[async_trait]
impl LorebookSessionStorage for PostgresStorage {
    async fn get_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<LorebookSession>, StorageError> {
        let session = sqlx::query_as::<_, LorebookSession>(
            "SELECT conversation_id, lorebook_id, activation_state, persistent_state, updated_at
             FROM lorebook_sessions WHERE conversation_id = $1",
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(session)
    }

    async fn save_lorebook_session(&self, session: &LorebookSession) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO lorebook_sessions
                (conversation_id, lorebook_id, activation_state, persistent_state, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (conversation_id) DO UPDATE SET
                lorebook_id      = EXCLUDED.lorebook_id,
                activation_state = EXCLUDED.activation_state,
                persistent_state = EXCLUDED.persistent_state,
                updated_at       = now()",
        )
        .bind(session.conversation_id)
        .bind(session.lorebook_id)
        .bind(&session.activation_state)
        .bind(&session.persistent_state)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_lorebook_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let res = sqlx::query("DELETE FROM lorebook_sessions WHERE conversation_id = $1")
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected() > 0)
    }
}
