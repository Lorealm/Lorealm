use std::sync::Arc;

use lorealm_storage::models::User;
use serde_json::json;

use crate::{core::AppState, errors::AppError, services::connection::ProfileInput};

pub async fn scaffold_user(state: &Arc<AppState>, user: &User) -> Result<(), AppError> {
    // Create the default persona
    let persona_id = state
        .services()
        .persona()
        .create_default(user.id, user.username.clone())
        .await?;

    state
        .services()
        .state()
        .set_active_persona(user.id, persona_id)
        .await?;

    // Create the default context template
    let template_id = state
        .services()
        .context()
        .create_template(user.id, None, None, true)
        .await?
        .id;

    state
        .services()
        .state()
        .set_active_template(user.id, Some(template_id))
        .await?;

    let profile = state
        .services()
        .connections()
        .create_profile(
            user.id,
            ProfileInput {
                secret_id: None,
                name: "Default Connection Profile".to_string(),
                model: None,
                token_budget: 10240,
                provider: "openrouter".to_string(),
                common_params: json!({}),
                provider_params: json!({}),
            },
        )
        .await?;

    state
        .services()
        .state()
        .set_active_profile(user.id, Some(profile.id))
        .await?;

    Ok(())
}
