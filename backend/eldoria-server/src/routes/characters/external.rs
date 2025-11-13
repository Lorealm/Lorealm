use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    app_state::AppState, middleware::authentication::AuthenticatedSession,
    services::character::parser::parse_character_card,
};

pub async fn import_card(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    mut multipart: Multipart,
) -> Response {
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if let Some(filename) = field.file_name().map(|s| s.to_string()) {
            // Prevent directory traversal
            if filename.contains("..") || filename.contains('/') {
                return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
            }

            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read file").into_response(),
            };

            if data.len() == 0 {
                return (StatusCode::BAD_REQUEST, "File is empty").into_response()
            }
            
            let storage_lock = state.storage();
            let character_service = storage_lock.write().await.character_service();

            let chara_card = parse_character_card(data.iter().as_slice(), "chara", auth.user_id);

            if chara_card.is_err() {
                return (StatusCode::BAD_REQUEST, "Failed to parse card").into_response();
            }

            if let Err(_) = character_service
                .save_character(&chara_card.unwrap().character)
                .await
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to save character",
                )
                    .into_response();
            }

            return (StatusCode::OK, "Character imported successfully").into_response();
        }
    }

    (StatusCode::BAD_REQUEST, "No file provided").into_response()
}
