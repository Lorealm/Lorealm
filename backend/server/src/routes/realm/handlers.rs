use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{AgentGrant, Realm, RealmAgent, RealmSummary, Snowflake};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::realm::dto::{CreateAgentRequest, CreateRealmRequest, GrantDto, UpdateRealmRequest},
    services::realm::ActionSpec,
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
        .create_realm(payload.name, payload.tokenizer_id, session.user_id)
        .await?;

    Ok(Json(created))
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
    Path((realm_id, agent_id)): Path<(Snowflake, Snowflake)>,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<RealmAgent>, AppError> {
    let updated = state
        .services()
        .realm()
        .update_agent(realm_id, agent_id, session.user_id, payload)
        .await?;

    Ok(Json(updated))
}

pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, agent_id)): Path<(Snowflake, Snowflake)>,
) -> Result<(), AppError> {
    state
        .services()
        .realm()
        .delete_agent(realm_id, agent_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn get_grants(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, agent_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<Vec<AgentGrant>>, AppError> {
    let grants = state
        .services()
        .realm()
        .list_agent_grants(realm_id, agent_id, session.user_id)
        .await?;

    Ok(Json(grants))
}

pub async fn set_grants(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, agent_id)): Path<(Snowflake, Snowflake)>,
    Json(payload): Json<Vec<GrantDto>>,
) -> Result<(), AppError> {
    let grants = payload
        .iter()
        .map(|g| AgentGrant {
            agent_id,
            scope: g.scope.clone(),
            access: g.access.clone(),
        })
        .collect();

    state
        .services()
        .realm()
        .set_grants(realm_id, agent_id, session.user_id, grants)
        .await?;

    Ok(())
}

/// The agent's action space as currently authored in the preset.
pub async fn list_actions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((realm_id, agent_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<Vec<ActionSpec>>, AppError> {
    let actions = state
        .services()
        .realm()
        .list_actions(realm_id, agent_id, session.user_id)
        .await?;

    Ok(Json(actions))
}
