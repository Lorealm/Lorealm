use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use lorealm_storage::models::{ConversationMessage, ConversationMetadata, Snowflake};
use futures_util::StreamExt;

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::chat::{
        StartChatRequest,
        dto::{ChatEdit, CompletionsRequest, GetConversationQuery},
    },
    services::{chat::FullConversation, state::ReconstructedState},
};

pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<ConversationMetadata>>, AppError> {
    let convos = state
        .services()
        .chat()
        .list_conversations(session.user_id)
        .await?;

    Ok(Json(convos))
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
) -> Result<Json<Vec<ConversationMessage>>, AppError> {
    let messages = state
        .services()
        .chat()
        .list_messages(conversation_id, session.user_id)
        .await?;

    Ok(Json(messages))
}

pub async fn get_conversation(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
    Query(query): Query<GetConversationQuery>,
) -> Result<Json<FullConversation>, AppError> {
    let convo = state
        .services()
        .chat()
        .get_full_conversation(conversation_id, session.user_id, query.full)
        .await?;

    Ok(Json(convo))
}

pub async fn start_chat(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<StartChatRequest>,
) -> Result<Json<FullConversation>, AppError> {
    let chat = state
        .services()
        .chat()
        .start_new(
            request.character_id,
            request.version_id,
            session.user_id,
            request.greeting_id,
            request.content,
        )
        .await?;

    Ok(Json(chat))
}

pub async fn get_message(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, message_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<ConversationMessage>, AppError> {
    let message = state
        .services()
        .chat()
        .get_message(conversation_id, message_id, session.user_id)
        .await?;

    Ok(Json(message))
}

pub async fn edit_message(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
    Json(request): Json<ChatEdit>,
) -> Result<Json<Vec<ConversationMessage>>, AppError> {
    let chat_service = state.services().chat();

    match request {
        ChatEdit::Edit { id, content } => {
            chat_service
                .edit_message(conversation_id, id, session.user_id, content)
                .await?;
        }
        ChatEdit::BranchUser { id, content } => {
            chat_service
                .branch_user_message(conversation_id, id, session.user_id, content)
                .await?;
        }
        ChatEdit::Activate { id } => {
            chat_service
                .activate_message(conversation_id, id, session.user_id)
                .await?;
        }
    }

    let conv = chat_service
        .list_messages(conversation_id, session.user_id)
        .await?;

    Ok(Json(conv))
}

pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .chat()
        .delete_conversation(conversation_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, message_id)): Path<(Snowflake, Snowflake)>,
) -> Result<(), AppError> {
    state
        .services()
        .chat()
        .delete_message(conversation_id, message_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn get_message_state(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, message_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<ReconstructedState>, AppError> {
    let messages: Vec<Snowflake> = state
        .services()
        .chat()
        .get_path_to(conversation_id, message_id, session.user_id)
        .await?
        .iter()
        .map(|m| m.id)
        .collect();

    let state = state.services().state().reconstruct(&messages).await?;

    Ok(Json(state))
}

pub async fn completions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
    Json(request): Json<CompletionsRequest>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, axum::Error>>>, AppError> {
    let events = state
        .services()
        .chat()
        .generate(
            conversation_id,
            session.user_id,
            request.content,
            request.target,
        )
        .await?;

    let sse = events.map(|item| match item {
        Ok(event) => Event::default().json_data(event),
        Err(err) => Event::default()
            .event("error")
            .json_data(serde_json::json!({ "message": err.to_string() })),
    });

    Ok(Sse::new(sse).keep_alive(KeepAlive::default()))
}
