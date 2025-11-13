use std::sync::Arc;

use chrono::Utc;
use storage_system::{Storage, StorageError};

use crate::models::{users::{UserAuthData, UserPermissions, UserRegistration}, StorageService};

pub struct UserService<S: Storage + ?Sized> {
    storage: Arc<S>,
}

impl<S: Storage + ?Sized> StorageService<S> for UserService<S> {
    fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

impl<S: Storage + ?Sized> UserService<S> {
    // Fetch only what's needed for authentication
    pub async fn get_auth_data(&self, username: &str) -> Result<UserAuthData, StorageError> {
        // Query: SELECT id, name, password_hash, permissions FROM users WHERE name = ?
        let user = self.storage.get_user_by_name(username).await?;

        if user.is_none() {
            return Err(StorageError::NotFound(format!(
                "User '{username}' not found."
            )));
        }

        let data = user.unwrap();

        if data.display_name.is_none() {
            return Err(StorageError::NotFound(format!(
                "Username was not returned from database!"
            )));
        }

        if data.password.is_none() {
            return Err(StorageError::NotFound(format!(
                "Password was not returned from database!"
            )));
        }

        if let Some(perms) = UserPermissions::from_bits(data.permissions) {
            return Ok(UserAuthData {
                id: data.id,
                name: data.display_name.unwrap(),
                password_hash: data.password.unwrap(),
                permissions: perms,
            });
        }

        Err(StorageError::InvalidOperation(format!(
            "Invalid permission bits: {}",
            data.permissions
        )))
    }

    pub async fn register_user(&self, registration_data: &UserRegistration) -> Result<(), StorageError> {
        let user = storage_system::User {
            id: registration_data.id, 
            permissions: registration_data.permissions.bits(), 
            email: registration_data.email.clone(), 
            display_name: Some(registration_data.name.clone()), 
            password: Some(registration_data.password.clone()), 
            created_at: Utc::now() 
        };
        
        let _ = self.storage.save_user(&user).await?;
        Ok(())
    }
}
