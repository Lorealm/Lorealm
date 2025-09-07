use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::CookieJar;
use tokio::sync::RwLock;

use crate::{state::AppState, systems::{context_manager::ContextBuilder, context_template::{ContextTemplate, TemplateEntry, TemplateGroup}}, users::{manager::User, user_session::UserState}};

pub async fn create_template(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    user: axum::Extension<User>,
    session_arc: axum::Extension<Arc<RwLock<UserState>>>,
) -> Result<String, String> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or::<String>("Unauthorized; no session token found".into())?;
    
    let mut user_state = session_arc.write().await; // Acquire write lock
    if user_state.context_manager().is_some() {
        match user_state.context_manager().unwrap().save(&state, token).await {
            Ok(_) => return Ok("Successfully saved context template".into()),
            Err(e) => return Err(e),
        }
    }
    let mut context_builder = ContextBuilder::new();

    context_builder.set_template(ContextTemplate::new("test template"));

    let mut entry = TemplateEntry::new("test");
    let entry_id = entry.id.clone();

    entry.set_content("test content");
    let _ = context_builder.add_entry(entry, (0, None).into())
        .map_err(|e| format!("{:?}", e))?;

    let group = TemplateGroup::new("test group");
    let mut entry2 = TemplateEntry::new("test2");
    entry2.set_content("heloo!");

    context_builder.add_group(group, 0)
        .map_err(|e| format!("{:?}", e))?;

    context_builder.add_entry(entry2, (0,0).into())
        .map_err(|e| format!("{:?}", e))?;

    context_builder.move_entry(entry_id, (0,1).into())
        .map_err(|e| format!("{:?}", e))?;

    let mutable_entry = context_builder.get_entry_mut(entry_id);
    mutable_entry.unwrap().set_content("test content through mutation");

    user_state.set_context_manager(context_builder);
    Ok(format!("Context Manager not yet initialized; Initializing now..."))
}