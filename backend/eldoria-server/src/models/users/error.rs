use encryption_service::EncryptionError;

use crate::models::users::permissions::UserPermissions;

#[derive(thiserror::Error, Debug)]
pub enum UserError {
    #[error("Registration error: {0}")]
    RegistrationError(String),
    #[error("Permission denied: {msg}. Expected: {expected}")]
    PermissionError { msg: String, expected: UserPermissions },
    #[error("Invalid user: {0}")]
    InvalidUser(String),
    #[error("Incorrect password")]
    IncorrectPassword,
    #[error("Encryption Error: {0}")]
    Encryption(EncryptionError)
}