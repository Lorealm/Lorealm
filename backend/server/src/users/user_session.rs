use uuid::Uuid;

use crate::systems::context_manager::ContextBuilder;
use std::time::{Duration, Instant};

/// The main container for user-to-user data.
/// 
/// Currently, the session is invalidated after 30 minutes. Later this will be configurable via a server config file
#[derive(Clone)]
pub struct UserState {
    user: Uuid,
    token: String,
    expires_at: Instant,
    max_life: Duration,
    context_manager: ContextBuilder,
}

impl UserState {
    pub fn new(user: Uuid, token: String) -> Self {
        let max_life = Duration::from_secs(30 * 60);
        Self {
            user,
            token,
            expires_at: Instant::now() + max_life,
            max_life,
            context_manager: ContextBuilder::new(),
        }
    }

    fn refresh(&mut self) {
        self.expires_at = Instant::now() + self.max_life;
    }

    /// Refreshes the session's TTL
    pub fn touch(&mut self) {
        self.refresh();
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    pub fn context_manager(&mut self) -> &ContextBuilder {
        &self.context_manager
    }

    pub fn set_token(&mut self, token: String) -> () {
        self.token = token
    }
}
