use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{Realm, RealmResourceRef, RealmSummary, RealmVersionInfo, Snowflake};

use crate::{
    core::AppState,
    errors::{AppError, RealmError},
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::realm::dto::ImportRealmRequest,
    services::permission::Permissions,
    util::weaver::json_to_weaver,
};

pub async fn list_realms(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<RealmSummary>>, AppError> {
    let realms = state
        .services()
        .realm()
        .list_realms(session.user_id)
        .await?;

    Ok(Json(realms))
}

pub async fn get_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<Json<Realm>, AppError> {
    let realm = state
        .services()
        .realm()
        .get_realm(realm_id, session.user_id)
        .await?;

    Ok(Json(realm))
}

pub async fn list_versions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<Json<Vec<RealmVersionInfo>>, AppError> {
    let versions = state
        .services()
        .realm()
        .list_versions(realm_id, session.user_id)
        .await?;

    Ok(Json(versions))
}

pub async fn get_version(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, version_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<Realm>, AppError> {
    let realm = state
        .services()
        .realm()
        .get_version(realm_id, version_id, session.user_id)
        .await?;

    Ok(Json(realm))
}

pub async fn delete_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .realm()
        .delete_realm(realm_id, session.user_id)
        .await?;
    Ok(())
}

/// Create a realm from a WTN document.
pub async fn import_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(payload): Json<ImportRealmRequest>,
) -> Result<Json<Realm>, AppError> {
    perms.require(Permissions::IMPORT_CONTENT)?;
    let inputs = payload
        .inputs
        .iter()
        .map(|(name, value)| {
            json_to_weaver(value)
                .map(|v| (name.clone(), v))
                .map_err(|e| RealmError::InvalidTemplate(format!("input '{name}': {e}")))
        })
        .collect::<Result<_, _>>()?;

    let realm = state
        .services()
        .realm()
        .import(session.user_id, &payload.source, inputs, None)
        .await?;

    Ok(Json(realm))
}

/// The realm as a WTN document: the form it is distributed in.
pub async fn export_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<String, AppError> {
    Ok(state
        .services()
        .realm()
        .export(realm_id, session.user_id)
        .await?)
}

pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<Json<Vec<RealmResourceRef>>, AppError> {
    let resources = state
        .services()
        .realm()
        .list_resources(realm_id, session.user_id)
        .await?;

    Ok(Json(resources))
}
