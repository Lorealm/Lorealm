use crate::UserStorage;
use crate::error::StorageError;
use crate::models::{CreateInvite, CreateUser, Invite, Snowflake, UpdateUser, User};
use async_trait::async_trait;
use chrono::Utc;

use super::MemoryStorage;

#[async_trait]
impl UserStorage for MemoryStorage {
    async fn create_user(&self, input: &CreateUser) -> Result<User, StorageError> {
        let mut users = self.users.write().await;

        if users.contains_key(&input.id) {
            return Err(StorageError::Conflict(format!(
                "User {} already exists",
                input.id
            )));
        }

        if users.values().any(|u| u.username == input.username) {
            return Err(StorageError::Conflict("Username already taken".into()));
        }

        // Several users may have no email, as in the SQL backends' UNIQUE column.
        if input.email.is_some() && users.values().any(|u| u.email == input.email) {
            return Err(StorageError::Conflict("Email already taken".into()));
        }

        let now = Utc::now();
        let user = User {
            id: input.id,
            username: input.username.clone(),
            email: input.email.clone(),
            display_name: None,
            avatar_uri: None,
            password_hash: input.password_hash.clone(),
            created_at: now,
            updated_at: now,
        };

        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn get_user_by_id(&self, id: Snowflake) -> Result<Option<User>, StorageError> {
        Ok(self.users.read().await.get(&id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, StorageError> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.email.as_deref() == Some(email))
            .cloned())
    }

    async fn update_user(&self, id: Snowflake, input: &UpdateUser) -> Result<User, StorageError> {
        let mut users = self.users.write().await;

        let user = users
            .get_mut(&id)
            .ok_or_else(|| StorageError::NotFound(format!("User {id} not found")))?;

        if let Some(ref email) = input.email {
            user.email = Some(email.clone());
        }
        if let Some(ref display_name) = input.display_name {
            user.display_name = display_name.clone();
        }
        if let Some(ref avatar_uri) = input.avatar_uri {
            user.avatar_uri = Some(avatar_uri.clone());
        }
        if let Some(ref hash) = input.password_hash {
            user.password_hash = hash.clone();
        }
        user.updated_at = Utc::now();

        Ok(user.clone())
    }

    async fn delete_user(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.users.write().await.remove(&id).is_some())
    }

    async fn list_users(&self) -> Result<Vec<User>, StorageError> {
        let mut users: Vec<User> = self.users.read().await.values().cloned().collect();
        users.sort_by_key(|u| (u.created_at, u.id));
        Ok(users)
    }

    async fn create_invite(&self, input: &CreateInvite) -> Result<Invite, StorageError> {
        let mut invites = self.invites.write().await;

        if invites.values().any(|i| i.code_hash == input.code_hash) {
            return Err(StorageError::Conflict("Invite code already exists".into()));
        }

        let invite = Invite {
            id: input.id,
            code_hash: input.code_hash.clone(),
            role_id: input.role_id,
            created_by: input.created_by,
            max_uses: input.max_uses,
            uses: 0,
            expires_at: input.expires_at,
            created_at: Utc::now(),
        };

        invites.insert(invite.id, invite.clone());
        Ok(invite)
    }

    async fn list_invites(&self) -> Result<Vec<Invite>, StorageError> {
        let mut invites: Vec<Invite> = self.invites.read().await.values().cloned().collect();
        invites.sort_by_key(|i| std::cmp::Reverse((i.created_at, i.id)));
        Ok(invites)
    }

    async fn delete_invite(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.invites.write().await.remove(&id).is_some())
    }

    async fn redeem_invite(&self, code_hash: &str) -> Result<Option<Invite>, StorageError> {
        let mut invites = self.invites.write().await;
        let now = Utc::now();

        let Some(invite) = invites.values_mut().find(|i| {
            i.code_hash == code_hash
                && i.max_uses.is_none_or(|max| i.uses < max)
                && i.expires_at.is_none_or(|at| at > now)
        }) else {
            return Ok(None);
        };

        invite.uses += 1;
        Ok(Some(invite.clone()))
    }

    async fn release_invite(&self, id: Snowflake) -> Result<(), StorageError> {
        if let Some(invite) = self.invites.write().await.get_mut(&id) {
            invite.uses = (invite.uses - 1).max(0);
        }
        Ok(())
    }
}
