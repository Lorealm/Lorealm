use std::{env, path::{Path, PathBuf}};

pub struct ResourceManager {
    resource_dir: PathBuf,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resource_dir: resolve_resource_dir(),
        }
    }

    pub fn resource_dir(&self) -> &Path {
        &self.resource_dir
    }
}

fn resolve_resource_dir() -> PathBuf {
    let resource_dir = env::var("DATA_DIR").unwrap_or("@/data".to_string());
    let project_root = env::var("PROJECT_ROOT").unwrap_or("../..".to_string());

    if resource_dir.starts_with("@/") {
        let relative = resource_dir.strip_prefix("@/").unwrap();
        Path::new(&project_root).join(relative)
    } else {
        PathBuf::from(resource_dir)
    }
}
