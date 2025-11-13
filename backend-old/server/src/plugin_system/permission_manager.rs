use std::{collections::HashMap, fs::File, io::{BufReader, Write}};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::file::get_path;

use super::{plugin::Plugin, plugin_permissions::{PluginPermission, PluginPermissionError}};

type PermissionMap = HashMap<Uuid, Vec<PluginPermission>>;

#[derive(PartialEq, Eq, Hash)]
pub enum PermissionState {
    Granted,
    Pending
}

#[derive(Default)]
pub struct PermissionManager {
    plugins: Vec<Plugin>,
    implicit_permissions: PermissionMap,
    explicit_permissions: PermissionMap,
    pending_plugins: Vec<Plugin>,

    // Plugins restored from persistent storage - used for staging
    restored_plugins: Vec<Uuid>
}

impl PermissionManager {
    pub fn new() -> Self {
        let manager = PermissionManager::load();
        
        match manager {
            Ok(mut man) => {
                if let Err(e) = man.verify_internal_permissions() {
                    log::error!("Failed to verify internal permissions: {}", e);
                }
                man
            },
            Err(_) => PermissionManager::default(),
        }
    }

    pub fn add_plugin(&mut self, plugin: Plugin) -> Result<PermissionState, PluginPermissionError> {
        if !self.plugin_registered(&plugin) {
            let plugin_id = plugin.id;
            let plugin_name = plugin.name.clone();
            let permissions = plugin.permissions();

            self.plugins.push(plugin.clone());

            // If the plugin was not restored from persistent storage,
            // It's a new plugin. Mark it as pending.
            let is_pending = if !self.explicit_permissions.contains_key(&plugin_id) {
                true
            } else {
                self.explicit_permissions.get(&plugin_id).unwrap().is_empty()
            };

            if !self.restored_plugins.iter().any(|p| *p == plugin_id) || is_pending {
                log::info!("Marked plugin '{}' as pending", plugin_name);
                self.implicit_permissions.insert(plugin_id, permissions);
                self.pending_plugins.push(plugin);
                return Ok(PermissionState::Pending);
            }

            return Ok(PermissionState::Granted);
        }

        Err(PluginPermissionError::InvalidPlugin("Plugin already registered".to_string()))
    }

    pub fn add_plugins(&mut self, plugins: Vec<Plugin>) -> Result<Vec<PermissionState>, PluginPermissionError> {
        let pl = plugins.clone();
        let mut permission_states = Vec::new();
        for plugin in pl {
            match self.add_plugin(plugin) {
                Ok(state) => permission_states.push(state),
                Err(e) => log::error!("Failed to add plugin: {}", e),
            };
        }

        Ok(permission_states)
    }

    fn plugin_registered(&self, plugin: &Plugin) -> bool {
        self.plugins.iter().any(|p| p.id == plugin.id)
    }

    fn contains_all<T: PartialEq>(&self, collection: &[T], items: &[&T]) -> bool {
        items.iter().all(|item| collection.contains(item))
    }

    fn has_permission(&self, plugin: &Plugin, permission: &PluginPermission, 
        permission_map: &HashMap<Uuid, Vec<PluginPermission>>) -> bool {
        if let Some(permissions) = permission_map.get(&plugin.id) {
            return permissions.contains(permission);
        }
        false
    }

    pub fn has_implicit_permission(&self, plugin: &Plugin, permission: &PluginPermission) -> bool {
        self.has_permission(plugin, permission, &self.implicit_permissions)
    }

    pub fn has_explicit_permission(&self, plugin: &Plugin, permission: &PluginPermission) -> bool {
        self.has_permission(plugin, permission, &self.explicit_permissions)
    }

    pub fn has_implicit_permissions(&self, plugin: &Plugin, permissions: Vec<&PluginPermission>) -> bool {
        if let Some(perms) = self.implicit_permissions.get(&plugin.id) {
            return self.contains_all::<PluginPermission>(perms, &permissions)
        }
        false
    }

    pub fn has_explicit_permissions(&self, plugin: &Plugin, permissions: Vec<&PluginPermission>) -> bool {
        if let Some(perms) = self.explicit_permissions.get(&plugin.id) {
            return self.contains_all::<PluginPermission>(perms, &permissions)
        }
        false
    }

    pub fn grant_permission(&mut self, plugin: &Plugin, permission: &PluginPermission) -> Result<(), PluginPermissionError> {
        if !self.plugin_registered(plugin) {
            return Err(PluginPermissionError::InvalidPlugin(format!("Plugin {} is not registered in the permission manager", plugin.name)));
        }
        if !self.has_implicit_permission(plugin, permission) {
            return Err(PluginPermissionError::InsufficientImplicitPermission);
        }

        if self.has_explicit_permission(plugin, permission) {
            log::warn!("Can't grant permission \"{}\" for plugin \"{}\" because it is already granted", permission.to_str(), plugin.name);
            return Ok(()); 
        }

        self.explicit_permissions
            .entry(plugin.id)
            .or_insert_with(Vec::new)
            .push(permission.clone());

        log::debug!("Granted permission \"{}\" for plugin \"{}\"", permission.to_str(), plugin.name);
        Ok(())
    }

    pub fn grant_permissions(&mut self, plugin: &Plugin, permissions: Vec<PluginPermission>) -> Result<(), PluginPermissionError> {
        for permission in permissions {
            self.grant_permission(plugin, &permission)?;
        }
        Ok(())
    }

    pub fn revoke_permission(&mut self, plugin: &Plugin, permission: &PluginPermission) -> Result<(), PluginPermissionError> {
        if !self.plugin_registered(plugin) {
            return Err(PluginPermissionError::InvalidPlugin(format!("Plugin {} is not registered in the permission manager", plugin.name)));
        }
        if !self.has_explicit_permission(plugin, permission) {
            log::warn!("Can't revoke permisison \"{}\" for plugin \"{}\" because it is not granted", permission.to_str(), plugin.name);
            return Ok(());
        }

        if let Some(perms) = self.explicit_permissions.get_mut(&plugin.id) {
            perms.retain(|p| p != permission);
        }
        log::debug!("Revoked permission \"{}\" for plugin \"{}\"", permission.to_str(), plugin.name);
        Ok(())
    }

    pub fn get_pending_plugins(&self) -> Vec<Plugin> {
        self.pending_plugins.clone()
    }

    pub fn resolve_pending_plugin(&mut self, plugin_id: Uuid, granted_permissions: Vec<PluginPermission>) -> Result<(), PluginPermissionError> {
        let plugin = self.plugins.iter().find(|p| p.id == plugin_id)
            .cloned()
            .ok_or(PluginPermissionError::InvalidPlugin("Plugin not found".to_string()))?;

        match self.grant_permissions(&plugin, granted_permissions) {
            Ok(_) => {
                self.pending_plugins.retain(|p| p.id != plugin_id);
                Ok(log::info!("Resolved pending plugin \"{}\"", plugin.name))
            },
            Err(e) => Err(e),
        }
    }

    /// Verifies the internal state of the permission manager
    /// 
    /// 1. Checks that all implicit permissions match the registered plugins
    /// 2. Checks that all explicit permissions are present in the plugin's implicit permissions
    /// 3. Removes any invalid permissions
    fn verify_internal_permissions(&mut self) -> Result<(), PluginPermissionError> {
        // Step 1: Make sure implicit permissions match registered plugins
        let mut implicit_to_remove = HashMap::new();
        
        for (plugin_id, perms) in &self.implicit_permissions {
            // Check if plugin with this ID exists
            if let Some(plugin) = self.plugins.iter().find(|p| p.id == *plugin_id) {
                let invalid_perms = perms.iter().filter(|p| !plugin.permissions().contains(p)).collect::<Vec<_>>();
                for perm in invalid_perms {
                    implicit_to_remove.insert(*plugin_id, perm.clone());
                    if let Some(explicit_perms) = self.explicit_permissions.get_mut(&plugin_id) {
                        explicit_perms.retain(|p| p != perm);
                    }
                }
            }
        }

        for (plugin_id, perm) in implicit_to_remove {
            self.implicit_permissions.get_mut(&plugin_id).unwrap().retain(|p| p != &perm);
        }

        Ok(())
    }
}

pub trait PersistentResource {
    type T: Serialize + DeserializeOwned;
    fn save(&self) -> std::io::Result<()>;
    fn load() -> std::io::Result<Self> where Self: Sized;
    fn prepare_data(&self) -> Self::T;
}

impl PersistentResource for PermissionManager {
    type T = Vec<PersistentPluginPermissions>;
    fn save(&self) -> std::io::Result<()> {
        let directory = get_path("plugins/.store");
        if directory.is_err() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Directory not found"));
        }
        let directory = directory.unwrap();
        let path = directory.join("permissions.json");
        let mut file = File::create(path).unwrap();
        let json = serde_json::to_string_pretty(&self.prepare_data()).unwrap();

        file.write_all(json.as_bytes())
    }

    fn load() -> std::io::Result<Self> where Self: Sized {
        let directory = get_path("plugins/.store");
        if directory.is_err() {
            return Ok(PermissionManager::default());
        }
        let directory = directory.unwrap();
        let path = directory.join("permissions.json");

        if !path.exists() {
            return Ok(PermissionManager::default());
        }

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data: Vec<PersistentPluginPermissions> = serde_json::from_reader(reader).unwrap();
        let mut manager = PermissionManager::default();
        for perm in data {
            log::info!("Restoring permissions for plugin {}", perm.id);
            manager.restored_plugins.push(perm.id);
            manager.implicit_permissions.insert(perm.id, perm.implicit_permissions);
            manager.explicit_permissions.insert(perm.id, perm.explicit_permissions);
        }

        Ok(manager)
    }

    fn prepare_data(&self) -> Self::T {
        let mut data = Vec::new();
        for plugin in &self.plugins {
            let plugin_id = plugin.id;
            let implicit_permissions = self.implicit_permissions.get(&plugin_id).cloned().unwrap_or(Vec::new());
            let explicit_permissions = self.explicit_permissions.get(&plugin_id).cloned().unwrap_or(Vec::new());

            data.push(PersistentPluginPermissions {
                id: plugin_id,
                implicit_permissions,
                explicit_permissions
            });
        }
        data
    }
}

#[derive(Serialize, Deserialize)]
pub struct PersistentPluginPermissions {
    id: Uuid,
    implicit_permissions: Vec<PluginPermission>,
    explicit_permissions: Vec<PluginPermission>
}