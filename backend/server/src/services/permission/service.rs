use std::sync::Arc;

use lorealm_storage::{
    Storage, StorageError,
    models::{CreateRole, Role as StoredRole, Snowflake, UpdateRole, User},
};

use crate::{
    errors::PermissionError,
    services::{
        permission::{ADMINISTRATOR_ROLE_ID, EVERYONE_ROLE_ID, Permissions, Role},
        snowflake::SnowflakeService,
    },
};

pub struct PermissionService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
}

/// Who is managing roles, as far as the guard cares.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    /// Effective permissions.
    pub held: Permissions,
    /// Position of the actor's highest role.
    pub top: i32,
}

impl Actor {
    fn is_administrator(&self) -> bool {
        self.held.contains(Permissions::ADMINISTRATOR)
    }

    fn check_outranks(&self, role: &StoredRole) -> Result<(), PermissionError> {
        if self.is_administrator() || role.position < self.top {
            Ok(())
        } else {
            Err(PermissionError::Outranked)
        }
    }

    fn check_position(&self, position: i32) -> Result<(), PermissionError> {
        if position < 1 {
            Err(PermissionError::InvalidPosition)
        } else if position >= self.top {
            Err(PermissionError::Outranked)
        } else {
            Ok(())
        }
    }

    fn check_may_grant(&self, permissions: Permissions) -> Result<(), PermissionError> {
        let grantable = if self.is_administrator() {
            Permissions::all()
        } else {
            self.held
        };
        let missing = permissions - grantable;
        if missing.is_empty() {
            Ok(())
        } else {
            Err(PermissionError::Missing(missing))
        }
    }

    fn check_may_assign(&self, role: &StoredRole) -> Result<(), PermissionError> {
        if role.id == EVERYONE_ROLE_ID {
            return Err(PermissionError::BuiltIn);
        }
        self.check_outranks(role)?;
        self.check_may_grant(Permissions::from_db(role.permissions))
    }
}

fn clean_name(name: &str) -> Result<String, PermissionError> {
    let name = name.trim();
    if name.is_empty() {
        Err(PermissionError::InvalidName)
    } else {
        Ok(name.to_owned())
    }
}

impl PermissionService {
    pub fn new(storage: Arc<dyn Storage>, snowflake: Arc<SnowflakeService>) -> Self {
        Self { storage, snowflake }
    }

    pub async fn get_user_roles(&self, user_id: Snowflake) -> Result<Vec<Role>, PermissionError> {
        Ok(self
            .storage
            .get_user_roles(user_id)
            .await?
            .iter()
            .map(Role::from)
            .collect())
    }

    /// Everything the user may do: `@everyone` united with each role they hold.
    pub async fn resolve(&self, user_id: Snowflake) -> Result<Permissions, PermissionError> {
        let everyone = self
            .storage
            .get_role(EVERYONE_ROLE_ID)
            .await?
            .map(|r| Permissions::from_db(r.permissions))
            .unwrap_or_else(Permissions::empty);

        Ok(self
            .get_user_roles(user_id)
            .await?
            .iter()
            .fold(everyone, |acc, role| acc | role.permissions))
    }

    pub async fn actor(&self, user_id: Snowflake) -> Result<Actor, PermissionError> {
        let roles = self.get_user_roles(user_id).await?;
        let held = self.resolve(user_id).await?.effective();
        let mut top = roles.iter().map(|r| r.position).max().unwrap_or(0).max(0);
        if held.contains(Permissions::ADMINISTRATOR)
            && let Some(admin) = self.storage.get_role(ADMINISTRATOR_ROLE_ID).await?
        {
            top = top.max(admin.position);
        }
        Ok(Actor { held, top })
    }

    /// Makes the user an administrator if nobody is one yet, so the first
    /// account on a server can run it.
    pub async fn claim_administrator_if_unclaimed(
        &self,
        user_id: Snowflake,
    ) -> Result<bool, PermissionError> {
        if self
            .storage
            .count_role_members(ADMINISTRATOR_ROLE_ID)
            .await?
            > 0
        {
            return Ok(false);
        }
        self.storage
            .assign_role(user_id, ADMINISTRATOR_ROLE_ID)
            .await?;
        Ok(true)
    }

    pub async fn check_may_manage_user(
        &self,
        actor: Snowflake,
        target: Snowflake,
    ) -> Result<(), PermissionError> {
        let actor = self.actor(actor).await?;
        if actor.is_administrator() || self.actor(target).await?.top < actor.top {
            Ok(())
        } else {
            Err(PermissionError::UserOutranked)
        }
    }

    pub async fn check_may_delete_user(
        &self,
        actor: Snowflake,
        target: Snowflake,
    ) -> Result<(), PermissionError> {
        if actor == target {
            return Err(PermissionError::OwnAccount);
        }
        self.check_may_manage_user(actor, target).await?;

        let is_admin = self
            .storage
            .get_user_roles(target)
            .await?
            .iter()
            .any(|r| r.id == ADMINISTRATOR_ROLE_ID);
        if is_admin
            && self
                .storage
                .count_role_members(ADMINISTRATOR_ROLE_ID)
                .await?
                <= 1
        {
            return Err(PermissionError::LastAdministrator);
        }
        Ok(())
    }

    /// Whether the actor could give this role to someone directly.
    pub async fn check_may_assign(
        &self,
        actor: Snowflake,
        role_id: Snowflake,
    ) -> Result<(), PermissionError> {
        let actor = self.actor(actor).await?;
        actor.check_may_assign(&self.role(role_id).await?)
    }

    /// Every role, highest first.
    pub async fn list_roles(&self) -> Result<Vec<StoredRole>, PermissionError> {
        let mut roles = self.storage.get_all_roles().await?;
        roles.sort_by(|a, b| {
            b.position
                .cmp(&a.position)
                .then(a.created_at.cmp(&b.created_at))
        });
        Ok(roles)
    }

    async fn role(&self, id: Snowflake) -> Result<StoredRole, PermissionError> {
        self.storage
            .get_role(id)
            .await?
            .ok_or(PermissionError::RoleNotFound(id))
    }

    async fn check_name_free(&self, name: &str) -> Result<(), PermissionError> {
        match self.storage.get_role_by_name(name).await? {
            Some(_) => Err(PermissionError::NameTaken),
            None => Ok(()),
        }
    }

    pub async fn create_role(
        &self,
        actor: Snowflake,
        name: &str,
        permissions: Permissions,
        position: i32,
    ) -> Result<StoredRole, PermissionError> {
        let actor = self.actor(actor).await?;
        let name = clean_name(name)?;
        actor.check_position(position)?;
        actor.check_may_grant(permissions)?;
        self.check_name_free(&name).await?;

        Ok(self
            .storage
            .create_role(&CreateRole {
                id: self.snowflake.generate(),
                name,
                permissions: permissions.bits(),
                position,
            })
            .await?)
    }

    pub async fn update_role(
        &self,
        actor: Snowflake,
        id: Snowflake,
        name: Option<&str>,
        permissions: Option<Permissions>,
        position: Option<i32>,
    ) -> Result<StoredRole, PermissionError> {
        let actor = self.actor(actor).await?;
        let role = self.role(id).await?;
        actor.check_outranks(&role)?;

        let name = name.map(clean_name).transpose()?;
        let name_changed = name.as_ref().is_some_and(|n| *n != role.name);
        let position_changed = position.is_some_and(|p| p != role.position);

        if id == EVERYONE_ROLE_ID && (name_changed || position_changed) {
            return Err(PermissionError::BuiltIn);
        }
        // Moving it, or taking its flag away, could leave nobody able to run
        // the server.
        if id == ADMINISTRATOR_ROLE_ID
            && (position_changed
                || permissions.is_some_and(|p| !p.contains(Permissions::ADMINISTRATOR)))
        {
            return Err(PermissionError::BuiltIn);
        }
        if let Some(position) = position.filter(|_| position_changed) {
            actor.check_position(position)?;
        }
        if let Some(permissions) = permissions {
            // Only the flags being switched need to be the actor's to give.
            actor.check_may_grant(permissions ^ Permissions::from_db(role.permissions))?;
        }
        if let Some(name) = name.as_deref().filter(|_| name_changed) {
            self.check_name_free(name).await?;
        }

        Ok(self
            .storage
            .update_role(
                id,
                &UpdateRole {
                    name,
                    permissions: permissions.map(|p| p.bits()),
                    position,
                },
            )
            .await?)
    }

    pub async fn delete_role(
        &self,
        actor: Snowflake,
        id: Snowflake,
    ) -> Result<(), PermissionError> {
        let actor = self.actor(actor).await?;
        let role = self.role(id).await?;
        if id == EVERYONE_ROLE_ID || id == ADMINISTRATOR_ROLE_ID {
            return Err(PermissionError::BuiltIn);
        }
        actor.check_outranks(&role)?;
        self.storage.delete_role(id).await?;
        Ok(())
    }

    pub async fn role_members(&self, id: Snowflake) -> Result<Vec<User>, PermissionError> {
        if id == EVERYONE_ROLE_ID {
            return Err(PermissionError::BuiltIn);
        }
        self.role(id).await?;
        Ok(self.storage.get_role_members(id).await?)
    }

    /// Gives the role to the user with that username. Giving it again is a
    /// no-op.
    pub async fn assign_role(
        &self,
        actor: Snowflake,
        role_id: Snowflake,
        username: &str,
    ) -> Result<User, PermissionError> {
        let actor = self.actor(actor).await?;
        let role = self.role(role_id).await?;
        actor.check_may_assign(&role)?;

        let user = self
            .storage
            .get_user_by_username(username.trim())
            .await?
            .ok_or(PermissionError::UserNotFound)?;

        match self.storage.assign_role(user.id, role_id).await {
            Ok(_) | Err(StorageError::Conflict(_)) => Ok(user),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn revoke_role(
        &self,
        actor: Snowflake,
        role_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), PermissionError> {
        let actor = self.actor(actor).await?;
        let role = self.role(role_id).await?;
        actor.check_may_assign(&role)?;

        if role_id == ADMINISTRATOR_ROLE_ID && self.storage.count_role_members(role_id).await? <= 1
        {
            return Err(PermissionError::LastAdministrator);
        }

        self.storage.revoke_role(user_id, role_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn role(id: i64, permissions: Permissions, position: i32) -> StoredRole {
        StoredRole {
            id: Snowflake(id),
            name: format!("role{id}"),
            permissions: permissions.bits(),
            position,
            created_at: Utc::now(),
        }
    }

    fn actor(held: Permissions, top: i32) -> Actor {
        Actor {
            held: held.effective(),
            top,
        }
    }

    #[test]
    fn only_roles_strictly_below_can_be_managed() {
        let a = actor(Permissions::MANAGE_ROLES, 10);
        assert!(a.check_outranks(&role(10, Permissions::empty(), 9)).is_ok());
        assert!(matches!(
            a.check_outranks(&role(10, Permissions::empty(), 10)),
            Err(PermissionError::Outranked)
        ));
    }

    #[test]
    fn positions_must_be_positive_and_below_the_actor() {
        let a = actor(Permissions::MANAGE_ROLES, 10);
        assert!(a.check_position(9).is_ok());
        assert!(matches!(
            a.check_position(0),
            Err(PermissionError::InvalidPosition)
        ));
        assert!(matches!(
            a.check_position(10),
            Err(PermissionError::Outranked)
        ));
    }

    #[test]
    fn only_held_permissions_can_be_granted() {
        let a = actor(Permissions::MANAGE_ROLES | Permissions::USE_PLUGINS, 10);
        assert!(a.check_may_grant(Permissions::USE_PLUGINS).is_ok());
        match a.check_may_grant(Permissions::USE_PLUGINS | Permissions::ADMINISTRATOR) {
            Err(PermissionError::Missing(m)) => assert_eq!(m, Permissions::ADMINISTRATOR),
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn administrators_can_grant_the_exempt_flags() {
        let a = actor(Permissions::ADMINISTRATOR, 1000);
        assert!(a.check_may_grant(Permissions::MANAGE_SERVER).is_ok());
        assert!(a.check_may_grant(Permissions::ADMIN_EXEMPT).is_ok());
    }

    #[test]
    fn everyone_cannot_be_assigned() {
        let a = actor(Permissions::ADMINISTRATOR, 1000);
        assert!(matches!(
            a.check_may_assign(&role(1, Permissions::empty(), 0)),
            Err(PermissionError::BuiltIn)
        ));
    }

    #[test]
    fn administrators_manage_roles_at_any_position() {
        let admin_role = role(
            2,
            Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS,
            1000,
        );
        let admin = actor(
            Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS,
            1000,
        );
        assert!(admin.check_outranks(&admin_role).is_ok());
        assert!(admin.check_may_assign(&admin_role).is_ok());

        let moderator = actor(Permissions::MANAGE_ROLES, 1000);
        assert!(matches!(
            moderator.check_outranks(&admin_role),
            Err(PermissionError::Outranked)
        ));
    }
}
