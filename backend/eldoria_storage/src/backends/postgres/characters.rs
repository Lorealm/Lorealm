//! Postgres implementation of `CharacterStorage`.
//!
//! All multi-table writes run in a single transaction. The head row and
//! version row are written together; child tables (tags, greetings,
//! example messages) are rewritten from scratch on every save — simpler
//! than diffing and correct by construction.

use crate::backends::PostgresStorage;
use crate::error::StorageError;
use crate::models::{
    Character, CharacterDefinition, CharacterMetadata, CharacterVersionInfo, ExampleDialogue,
    Greeting, Snowflake,
};
use crate::storage::characters::CharacterStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[async_trait]
impl CharacterStorage for PostgresStorage {
    async fn get_character(&self, id: Snowflake) -> Result<Option<Character>, StorageError> {
        let head: Option<CharacterHeadRow> = sqlx::query_as(
            r#"
            SELECT c.id, c.user_id, c.current_version_id, c.lorebook_id,
                   c.extensions, c.created_at, c.updated_at,
                   v.snapshot
            FROM characters c
            JOIN character_versions v ON v.id = c.current_version_id
            WHERE c.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = head else { return Ok(None) };

        let tags = fetch_tags(&self.pool, id).await?;
        let greetings = fetch_greetings(&self.pool, id).await?;
        let examples = fetch_examples(&self.pool, id).await?;

        Ok(Some(row.into_character(tags, greetings, examples)?))
    }

    async fn get_character_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Character>, StorageError> {
        let row: Option<CharacterVersionRow> = sqlx::query_as(
            r#"
            SELECT v.id AS version_id, v.character_id, v.snapshot,
                   c.user_id, c.lorebook_id, c.extensions
            FROM character_versions v
            JOIN characters c ON c.id = v.character_id
            WHERE v.character_id = $1 AND v.id = $2
            "#,
        )
        .bind(id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(CharacterVersionRow::into_character).transpose()
    }

    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError> {
        let rows: Vec<CharacterHeadRow> = sqlx::query_as(
            r#"
            SELECT c.id, c.user_id, c.current_version_id, c.lorebook_id,
                   c.extensions, c.created_at, c.updated_at,
                   v.snapshot
            FROM characters c
            JOIN character_versions v ON v.id = c.current_version_id
            WHERE c.user_id = $1
            ORDER BY c.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let tags = fetch_tags(&self.pool, id).await?;
            let greetings = fetch_greetings(&self.pool, id).await?;
            let examples = fetch_examples(&self.pool, id).await?;
            out.push(row.into_character(tags, greetings, examples)?);
        }
        Ok(out)
    }

    async fn save_character(&self, character: &Character) -> Result<Snowflake, StorageError> {
        let mut tx = self.pool.begin().await?;

        let next_version_num: i32 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(version_num), 0) + 1
            FROM character_versions
            WHERE character_id = $1
            "#,
        )
        .bind(character.id)
        .fetch_one(&mut *tx)
        .await?;

        let snapshot = serde_json::to_value(CharacterSnapshot {
            metadata: &character.metadata,
            definition: &character.definition,
        })?;

        sqlx::query(
            r#"
            INSERT INTO characters
                (id, user_id, current_version_id, lorebook_id, extensions,
                 created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, now(), now())
            ON CONFLICT (id) DO UPDATE SET
                current_version_id = EXCLUDED.current_version_id,
                lorebook_id = EXCLUDED.lorebook_id,
                extensions = EXCLUDED.extensions,
                updated_at = now()
            "#,
        )
        .bind(character.id)
        .bind(character.user_id)
        .bind(character.current_version_id)
        .bind(character.lorebook_id)
        .bind(&character.extensions)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO character_versions
                (id, character_id, version_num, card_version, avatar_uri, snapshot)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(character.current_version_id)
        .bind(character.id)
        .bind(next_version_num)
        .bind(&character.metadata.card_version)
        .bind(character.metadata.avatar_uri.as_deref())
        .bind(&snapshot)
        .execute(&mut *tx)
        .await?;

        // Rewrite child tables from scratch. Correct regardless of
        // whether this was an insert or update.
        sqlx::query("DELETE FROM character_tags WHERE character_id = $1")
            .bind(character.id)
            .execute(&mut *tx)
            .await?;

        for tag in &character.metadata.tags {
            sqlx::query("INSERT INTO character_tags (character_id, tag) VALUES ($1, $2)")
                .bind(character.id)
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("DELETE FROM character_greetings WHERE character_id = $1")
            .bind(character.id)
            .execute(&mut *tx)
            .await?;

        for g in &character.definition.greetings {
            sqlx::query(
                r#"
                INSERT INTO character_greetings
                    (id, character_id, display_order, text_content)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(g.id)
            .bind(character.id)
            .bind(g.order as i32)
            .bind(&g.text)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("DELETE FROM character_example_messages WHERE character_id = $1")
            .bind(character.id)
            .execute(&mut *tx)
            .await?;

        for ex in &character.definition.example_messages {
            sqlx::query(
                r#"
                INSERT INTO character_example_messages
                    (id, character_id, display_order, content)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(ex.id)
            .bind(character.id)
            .bind(ex.order as i32)
            .bind(&ex.content)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(character.current_version_id)
    }

    async fn delete_character(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result: sqlx::postgres::PgQueryResult =
            sqlx::query("DELETE FROM characters WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_character_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, StorageError> {
        let rows: Vec<VersionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, version_num, card_version, created_at
            FROM character_versions
            WHERE character_id = $1
            ORDER BY version_num DESC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn restore_character_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let Some(character) = self.get_character_at_version(id, version_id).await? else {
            return Err(StorageError::NotFound(format!(
                "character {id} version {version_id} not found"
            )));
        };

        let mut to_save = character;
        to_save.current_version_id = new_version_id;
        self.save_character(&to_save).await
    }
}

// -----------------------------------------------------------------------------
// Row types
// -----------------------------------------------------------------------------

/// Row from `characters` joined with the current `character_versions.snapshot`.
#[derive(FromRow)]
struct CharacterHeadRow {
    id: Snowflake,
    user_id: Snowflake,
    current_version_id: Snowflake,
    lorebook_id: Option<Snowflake>,
    extensions: serde_json::Value,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
    #[allow(dead_code)]
    updated_at: DateTime<Utc>,
    snapshot: serde_json::Value,
}

impl CharacterHeadRow {
    fn into_character(
        self,
        tags: Vec<String>,
        greetings: Vec<Greeting>,
        example_messages: Vec<ExampleDialogue>,
    ) -> Result<Character, StorageError> {
        let snap: PersistedSnapshot = serde_json::from_value(self.snapshot)?;

        // Child tables are authoritative for the current view; they override
        // whatever the snapshot carried.
        let mut metadata = snap.metadata;
        metadata.tags = tags;

        let mut definition = snap.definition;
        definition.greetings = greetings;
        definition.example_messages = example_messages;

        Ok(Character {
            id: self.id,
            user_id: self.user_id,
            current_version_id: self.current_version_id,
            lorebook_id: self.lorebook_id,
            extensions: self.extensions,
            metadata,
            definition,
        })
    }
}

/// Row from `character_versions` joined with a subset of `characters`, used
/// to reconstruct a historical version.
#[derive(FromRow)]
struct CharacterVersionRow {
    version_id: Snowflake,
    character_id: Snowflake,
    user_id: Snowflake,
    lorebook_id: Option<Snowflake>,
    extensions: serde_json::Value,
    snapshot: serde_json::Value,
}

impl CharacterVersionRow {
    fn into_character(self) -> Result<Character, StorageError> {
        let snap: PersistedSnapshot = serde_json::from_value(self.snapshot)?;

        Ok(Character {
            id: self.character_id,
            user_id: self.user_id,
            current_version_id: self.version_id,
            lorebook_id: self.lorebook_id,
            extensions: self.extensions,
            metadata: snap.metadata,
            definition: snap.definition,
        })
    }
}

#[derive(FromRow)]
struct VersionInfoRow {
    id: Snowflake,
    version_num: i32,
    card_version: String,
    created_at: DateTime<Utc>,
}

impl From<VersionInfoRow> for CharacterVersionInfo {
    fn from(row: VersionInfoRow) -> Self {
        CharacterVersionInfo {
            id: row.id,
            version_num: row.version_num as u32,
            card_version: row.card_version,
            created_at: row.created_at,
        }
    }
}

// -----------------------------------------------------------------------------
// Snapshot serde shapes
// -----------------------------------------------------------------------------

/// Written into `character_versions.snapshot`. Borrowed on write; see
/// `PersistedSnapshot` for the owned read counterpart.
#[derive(serde::Serialize)]
struct CharacterSnapshot<'a> {
    metadata: &'a CharacterMetadata,
    definition: &'a CharacterDefinition,
}

#[derive(serde::Deserialize)]
struct PersistedSnapshot {
    metadata: CharacterMetadata,
    definition: CharacterDefinition,
}

// -----------------------------------------------------------------------------
// Child-table fetches
// -----------------------------------------------------------------------------

async fn fetch_tags(
    pool: &sqlx::PgPool,
    character_id: Snowflake,
) -> Result<Vec<String>, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT tag FROM character_tags WHERE character_id = $1")
            .bind(character_id)
            .fetch_all(pool)
            .await?,
    )
}

#[derive(FromRow)]
struct GreetingRow {
    id: Snowflake,
    display_order: i32,
    text_content: String,
}

async fn fetch_greetings(
    pool: &sqlx::PgPool,
    character_id: Snowflake,
) -> Result<Vec<Greeting>, StorageError> {
    let rows: Vec<GreetingRow> = sqlx::query_as(
        r#"
        SELECT id, display_order, text_content
        FROM character_greetings
        WHERE character_id = $1
        ORDER BY display_order ASC
        "#,
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Greeting {
            id: r.id,
            order: r.display_order as u32,
            text: r.text_content,
        })
        .collect())
}

#[derive(FromRow)]
struct ExampleRow {
    id: Snowflake,
    display_order: i32,
    content: String,
}

async fn fetch_examples(
    pool: &sqlx::PgPool,
    character_id: Snowflake,
) -> Result<Vec<ExampleDialogue>, StorageError> {
    let rows: Vec<ExampleRow> = sqlx::query_as(
        r#"
        SELECT id, display_order, content
        FROM character_example_messages
        WHERE character_id = $1
        ORDER BY display_order ASC
        "#,
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ExampleDialogue {
            id: r.id,
            order: r.display_order as u32,
            content: r.content,
        })
        .collect())
}
