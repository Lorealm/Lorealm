use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_providers::ModelInfo;
use lorealm_storage::models::{ConnectionProfile, SecretMetadata, Snowflake};

use crate::{core::AppState, errors::AppError, extract::AuthenticatedSession};

pub async fn create_secret(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::CreateSecretRequest>,
) -> Result<Json<SecretMetadata>, AppError> {
    let secret = state
        .services()
        .connections()
        .create_secret(
            session.user_id,
            request.provider,
            request.label,
            request.key,
        )
        .await?;

    Ok(Json(secret))
}

pub async fn list_secrets(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<SecretMetadata>>, AppError> {
    let secrets = state
        .services()
        .connections()
        .list_secrets(session.user_id)
        .await?;

    Ok(Json(secrets))
}

pub async fn rename_secret(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(secret_id): Path<Snowflake>,
    Json(request): Json<super::RenameSecretRequest>,
) -> Result<Json<SecretMetadata>, AppError> {
    let secret = state
        .services()
        .connections()
        .rename_secret(session.user_id, secret_id, request.label)
        .await?;

    Ok(Json(secret))
}

pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(secret_id): Path<Snowflake>,
) -> Result<Json<super::DeleteSecretResponse>, AppError> {
    let detached_profiles = state
        .services()
        .connections()
        .delete_secret(session.user_id, secret_id)
        .await?;

    Ok(Json(super::DeleteSecretResponse { detached_profiles }))
}

pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::ProfileRequest>,
) -> Result<Json<ConnectionProfile>, AppError> {
    let profile = state
        .services()
        .connections()
        .create_profile(session.user_id, request.into())
        .await?;

    Ok(Json(profile))
}

pub async fn list_profiles(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<ConnectionProfile>>, AppError> {
    let profiles = state
        .services()
        .connections()
        .list_profiles(session.user_id)
        .await?;

    Ok(Json(profiles))
}

pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(profile_id): Path<Snowflake>,
) -> Result<Json<ConnectionProfile>, AppError> {
    let profile = state
        .services()
        .connections()
        .get_profile(session.user_id, profile_id)
        .await?;

    Ok(Json(profile))
}

/// Models the profile's endpoint offers
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(profile_id): Path<Snowflake>,
) -> Result<Json<Vec<ModelInfo>>, AppError> {
    let models = state
        .services()
        .connections()
        .list_models(session.user_id, profile_id)
        .await?;

    Ok(Json(models))
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(profile_id): Path<Snowflake>,
    Json(request): Json<super::ProfileRequest>,
) -> Result<Json<ConnectionProfile>, AppError> {
    let profile = state
        .services()
        .connections()
        .update_profile(session.user_id, profile_id, request.into())
        .await?;

    Ok(Json(profile))
}

pub async fn delete_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(profile_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .connections()
        .delete_profile(session.user_id, profile_id)
        .await?;

    Ok(())
}

pub async fn get_active_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Snowflake>, AppError> {
    let profile = state
        .services()
        .state()
        .get_active_profile(session.user_id)
        .await?;

    Ok(Json(profile.id))
}

pub async fn set_active_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::SetActiveRequest>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_profile(session.user_id, request.id)
        .await?;

    Ok(())
}
