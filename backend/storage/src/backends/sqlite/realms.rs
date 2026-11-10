use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

use super::NOW;
use crate::{
    RealmStorage, StorageError,
    backends::SqliteStorage,
    models::{
        Realm, RealmCast, RealmDefinition, RealmSummary, RealmVersionInfo, Snowflake,
        VersionedSaveMode,
    },
};

#[async_trait]
impl RealmStorage for SqliteStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        let rows: Vec<RealmSummaryRow> = sqlx::query_as(
            r#"
            SELECT
                r.id,
                r.owner_id,
                r.origin,
                r.updated_at,
                COALESCE(v.snapshot->>'name', '') AS name,
                COALESCE(v.snapshot->>'description', '') AS description,
                -- SQLite has no ARRAY(); json_group_array yields a JSON array,
                -- decoded below as Json<Vec<String>>. Like ARRAY() it returns an
                -- empty array rather than NULL when the realm has no tags.
                (
                    SELECT json_group_array(t.tag) FROM realm_tags t
                    WHERE t.realm_id = r.id
                ) AS tags
            FROM realms r
            JOIN realm_versions v ON v.id = r.current_version_id
            WHERE r.owner_id = $1
            ORDER BY r.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(RealmSummaryRow::into_summary)
            .collect()
    }

    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        let head: Option<RealmHeadRow> = sqlx::query_as(
            r#"
            SELECT r.id, r.owner_id, r.current_version_id, r.origin, v.snapshot
            FROM realms r
            JOIN realm_versions v ON v.id = r.current_version_id
            WHERE r.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = head else { return Ok(None) };

        let tags = fetch_tags(&self.pool, id).await?;
        Ok(Some(row.into_realm(tags)?))
    }

    async fn get_realm_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Realm>, StorageError> {
        let row: Option<RealmVersionRow> = sqlx::query_as(
            r#"
            SELECT v.id AS version_id, v.realm_id, v.snapshot, r.owner_id, r.origin
            FROM realm_versions v
            JOIN realms r ON r.id = v.realm_id
            WHERE v.realm_id = $1 AND v.id = $2
            "#,
        )
        .bind(id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else { return Ok(None) };

        let tags = fetch_tags(&self.pool, id).await?;
        Ok(Some(row.into_realm(tags)?))
    }

    async fn save_realm(
        &self,
        realm: &Realm,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        let mut tx = self.pool.begin().await?;

        let snapshot = serde_json::to_value(&realm.definition)?;

        sqlx::query(&format!(
            r#"
            INSERT INTO realms (id, owner_id, current_version_id, origin, created_at, updated_at)
            VALUES ($1, $2, $3, $4, {NOW}, {NOW})
            ON CONFLICT (id) DO UPDATE SET
                current_version_id = EXCLUDED.current_version_id,
                origin = EXCLUDED.origin,
                updated_at = {NOW}
            "#
        ))
        .bind(realm.id)
        .bind(realm.owner_id)
        .bind(realm.current_version_id)
        .bind(
            realm
                .origin
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
        )
        .execute(&mut *tx)
        .await?;

        match mode {
            VersionedSaveMode::NewVersion => {
                let next_version_num: i32 = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(MAX(version_num), 0) + 1
                    FROM realm_versions
                    WHERE realm_id = $1
                    "#,
                )
                .bind(realm.id)
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query(
                    r#"
                    INSERT INTO realm_versions (id, realm_id, version_num, snapshot)
                    VALUES ($1, $2, $3, $4)
                    "#,
                )
                .bind(realm.current_version_id)
                .bind(realm.id)
                .bind(next_version_num)
                .bind(&snapshot)
                .execute(&mut *tx)
                .await?;
            }
            VersionedSaveMode::UpdateCurrent => {
                let affected = sqlx::query(
                    r#"
                    UPDATE realm_versions SET snapshot = $2
                    WHERE id = $1
                    "#,
                )
                .bind(realm.current_version_id)
                .bind(&snapshot)
                .execute(&mut *tx)
                .await?
                .rows_affected();

                if affected == 0 {
                    return Err(StorageError::NotFound(format!(
                        "save_realm: version '{}' not found for in-place update",
                        realm.current_version_id
                    )));
                }
            }
        }

        sqlx::query("DELETE FROM realm_tags WHERE realm_id = $1")
            .bind(realm.id)
            .execute(&mut *tx)
            .await?;

        for tag in &realm.tags {
            sqlx::query("INSERT INTO realm_tags (realm_id, tag) VALUES ($1, $2)")
                .bind(realm.id)
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(realm.current_version_id)
    }

    async fn delete_realm(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM realms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_realm_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, StorageError> {
        let rows: Vec<VersionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, version_num, created_at
            FROM realm_versions
            WHERE realm_id = $1
            ORDER BY version_num DESC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn restore_realm_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let Some(realm) = self.get_realm_at_version(id, version_id).await? else {
            return Err(StorageError::NotFound(format!(
                "realm {id} version {version_id} not found"
            )));
        };

        let mut to_save = realm;
        to_save.current_version_id = new_version_id;
        self.save_realm(&to_save, VersionedSaveMode::NewVersion)
            .await
    }

    async fn get_cast(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<RealmCast>, StorageError> {
        let row: Option<sqlx::types::Json<RealmCast>> = sqlx::query_scalar(
            "SELECT snapshot FROM conversation_casts WHERE conversation_id = $1",
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|j| j.0))
    }

    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO conversation_casts (conversation_id, snapshot)
             VALUES ($1, $2)
             ON CONFLICT (conversation_id) DO UPDATE SET snapshot = $2",
        )
        .bind(conversation_id)
        .bind(sqlx::types::Json(cast))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(FromRow)]
struct RealmSummaryRow {
    id: Snowflake,
    owner_id: Snowflake,
    name: String,
    description: String,
    /// `json_group_array` output; see the query in `list_realms`.
    tags: sqlx::types::Json<Vec<String>>,
    origin: Option<serde_json::Value>,
    updated_at: DateTime<Utc>,
}

impl RealmSummaryRow {
    fn into_summary(self) -> Result<RealmSummary, StorageError> {
        Ok(RealmSummary {
            id: self.id,
            owner_id: self.owner_id,
            name: self.name,
            description: self.description,
            tags: self.tags.0,
            origin: self.origin.map(serde_json::from_value).transpose()?,
            updated_at: self.updated_at,
        })
    }
}

#[derive(FromRow)]
struct RealmHeadRow {
    id: Snowflake,
    owner_id: Snowflake,
    current_version_id: Snowflake,
    origin: Option<serde_json::Value>,
    snapshot: serde_json::Value,
}

impl RealmHeadRow {
    fn into_realm(self, tags: Vec<String>) -> Result<Realm, StorageError> {
        let definition: RealmDefinition = serde_json::from_value(self.snapshot)?;
        let origin = self.origin.map(serde_json::from_value).transpose()?;
        Ok(Realm {
            id: self.id,
            owner_id: self.owner_id,
            current_version_id: self.current_version_id,
            definition,
            tags,
            origin,
        })
    }
}

#[derive(FromRow)]
struct RealmVersionRow {
    version_id: Snowflake,
    realm_id: Snowflake,
    owner_id: Snowflake,
    snapshot: serde_json::Value,
    origin: Option<serde_json::Value>,
}

impl RealmVersionRow {
    fn into_realm(self, tags: Vec<String>) -> Result<Realm, StorageError> {
        let definition: RealmDefinition = serde_json::from_value(self.snapshot)?;
        let origin = self.origin.map(serde_json::from_value).transpose()?;
        Ok(Realm {
            id: self.realm_id,
            owner_id: self.owner_id,
            current_version_id: self.version_id,
            definition,
            tags,
            origin,
        })
    }
}

#[derive(FromRow)]
struct VersionInfoRow {
    id: Snowflake,
    version_num: i32,
    created_at: DateTime<Utc>,
}

impl From<VersionInfoRow> for RealmVersionInfo {
    fn from(row: VersionInfoRow) -> Self {
        RealmVersionInfo {
            id: row.id,
            version_num: row.version_num as u32,
            created_at: row.created_at,
        }
    }
}

async fn fetch_tags(
    pool: &sqlx::SqlitePool,
    realm_id: Snowflake,
) -> Result<Vec<String>, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT tag FROM realm_tags WHERE realm_id = $1")
            .bind(realm_id)
            .fetch_all(pool)
            .await?,
    )
}
