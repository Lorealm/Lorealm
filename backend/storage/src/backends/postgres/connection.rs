use async_trait::async_trait;

use crate::{
    ConnectionStorage, SecretStorage, StorageError,
    backends::PostgresStorage,
    models::{ConnectionProfile, Secret, SecretMetadata, Snowflake},
};

#[async_trait]
impl SecretStorage for PostgresStorage {
    async fn get_secret(&self, id: Snowflake) -> Result<Option<Secret>, StorageError> {
        let secret = sqlx::query_as::<_, Secret>(
            "SELECT id, user_id, provider, label, key_hint, enc_version, context,
                    salt, nonce, ciphertext, tag, created_at, updated_at
             FROM secrets WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(secret)
    }

    async fn list_secrets_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<SecretMetadata>, StorageError> {
        let secrets = sqlx::query_as::<_, SecretMetadata>(
            "SELECT id, user_id, provider, label, key_hint, created_at, updated_at
             FROM secrets WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(secrets)
    }

    async fn save_secret(&self, secret: &Secret) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO secrets
                (id, user_id, provider, label, key_hint, enc_version, context,
                 salt, nonce, ciphertext, tag, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now())
             ON CONFLICT (id) DO UPDATE SET
                label       = EXCLUDED.label,
                key_hint    = EXCLUDED.key_hint,
                enc_version = EXCLUDED.enc_version,
                context     = EXCLUDED.context,
                salt        = EXCLUDED.salt,
                nonce       = EXCLUDED.nonce,
                ciphertext  = EXCLUDED.ciphertext,
                tag         = EXCLUDED.tag,
                updated_at  = now()",
        )
        .bind(secret.id)
        .bind(secret.user_id)
        .bind(&secret.provider)
        .bind(&secret.label)
        .bind(&secret.key_hint)
        .bind(secret.enc_version)
        .bind(&secret.context)
        .bind(&secret.salt)
        .bind(&secret.nonce)
        .bind(&secret.ciphertext)
        .bind(&secret.tag)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_secret(&self, id: Snowflake) -> Result<bool, StorageError> {
        // Profiles referencing this secret are detached by the foreign
        // key (ON DELETE SET NULL).
        let res = sqlx::query("DELETE FROM secrets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

#[async_trait]
impl ConnectionStorage for PostgresStorage {
    async fn get_profile(&self, id: Snowflake) -> Result<Option<ConnectionProfile>, StorageError> {
        let profile = sqlx::query_as::<_, ConnectionProfile>(
            "SELECT id, user_id, secret_id, name, provider, model, token_budget,
                    common_params, provider_params, created_at, updated_at
             FROM connection_profiles WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    async fn list_profiles_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConnectionProfile>, StorageError> {
        let profiles = sqlx::query_as::<_, ConnectionProfile>(
            "SELECT id, user_id, secret_id, name, provider, model, token_budget,
                    common_params, provider_params, created_at, updated_at
             FROM connection_profiles WHERE user_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    async fn save_profile(&self, profile: &ConnectionProfile) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO connection_profiles
                (id, user_id, secret_id, name, provider, model, token_budget,
                 common_params, provider_params, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now(), now())
             ON CONFLICT (id) DO UPDATE SET
                secret_id       = EXCLUDED.secret_id,
                name            = EXCLUDED.name,
                provider        = EXCLUDED.provider,
                model           = EXCLUDED.model,
                token_budget    = EXCLUDED.token_budget,
                common_params   = EXCLUDED.common_params,
                provider_params = EXCLUDED.provider_params,
                updated_at      = now()",
        )
        .bind(profile.id)
        .bind(profile.user_id)
        .bind(profile.secret_id)
        .bind(&profile.name)
        .bind(&profile.provider)
        .bind(&profile.model)
        .bind(profile.token_budget)
        .bind(&profile.common_params)
        .bind(&profile.provider_params)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_profile(&self, id: Snowflake) -> Result<bool, StorageError> {
        let res = sqlx::query("DELETE FROM connection_profiles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn profiles_using_secret(
        &self,
        secret_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let ids: Vec<Snowflake> = sqlx::query_scalar(
            "SELECT id FROM connection_profiles WHERE secret_id = $1 ORDER BY updated_at DESC",
        )
        .bind(secret_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ids)
    }

    async fn get_active_profile(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let id: Option<Snowflake> = sqlx::query_scalar(
            "SELECT active_connection_profile_id FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    async fn set_active_profile(
        &self,
        user_id: Snowflake,
        profile_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO user_settings (user_id, active_connection_profile_id) VALUES ($1, $2)
            ON CONFLICT (user_id)
                DO UPDATE SET
                active_connection_profile_id = EXCLUDED.active_connection_profile_id
            "#,
        )
        .bind(user_id)
        .bind(profile_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
