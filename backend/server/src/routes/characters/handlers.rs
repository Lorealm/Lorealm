use std::{borrow::Cow, sync::Arc};

use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use lorealm_storage::models::{
    Character, CharacterSummary, CharacterVersionInfo, ConversationMetadata, Snowflake,
};

use crate::{
    core::AppState,
    errors::{AppError, CharacterError},
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::characters::{
        dto::{AvatarUpdateResponse, CharacterEdit, CreateCharacterRequest, ImportCharResponse},
        types::ImportResult,
    },
    services::permission::Permissions,
};

pub async fn list_characters(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<CharacterSummary>>, AppError> {
    let chars = state
        .services()
        .characters()
        .list_characters(session.user_id)
        .await?;

    Ok(Json(chars))
}

async fn get_character(
    state: Arc<AppState>,
    session: AuthenticatedSession,
    character_id: Snowflake,
    version_id: Snowflake,
) -> Result<Json<Character>, AppError> {
    let char = state
        .services()
        .characters()
        .load_version(character_id, version_id, session.user_id)
        .await?;

    return Ok(Json(char));
}

pub async fn create_character(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<CreateCharacterRequest>,
) -> Result<Json<Character>, AppError> {
    let char = state
        .services()
        .characters()
        .create_character(session.user_id, request.name)
        .await?;

    Ok(Json(char))
}

pub async fn list_character_versions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
) -> Result<Json<Vec<CharacterVersionInfo>>, AppError> {
    let versions = state
        .services()
        .characters()
        .list_versions(character_id, session.user_id)
        .await?;

    Ok(Json(versions))
}

pub async fn get_character_version(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((character_id, version_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<Character>, AppError> {
    get_character(state, session, character_id, version_id).await
}

pub async fn get_character_latest(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
) -> Result<Json<Character>, AppError> {
    let latest_version_id = state
        .services()
        .characters()
        .get_latest_version_id(character_id)
        .await?;

    get_character(state, session, character_id, latest_version_id).await
}

pub async fn edit_character(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
    Json(request): Json<Vec<CharacterEdit>>,
) -> Result<Json<Character>, AppError> {
    let char_service = state.services().characters();
    let latest_version = char_service.get_latest_version_id(character_id).await?;
    let mut latest_char: Character = char_service
        .load_version(character_id, latest_version, session.user_id)
        .await?;
    for edit in request {
        latest_char = match edit {
            CharacterEdit::Definition(edit_definition) => {
                char_service
                    .patch_definition(character_id, session.user_id, edit_definition)
                    .await?
            }
            CharacterEdit::Metadata(edit_metadata) => {
                char_service
                    .patch_metadata(character_id, session.user_id, edit_metadata)
                    .await?
            }
            CharacterEdit::AddGreeting { text } => {
                char_service
                    .add_greeting(character_id, session.user_id, text)
                    .await?
            }
            CharacterEdit::UpdateGreeting { id, text } => {
                char_service
                    .update_greeting(character_id, session.user_id, id, text)
                    .await?
            }
            CharacterEdit::RemoveGreeting { id } => {
                char_service
                    .remove_greeting(character_id, session.user_id, id)
                    .await?
            }
            CharacterEdit::ReorderGreetings { ids } => {
                char_service
                    .reorder_greetings(character_id, session.user_id, ids)
                    .await?
            }
            CharacterEdit::AddExample { content } => {
                char_service
                    .add_example(character_id, session.user_id, content)
                    .await?
            }
            CharacterEdit::UpdateExample { id, content } => {
                char_service
                    .update_example(character_id, session.user_id, id, content)
                    .await?
            }
            CharacterEdit::RemoveExample { id } => {
                char_service
                    .remove_example(character_id, session.user_id, id)
                    .await?
            }
            CharacterEdit::ReorderExamples { ids } => {
                char_service
                    .reorder_example(character_id, session.user_id, ids)
                    .await?
            }
            CharacterEdit::SetTags { tags } => {
                char_service
                    .set_tags(character_id, session.user_id, tags)
                    .await?
            }
            CharacterEdit::SetAltNames { alt_names } => {
                char_service
                    .set_alt_names(character_id, session.user_id, alt_names)
                    .await?
            }
            CharacterEdit::SetWidgets { widget_ids } => {
                char_service
                    .set_widgets(character_id, session.user_id, widget_ids)
                    .await?
            }
        };
    }
    Ok(Json(latest_char))
}

async fn get_version_image(
    state: Arc<AppState>,
    session: AuthenticatedSession,
    character_id: Snowflake,
    version_id: Snowflake,
    cache_control: &'static str,
) -> Result<Response, AppError> {
    let version = state
        .services()
        .characters()
        .load_version(character_id, version_id, session.user_id)
        .await?;

    let avatar_bytes = state
        .services()
        .characters()
        .avatar_from_uri(version.metadata.avatar_uri)
        .await?;

    let content_type = infer::get(&avatar_bytes)
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream");

    Ok((
        [(CONTENT_TYPE, content_type), (CACHE_CONTROL, cache_control)],
        avatar_bytes,
    )
        .into_response())
}

pub async fn get_image_for_version(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((character_id, version_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Response, AppError> {
    get_version_image(
        state,
        session,
        character_id,
        version_id,
        "public, max-age=31536000, immutable",
    )
    .await
}

pub async fn get_image_for_latest(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
) -> Result<Response, AppError> {
    let latest_version_id = state
        .services()
        .characters()
        .get_latest_version_id(character_id)
        .await?;
    get_version_image(state, session, character_id, latest_version_id, "no-cache").await
}

/// NOTE: pushes an image to the latest version of a character. Changing images for a specific version
/// does not make sense at the moment of writing this, but this may change.
pub async fn put_image(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
    body: Bytes,
) -> Result<Json<AvatarUpdateResponse>, AppError> {
    let kind = infer::get(&body).ok_or(CharacterError::UnsupportedMediaType)?;
    if !matches!(
        kind.mime_type(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return Err(CharacterError::UnsupportedMediaType)?;
    }

    let avatar_uri = state
        .services()
        .characters()
        .update_image(character_id, session.user_id, body)
        .await?;

    Ok(Json(AvatarUpdateResponse { avatar_uri }))
}

pub async fn import_card_blob(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    perms: ResolvedPermissions,
    body: Bytes,
) -> Result<Json<ImportCharResponse>, AppError> {
    perms.require(Permissions::IMPORT_3RD_PARTY_CHARACTER)?;
    let kind = infer::get(&body).ok_or(CharacterError::UnsupportedMediaType)?;
    if !matches!(
        kind.mime_type(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return Err(CharacterError::UnsupportedMediaType)?;
    }

    let mut results: Vec<ImportResult> = Vec::new();

    if body.is_empty() {
        results.push(ImportResult::failed("File is empty."));
    }

    match state
        .services()
        .characters()
        .import_from_png(body.as_ref(), auth.user_id)
        .await
    {
        Ok(card) => results.push(ImportResult::ok(card)),
        Err(e) => results.push(ImportResult::failed(&e.to_string())),
    }

    Ok(Json(ImportCharResponse { results }))
}

pub async fn import_card_multipart(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    perms: ResolvedPermissions,
    mut multipart: Multipart,
) -> Result<Json<ImportCharResponse>, AppError> {
    perms.require(Permissions::IMPORT_3RD_PARTY_CHARACTER)?;

    let mut results: Vec<ImportResult> = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => {
                return Err(AppError::BadRequest {
                    code: "MALFORMED_MULTIPART",
                    message: Cow::Borrowed("Failed to read multipart stream."),
                });
            }
        };

        if field.file_name().is_none() {
            continue;
        }

        let data = field.bytes().await.map_err(|_| AppError::BadRequest {
            code: "INVALID_FILE",
            message: Cow::Borrowed("Failed to read file."),
        })?;

        if data.is_empty() {
            results.push(ImportResult::failed("File is empty."));
            continue;
        }

        // Best-effort: record per-file success/failure instead of aborting.
        match state
            .services()
            .characters()
            .import_from_png(data.as_ref(), auth.user_id)
            .await
        {
            Ok(card) => results.push(ImportResult::ok(card)),
            Err(e) => results.push(ImportResult::failed(&e.to_string())),
        }
    }

    if results.is_empty() {
        return Err(AppError::BadRequest {
            code: "NO_FILES",
            message: Cow::Borrowed("No files were provided."),
        });
    }

    Ok(Json(ImportCharResponse { results }))
}

pub async fn list_character_chats(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(character_id): Path<Snowflake>,
) -> Result<Json<Vec<ConversationMetadata>>, AppError> {
    let mut convos = state
        .services()
        .chat()
        .list_conversations(session.user_id)
        .await?;

    convos.retain(|c| c.character_id == Some(character_id));

    Ok(Json(convos))
}
