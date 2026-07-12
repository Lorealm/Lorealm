use lorealm_storage::models::Snowflake;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LorebookError {
    #[error("User {0} does not own this template: {1}")]
    NotOwned(Snowflake, Snowflake),
    #[error("ContextWeaver error: {0}")]
    ContextWeaver(#[from] context_weaver::ContextWeaverError),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

impl LorebookError {
    pub fn client_message(&self) -> String {
        match self {
            LorebookError::NotOwned(_, lorebook_id) => {
                format!("{lorebook_id} doesn't exist, or you don't have permission to access it.")
            }
            LorebookError::ContextWeaver(context_weaver_error) => context_weaver_error.to_string(),
            LorebookError::Yaml(error) => error.to_string(),
        }
    }
}
