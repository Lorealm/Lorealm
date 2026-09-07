use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use lorealm_storage::models::{Snowflake, TemplateMetadata};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::templates::{SetActiveRequest, WeaverVocabulary},
    services::context::{ContextTemplate, SourceDefinition, weaver::check::check},
    services::weaver::WeaverHost,
};

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<TemplateMetadata>>, AppError> {
    let templates = state
        .services()
        .context()
        .list_templates(session.user_id)
        .await?;

    Ok(Json(templates))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
) -> Result<Json<ContextTemplate>, AppError> {
    Ok(Json(
        state
            .services()
            .context()
            .get_template(template_id, session.user_id)
            .await?,
    ))
}

pub async fn create_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::CreateTemplateRequest>,
) -> Result<Json<ContextTemplate>, AppError> {
    let template = state
        .services()
        .context()
        .create_template(
            session.user_id,
            request.name,
            request.description,
            request.default,
        )
        .await?;

    Ok(Json(template))
}

pub async fn update_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
    Json(request): Json<super::UpdateTemplateRequest>,
) -> Result<Json<ContextTemplate>, AppError> {
    let updated_template = state
        .services()
        .context()
        .update_template(
            template_id,
            session.user_id,
            request.name,
            request.description,
        )
        .await?;

    Ok(Json(updated_template))
}

pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .context()
        .delete_template(template_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn list_sources(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceDefinition>>, AppError> {
    let sources = state
        .services()
        .context()
        .with_registry(|reg| reg.definitions());

    Ok(Json(sources))
}

/// Lists every weaver command and processor available to editors.
pub async fn list_weaver_vocabulary() -> Json<WeaverVocabulary> {
    let host = WeaverHost::builder().declarations_only().build();
    let registry = host.registry();

    let mut commands = registry.command_signatures();
    commands.extend(context_weaver::host_command_signatures());
    commands.sort_by(|a, b| a.name.cmp(&b.name));

    let mut processors = registry.processor_signatures();
    processors.sort_by(|a, b| {
        (a.namespace.as_str(), a.name.as_str()).cmp(&(b.namespace.as_str(), b.name.as_str()))
    });

    Json(WeaverVocabulary {
        commands,
        processors,
    })
}

/// Check weaver source and report what is wrong with it.
pub async fn check_weaver(
    Json(request): Json<super::WeaverCheckRequest>,
) -> Json<super::WeaverCheckResponse> {
    Json(super::WeaverCheckResponse {
        diagnostics: check(&request.source, request.dialect),
    })
}

pub async fn add_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
    Json(request): Json<super::CreateNodeRequest>,
) -> Result<Json<super::NodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .add_node(template_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn update_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
    Json(request): Json<super::UpdateNodeRequest>,
) -> Result<Json<super::NodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .update_node(template_id, node_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn delete_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .context()
        .delete_node(template_id, node_id, session.user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
    Json(request): Json<super::MoveNodeRequest>,
) -> Result<Json<super::MoveNodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .move_node(template_id, node_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn toggle_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<super::ToggleNodeResponse>, AppError> {
    let changes = state
        .services()
        .context()
        .toggle_node(template_id, node_id, session.user_id)
        .await?;

    Ok(Json(super::ToggleNodeResponse { delta: changes }))
}

pub async fn get_active(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Option<Snowflake>>, AppError> {
    let template = state
        .services()
        .state()
        .get_active_template_id(session.user_id)
        .await?;

    Ok(Json(template))
}

async fn set_active(
    state: Arc<AppState>,
    session: AuthenticatedSession,
    value: Option<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_template(session.user_id, value)
        .await?;
    Ok(())
}

pub async fn set_active_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<SetActiveRequest>,
) -> Result<(), AppError> {
    set_active(state, session, Some(request.id)).await
}

pub async fn unset_active_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<(), AppError> {
    set_active(state, session, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn vocabulary_carries_editor_metadata() {
        let vocab = list_weaver_vocabulary().await.0;

        // Standard library and the lorebook host commands are both present.
        let names: Vec<&str> = vocab.commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"set_var"));
        assert!(names.contains(&"inc_var"));
        assert!(names.contains(&"is_active"));
        assert!(names.contains(&"is_active_global"));

        // Descriptions come through from the callables' doc comments.
        let set_var = vocab.commands.iter().find(|c| c.name == "set_var").unwrap();
        assert!(!set_var.description.is_empty());
        assert_eq!(set_var.returns, weaver_lang::registry::ValueType::None);
        assert!(set_var.mutates_context);

        // The drift fix flows all the way to the wire: text.substr's `length`
        // is optional and typed number, not required-and-any.
        let substr = vocab
            .processors
            .iter()
            .find(|p| p.namespace == "text" && p.name == "substr")
            .unwrap();
        let length = substr
            .properties
            .iter()
            .find(|p| p.key == "length")
            .unwrap();
        assert!(!length.required);
        assert_eq!(
            length.expected_type,
            Some(weaver_lang::registry::ValueType::Number)
        );

        // And it serializes to the shape the frontend consumes.
        let json = serde_json::to_value(&vocab).unwrap();
        assert!(json["commands"].is_array());
        assert!(json["processors"].is_array());
    }

    async fn check_json(body: serde_json::Value) -> serde_json::Value {
        let request = serde_json::from_value(body).expect("request shape the client sends");
        serde_json::to_value(check_weaver(Json(request)).await.0).unwrap()
    }

    #[tokio::test]
    async fn check_reports_problems_in_the_body_not_as_an_error() {
        let json = check_json(serde_json::json!({ "source": "{# break #}" })).await;

        // Positions and severity arrive in the shape CodeMirror wants.
        let diagnostic = &json["diagnostics"][0];
        assert_eq!(diagnostic["from"], 0);
        assert_eq!(diagnostic["to"], 11);
        assert_eq!(diagnostic["severity"], "error");
        assert!(diagnostic["message"].is_string());
    }

    #[tokio::test]
    async fn check_defaults_to_the_template_dialect() {
        let json = check_json(serde_json::json!({ "source": "just prose" })).await;
        assert_eq!(json["diagnostics"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn check_accepts_the_expression_dialect_by_name() {
        let json =
            check_json(serde_json::json!({ "source": "just prose", "dialect": "expression" }))
                .await;
        assert_eq!(json["diagnostics"].as_array().unwrap().len(), 1);
    }
}
