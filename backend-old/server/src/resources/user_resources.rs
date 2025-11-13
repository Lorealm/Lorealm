
use std::path::PathBuf;
use std::sync::Arc; // Assuming for locking, based on example usage
use uuid::Uuid;

use crate::jwt::jwt_utils::FileAccessClaim;
use crate::resources::resource_config::{FileLike, ResourceError};
use crate::resources::resource_manager::{AuthenticationResult, ResourceManager};
use crate::state::AppState;
use crate::systems::context_template::ContextTemplate;
use crate::users::user_state::UserState; // Assuming log crate

// Assuming FileAccessClaim has user_id
// If not, it would need to be added or derived from claims

/// A reusable struct for accessing per-user resources after authentication.
/// This is not a builder; it's meant to be used as a library-like interface for user-specific files and directories.
pub struct UserResources<'a> {
    manager: Arc<&'a ResourceManager>, // Assuming Arc for shared access; adjust if not needed
    claims: FileAccessClaim,
    jwt_token: String, // Held for potential re-auth or payload returns
    user_id: Uuid,
    base_path: PathBuf, // res://users/[user_id] resolved to absolute path
}

impl<'a> UserResources<'a> {
    /// Creates a new UserResources instance by authenticating the provided credentials.
    /// Assumes ResourceManager has a root_dir field; if not, pass it separately.
    pub async fn new(
        manager: Arc<&'a ResourceManager>,
        credentials: String,
        app_state: &AppState,
        root_dir: PathBuf
    ) -> Result<Self, ResourceError> {
        let auth_result = Self::authenticate(&manager, &credentials, app_state, false).await?;

        // Extract user_id from claims (assuming FileAccessClaim has a user_id field)
        let user_id = auth_result.file_access_claim.user.clone(); // Adjust if field name differs

        // Resolve base_path using manager's root_dir (assuming it's public or accessible)
        let base_path = root_dir.clone().join("users").join(&user_id.to_string());

        Ok(Self {
            manager,
            claims: auth_result.file_access_claim,
            jwt_token: auth_result.jwt_token,
            user_id,
            base_path
        })
    }

    /// Convenience method to get the user's profile picture (pfp.png).
    pub fn get_pfp(&self) -> Result<Box<dyn FileLike>, ResourceError> {
        let path = self.base_path.join("pfp.png"); // In the future, this will read the user's config file and find the correct file name
        self.manager.request_file_contents(&path, &self.claims)
    }

    /// Convenience method to get a specific chat file (conversations/[chat_id].json).
    pub fn get_chat(&self, chat_id: &str) -> Result<Box<dyn FileLike>, ResourceError> {
        let path = self.base_path.join("conversations").join(format!("{}.json", chat_id));
        self.manager.request_file_contents(&path, &self.claims)
    }

    pub fn get_file_content(&self, path: &str) -> Result<Box<dyn FileLike>, ResourceError> {
        let path = self.base_path.join(path.strip_prefix("res://").unwrap_or(path));
        self.manager.request_file_contents(&path, &self.claims)
    }

    pub async fn save_context_template(&self, template: ContextTemplate, app_state: &AppState) -> Result<(), ResourceError> {
        let path = format!("res://context_templates/{}.json", template.id);
        self.write_file_serde(app_state, &path, template).await
    }

    pub async fn load_context_template(&self, id: Uuid, app_state: &AppState) -> Result<ContextTemplate, ResourceError> {
        let path = format!("res://context_templates/{}.json", id);
        self.read_file_serde(app_state, &path).await
    }

    pub async fn delete_context_template(&self, id: Uuid) -> Result<(), ResourceError> {
        let path = self.base_path.join("context_templates").join(format!("{}.json", id));
        self.manager.delete_file(&path, &self.claims, true)
    }

    pub async fn list_context_templates(&self, app_state: &AppState) -> Result<Vec<ContextTemplate>, ResourceError> {
        let mut templates = Vec::new();
        let path = self.base_path.join("context_templates");
        let dir = self.manager.request_file_structure(&self.claims, path)?;

        for file in dir.files {
            log::debug!("Requesting file: {:?}", file);
            if let Some(file_name) = file.file_name() {
                let template_path = format!("res://context_templates/{}", file_name.to_str().unwrap());
                log::debug!("Found path: {}", template_path);
                let template: ContextTemplate = self.read_file_serde(app_state, &template_path).await?;
                templates.push(template);
            }   
        }

        Ok(templates)
    }

    pub async fn save_user_state(&self, state: UserState, app_state: &AppState) -> Result<(), ResourceError> {
        let path = ".state";
        self.write_file_serde(app_state, &path, state).await
    }

    pub async fn load_user_state(&self, app_state: &AppState) -> Result<UserState, ResourceError> {
        let path = ".state";
        self.read_file_serde(app_state, &path).await
    }

    pub async fn write_file_serde<T>(
        &self,
        app_state: &AppState,
        path: &str,
        value: T
    ) -> Result<(), ResourceError> where T: serde::Serialize {
        let path = self.base_path.join(path.strip_prefix("res://").unwrap_or(path));
        self.manager.write_file_serde(app_state, &path, self.jwt_token.clone(), &value).await
    }

    pub async fn read_file_serde<T: for<'de> serde::Deserialize<'de>>(&self, app_state: &AppState, path: &str) -> Result<T, ResourceError> {
        let path = self.base_path.join(path.strip_prefix("res://").unwrap_or(path));
        self.manager.read_file_serde(app_state, &path, self.jwt_token.clone()).await
    }

    

    // Reusable authentication logic, adapted from try_authenticate
    async fn authenticate(
        manager: &ResourceManager,
        creds: &str,
        app_state: &AppState,
        is_system: bool,
    ) -> Result<AuthenticationResult, ResourceError> {
        let creds = creds.to_string();

        if let Ok(claims) = manager.auth.authenticate_jwt(&creds) {
            return Ok(AuthenticationResult {
                jwt_token: creds,
                file_access_claim: claims,
                is_new_token: false,
            });
        }

        let user_manager = app_state.user_manager.write().await;
        let token_result = manager
            .auth
            .authenticate_session(&creds, &user_manager, is_system)
            .await;

        drop(user_manager);

        token_result
            .map(|(jwt_token, file_access_claim)| AuthenticationResult {
                jwt_token,
                file_access_claim,
                is_new_token: true,
            })
            .map_err(|e| ResourceError::IoError(e.to_string()))
    }
}