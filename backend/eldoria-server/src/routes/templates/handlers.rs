use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{app_state::AppState, middleware::authentication::AuthenticatedSession};
use crate::services::context::{
    ContextEvaluator, ContextNode, ContextTemplate, EvaluationContext, SegmentSource,
};
use crate::services::conversation::ChatMessage;

use super::dto::*;
use super::error::TemplateApiError;

/// Type alias for the result of route handlers
type RouteResult<T> = Result<T, TemplateApiError>;

// ==================== Template CRUD ====================

/// List all templates for the authenticated user
/// GET /templates
pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
) -> RouteResult<Json<TemplateListResponse>> {
    tracing::info!("listing templates");
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let templates = template_service.list_templates(auth.user_id).await?;

    let summaries: Vec<TemplateSummary> = templates
        .into_iter()
        .map(|t| TemplateSummary {
            id: t.id,
            name: t.name,
            description: t.description,
            node_count: t.node_count,
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    Ok(Json(TemplateListResponse {
        total: summaries.len(),
        templates: summaries,
    }))
}

/// Get a single template with full details
/// GET /templates/:id
pub async fn get_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
) -> RouteResult<Json<TemplateResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    let sources = state.source_registry().all_sources_owned();
    let used_nodes = template.get_used_source_nodes();

    Ok(Json(TemplateResponse {
        template,
        available_sources: sources,
        used_source_nodes: used_nodes,
    }))
}

/// Create a new template
/// POST /templates
pub async fn create_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Json(request): Json<CreateTemplateRequest>,
) -> RouteResult<(StatusCode, Json<TemplateResponse>)> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = ContextTemplate::new(request.name, auth.user_id);
    template.description = request.description;

    let template = template_service.create_template(template).await?;

    let sources = state.source_registry().all_sources_owned();
    let used_nodes = template.get_used_source_nodes();

    Ok((
        StatusCode::CREATED,
        Json(TemplateResponse {
            template,
            available_sources: sources,
            used_source_nodes: used_nodes,
        }),
    ))
}

/// Update a template's metadata
/// PUT /templates/:id
pub async fn update_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
    Json(request): Json<UpdateTemplateRequest>,
) -> RouteResult<Json<TemplateResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    if let Some(name) = request.name {
        template.name = name;
    }
    if let Some(description) = request.description {
        template.description = description;
    }

    let template = template_service.update_template(template, auth.user_id).await?;

    let sources = state.source_registry().all_sources_owned();
    let used_nodes = template.get_used_source_nodes();

    Ok(Json(TemplateResponse {
        template,
        available_sources: sources,
        used_source_nodes: used_nodes,
    }))
}

/// Delete a template
/// DELETE /templates/:id
pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
) -> RouteResult<StatusCode> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    template_service
        .delete_template(template_id, auth.user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Node Operations ====================

/// Add a new node to a template
/// POST /templates/:id/nodes
pub async fn add_node(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
    Json(request): Json<CreateNodeRequest>,
) -> RouteResult<(StatusCode, Json<NodeResponse>)> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    let node = ContextNode {
        id: Uuid::new_v4(),
        title: request.title,
        description: request.description,
        enabled: true,
        content: request.content,
    };

    let path = template.add_node(node, request.parent_path.as_ref(), request.index)?;

    template_service.update_template(template.clone(), auth.user_id).await?;

    // Get the node we just added
    let node = template.get_node(&path).cloned().unwrap();

    Ok((StatusCode::CREATED, Json(NodeResponse { node, path })))
}

/// Update a node
/// PUT /templates/:id/nodes/:node_id
pub async fn update_node(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateNodeRequest>,
) -> RouteResult<Json<NodeResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    template.update_node(
        node_id,
        request.title,
        request.description,
        request.enabled,
        request.content,
    )?;

    template_service.update_template(template.clone(), auth.user_id).await?;

    let path = template.find_node_path(node_id).unwrap();
    let node = template.get_node(&path).cloned().unwrap();

    Ok(Json(NodeResponse { node, path }))
}

/// Remove a node from a template
/// DELETE /templates/:id/nodes/:node_id
pub async fn remove_node(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Uuid, Uuid)>,
) -> RouteResult<StatusCode> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    template.remove_node(node_id)?;

    template_service.update_template(template, auth.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Move a node to a new position
/// POST /templates/:id/nodes/:node_id/move
pub async fn move_node(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveNodeRequest>,
) -> RouteResult<Json<MoveNodeResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;
    
    
    tracing::info!("moving node: {}", node_id);
    let new_path =
        template.move_node(node_id, request.new_parent_path.as_ref(), request.new_index)?;

    template_service.update_template(template, auth.user_id).await?;

    Ok(Json(MoveNodeResponse { new_path }))
}

/// Toggle a node's enabled state
/// POST /templates/:id/nodes/:node_id/toggle
pub async fn toggle_node(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Uuid, Uuid)>,
) -> RouteResult<Json<ToggleNodeResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let mut template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    let enabled = template.toggle_node(node_id)?;

    template_service.update_template(template, auth.user_id).await?;

    Ok(Json(ToggleNodeResponse { enabled }))
}

// ==================== Sources ====================

/// List all available sources
/// GET /sources
pub async fn list_sources(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
) -> Json<SourcesResponse> {
    let sources = state.source_registry().all_sources_owned();
    Json(SourcesResponse { sources })
}

// ==================== Validation & Preview ====================

/// Validate a template
/// GET /templates/:id/validate
pub async fn validate_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
) -> RouteResult<Json<ValidationResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    match template.validate(state.source_registry()) {
        Ok(()) => Ok(Json(ValidationResponse {
            valid: true,
            errors: vec![],
        })),
        Err(errors) => Ok(Json(ValidationResponse {
            valid: false,
            errors: errors.iter().map(|e| e.to_string()).collect(),
        })),
    }
}

/// Preview the evaluated context
/// POST /templates/:id/preview
pub async fn preview_context(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(template_id): Path<Uuid>,
    Json(request): Json<PreviewContextRequest>,
) -> RouteResult<Json<PreviewResponse>> {
    let storage_lock = state.storage();
    let storage = storage_lock.read().await;
    let template_service = storage.template_service();

    let template = template_service
        .get_template_owned(template_id, auth.user_id)
        .await?;

    // Create a mock evaluation context for preview
    // In production, you could fetch actual chat messages if chat_id is provided
    let messages: Vec<ChatMessage> = vec![
        ChatMessage::user("[Preview] Sample user message"),
        ChatMessage::assistant("[Preview] Sample assistant response"),
    ];

    let eval_context = EvaluationContext::new(
        request.chat_id.unwrap_or_else(Uuid::new_v4),
        auth.user_id,
        &messages,
    );

    let eval_context = if let Some(char_id) = request.character_id {
        eval_context.with_character(char_id)
    } else {
        eval_context
    };

    let evaluator = ContextEvaluator::with_simple_tokenizer();
    // TODO: Register actual providers based on available sources

    let result = evaluator.evaluate(&template, &eval_context)?;

    let segments: Vec<PreviewSegment> = result
        .segments
        .iter()
        .map(|seg| {
            let (source_type, source_id) = match &seg.source {
                SegmentSource::TextNode { .. } => ("text".to_string(), None),
                SegmentSource::SourceNode {
                    source_id,
                    node_id,
                } => (
                    "source".to_string(),
                    Some(format!("{}:{}", source_id, node_id)),
                ),
                SegmentSource::GroupWrapper { is_prefix, .. } => {
                    let wrapper_type = if *is_prefix { "prefix" } else { "suffix" };
                    ("wrapper".to_string(), Some(wrapper_type.to_string()))
                }
            };

            PreviewSegment {
                source_type,
                source_id,
                content: seg.content.clone(),
                role: seg.role.as_ref().map(|r| format!("{:?}", r).to_lowercase()),
                token_count: seg.token_count,
            }
        })
        .collect();

    Ok(Json(PreviewResponse {
        combined_text: result.to_string(),
        total_tokens: result.total_tokens,
        segments,
    }))
}
