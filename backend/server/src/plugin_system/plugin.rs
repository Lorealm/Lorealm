use uuid::Uuid;

use super::plugin_permissions::PluginPermission;

#[derive(serde::Deserialize, Clone)]
pub struct Plugin {
    #[serde(skip)]
    pub(super) id: Uuid,

    pub(super) name: String,
    pub(super) version: String,
    pub(super) description: String,
    pub(super) author: String,
    pub(super) permissions: Vec<PluginPermission>,

    #[serde(skip)]
    pub(super) binary_path: String
}

impl Plugin {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn permissions(&self) -> Vec<PluginPermission> {
        self.permissions.clone()
    }
}