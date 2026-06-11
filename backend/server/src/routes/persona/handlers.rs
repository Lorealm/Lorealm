use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use lorealm_storage::models::{Snowflake, UserPersona};

use crate::{
    core::AppState,
    errors::{AppError, PersonaError},
    extract::AuthenticatedSession,
    routes::persona::dto::{AvatarUpdateResponse, SetActiveRequest},
    services::persona::UpdatePersona,
};

pub async fn list_personas(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<UserPersona>>, AppError> {
    let personas = state
        .services()
        .persona()
        .list_personas(session.user_id)
        .await?;

    Ok(Json(personas))
}

pub async fn create_persona(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<(), AppError> {
    state
        .services()
        .persona()
        .create_default(session.user_id, "New persona".to_string())
        .await?;

    Ok(())
}

pub async fn save_persona(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(persona_id): Path<Snowflake>,
    Json(request): Json<UpdatePersona>,
) -> Result<Json<UserPersona>, AppError> {
    let persona = state
        .services()
        .persona()
        .update_persona(persona_id, session.user_id, request)
        .await?;

    Ok(Json(persona))
}

pub async fn get_avatar(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(persona_id): Path<Snowflake>,
) -> Result<Response, AppError> {
    let avatar_bytes = state
        .services()
        .persona()
        .get_avatar(persona_id, session.user_id)
        .await?;

    let content_type = infer::get(&avatar_bytes)
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream");

    Ok((
        [(CONTENT_TYPE, content_type), (CACHE_CONTROL, "no-cache")],
        avatar_bytes,
    )
        .into_response())
}

pub async fn put_avatar(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(persona_id): Path<Snowflake>,
    body: Bytes,
) -> Result<Json<AvatarUpdateResponse>, AppError> {
    let kind = infer::get(&body).ok_or(PersonaError::UnsupportedMediaType)?;
    if !matches!(
        kind.mime_type(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return Err(PersonaError::UnsupportedMediaType)?;
    }

    let avatar_uri = state
        .services()
        .persona()
        .update_image(persona_id, session.user_id, body)
        .await?;

    Ok(Json(AvatarUpdateResponse { avatar_uri }))
}

pub async fn set_active_persona(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<SetActiveRequest>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_persona(session.user_id, request.id)
        .await?;
    Ok(())
}

pub async fn get_active_persona(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Option<Snowflake>>, AppError> {
    let persona = state
        .services()
        .state()
        .get_active_persona(session.user_id)
        .await?;

    Ok(Json(Some(persona.id)))
}
