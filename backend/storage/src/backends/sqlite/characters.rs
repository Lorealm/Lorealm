use crate::backends::SqliteStorage;
use crate::error::StorageError;
use crate::models::{
    Character, CharacterDefinition, CharacterMetadata, CharacterSummary, CharacterVersionInfo,
    ExampleDialogue, Greeting, Snowflake, VersionedSaveMode,
};
use crate::storage::CharacterStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use super::NOW;

/// `CharacterSummary` carries `tags: Vec<String>`, which sqlx can decode from a
/// Postgres `TEXT[]` but not from SQLite. This row type takes the JSON array
/// produced by `json_group_array` instead and converts.
#[derive(FromRow)]
struct CharacterSummaryRow {
    id: Snowflake,
    user_id: Snowflake,
    name: String,
    display_name: Option<String>,
    tags: sqlx::types::Json<Vec<String>>,
    avatar_uri: Option<String>,
    card_version: String,
    lorebook_id: Option<Snowflake>,
    updated_at: DateTime<Utc>,
}

impl From<CharacterSummaryRow> for CharacterSummary {
    fn from(r: CharacterSummaryRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            display_name: r.display_name,
            tags: r.tags.0,
            avatar_uri: r.avatar_uri,
            card_version: r.card_version,
            lorebook_id: r.lorebook_id,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl CharacterStorage for SqliteStorage {
    async fn list_characters(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<CharacterSummary>, StorageError> {
        let rows = sqlx::query_as::<_, CharacterSummaryRow>(
            r#"
            SELECT
                c.id,
                c.user_id,
                c.lorebook_id,
                c.updated_at,
                COALESCE(v.snapshot->'definition'->>'name', '')       AS name,
                v.snapshot->'metadata'->>'display_name'               AS display_name,
                v.snapshot->'metadata'->>'avatar_uri'                 AS avatar_uri,
                COALESCE(v.snapshot->'metadata'->>'card_version', '') AS card_version,
                -- SQLite has no ARRAY(); json_group_array yields a JSON array,
                -- decoded as Json<Vec<String>>. Like ARRAY() it returns an empty
                -- array rather than NULL when the character has no tags.
                (
                    SELECT json_group_array(t.tag) FROM character_tags t
                    WHERE t.character_id = c.id
                ) AS tags
            FROM characters c
            JOIN character_versions v ON v.id = c.current_version_id
            WHERE c.user_id = $1
            ORDER BY c.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

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
        let widgets = fetch_widgets(&self.pool, id).await?;

        Ok(Some(row.into_character(
            tags, greetings, examples, widgets,
        )?))
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

        let Some(row) = row else { return Ok(None) };

        let widgets = fetch_widgets(&self.pool, row.character_id).await?;

        Ok(Some(row.into_character(widgets)?))
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
            let widgets = fetch_widgets(&self.pool, id).await?;
            out.push(row.into_character(tags, greetings, examples, widgets)?);
        }
        Ok(out)
    }

    async fn save_character(
        &self,
        character: &Character,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        let mut tx = self.pool.begin().await?;

        let snapshot = serde_json::to_value(CharacterSnapshot {
            metadata: &character.metadata,
            definition: &character.definition,
        })?;

        sqlx::query(
            &format!(r#"
            INSERT INTO characters
                (id, user_id, current_version_id, lorebook_id, extensions,
                 created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, {NOW}, {NOW})
            ON CONFLICT (id) DO UPDATE SET
                current_version_id = EXCLUDED.current_version_id,
                lorebook_id = EXCLUDED.lorebook_id,
                extensions = EXCLUDED.extensions,
                updated_at = {NOW}
            "#),
        )
        .bind(character.id)
        .bind(character.user_id)
        .bind(character.current_version_id)
        .bind(character.lorebook_id)
        .bind(&character.extensions)
        .execute(&mut *tx)
        .await?;

        match mode {
            VersionedSaveMode::NewVersion => {
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
            }
            VersionedSaveMode::UpdateCurrent => {
                let affected = sqlx::query(
                    r#"
                        UPDATE character_versions SET
                            card_version = $2,
                            avatar_uri   = $3,
                            snapshot     = $4
                        WHERE id = $1
                        "#,
                )
                .bind(character.current_version_id)
                .bind(&character.metadata.card_version)
                .bind(character.metadata.avatar_uri.as_deref())
                .bind(&snapshot)
                .execute(&mut *tx)
                .await?
                .rows_affected();

                if affected == 0 {
                    return Err(StorageError::NotFound(format!(
                        "save_character: version '{}' not found for in-place update",
                        character.current_version_id
                    )));
                }
            }
        }

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

        sqlx::query("DELETE FROM character_widgets WHERE character_id = $1")
            .bind(character.id)
            .execute(&mut *tx)
            .await?;

        for (position, widget_id) in character.widget_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO character_widgets (character_id, widget_id, position)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(character.id)
            .bind(widget_id)
            .bind(position as i32)
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
        let result: sqlx::sqlite::SqliteQueryResult =
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
        self.save_character(&to_save, VersionedSaveMode::NewVersion)
            .await
    }

    async fn get_latest_version(&self, id: Snowflake) -> Result<Snowflake, StorageError> {
        let version_id: Snowflake = sqlx::query_scalar(
            r#"
            SELECT current_version_id
            FROM characters
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(version_id)
    }
}

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
        widget_ids: Vec<Snowflake>,
    ) -> Result<Character, StorageError> {
        let snap: PersistedSnapshot = serde_json::from_value(self.snapshot)?;

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
            widget_ids,
            extensions: self.extensions,
            metadata,
            definition,
        })
    }
}

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
    fn into_character(self, widget_ids: Vec<Snowflake>) -> Result<Character, StorageError> {
        let snap: PersistedSnapshot = serde_json::from_value(self.snapshot)?;

        Ok(Character {
            id: self.character_id,
            user_id: self.user_id,
            current_version_id: self.version_id,
            lorebook_id: self.lorebook_id,
            widget_ids,
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

async fn fetch_tags(
    pool: &sqlx::SqlitePool,
    character_id: Snowflake,
) -> Result<Vec<String>, StorageError> {
    Ok(
        sqlx::query_scalar("SELECT tag FROM character_tags WHERE character_id = $1")
            .bind(character_id)
            .fetch_all(pool)
            .await?,
    )
}

async fn fetch_widgets(
    pool: &sqlx::SqlitePool,
    character_id: Snowflake,
) -> Result<Vec<Snowflake>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT widget_id FROM character_widgets WHERE character_id = $1 ORDER BY position ASC",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?)
}

#[derive(FromRow)]
struct GreetingRow {
    id: Snowflake,
    display_order: i32,
    text_content: String,
}

async fn fetch_greetings(
    pool: &sqlx::SqlitePool,
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
    pool: &sqlx::SqlitePool,
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
