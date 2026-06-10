use bitflags::bitflags;
use lorealm_storage::models::Snowflake;

pub mod service;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: i64 {
        const MANAGE_CHARACTERS                 = 1 << 0;
        const MANAGE_LOREBOOKS                  = 1 << 1;
        const MANAGE_WORLDS                     = 1 << 2;
        const MANAGE_PRESETS                    = 1 << 3;
        const ALLOW_WEAVER_SCRIPTING            = 1 << 4;
        // Imports
        const IMPORT_3RD_PARTY_CHARACTER        = 1 << 5;
        const IMPORT_3RD_PARTY_LOREBOOK         = 1 << 6;
        const IMPORT_3RD_PARTY_WORLD            = 1 << 7;
        const IMPORT_3RD_PARTY_PRESETS          = 1 << 8;

        // Plugins
        const ALLOW_TRUSTED_PLUGINS             = 1 << 9;
        const ALLOW_UNSAFE_PLUGINS              = 1 << 10;

        // Moderation
        const CREATE_USERS                       = 1 << 11;
        const EDIT_USERS                         = 1 << 12;
        const DELETE_USERS                       = 1 << 13;
        const MANAGE_PERMISSIONS                 = 1 << 14;
        const VIEW_METRICS                       = 1 << 15;
        const EDIT_OWN_USER                      = 1 << 16;

        const ADMINISTRATOR                      = 1 << 62;
    }
}

impl Permissions {
    /// Wrap a raw i64 from the database into a Permissions value.
    pub fn from_db(bits: i64) -> Self {
        Self::from_bits_truncate(bits)
    }

    /// Extract the raw i64 for writing back to the database.
    pub fn to_db(self) -> i64 {
        self.bits()
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions::MANAGE_CHARACTERS
            | Permissions::MANAGE_LOREBOOKS
            | Permissions::MANAGE_WORLDS
            | Permissions::MANAGE_PRESETS
            | Permissions::ALLOW_WEAVER_SCRIPTING
            | Permissions::ALLOW_TRUSTED_PLUGINS
            | Permissions::EDIT_OWN_USER
    }
}

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}"))
    }
}

#[derive(Debug, Clone)]
pub struct Role {
    pub id: Snowflake,
    pub name: String,
    pub permissions: Permissions,
    pub position: i32,
}

impl From<&lorealm_storage::models::Role> for Role {
    fn from(value: &lorealm_storage::models::Role) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            permissions: Permissions::from_db(value.permissions),
            position: value.position,
        }
    }
}