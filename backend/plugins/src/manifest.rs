use std::collections::HashMap;
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

pub const MANIFEST_FILE: &str = "manifest.toml";

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid manifest: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub plugin: PluginInfo,
    #[serde(default)]
    pub compatibility: Compatibility,
    #[serde(default)]
    pub structure: Structure,
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
    pub requires: HashMap<String, VersionReq>,
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
}

impl Default for Structure {
    fn default() -> Self {
        Self {
            backend: default_backend(),
        }
    }
}

fn default_backend() -> PathBuf {
    PathBuf::from("plugin.wasm")
}

impl Manifest {
    pub fn from_dir(dir: &Path) -> Result<Self, ManifestError> {
        let path = dir.join(MANIFEST_FILE);
        let source = std::fs::read_to_string(&path).map_err(|source| ManifestError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(toml::from_str(&source)?)
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
            permissions = ["fs:read", "ui:modal"]

            [compatibility]
            loaded_before = ["mermaid_render"]
            loaded_after = ["markdown_extensions"]
            requires = { markdown_extensions = "^1.4" }
            recommends = ["syntax_highlight"]
            conflicts = ["other_md_preview"]

            [structure]
            backend = "build/plugin.wasm"
            "#,
        )
        .unwrap();

        assert_eq!(manifest.plugin.version, Version::new(1, 2, 0));
        assert!(
            manifest.compatibility.requires["markdown_extensions"].matches(&Version::new(1, 5, 0))
        );
        assert_eq!(
            manifest.structure.backend,
            PathBuf::from("build/plugin.wasm")
        );
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
            "#,
        );

        assert!(result.is_err());
    }
}
