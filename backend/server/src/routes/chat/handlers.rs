use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use futures_util::StreamExt;
use lorealm_storage::models::{ConversationMessage, ConversationMetadata, RealmCast, Snowflake};
use reqwest::StatusCode;

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::chat::dto::{
        AgentStateQuery, ChatEdit, GetConversationQuery, SetAuthorNoteRequest, StartChatRequest,
        StartRealmChatRequest,
    },
    services::{
        chat::{ActionSpec, FullConversation, NextTurn, TurnRequest},
        state::{ReconstructedState, StatePool},
    },
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
        .start_with_character(
            session.user_id,
            request.character_id,
            request.version_id,
            request.greeting_id,
            request.author_note,
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

pub async fn get_message_context(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, message_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let context = state
        .services()
        .chat()
        .get_message_context(conversation_id, message_id, session.user_id)
        .await?;

    Ok(Json(serde_json::json!({ "context": context })))
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

/// Render one stream item as an SSE frame.
fn sse_event<T, E>(item: Result<T, E>) -> Result<Event, axum::Error>
where
    T: serde::Serialize,
    E: Into<AppError>,
{
    match item {
        Ok(event) => Event::default().json_data(event),
        Err(err) => Event::default()
            .event("error")
            .json_data(err.into().response_body()),
    }
}

pub async fn turn(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
    Json(request): Json<TurnRequest>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, axum::Error>>>, AppError> {
    let events = state
        .services()
        .chat()
        .turn(conversation_id, session.user_id, request)
        .await?;

    Ok(Sse::new(events.map(sse_event)).keep_alive(KeepAlive::default()))
}

pub async fn cancel_turn(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    let cancelled = state
        .services()
        .chat()
        .cancel(conversation_id, session.user_id)
        .await?;
    Ok(if cancelled {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    })
}

pub async fn start_realm_chat(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<StartRealmChatRequest>,
) -> Result<Json<FullConversation>, AppError> {
    let chat = state
        .services()
        .chat()
        .start_with_realm(session.user_id, request.realm_id, request.scenario)
        .await?;

    Ok(Json(chat))
}

pub async fn next_turn(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
) -> Result<Json<NextTurn>, AppError> {
    let handle = state
        .services()
        .chat()
        .next_agent(conversation_id, session.user_id)
        .await?;

    Ok(Json(NextTurn { handle }))
}

pub async fn get_cast(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
) -> Result<Json<RealmCast>, AppError> {
    let cast = state
        .services()
        .chat()
        .cast(conversation_id, session.user_id)
        .await?;

    Ok(Json(cast))
}

pub async fn agent_state(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, handle)): Path<(Snowflake, String)>,
    Query(query): Query<AgentStateQuery>,
) -> Result<Json<StatePool>, AppError> {
    let slice = state
        .services()
        .chat()
        .agent_state(conversation_id, &handle, session.user_id, query.at)
        .await?;

    Ok(Json(slice))
}

pub async fn list_agent_actions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((conversation_id, handle)): Path<(Snowflake, String)>,
) -> Result<Json<Vec<ActionSpec>>, AppError> {
    let actions = state
        .services()
        .chat()
        .agent_actions(conversation_id, &handle, session.user_id)
        .await?;

    Ok(Json(actions))
}

pub async fn set_author_note(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(conversation_id): Path<Snowflake>,
    Json(request): Json<SetAuthorNoteRequest>,
) -> Result<(), AppError> {
    state
        .services()
        .chat()
        .set_author_note(conversation_id, session.user_id, request.author_note)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use futures_util::stream;

    use super::*;
    use crate::errors::ChatError;

    #[tokio::test]
    async fn a_failed_stream_emits_an_error_frame() {
        let events = stream::iter(vec![
            Ok(serde_json::json!({"type": "delta", "content": "Hel"})),
            Err(ChatError::Connection("no api key".into())),
        ]);

        let response = Sse::new(events.map(sse_event)).into_response();
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body.contains(r#"data: {"type":"delta","content":"Hel"}"#),
            "{body}"
        );
        assert!(body.contains("event: error"), "{body}");
        assert!(
            body.contains(r#""error_code":"chat.provider_failed""#),
            "{body}"
        );
    }
}
