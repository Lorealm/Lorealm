use lorealm_storage::models::Session;

pub struct SessionCreated {
    pub session: Session,
    pub raw_token: String,
    pub raw_csrf_token: String,
}

pub struct ValidatedSession {
    pub session: Session,
    pub new_token: Option<String>,
}
