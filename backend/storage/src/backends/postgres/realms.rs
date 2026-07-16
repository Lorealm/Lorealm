use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::{
    RealmStorage, StorageError,
    backends::PostgresStorage,
    models::{
        AgentObservation, Realm, RealmCast, RealmConfig, RealmDefinition, RealmEvent, RealmSummary,
        RealmVersionInfo, Snowflake, TurnCommit, VersionedSaveMode,
    },
};

#[async_trait]
impl RealmStorage for PostgresStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        let rows: Vec<RealmSummaryRow> = sqlx::query_as(
            r#"
            SELECT
                r.id,
                r.owner_id,
                r.updated_at,
                COALESCE(v.snapshot->>'name', '') AS name,
                ARRAY(
                    SELECT t.tag FROM realm_tags t
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

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        let head: Option<RealmHeadRow> = sqlx::query_as(
            r#"
            SELECT r.id, r.owner_id, r.current_version_id, v.snapshot
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
            SELECT v.id AS version_id, v.realm_id, v.snapshot, r.owner_id
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

        sqlx::query(
            r#"
            INSERT INTO realms (id, owner_id, current_version_id, created_at, updated_at)
            VALUES ($1, $2, $3, now(), now())
            ON CONFLICT (id) DO UPDATE SET
                current_version_id = EXCLUDED.current_version_id,
                updated_at = now()
            "#,
        )
        .bind(realm.id)
        .bind(realm.owner_id)
        .bind(realm.current_version_id)
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

    async fn list_realm_events(
        &self,
        conversation_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<RealmEvent>, StorageError> {
        let mut rows = sqlx::query_as::<_, RealmEvent>(
            "SELECT id, conversation_id, message_id, seq, source, scope, kind, content, payload
             FROM realm_events
             WHERE conversation_id = $1 AND ($2::BIGINT IS NULL OR id < $2)
             ORDER BY id DESC
             LIMIT $3",
        )
        .bind(conversation_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.reverse();
        Ok(rows)
    }

    async fn list_observations(
        &self,
        conversation_id: Snowflake,
        agent_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError> {
        let mut rows = sqlx::query_as::<_, AgentObservation>(
            "SELECT id, conversation_id, message_id, agent_id, event_id, role, content
             FROM agent_observations
             WHERE conversation_id = $1 AND agent_id = $2
               AND ($3::BIGINT IS NULL OR id < $3)
             ORDER BY id DESC
             LIMIT $4",
        )
        .bind(conversation_id)
        .bind(agent_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.reverse();
        Ok(rows)
    }

    async fn list_observations_for_messages(
        &self,
        agent_id: Snowflake,
        message_ids: &[Snowflake],
    ) -> Result<Vec<AgentObservation>, StorageError> {
        Ok(sqlx::query_as::<_, AgentObservation>(
            "SELECT id, conversation_id, message_id, agent_id, event_id, role, content
             FROM agent_observations
             WHERE agent_id = $1 AND message_id = ANY($2)
             ORDER BY id",
        )
        .bind(agent_id)
        .bind(message_ids)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn commit_turn(&self, commit: TurnCommit) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        for e in &commit.events {
            sqlx::query(
                "INSERT INTO realm_events
                     (id, conversation_id, message_id, seq, source, scope, kind, content, payload)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(e.id)
            .bind(e.conversation_id)
            .bind(e.message_id)
            .bind(e.seq)
            .bind(e.source)
            .bind(&e.scope)
            .bind(&e.kind)
            .bind(&e.content)
            .bind(&e.payload)
            .execute(&mut *tx)
            .await?;
        }

        for o in &commit.observations {
            sqlx::query(
                "INSERT INTO agent_observations
                     (id, conversation_id, message_id, agent_id, event_id, role, content)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(o.id)
            .bind(o.conversation_id)
            .bind(o.message_id)
            .bind(o.agent_id)
            .bind(o.event_id)
            .bind(&o.role)
            .bind(&o.content)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_realm_config(&self, realm_id: Snowflake) -> Result<RealmConfig, StorageError> {
        let row: Option<RealmConfigRow> = sqlx::query_as(
            "SELECT speech_format, action_format, tool_result_format,
                    state_change_format, state_write_format, state_delete_format
             FROM realm_config WHERE realm_id = $1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into).unwrap_or_default())
    }

    async fn put_realm_config(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        config: &RealmConfig,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO realm_config
                 (realm_id, user_id, speech_format, action_format, tool_result_format,
                  state_change_format, state_write_format, state_delete_format)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (realm_id) DO UPDATE SET
                 user_id = $2, speech_format = $3, action_format = $4,
                 tool_result_format = $5, state_change_format = $6,
                 state_write_format = $7, state_delete_format = $8",
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(&config.speech_format)
        .bind(&config.action_format)
        .bind(&config.tool_result_format)
        .bind(&config.state_change_format)
        .bind(&config.state_write_format)
        .bind(&config.state_delete_format)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(FromRow)]
struct RealmConfigRow {
    speech_format: Option<String>,
    action_format: Option<String>,
    tool_result_format: Option<String>,
    state_change_format: Option<String>,
    state_write_format: Option<String>,
    state_delete_format: Option<String>,
}

impl From<RealmConfigRow> for RealmConfig {
    fn from(r: RealmConfigRow) -> Self {
        RealmConfig {
            speech_format: r.speech_format,
            action_format: r.action_format,
            tool_result_format: r.tool_result_format,
            state_change_format: r.state_change_format,
            state_write_format: r.state_write_format,
            state_delete_format: r.state_delete_format,
        }
    }
}

#[derive(FromRow)]
struct RealmSummaryRow {
    id: Snowflake,
    owner_id: Snowflake,
    name: String,
    tags: Vec<String>,
    updated_at: DateTime<Utc>,
}

impl From<RealmSummaryRow> for RealmSummary {
    fn from(r: RealmSummaryRow) -> Self {
        RealmSummary {
            id: r.id,
            owner_id: r.owner_id,
            name: r.name,
            tags: r.tags,
            updated_at: r.updated_at,
        }
    }
}

#[derive(FromRow)]
struct RealmHeadRow {
    id: Snowflake,
    owner_id: Snowflake,
    current_version_id: Snowflake,
    snapshot: serde_json::Value,
}

impl RealmHeadRow {
    fn into_realm(self, tags: Vec<String>) -> Result<Realm, StorageError> {
        let definition: RealmDefinition = serde_json::from_value(self.snapshot)?;
        Ok(Realm {
            id: self.id,
            owner_id: self.owner_id,
            current_version_id: self.current_version_id,
            definition,
            tags,
        })
    }
}

#[derive(FromRow)]
struct RealmVersionRow {
    version_id: Snowflake,
    realm_id: Snowflake,
    owner_id: Snowflake,
    snapshot: serde_json::Value,
}

impl RealmVersionRow {
    fn into_realm(self, tags: Vec<String>) -> Result<Realm, StorageError> {
        let definition: RealmDefinition = serde_json::from_value(self.snapshot)?;
        Ok(Realm {
            id: self.realm_id,
            owner_id: self.owner_id,
            current_version_id: self.version_id,
            definition,
            tags,
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

async fn fetch_tags(pool: &sqlx::PgPool, realm_id: Snowflake) -> Result<Vec<String>, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT tag FROM realm_tags WHERE realm_id = $1")
            .bind(realm_id)
            .fetch_all(pool)
            .await?,
    )
}
