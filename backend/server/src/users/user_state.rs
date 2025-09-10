use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{resources::{resource_config::ResourceError}, state::AppState, systems::context_manager::ContextBuilder};
use std::{time::{Duration, Instant}};

#[derive(Clone, Serialize, Deserialize)]
pub struct UserState {
    pub username: String,
    pub user_id: Uuid,
    pub profile_picture: String,
    pub active_template: Option<Uuid>
}

/// The main container for user-to-user data.
/// 
/// Currently, the session is invalidated after 30 minutes. Later this will be configurable via a server config file
#[derive(Clone)]
pub struct UserStateManager {
    expires_at: Instant,
    state: UserState,
    max_life: Duration,
    last_interact: Instant
}

impl UserStateManager {
    pub async fn load(app_state: &AppState, token: &str) -> Result<Self, ResourceError> {
        Ok(Self::from(Self::load_state(app_state, token).await?))
    }

    async fn load_state(app_state: &AppState, token: &str) -> Result<UserState, ResourceError> {
        let resource_manager = app_state.resource_manager.lock().await;
        let user_resources = resource_manager.user_resources(token, app_state).await?;
        let state = user_resources.load_user_state(app_state).await;

        match state {
            Ok(state) => {
                Ok(state)
            },
            Err(_) => {
                let user_mgr = app_state.user_manager.read().await;
                let user = user_mgr.validate_session(token).await
                    .map_err(|_| ResourceError::PermissionDenied)?;

                Ok(UserState {
                    username: user.username,
                    user_id: user.id,
                    profile_picture: String::new(),
                    active_template: None,
                })
            },
        }
    }

    fn refresh(&mut self) {
        self.expires_at = Instant::now() + self.max_life;
    }

    /// Refreshes the session's TTL
    pub fn touch(&mut self) {
        self.last_interact = Instant::now();
        self.refresh();
    }

    /// Checks whether the given context template ID is the current active one
    /// and updates it if it is.
    pub async fn touch_template_id(&mut self, id: Uuid, app_state: &AppState, token: &str) -> Result<(), ResourceError> {
        self.touch();
        if self.state.active_template.is_none() { return Ok(()) }
        if self.state.active_template.unwrap() == id {
            let state = Self::load_state(app_state, token).await?;
            self.state = state;
        }
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
}   

    pub fn set_context_template(&mut self, template_id: Option<Uuid>) {
        self.state.active_template = template_id
    }

    pub async fn context_manager(&mut self, app_state: &AppState, credentials: &str) -> Result<ContextBuilder, ResourceError> {
        if self.state.active_template.is_none() {
            return Err(ResourceError::PermissionDenied);
        }

        let resource_manager = app_state.resource_manager.lock().await;
        let user_resources = resource_manager.user_resources(credentials, app_state).await?;
        let template = user_resources.load_context_template(self.state.active_template.unwrap(), app_state).await?;

        drop(user_resources);
        drop(resource_manager);

        return Ok(ContextBuilder::from(&template));
    }

    pub fn get_state(&mut self) -> UserState {
        return self.state.clone();
    }

    // /// Temporary, down the road we want to use signed links for pfps, peferably using a single-use JWT token for client-side access
    // fn populate_state(&mut self, user_resources: UserResources) -> UserState {
    //     let username = String::from("temporary"); // will be read from settings file eventually
    //     let pfp_link = match user_resources.get_pfp() {
    //         Ok(path) => {
    //             let filename = path
    //                 .path()
    //                 .file_name()
    //                 .and_then(|f| f.to_str())
    //                 .unwrap_or("default.png");

    //             format!("/api/users/{}/{}", self.state.user_id, filename)
    //         }
    //         Err(_) => String::from("default.png"),
    //     };

    //     let state = UserState { 
    //         username,
    //         user_id: self.state.user_id,
    //         profile_picture: pfp_link,
    //         active_template: self.state.active_template
    //     };

    //     self.state = state.clone();

    //     state
    // }
}

impl From<UserState> for UserStateManager {
    fn from(value: UserState) -> Self {
        let max_life = Duration::from_secs(30 * 60);
        Self {
            expires_at: Instant::now() + max_life,
            max_life,
            last_interact: Instant::now(),
            state: value
        }
    }
}