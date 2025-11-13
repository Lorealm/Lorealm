use std::{fs, path::Path};

use uuid::Uuid;

use super::plugin::Plugin;

fn generate_uuid(plugin: &Plugin) -> Uuid {
    let identity = format!("{}:{}", plugin.name, plugin.version);

    Uuid::new_v5(&Uuid::NAMESPACE_OID, identity.as_bytes())
}

pub fn parse_plugins(plugin_dir: impl AsRef<Path>) -> Vec<Plugin> {
    let mut plugins = Vec::new();
    let paths = fs::read_dir(plugin_dir).unwrap();

    for path in paths {
        // Ensure manifest.json and plugin.wasm are present
        let manifest_path = path.as_ref().unwrap().path().join("manifest.json");
        let wasm_path = path.as_ref().unwrap().path().join("plugin.wasm");

        if !manifest_path.exists() || !wasm_path.exists() {
            continue;
        }

        let manifest = fs::read_to_string(manifest_path).unwrap();
        let mut plugin: Plugin = serde_json::from_str(&manifest).unwrap();
        
        plugin.id = generate_uuid(&plugin);
        plugin.binary_path = wasm_path.to_str().unwrap().to_string();

        plugins.push(plugin);
    }

    log::info!("Loaded {} plugins", plugins.len());

    plugins
}