use async_trait::async_trait;

use crate::{
    StorageError, TokenizerStorage,
    backends::PostgresStorage,
    models::{Snowflake, Tokenizer},
};

#[async_trait]
impl TokenizerStorage for PostgresStorage {
    async fn get_tokenizer(&self, id: Snowflake) -> Result<Option<Tokenizer>, StorageError> {
        let tokenizer = sqlx::query_as::<_, Tokenizer>(
            "SELECT id, name, blob_uri, owner_id, is_public
             FROM tokenizers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tokenizer)
    }

    async fn list_tokenizers_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        let tokenizers = sqlx::query_as::<_, Tokenizer>(
            "SELECT id, name, blob_uri, owner_id, is_public
             FROM tokenizers WHERE owner_id = $1 ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokenizers)
    }

    async fn list_accessible_tokenizers(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, StorageError> {
        // The (tok_id, user_id) primary key on tokenizer_shares means the
        // LEFT JOIN matches at most one row per tokenizer, so no fan-out.
        let tokenizers = sqlx::query_as::<_, Tokenizer>(
            "SELECT t.id, t.name, t.blob_uri, t.owner_id, t.is_public
             FROM tokenizers t
             LEFT JOIN tokenizer_shares s ON s.tok_id = t.id AND s.user_id = $1
             WHERE t.owner_id = $1 OR t.is_public OR s.user_id IS NOT NULL
             ORDER BY t.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokenizers)
    }

    async fn save_tokenizer(&self, tokenizer: &Tokenizer) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO tokenizers (id, name, blob_uri, owner_id, is_public)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (id) DO UPDATE SET
                name      = EXCLUDED.name,
                blob_uri  = EXCLUDED.blob_uri,
                owner_id  = EXCLUDED.owner_id,
                is_public = EXCLUDED.is_public",
        )
        .bind(tokenizer.id)
        .bind(&tokenizer.name)
        .bind(&tokenizer.blob_uri)
        .bind(tokenizer.owner_id)
        .bind(tokenizer.is_public)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_tokenizer(&self, id: Snowflake) -> Result<bool, StorageError> {
        // Relies on ON DELETE CASCADE for share rows.
        let res = sqlx::query("DELETE FROM tokenizers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn can_access_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let accessible: Option<bool> = sqlx::query_scalar(
            "SELECT true
             FROM tokenizers t
             LEFT JOIN tokenizer_shares s ON s.tok_id = t.id AND s.user_id = $2
             WHERE t.id = $1
               AND (t.owner_id = $2 OR t.is_public OR s.user_id IS NOT NULL)",
        )
        .bind(tok_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(accessible.unwrap_or(false))
    }

    async fn share_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO tokenizer_shares (tok_id, user_id)
             VALUES ($1, $2)
             ON CONFLICT (tok_id, user_id) DO NOTHING",
        )
        .bind(tok_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn unshare_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<bool, StorageError> {
        let res = sqlx::query("DELETE FROM tokenizer_shares WHERE tok_id = $1 AND user_id = $2")
            .bind(tok_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_tokenizer_shares(
        &self,
        tok_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let user_ids: Vec<Snowflake> = sqlx::query_scalar(
            "SELECT user_id FROM tokenizer_shares WHERE tok_id = $1 ORDER BY user_id",
        )
        .bind(tok_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(user_ids)
    }

    async fn get_active_tokenizer(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let id: Option<Snowflake> =
            sqlx::query_scalar("SELECT active_tokenizer_id FROM user_settings WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(id)
    }
    async fn set_active_tokenizer(
        &self,
        user_id: Snowflake,
        tokenizer_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO user_settings (user_id, active_tokenizer_id) VALUES ($1, $2)
            ON CONFLICT (user_id)
                DO UPDATE SET
                active_tokenizer_id = EXCLUDED.active_tokenizer_id
            "#,
        )
        .bind(user_id)
        .bind(tokenizer_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
