use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

pub const MANIFEST_FILE: &str = "manifest.toml";

/// Every permission a manifest may declare.
pub const PERMISSIONS: &[&str] = &[
    "chat:read",
    "chat:write",
    "data:persist",
    "media:write",
    "model:generate",
    "net:fetch",
    "persona:read",
    "prompt:write",
    "state:read",
    "state:write",
    "tools:write",
    "ui:autoplay",
    "ui:unsafe",
];

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid manifest: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unknown permission `{0}`")]
    UnknownPermission(String),
    #[error("`{0}` isn't a host; write it like `api.example.com` or `localhost:8080`")]
    InvalidHost(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub plugin: PluginInfo,
    #[serde(default)]
    pub compatibility: Compatibility,
    #[serde(default)]
    pub structure: Structure,
    #[serde(default)]
    pub frontend: Frontend,
    #[serde(default)]
    pub build: Build,
    #[serde(default)]
    pub definitions: Definitions,
    #[serde(default)]
    pub network: Network,
    #[serde(default)]
    pub provides: Provides,
}

/// What the plugin offers other plugins.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Provides {
    /// The events it may emit.
    #[serde(default)]
    pub events: Vec<String>,
}

/// Where the plugin may send requests.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub hosts: Vec<String>,
}

/// Auxilliary types registered by a plugin
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Definitions {
    /// Custom API key types.
    #[serde(default)]
    pub api_keys: Vec<String>,
}

/// How to build the plugin from source.
///
/// Each command runs in the plugin's directory.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub backend: Vec<String>,
    #[serde(default)]
    pub frontend: Vec<String>,
}

/// What the plugin shows in the client.
#[derive(Debug, Clone, Deserialize)]
pub struct Frontend {
    /// Directory holding the slot entries, relative to the plugin directory.
    #[serde(default = "default_frontend")]
    pub path: PathBuf,
    #[serde(default)]
    pub slots: Vec<FrontendSlot>,
}

impl Default for Frontend {
    fn default() -> Self {
        Self {
            path: default_frontend(),
            slots: Vec::new(),
        }
    }
}

fn default_frontend() -> PathBuf {
    PathBuf::from("frontend")
}

/// One place in the client the plugin fills, such as `inspector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrontendSlot {
    pub slot: String,
    /// A self-contained HTML file in the frontend directory, or for an unsafe
    /// slot, an ES module exporting `mount(element, api)`.
    pub entry: String,
    pub title: String,
    /// Runs directly in the client instead of a sandboxed frame. Needs the
    /// `ui:unsafe` permission, and a user allowed unsafe plugins.
    #[serde(default)]
    pub r#unsafe: bool,
    /// For `message` slots: on every message, or only the latest.
    #[serde(default)]
    pub mode: SlotMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotMode {
    #[default]
    Latest,
    Every,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub handle: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: String,
    pub version: Version,
    /// The plugin API the backend was built against
    pub api: Version,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Compatibility {
    #[serde(default)]
    pub loaded_before: Vec<String>,
    #[serde(default)]
    pub loaded_after: Vec<String>,
    #[serde(default)]
    pub requires: BTreeMap<String, VersionReq>,
    #[serde(default)]
    pub recommends: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Structure {
    /// Path to the backend component, relative to the plugin directory.
    #[serde(default = "default_backend")]
    pub backend: PathBuf,
    /// Shown on the plugin's page. Optional.
    #[serde(default = "default_readme")]
    pub readme: PathBuf,
    /// The `#inputs` block the user fills in on the plugin's page. Optional.
    #[serde(default = "default_settings")]
    pub settings: PathBuf,
}

impl Default for Structure {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            readme: default_readme(),
            settings: default_settings(),
        }
    }
}

fn default_backend() -> PathBuf {
    PathBuf::from("plugin.wasm")
}

fn default_readme() -> PathBuf {
    PathBuf::from("README.md")
}

fn default_settings() -> PathBuf {
    PathBuf::from("settings.wtn")
}

impl Manifest {
    pub fn from_dir(dir: &Path) -> Result<Self, ManifestError> {
        let path = dir.join(MANIFEST_FILE);
        let source = std::fs::read_to_string(&path).map_err(|source| ManifestError::Io {
            path: path.clone(),
            source,
        })?;
        let manifest: Self = toml::from_str(&source)?;
        if let Some(p) = manifest
            .plugin
            .permissions
            .iter()
            .find(|p| !PERMISSIONS.contains(&p.as_str()))
        {
            return Err(ManifestError::UnknownPermission(p.clone()));
        }
        if let Some(host) = manifest
            .network
            .hosts
            .iter()
            .find(|h| h.is_empty() || h.contains(['/', '@', ' ']))
        {
            return Err(ManifestError::InvalidHost(host.clone()));
        }
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_manifest() {
        let manifest: Manifest = toml::from_str(
            r#"
            [plugin]
            name = "Markdown Preview"
            handle = "janedoe"
            description = "Live-renders Markdown files in a side panel as you type."
            tags = ["markdown", "preview", "editor"]
            author = "Jane Doe <jane@example.com>"
            version = "1.2.0"
            api = "0.1.0"
            permissions = ["prompt:write", "persona:read"]

            [compatibility]
            loaded_before = ["mermaid_render"]
            loaded_after = ["markdown_extensions"]
            requires = { markdown_extensions = "^1.4" }
            recommends = ["syntax_highlight"]
            conflicts = ["other_md_preview"]

            [structure]
            backend = "build/plugin.wasm"

            [[frontend.slots]]
            slot = "inspector"
            entry = "inspector.html"
            title = "Preview"
            "#,
        )
        .unwrap();

        assert_eq!(manifest.plugin.version, Version::new(1, 2, 0));
        assert_eq!(manifest.plugin.api, Version::new(0, 1, 0));
        assert!(
            manifest.compatibility.requires["markdown_extensions"].matches(&Version::new(1, 5, 0))
        );
        assert_eq!(
            manifest.structure.backend,
            PathBuf::from("build/plugin.wasm")
        );
        assert_eq!(manifest.frontend.path, PathBuf::from("frontend"));
        assert_eq!(manifest.frontend.slots[0].entry, "inspector.html");
    }

    #[test]
    fn optional_sections_default() {
        let manifest: Manifest = toml::from_str(
            r#"
            [plugin]
            name = "Minimal"
            handle = "minimal"
            description = ""
            author = "Jane Doe"
            version = "0.1.0"
            api = "0.1.0"
            "#,
        )
        .unwrap();

        assert!(manifest.plugin.permissions.is_empty());
        assert!(manifest.compatibility.requires.is_empty());
        assert_eq!(manifest.structure.backend, PathBuf::from("plugin.wasm"));
    }

    #[test]
    fn rejects_invalid_version() {
        let result = toml::from_str::<Manifest>(
            r#"
            [plugin]
            name = "Bad"
            handle = "bad"
            description = ""
            author = "Jane Doe"
            version = "one"
            api = "0.1.0"
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn requires_an_api_version() {
        let result = toml::from_str::<Manifest>(
            r#"
            [plugin]
            name = "Old"
            handle = "old"
            description = ""
            author = "Jane Doe"
            version = "0.1.0"
            "#,
        );

        assert!(result.unwrap_err().to_string().contains("api"));
    }

    #[test]
    fn rejects_unknown_permission() {
        let dir = std::env::temp_dir().join(format!("lorealm-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(MANIFEST_FILE),
            r#"
            [plugin]
            name = "Bad"
            handle = "bad"
            description = ""
            author = "Jane Doe"
            version = "0.1.0"
            api = "0.1.0"
            permissions = ["prompt:writ"]
            "#,
        )
        .unwrap();

        let result = Manifest::from_dir(&dir);
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(matches!(
            result,
            Err(ManifestError::UnknownPermission(p)) if p == "prompt:writ"
        ));
    }
}
