use bitflags::bitflags;
use lorealm_storage::models::Snowflake;

pub mod service;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: i64 {
        /// Import character cards, realms, and other outside formats.
        const IMPORT_CONTENT     = 1 << 0;
        /// Upload tokenizers or have the server download them.
        const UPLOAD_TOKENIZERS  = 1 << 1;
        /// Make something visible to every user, such as a public tokenizer.
        const PUBLISH_PUBLIC     = 1 << 2;
        /// Point a connection profile at any URL. The server calls it, so this
        /// reaches localhost and the LAN too.
        const CUSTOM_ENDPOINTS   = 1 << 3;
        /// Enable, configure and call plugins already installed on the server.
        const USE_PLUGINS        = 1 << 16;
        /// Install pre-built plugins server-wide, from an upload or a GitHub
        /// release, and uninstall them.
        const INSTALL_PLUGINS    = 1 << 17;
        /// Build and install plugins from source, server-wide. The build runs
        /// the plugin's own commands on the host, so this amounts to running
        /// code there. Administrator does not grant this.
        const BUILD_PLUGINS      = 1 << 18;
        /// Allow unsafe plugins to run on the frontend. Administrator does not
        /// grant this.
        const UNSAFE_PLUGINS     = 1 << 19;

        // Administration
        const MANAGE_USERS       = 1 << 32;
        const MANAGE_ROLES       = 1 << 33;
        /// Change server config and see it unredacted.
        const MANAGE_SERVER      = 1 << 34;

        /// Grants every permission apart from `BUILD_PLUGINS` and `UNSAFE_PLUGINS`
        const ADMINISTRATOR      = 1 << 62;
    }
}

/// The role every user holds without being assigned it. Seeded by migration.
pub const EVERYONE_ROLE_ID: Snowflake = Snowflake(1);
/// The role the server's first user is given. Seeded by migration.
pub const ADMINISTRATOR_ROLE_ID: Snowflake = Snowflake(2);

impl Permissions {
    /// Wrap a raw i64 from the database into a Permissions value.
    pub fn from_db(bits: i64) -> Self {
        Self::from_bits_truncate(bits)
    }
}

impl Permissions {
    pub const ADMIN_EXEMPT: Self = Self::BUILD_PLUGINS.union(Self::UNSAFE_PLUGINS);

    /// What these permissions actually allow, with `ADMINISTRATOR` expanded.
    pub fn effective(self) -> Self {
        if self.contains(Self::ADMINISTRATOR) {
            self | (Self::all() - Self::ADMIN_EXEMPT)
        } else {
            self
        }
    }
}

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}"))
    }
}

#[derive(Debug, Clone)]
pub struct Role {
    pub _id: Snowflake,
    pub _name: String,
    pub permissions: Permissions,
    pub position: i32,
}

impl From<&lorealm_storage::models::Role> for Role {
    fn from(value: &lorealm_storage::models::Role) -> Self {
        Self {
            _id: value.id,
            _name: value.name.clone(),
            permissions: Permissions::from_db(value.permissions),
            position: value.position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Permissions;

    /// The built-in roles are seeded by `0002_roles.sql` as raw numbers; this
    /// fails if the flags move out from under them.
    #[test]
    fn seeded_role_masks_match_the_flags() {
        let everyone = Permissions::IMPORT_CONTENT
            | Permissions::UPLOAD_TOKENIZERS
            | Permissions::CUSTOM_ENDPOINTS
            | Permissions::USE_PLUGINS;
        assert_eq!(everyone.bits(), 65547);

        let administrator = Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS;
        assert_eq!(administrator.bits(), 4611686018427650048);
    }
}
