use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct UserPermissions: u64 {
        // Admin-only (implies everything below)
        const ADMINISTRATOR = 1 << 63;  // Highest bit for override

        // User Management (for mods/admins in hosted setups)
        const MANAGE_USERS = Self::CREATE_USER.bits() | Self::DELETE_USER.bits() | Self::EDIT_USER.bits() | Self::MODERATE_USER.bits();
        const CREATE_USER = 1 << 0;  // Generates registration link
        const DELETE_USER = 1 << 1;
        const EDIT_USER = 1 << 2;
        const MODERATE_USER = 1 << 3;  // Covers WARN/BAN/GHOST as a bundle—split if needed later

        // Server Management (admins only)
        const MANAGE_SERVER = Self::CONFIG_SERVER.bits() | Self::MANAGE_PLUGINS.bits() | Self::EDIT_POLICIES.bits() | Self::READ_LOGS.bits();
        const CONFIG_SERVER = 1 << 4;
        const MANAGE_PLUGINS = 1 << 5;  // Bundle install/remove/configure—granularity via separate checks if plugins are critical
        const EDIT_POLICIES = 1 << 6;  // Scripting, lorebooks, etc.
        const READ_LOGS = 1 << 7;

        // Own Data Management (default for users)
        const MANAGE_OWN_DATA = Self::MANAGE_OWN_WORLDS.bits() | Self::MANAGE_OWN_CHARACTERS.bits() | Self::MANAGE_OWN_LOREBOOKS.bits() | Self::MANAGE_OWN_TEMPLATES.bits();
        const MANAGE_OWN_WORLDS = 1 << 8;  // Implies CRUD
        const MANAGE_OWN_CHARACTERS = 1 << 9;
        const MANAGE_OWN_LOREBOOKS = 1 << 10;
        const MANAGE_OWN_TEMPLATES = 1 << 11;

        // Third-Party/Extensions (users with caveats)
        const MANAGE_THIRD_PARTY = Self::IMPORT_EXTERNAL.bits() | Self::MANAGE_USER_PLUGINS.bits();
        const IMPORT_EXTERNAL = 1 << 12;  // Covers characters/worlds/lorebooks as one—low risk
        const MANAGE_USER_PLUGINS = 1 << 13;  // Install/remove/configure user-level plugins
    }
}

impl UserPermissions {
    pub fn default_user() -> UserPermissions {
        return Self::MANAGE_OWN_DATA | Self::MANAGE_THIRD_PARTY;
    }
}

impl std::fmt::Display for UserPermissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut permissions = Vec::new();
        let mut remaining = *self;
        
        // Check composite flags first (in priority order)
        // This shows higher-level permissions when all constituent bits are set
        let composites = [
            (Self::ADMINISTRATOR, "ADMINISTRATOR"),
            (Self::MANAGE_USERS, "MANAGE_USERS"),
            (Self::MANAGE_SERVER, "MANAGE_SERVER"),
            (Self::MANAGE_OWN_DATA, "MANAGE_OWN_DATA"),
            (Self::MANAGE_THIRD_PARTY, "MANAGE_THIRD_PARTY"),
        ];
        
        for (flag, name) in &composites {
            if remaining.contains(*flag) {
                permissions.push(*name);
                remaining.remove(*flag);
            }
        }
        
        // Then check individual flags that weren't part of a composite
        let individuals = [
            (Self::CREATE_USER, "CREATE_USER"),
            (Self::DELETE_USER, "DELETE_USER"),
            (Self::EDIT_USER, "EDIT_USER"),
            (Self::MODERATE_USER, "MODERATE_USER"),
            (Self::CONFIG_SERVER, "CONFIG_SERVER"),
            (Self::MANAGE_PLUGINS, "MANAGE_PLUGINS"),
            (Self::EDIT_POLICIES, "EDIT_POLICIES"),
            (Self::READ_LOGS, "READ_LOGS"),
            (Self::MANAGE_OWN_WORLDS, "MANAGE_OWN_WORLDS"),
            (Self::MANAGE_OWN_CHARACTERS, "MANAGE_OWN_CHARACTERS"),
            (Self::MANAGE_OWN_LOREBOOKS, "MANAGE_OWN_LOREBOOKS"),
            (Self::MANAGE_OWN_TEMPLATES, "MANAGE_OWN_TEMPLATES"),
            (Self::IMPORT_EXTERNAL, "IMPORT_EXTERNAL"),
            (Self::MANAGE_USER_PLUGINS, "MANAGE_USER_PLUGINS"),
        ];
        
        for (flag, name) in &individuals {
            if remaining.contains(*flag) {
                permissions.push(*name);
                remaining.remove(*flag);
            }
        }
        
        // Format output
        match permissions.len() {
            0 => write!(f, "[]"),
            1 => write!(f, "{}", permissions[0]),
            _ => {
                write!(f, "[")?;
                for (i, perm) in permissions.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", perm)?;
                }
                write!(f, "]")
            }
        }
    }
}