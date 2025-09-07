use jsonwebtoken::{DecodingKey, EncodingKey};
use uuid::Uuid;

use crate::{jwt::jwt_utils::{issue_scoped_token, verify_and_authorize, FileAccessClaim}, users::manager::UserManager};

const JWT_AUDIENCE: &str = "file-access";
const JWT_TTL: i64 = 3600; // 1 hour
const TEST_KEY: &str = "test-key";
pub struct ResourceAuthenticator {
    db: sqlx::Pool<sqlx::Postgres>
}

impl ResourceAuthenticator {
    pub fn new(db: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self { db }
    }

    /// Authenticates a session token and generates a new JWT token for file access
    /// 
    /// **Returns:** A new JWT token for file access, along with the file access claim
    pub async fn authenticate_session(&self, token: &str, user_manager: &UserManager, is_system: bool) -> Result<(String, FileAccessClaim), Box<dyn std::error::Error>> {
        let user_id = match is_system {
            true => Uuid::nil(),
            false => user_manager.validate_session(token).await?.id,
        };

        let resource = FileAccessClaim {
            path: "*".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            user: user_id,
        };

        let token = issue_scoped_token("eldora-hub-server", &user_id.to_string(), JWT_AUDIENCE, resource.clone(), JWT_TTL, &EncodingKey::from_secret(TEST_KEY.as_bytes()));
        match token {
            Ok(tok) => Ok((tok, resource)),
            Err(e) => Err(e.into()),
        }
    }

    pub fn authenticate_jwt(&self, token: &str) -> Result<FileAccessClaim, Box<dyn std::error::Error>> {
        let data_claims = verify_and_authorize(token, &DecodingKey::from_secret(TEST_KEY.as_bytes()), "eldora-hub-server", JWT_AUDIENCE, "*", "read");
        match data_claims {
            Ok(data) => Ok(data.res),
            Err(e) => {
                log::warn!("Failed to decode claims {}", e);
                Err(e.into())
            },
        }
    }
}