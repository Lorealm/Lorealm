use async_trait::async_trait;

use crate::{
    RealmStorage, StorageError,
    backends::PostgresStorage,
    models::{
        AgentGrant, AgentObservation, AgentToolBinding, Realm, RealmAgent, RealmCast, RealmEvent,
        RealmSummary, Snowflake, TurnCommit,
    },
};

#[async_trait]
impl RealmStorage for PostgresStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        let summaries = sqlx::query_as::<_, RealmSummary>(
            r#"
            SELECT id, owner_id, name, status
            FROM realms WHERE owner_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(summaries)
    }

    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        Ok(sqlx::query_as::<_, Realm>(
            "SELECT id, owner_id, name, tokenizer_id, status
             FROM realms WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO realms (id, owner_id, name, tokenizer_id, status)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(realm.id)
        .bind(realm.owner_id)
        .bind(&realm.name)
        .bind(realm.tokenizer_id)
        .bind(&realm.status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE realms
             SET name = $2, tokenizer_id = $3, status = $4, updated_at = now()
             WHERE id = $1",
        )
        .bind(realm.id)
        .bind(&realm.name)
        .bind(realm.tokenizer_id)
        .bind(&realm.status)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("realm: {}", realm.id)));
        }
        Ok(())
    }

    async fn delete_realm(&self, id: Snowflake) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM realms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_realm_agents(
        &self,
        realm_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, StorageError> {
        Ok(sqlx::query_as::<_, RealmAgent>(
            "SELECT id, realm_id, name, kind, character_id, char_version_id,
                    template_id, profile_id, user_id, persona_id, turn_order,
                    extensions, created_at, updated_at
             FROM realm_agents WHERE realm_id = $1
             ORDER BY turn_order, id",
        )
        .bind(realm_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn get_realm_agent(
        &self,
        agent_id: Snowflake,
    ) -> Result<Option<RealmAgent>, StorageError> {
        Ok(sqlx::query_as::<_, RealmAgent>(
            "SELECT id, realm_id, name, kind, character_id, char_version_id,
                    template_id, profile_id, user_id, persona_id, turn_order,
                    extensions, created_at, updated_at
             FROM realm_agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn create_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO realm_agents (id, realm_id, name, kind, character_id,
                 char_version_id, template_id, profile_id, user_id, persona_id,
                 turn_order, extensions)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(agent.id)
        .bind(agent.realm_id)
        .bind(&agent.name)
        .bind(&agent.kind)
        .bind(agent.character_id)
        .bind(agent.char_version_id)
        .bind(agent.template_id)
        .bind(agent.profile_id)
        .bind(agent.user_id)
        .bind(agent.persona_id)
        .bind(agent.turn_order)
        .bind(&agent.extensions)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE realm_agents
             SET name = $2, kind = $3, character_id = $4, char_version_id = $5,
                 template_id = $6, profile_id = $7, user_id = $8, persona_id = $9,
                 turn_order = $10, extensions = $11, updated_at = now()
             WHERE id = $1",
        )
        .bind(agent.id)
        .bind(&agent.name)
        .bind(&agent.kind)
        .bind(agent.character_id)
        .bind(agent.char_version_id)
        .bind(agent.template_id)
        .bind(agent.profile_id)
        .bind(agent.user_id)
        .bind(agent.persona_id)
        .bind(agent.turn_order)
        .bind(&agent.extensions)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("realm agent: {}", agent.id)));
        }
        Ok(())
    }

    async fn delete_realm_agent(&self, agent_id: Snowflake) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM realm_agents WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_agent_grants(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, StorageError> {
        Ok(sqlx::query_as::<_, AgentGrant>(
            "SELECT agent_id, scope, access
             FROM agent_grants WHERE agent_id = $1 ORDER BY scope",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn set_agent_grants(
        &self,
        agent_id: Snowflake,
        grants: &[AgentGrant],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM agent_grants WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;
        for g in grants {
            sqlx::query("INSERT INTO agent_grants (agent_id, scope, access) VALUES ($1, $2, $3)")
                .bind(agent_id)
                .bind(&g.scope)
                .bind(&g.access)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_agent_tools(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentToolBinding>, StorageError> {
        Ok(sqlx::query_as::<_, AgentToolBinding>(
            "SELECT tool_id, event_scope FROM agent_tools WHERE agent_id = $1 ORDER BY tool_id",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn set_agent_tools(
        &self,
        agent_id: Snowflake,
        bindings: &[AgentToolBinding],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM agent_tools WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;
        for b in bindings {
            sqlx::query(
                "INSERT INTO agent_tools (agent_id, tool_id, event_scope) VALUES ($1, $2, $3)",
            )
            .bind(agent_id)
            .bind(b.tool_id)
            .bind(&b.event_scope)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
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
}
