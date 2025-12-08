use encryption_service::Hashable;
use storage_system::Storage;
use uuid::Uuid;

use crate::models::{
    UserService,
    users::{error::UserError, permissions::UserPermissions},
};

/// A barebones struct used for identification. Does not contain any fancy information.
pub struct EldoriaUser {
    pub id: Uuid,
    pub permissions: UserPermissions,
}

pub struct EldoriaUserExtended {
    pub inner: EldoriaUser,
    pub name: String,
    pub email: Option<String>,
}

pub struct UserRegistration {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub password: String,
    pub permissions: UserPermissions,
}

pub struct UserAuthData {
    pub id: Uuid,
    pub name: String,
    pub password_hash: String,
    pub permissions: UserPermissions,
}

impl EldoriaUser {
    pub async fn create(
        username: String,
        password: String,
        permissions: Option<UserPermissions>,
    ) -> Result<UserRegistration, UserError> {
        let hashed_password = password
            .hash()
            .await
            .map_err(|e| UserError::RegistrationError(format!("Password hashing failed: {e}")))?;

        let perms = permissions.unwrap_or(UserPermissions::default_user());

        Ok(UserRegistration {
            id: Uuid::new_v4(),
            name: username,
            email: None,
            password: hashed_password,
            permissions: perms,
        })
    }

    pub async fn login<S>(
        username: String,
        password: String,
        service: &UserService<S>,
    ) -> Result<Self, UserError>
    where
        S: Storage + ?Sized,
    {
        let auth_data = service
            .get_auth_data(&username)
            .await
            .map_err(|x| UserError::InvalidUser(x.to_string()))?;

        match encryption_service::verify_password(&password, &auth_data.password_hash) {
            Ok(success) => {
                if !success {
                    return Err(UserError::IncorrectPassword);
                }

                Ok(Self {
                    id: auth_data.id,
                    permissions: auth_data.permissions,
                })
            }
            Err(e) => Err(UserError::Encryption(e)),
        }
    }
}

impl TryInto<EldoriaUserExtended> for storage_system::User {
    type Error = UserError;

    fn try_into(self) -> Result<EldoriaUserExtended, Self::Error> {
            let permissions = UserPermissions::from_bits(self.permissions)
                .ok_or(UserError::PermissionError { msg: "Could not parse permissions".to_string(), expected: UserPermissions::NONE })?;

            Ok(EldoriaUserExtended {
                inner: EldoriaUser {
                    id: self.id,
                    permissions,
                },
                name: self.name,
                email: self.email,
            })
        }
}
