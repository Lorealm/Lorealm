use std::{
    collections::HashMap, env, path::{Path, PathBuf}, sync::Arc
};
use rand::{rngs::OsRng, TryRngCore};

use crate::{
    jwt::jwt_utils::FileAccessClaim,
    resources::{
        resource_authentication::ResourceAuthenticator,
        resource_config::{DirectoryStructure, FileLike, ResourceConfig, ResourceError}, user_resources::UserResources,
    },
    state::AppState,
};

#[derive(Debug)]
pub struct RequestPayload<T> {
    pub data: T,
    pub token: Option<String>,
}

/// Represents a directory on the file system with file access
#[derive(Debug)]
pub struct DirectoryData {
    pub path: PathBuf,
    pub files: Vec<Box<dyn FileLike>>,
    pub subdirectories: Vec<DirectoryData>,
}

pub struct DiscoveryDirectory {
    pub(super) path: PathBuf,
    pub(super) files: Vec<PathBuf>,
    pub(super) subdirectories: Vec<DiscoveryDirectory>,
}

pub struct ResourceRequestBuilder<'a> {
    manager: &'a ResourceManager,
    credentials: Option<String>,
    is_system: bool,
    dirs: Vec<PathBuf>,
    files: Vec<PathBuf>,
    root_dir: PathBuf,
}

pub struct AuthenticationResult {
    pub(super) jwt_token: String,
    pub file_access_claim: FileAccessClaim,
    pub(super) is_new_token: bool,
}

impl<'a> ResourceRequestBuilder<'a> {
    pub fn with_credentials(mut self, credentials: Option<String>, is_system: bool) -> Self {
        if is_system {
            if let Some(token) = &credentials {
                if token == &self.manager.system_token {
                    self.is_system = true;
                    self.credentials = Some(token.clone());
                }
            }
        }
        self.credentials = credentials;
        self
    }

    fn resolve_path(&self, input: impl Into<PathBuf>) -> PathBuf {
        let input_path: PathBuf = input.into();
        let input_str = input_path.to_str().unwrap_or("");
        if input_str.starts_with("res://") {
            self.root_dir.join(&input_str[6..])
        } else if input_path.is_absolute() {
            input_path
        } else {
            self.root_dir.join(input_path)
        }
    }

    fn resolve_pattern(&self, pattern: &str) -> String {
        if pattern.starts_with("res://") {
            self.root_dir.join(&pattern[6..]).to_string_lossy().to_string()
        } else if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            self.root_dir.join(pattern).to_string_lossy().to_string()
        }
    }

    /// Adds a directory to the request if it exists.
    pub fn with_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        let resolved_dir = self.resolve_path(dir);
        if resolved_dir.is_dir() {
            self.dirs.push(resolved_dir);
        } else {
            log::warn!("Attempted to add non-existent file: {:?}", resolved_dir.display())
        }
        self
    }

    /// Adds a file to the request if it exists.
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        let resolved_file = self.resolve_path(file);
        if resolved_file.is_file() {
            self.files.push(resolved_file);
        } else {
           log::warn!("Attempted to add non-existent file: {:?}", resolved_file.display())
        }
        self
    }

    /// Adds paths matching the glob pattern, distinguishing between files and directories.
    pub fn with_glob(mut self, glob_pattern: &str) -> Self {
        let resolved_pattern = self.resolve_pattern(glob_pattern);
        match glob::glob(&resolved_pattern) {
            Ok(paths) => {
                for path_result in paths {
                    match path_result {
                        Ok(path) => {
                            if path.is_dir() {
                                self.dirs.push(path);
                            } else if path.is_file() {
                                self.files.push(path);
                            }
                        }
                        Err(e) => log::error!("Glob path error: {}", e),
                    }
                }
            }
            Err(e) => log::error!("Glob pattern error: {}", e),
        }
        self
    }

    pub async fn request_structure(
        self,
        app_state: &AppState,
    ) -> Result<RequestPayload<DirectoryStructure>, ResourceError> {
        let auth_result = try_authenticate(app_state, self.credentials.clone(), self.is_system, &self.manager.auth).await?;

        let structure = self
            .manager
            .request_file_structure(&auth_result.file_access_claim, self.dirs.first().cloned().ok_or(ResourceError::InvalidPath(PathBuf::new()))?)?;

        Ok(RequestPayload {
            data: structure,
            token: if auth_result.is_new_token {
                Some(auth_result.jwt_token)
            } else {
                None
            },
        })
    }

    pub async fn request_content(
        &self,
        app_state: &AppState,
    ) -> Result<RequestPayload<DirectoryData>, ResourceError> {
        let auth_result = try_authenticate(app_state, self.credentials.clone(), self.is_system, &self.manager.auth).await?;

        let discovery = self.generate_discovery_structure();

        let data = self.build_directory_data(&discovery, &auth_result.file_access_claim);

        Ok(RequestPayload {
            data,
            token: Some(auth_result.jwt_token),
        })
    }

    pub fn request_qualified_paths(&self) -> Result<Vec<PathBuf>, ResourceError> {
        if !self.is_system {
            return Err(ResourceError::PermissionDenied);
        }
        self.files
            .iter()
            .map(|path| self.manager.request_qualified_path(path))
            .collect()
    }

    fn generate_discovery_structure(&self) -> DiscoveryDirectory {
        let mut directory_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        // Collect directories and their files
        for dir in &self.dirs {
            directory_map
                .entry(dir.to_string_lossy().to_string())
                .or_insert_with(Vec::new);
        }

        for file in &self.files {
            if let Some(parent) = file.parent() {
                directory_map
                    .entry(parent.to_string_lossy().to_string())
                    .or_insert_with(Vec::new)
                    .push(file.clone());
            }
        }

        fn build_directory(
            path_str: &str,
            map: &HashMap<String, Vec<PathBuf>>,
        ) -> DiscoveryDirectory {
            let files = map.get(path_str).cloned().unwrap_or_default();
            let mut subdirs = Vec::new();

            for (key, _) in map.iter().filter(|(k, _)| k.starts_with(path_str) && *k != path_str) {
                if key
                    .strip_prefix(path_str)
                    .map_or(false, |suffix| suffix.starts_with('/') && !suffix[1..].contains('/'))
                {
                    subdirs.push(build_directory(key, map));
                }
            }

            DiscoveryDirectory {
                path: PathBuf::from(path_str),
                files,
                subdirectories: subdirs,
            }
        }

        let root_key = directory_map
            .keys()
            .min_by_key(|k| k.split('/').count())
            .cloned()
            .unwrap_or_default();

        build_directory(&root_key, &directory_map)
    }

    fn build_directory_data(
        &self,
        discovery: &DiscoveryDirectory,
        claim: &FileAccessClaim,
    ) -> DirectoryData {
        let files = discovery
            .files
            .iter()
            .filter_map(|file| {
                self.manager
                    .request_file_contents(file, claim)
                    .map_err(|e| log::error!("Failed to request file {}: {}", file.display(), e))
                    .ok()
            })
            .collect();

        let subdirectories = discovery
            .subdirectories
            .iter()
            .map(|subdir| self.build_directory_data(subdir, claim))
            .collect();

        DirectoryData {
            path: discovery.path.clone(),
            files,
            subdirectories,
        }
    }
}

pub struct ResourceManager {
    pub(super) resource_config: ResourceConfig,
    pub(super) auth: ResourceAuthenticator,
    pub(super) system_token: String,
}

impl ResourceManager {
    pub fn new() -> Self {
        let system_token = Self::ensure_system_token();
        Self {
            resource_config: ResourceConfig::new(resolve_resource_dir(), None),
            auth: ResourceAuthenticator::new(),
            system_token,
        }
    }

    pub fn resource_dir(&self) -> &Path {
        self.resource_config.resource_dir()
    }

    pub fn request_builder(&self) -> ResourceRequestBuilder {
        ResourceRequestBuilder {
            manager: self,
            credentials: None,
            is_system: false,
            dirs: vec![],
            files: vec![],
            root_dir: self.resource_dir().to_path_buf(),
        }
    }

    pub async fn user_resources(
        &self,
        credentials: &str,
        app_state: &AppState,
    ) -> Result<UserResources, ResourceError> {
        UserResources::new(Arc::new(self), credentials.to_string(), app_state, self.resource_dir().to_path_buf()).await
    }

    pub(super) fn request_file_structure(
        &self,
        claim: &FileAccessClaim,
        start_point: PathBuf,
    ) -> Result<DirectoryStructure, ResourceError> {
        if !claim.permissions.contains(&"read".to_string()) {
            return Err(ResourceError::PermissionDenied);
        }

        self.resource_config
            .directory_manager()
            .request_directory_structure(&start_point, claim.user)
    }

    fn request_qualified_path(&self, path: &Path) -> Result<PathBuf, ResourceError> {
        self.resource_config.directory_manager().request_qualifed_path(path)
    }

    pub(super) async fn write_file_serde<T>(
        &self,
        app_state: &AppState,
        path: &Path,
        credentials: String,
        value: T
    ) -> Result<(), ResourceError> where T: serde::Serialize {
        let auth = try_authenticate(app_state, Some(credentials), false, &self.auth).await?;

        if !auth.file_access_claim.permissions.contains(&"write".to_string()) {
            return Err(ResourceError::PermissionDenied);
        }

        self.resource_config
            .directory_manager()
            .write_file(path, auth.file_access_claim.user, &value)
    }

    pub(super) async fn read_file_serde<T: for<'de> serde::Deserialize<'de>>(
        &self,
        app_state: &AppState,
        path: &Path,
        credentials: String,
    ) -> Result<T, ResourceError>{
        let auth = try_authenticate(app_state, Some(credentials), false, &self.auth).await?;

        if !auth.file_access_claim.permissions.contains(&"read".to_string()) {
            return Err(ResourceError::PermissionDenied);
        }

        self.resource_config
            .directory_manager()
            .read_file(path, auth.file_access_claim.user)
    }

    pub(super) fn request_file_contents(
        &self,
        path: &Path,
        claim: &FileAccessClaim,
    ) -> Result<Box<dyn FileLike>, ResourceError> {
        if !claim.permissions.contains(&"read".to_string()) {
            return Err(ResourceError::PermissionDenied);
        }

        self.resource_config
            .directory_manager()
            .request_file_contents(path, claim.user)
    }

    pub(super) fn delete_file(
        &self,
        path: &Path,
        claim: &FileAccessClaim,
        soft_delete: bool
    ) -> Result<(), ResourceError>{
        if !claim.permissions.contains(&"write".to_string()) {
            return Err(ResourceError::PermissionDenied);
        }

        self.resource_config
            .directory_manager()
            .delete_file(path, claim.user, soft_delete)
    }

    fn ensure_system_token() -> String {
        env::var("SYSTEM_FILE_ACCESS_TOKEN").unwrap_or_else(|_| {
            let token = generate_token().expect("Failed to generate system token");
            unsafe { env::set_var("SYSTEM_FILE_ACCESS_TOKEN", &token) };
            token
        })
    }
}

fn generate_token() -> Result<String, Box<dyn std::error::Error>> {
    let mut rng = OsRng;
    let mut key = [0u8; 32];
    rng.try_fill_bytes(&mut key)?;
    Ok(hex::encode(key))
}

fn resolve_resource_dir() -> PathBuf {
    let resource_dir = env::var("DATA_DIR").unwrap_or_else(|_| "@/data".to_string());
    let project_root = env::var("PROJECT_ROOT")
        .unwrap_or_else(|_| format!("{}/../../", env!("CARGO_MANIFEST_DIR")));

    if resource_dir.starts_with("@/") {
        let relative = resource_dir.strip_prefix("@/").unwrap();
        std::fs::canonicalize(Path::new(&project_root).join(relative)).unwrap_or_else(|_| PathBuf::from(resource_dir))
    } else {
        PathBuf::from(resource_dir)
    }
}

async fn try_authenticate(
    app_state: &AppState,
    credentials: Option<String>,
    is_system: bool,
    auth: &ResourceAuthenticator
) -> Result<AuthenticationResult, ResourceError> {
    let creds = credentials.as_ref().ok_or(ResourceError::PermissionDenied)?.clone();

    log::debug!("----- Attempting authenticate JWT: -----");
    log::debug!("token: {}", &creds);
    if let Ok(claims) = auth.authenticate_jwt(&creds) {
        log::debug!("JWT Authentication Sucessful");
        return Ok(AuthenticationResult {
            jwt_token: creds,
            file_access_claim: claims,
            is_new_token: false,
        });
    }
    log::debug!("----- Attempting authenticate session token: -----");
    let user_manager = app_state.user_manager.write().await;
    let token_result = auth
        .authenticate_session(&creds, &user_manager, is_system)
        .await;
    log::debug!("Session Authentication Sucessful");

    drop(user_manager);

    token_result.map(|(jwt_token, file_access_claim)| AuthenticationResult {
        jwt_token,
        file_access_claim,
        is_new_token: true,
    }).map_err(|e| ResourceError::IoError(e.to_string()))
}