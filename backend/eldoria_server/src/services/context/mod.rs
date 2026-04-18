use serde::{Deserialize, Serialize};

mod service;
mod template;
mod types;

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
