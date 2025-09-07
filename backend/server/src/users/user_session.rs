use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{llm::prompt::types::ContextEntry, resources::user_resources::UserResources, systems::context_manager::ContextBuilder};
use std::{task::Context, time::{Duration, Instant}};

/// The JSON payload for a [`UserState`]
#[derive(Clone, Serialize, Deserialize)]
pub struct StatePayload {
    username: String,
    user_id: Uuid,
    profile_picture: String
}

/// The main container for user-to-user data.
/// 
/// Currently, the session is invalidated after 30 minutes. Later this will be configurable via a server config file
#[derive(Clone)]
pub struct UserState {
    user: Uuid,
    token: String,
    expires_at: Instant,
    max_life: Duration,
    context_manager: Option<ContextBuilder>,
    cached_state: Option<StatePayload>,
    last_interact: Instant,
    state_updated: bool
}

impl UserState {
    pub fn new(user: Uuid, token: String) -> Self {
        let max_life = Duration::from_secs(30 * 60);
        Self {
            user,
            token,
            expires_at: Instant::now() + max_life,
            max_life,
            context_manager: None,
            cached_state: None,
            last_interact: Instant::now(),
            state_updated: false
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

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
}   

    pub fn set_context_manager(&mut self, context_manager: ContextBuilder) {
        self.context_manager = Some(context_manager)
    }

    pub fn context_manager(&mut self) -> Option<&ContextBuilder> {
        self.context_manager.as_ref()
    }

    pub fn set_token(&mut self, token: String) -> () {
        self.token = token
    }

    pub fn get_state(&mut self, user_resources: UserResources) -> StatePayload {
        if let Some(state) = &self.cached_state && !self.state_updated {
            return state.clone();
        }

        self.populate_state(user_resources)
    }

    /// Temporary, down the road we want to use signed links for pfps, peferably using a single-use JWT token for client-side access
    fn populate_state(&mut self, user_resources: UserResources) -> StatePayload {
        let username = String::from("temporary"); // will be read from settings file eventually
        let pfp_link = match user_resources.get_pfp() {
            Ok(path) => {
                let filename = path
                    .path()
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("default.png");

                format!("/api/users/{}/{}", self.user, filename)
            }
            Err(_) => String::from("default.png"),
        };

        StatePayload { username, user_id: self.user, profile_picture: pfp_link }
    }
}
