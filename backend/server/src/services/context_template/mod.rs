use serde::{Deserialize, Serialize};

mod resolve;
mod service;
mod template;
mod types;
pub mod weaver;

pub use resolve::*;
pub use service::TemplateService;
pub use template::*;
pub use types::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    System,
    User,
    Assistant,
}

impl From<lorealm_storage::models::ChatRole> for PromptRole {
    fn from(value: lorealm_storage::models::ChatRole) -> Self {
        match value {
            lorealm_storage::models::ChatRole::System => Self::System,
            lorealm_storage::models::ChatRole::User => Self::User,
            lorealm_storage::models::ChatRole::Assistant => Self::Assistant,
        }
    }
}

impl From<context_weaver::ChatRole> for PromptRole {
    fn from(value: context_weaver::ChatRole) -> Self {
        match value {
            context_weaver::ChatRole::User => Self::User,
            context_weaver::ChatRole::Assistant => Self::Assistant,
            context_weaver::ChatRole::System => Self::System,
        }
    }
}
