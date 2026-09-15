use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{
    AgentGrant, Realm, RealmAgent, RealmConfig, RealmResourceRef, RealmSummary, RealmVersionInfo,
    Scenario, Snowflake,
};

use crate::{
    core::AppState,
    errors::{AppError, RealmError},
    extract::AuthenticatedSession,
    routes::realm::dto::{
        CreateAgentRequest, CreateRealmRequest, CreateScenarioRequest, ImportRealmRequest,
        UpdateRealmRequest,
    },
    services::realm::ActionSpec,
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

pub async fn create_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<CreateRealmRequest>,
) -> Result<Json<Realm>, AppError> {
    let created = state
        .services()
        .realm()
        .create_realm(payload.name, session.user_id)
        .await?;

    Ok(Json(created))
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

pub async fn restore_version(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, version_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<Realm>, AppError> {
    let realm = state
        .services()
        .realm()
        .restore_version(realm_id, version_id, session.user_id)
        .await?;

    Ok(Json(realm))
}

pub async fn update_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
    Json(payload): Json<UpdateRealmRequest>,
) -> Result<Json<Realm>, AppError> {
    let updated = state
        .services()
        .realm()
        .update_realm(realm_id, session.user_id, payload)
        .await?;

    Ok(Json(updated))
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

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<Json<Vec<RealmAgent>>, AppError> {
    let agents = state
        .services()
        .realm()
        .list_agents(realm_id, session.user_id)
        .await?;

    Ok(Json(agents))
}

pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<RealmAgent>, AppError> {
    let created = state
        .services()
        .realm()
        .create_agent(realm_id, session.user_id, payload)
        .await?;

    Ok(Json(created))
}

pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, handle)): Path<(Snowflake, String)>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<RealmAgent>, AppError> {
    let updated = state
        .services()
        .realm()
        .update_agent(realm_id, &handle, session.user_id, payload)
        .await?;

    Ok(Json(updated))
}

/// Rewrite the cast's turn order; the body must be a permutation of the
/// current handles.
pub async fn reorder_agents(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
    Json(payload): Json<Vec<String>>,
) -> Result<Json<Vec<RealmAgent>>, AppError> {
    let agents = state
        .services()
        .realm()
        .reorder_agents(realm_id, session.user_id, payload)
        .await?;

    Ok(Json(agents))
}

pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, handle)): Path<(Snowflake, String)>,
) -> Result<(), AppError> {
    state
        .services()
        .realm()
        .delete_agent(realm_id, &handle, session.user_id)
        .await?;

    Ok(())
}

pub async fn create_scenario(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
    Json(payload): Json<CreateScenarioRequest>,
) -> Result<Json<Scenario>, AppError> {
    let created = state
        .services()
        .realm()
        .create_scenario(realm_id, session.user_id, payload)
        .await?;

    Ok(Json(created))
}

pub async fn update_scenario(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, scenario_id)): Path<(Snowflake, Snowflake)>,
    Json(payload): Json<CreateScenarioRequest>,
) -> Result<Json<Scenario>, AppError> {
    let updated = state
        .services()
        .realm()
        .update_scenario(realm_id, scenario_id, session.user_id, payload)
        .await?;

    Ok(Json(updated))
}

pub async fn delete_scenario(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, scenario_id)): Path<(Snowflake, Snowflake)>,
) -> Result<(), AppError> {
    state
        .services()
        .realm()
        .delete_scenario(realm_id, scenario_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn get_grants(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, handle)): Path<(Snowflake, String)>,
) -> Result<Json<Vec<AgentGrant>>, AppError> {
    let grants = state
        .services()
        .realm()
        .list_agent_grants(realm_id, &handle, session.user_id)
        .await?;

    Ok(Json(grants))
}

pub async fn set_grants(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, handle)): Path<(Snowflake, String)>,
    Json(payload): Json<Vec<AgentGrant>>,
) -> Result<(), AppError> {
    state
        .services()
        .realm()
        .set_grants(realm_id, &handle, session.user_id, payload)
        .await?;

    Ok(())
}

/// Create a realm from a WTN document.
pub async fn import_realm(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<ImportRealmRequest>,
) -> Result<Json<Realm>, AppError> {
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

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
) -> Result<Json<RealmConfig>, AppError> {
    let config = state
        .services()
        .realm()
        .get_config(realm_id, session.user_id)
        .await?;

    Ok(Json(config))
}

pub async fn set_config(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(realm_id): Path<Snowflake>,
    Json(payload): Json<RealmConfig>,
) -> Result<Json<RealmConfig>, AppError> {
    let config = state
        .services()
        .realm()
        .set_config(realm_id, session.user_id, payload)
        .await?;

    Ok(Json(config))
}

/// The agent's action space as currently authored in the preset.
pub async fn list_actions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, handle)): Path<(Snowflake, String)>,
) -> Result<Json<Vec<ActionSpec>>, AppError> {
    let actions = state
        .services()
        .realm()
        .list_actions(realm_id, &handle, session.user_id)
        .await?;

    Ok(Json(actions))
}
