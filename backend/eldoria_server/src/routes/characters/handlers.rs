use std::{borrow::Cow, sync::Arc};

use axum::{
    Json,
    extract::{Multipart, State},
};

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::characters::types::ImportResult,
    services::permission::Permissions,
};

pub async fn import_card(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    perms: ResolvedPermissions,
    mut multipart: Multipart,
) -> Result<Json<super::dto::ImportCharResponse>, AppError> {
    perms.require(Permissions::IMPORT_3RD_PARTY_CHARACTER)?;

    let mut results: Vec<ImportResult> = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => {
                return Err(AppError::BadRequest {
                    code: "MALFORMED_MULTIPART",
                    message: Cow::Borrowed("Failed to read multipart stream."),
                });
            }
        };

        if field.file_name().is_none() {
            continue;
        }

        let data = field.bytes().await.map_err(|_| AppError::BadRequest {
            code: "INVALID_FILE",
            message: Cow::Borrowed("Failed to read file."),
        })?;

        if data.is_empty() {
            results.push(ImportResult::failed("File is empty."));
            continue;
        }

        // Best-effort: record per-file success/failure instead of aborting.
        match state
            .services()
            .characters()
            .import_from_png(data.as_ref(), auth.user_id)
            .await
        {
            Ok(id) => results.push(ImportResult::ok(id)),
            Err(e) => results.push(ImportResult::failed(&e.to_string())),
        }
    }

    if results.is_empty() {
        return Err(AppError::BadRequest {
            code: "NO_FILES",
            message: Cow::Borrowed("No files were provided."),
        });
    }

    Ok(Json(super::dto::ImportCharResponse { results }))
}
