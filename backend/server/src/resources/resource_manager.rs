use std::{env, fmt::format, path::{Path, PathBuf}};

use glob::glob;
use rand::{rngs::OsRng, TryRngCore};

use crate::{jwt::jwt_utils::FileAccessClaim, resources::{resource_authentication::ResourceAuthenticator, resource_config::{DirectoryData, ResourceConfig}}, state::AppState};

#[derive(Debug)]
pub struct ResourcePayload {
    data: Option<DirectoryData>,
    token: Option<String>
}

pub struct ResourceRequestBuilder<'a> {
    manager: &'a ResourceManager,
    credentials: Option<String>,
    is_system: bool, // The system token is a special token that allows access to all files
    dirs: Vec<PathBuf>,
    files: Vec<PathBuf>,
    root_dir: PathBuf
}

struct AuthenticationResult {
    jwt_token: String,
    file_access_claim: FileAccessClaim,
    /// New tokens are sent back to the client
    is_new_token: bool
}

impl<'a> ResourceRequestBuilder<'a> {
    pub fn with_credentials(mut self, credentials: Option<String>, is_system: bool) -> Self {
        if is_system {
            let system_token = env::var("SYSTEM_FILE_ACCESS_TOKEN").unwrap_or("".to_string());
            if let Some(token) = &credentials {
                if token == &system_token {
                    self.is_system = true;
                    self.credentials = Some(system_token);
                }
            }
        }
        self.credentials = credentials;
        self
    }

    /// Add a directory to the request
    /// 
    /// Parameters:
    /// - dir: The directory to add
    pub fn with_dir(mut self, dir: String) -> Self {
        let dir = self.root_dir.join(dir);
        println!("Adding directory: {}", dir.display());
        if dir.is_dir() {
            self.dirs.push(dir);
        }
        self
    }

    /// Add a file to the request
    /// 
    /// Parameters:
    /// - file: The file to add
    pub fn with_file(mut self, file: String) -> Self {
        let files = glob(&file);
        if let Ok(files) = files {
            for file in files {
                if let Ok(file) = file {
                    if !file.is_file() { continue; }
                    self.files.push(file);
                }
            }
        }
        self
    }

    pub fn with_glob(mut self, glob: String) -> Self {
        let paths = glob::glob(&glob);
        if let Ok(paths) = paths {
            println!("Found paths: {:#?}", paths);
            for path in paths {
                if let Ok(path) = path {
                    print!("Found path: {}", path.display());
                    if !path.is_dir() { continue; }
                    self.dirs.push(path);
                }
            }
        }
        else if let Err(e) = paths {
            println!("Error: {}", e);
        }
        self
    }

    pub async fn build(self, app_state: &AppState) -> ResourcePayload {
        let auth_result = self.try_authenticate(app_state).await.unwrap();

        let request_result = self.manager.request_files(&auth_result.file_access_claim, self.dirs[0].clone());

        if let Ok(data) = request_result {
            ResourcePayload {
                data: Some(data),
                token: if auth_result.is_new_token { Some(auth_result.jwt_token) } else { None }
            }
        } else {
            ResourcePayload {
                data: None,
                token: None
            }
        }
    }

    async fn try_authenticate(&self, app_state: &AppState) -> Result<AuthenticationResult, Box<dyn std::error::Error>> {
        // Step 1: Check if the provided token is a valid JWT token
        // Step 2: If not, check if the provided token is a valid session token
        // Step 2a: If valid, generate a new JWT token for file access

        if self.credentials.is_none() {
            return Err("No credentials provided".into());
        };

        let creds = self.credentials.as_ref().unwrap().to_string();

        if let Ok(claims) = self.manager.auth.authenticate_jwt(&creds) {
            return Ok(AuthenticationResult {
                jwt_token: creds,
                file_access_claim: claims,
                is_new_token: false
            });
        }

        let user_manager = app_state.user_manager.lock().await;
        let token = self.manager.auth.authenticate_session(&creds, &user_manager, self.is_system).await;

        drop(user_manager);

        match token {
            Ok(jwt_claims) => Ok(AuthenticationResult {
                jwt_token: jwt_claims.0,
                file_access_claim: jwt_claims.1,
                is_new_token: true
            }),
            Err(e) => Err(e),
        }
    }
}

pub struct ResourceManager {
    resource_config: ResourceConfig,
    auth: ResourceAuthenticator
}

impl ResourceManager {
    pub fn new(db: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self {
            resource_config: ResourceConfig::new(resolve_resource_dir(), None),
            auth: ResourceAuthenticator::new(db)
        }
    }

    pub fn resource_dir(&self) -> &Path {
        &self.resource_config.resource_dir()
    }

    pub fn request_builder(&self) -> ResourceRequestBuilder {
        let _ = self.ensure_system_token();
        ResourceRequestBuilder {
            manager: self,
            credentials: None,
            is_system: false,
            dirs: vec![],
            files: vec![],
            root_dir: self.resource_dir().to_path_buf()
        }
    }

    /// Requests a file in accordance with the provided FileAccessClaim and permissions
    fn request_files(&self, claim: &FileAccessClaim, start_point: PathBuf) -> Result<DirectoryData, Box<dyn std::error::Error>> {
        if !claim.permissions.contains(&"read".to_string()) {
            return Err("Permission denied".into());
        }

        let directory_config = self.resource_config.directory_config();
        let directories = directory_config.request_directory_structure(&start_point.to_string_lossy().to_string(), claim.user);

        if let Err(e) = directories {
            println!("Error: {}", e);
            return Err(e);
        }

        Ok(directories.unwrap())
    }

    fn ensure_system_token(&self) -> String {
        let var = env::var("SYSTEM_FILE_ACCESS_TOKEN");
        if let Err(_) = var {
            let token = generate_token().unwrap();
            unsafe {
                env::set_var("SYSTEM_FILE_ACCESS_TOKEN", token.as_str());
            }
            token
        }   
        else {
            return var.unwrap();
        }
    }
}

fn generate_token() -> Result<String, Box<dyn std::error::Error>> {
    let mut key = [0u8; 64];
    OsRng.try_fill_bytes(&mut key)?;
    let token: String = OsRng.try_next_u32()?.to_string();
    Ok(token)
}

fn resolve_resource_dir() -> PathBuf {
    let resource_dir = env::var("DATA_DIR").unwrap_or("@/data".to_string());
    let project_root = env::var("PROJECT_ROOT").unwrap_or(format!("{}/../../", env!("CARGO_MANIFEST_DIR")));

    if resource_dir.starts_with("@/") {
        let relative = resource_dir.strip_prefix("@/").unwrap();
        std::fs::canonicalize(Path::new(&project_root).join(relative)).unwrap()
    } else {
        PathBuf::from(resource_dir)
    }
}
