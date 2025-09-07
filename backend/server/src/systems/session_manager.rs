use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::users::user_session::UserState;

pub struct StateManager {
    sessions: HashMap<String, Arc<RwLock<UserState>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn get_session(&self, session_token: &str) -> Option<&Arc<RwLock<UserState>>> {
        self.sessions.get(session_token)
    }

    pub fn insert_session(&mut self, session_token: String, user_session: UserState) {
        self.sessions
            .insert(session_token, Arc::new(RwLock::new(user_session)));
    }

    pub fn migrate_session(
        &mut self,
        old_token: String,
        new_token: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.sessions.remove(&old_token) {
            Some(session) => {
                let mut session = session.blocking_write();
                session.set_token(new_token.clone());
                self.sessions.insert(new_token, Arc::new(RwLock::new(session.clone())));
                Ok(())
            }
            None => Err(format!("session '{}' not found", old_token).into()),
        }
    }

    pub fn remove_session(&mut self, token: &str) {
        self.sessions.remove(token);
    }
}