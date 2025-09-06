use std::collections::HashMap;
use crate::users::user_session::UserState;

pub struct StateManager {
    sessions: HashMap<String, UserState>
}

impl StateManager {
    pub fn new() -> Self {
        Self { sessions: HashMap::new() }
    }

    // you had this signature; changed to borrow &str for convenience (feel free to adapt)
    pub fn get_session(&self, session_token: &str) -> Option<&UserState> {
        self.sessions.get(session_token)
    }

    pub fn insert_session(&mut self, session_token: String, user_session: UserState) {
        self.sessions.insert(session_token, user_session);
    }

    /// Move session from old_token -> new_token updating user_session.token as well.
    pub fn migrate_session(&mut self, old_token: String, new_token: String) -> Result<(), Box<dyn std::error::Error>> {
        match self.sessions.remove(&old_token) {
            Some(mut session) => {
                // update the token inside the session so it remains consistent
                session.set_token(new_token.clone());
                self.sessions.insert(new_token, session);
                Ok(())
            }
            None => Err(format!("session '{}' not found", old_token).into()),
        }
    }

    /// Remove a session explicitly
    pub fn remove_session(&mut self, token: &str) {
        self.sessions.remove(token);
    }
}
