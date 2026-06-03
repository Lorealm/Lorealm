use serde::{Deserialize, Serialize};

mod resolve;
mod service;
mod template;
mod types;

pub use resolve::*;
pub use service::ContextService;
pub use template::*;
pub use types::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    System,
    User,
    Assistant,
}

impl From<eldoria_storage::models::ChatRole> for PromptRole {
    fn from(value: eldoria_storage::models::ChatRole) -> Self {
        match value {
            eldoria_storage::models::ChatRole::System => Self::System,
            eldoria_storage::models::ChatRole::User => Self::User,
            eldoria_storage::models::ChatRole::Assistant => Self::Assistant,
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
