use context_weaver::LorebookConfig;
use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::services::{
    cards::export::ExportWarning,
    lorebook::{CreateLorebook, UpdateLorebook},
};

#[derive(Deserialize, Clone)]
pub struct CreateLorebookRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateLorebookRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<LorebookConfig>,
}

#[derive(Deserialize)]
pub struct UpdateEntryRequest {
    pub source: String,
}

#[derive(Serialize)]
pub struct SillyTavernExport {
    pub file: serde_json::Value,
    pub warnings: Vec<ExportWarning>,
}

#[derive(Deserialize)]
pub struct ImportQuery {
    pub name: String,
}

#[derive(Deserialize)]
pub struct SetActiveLorebooksRequest {
    pub ids: Vec<Snowflake>,
}

impl Into<CreateLorebook> for CreateLorebookRequest {
    fn into(self) -> CreateLorebook {
        CreateLorebook {
            name: self.name.clone(),
            description: self.description.clone(),
            preset: None,
        }
    }
}

impl Into<UpdateLorebook> for UpdateLorebookRequest {
    fn into(self) -> UpdateLorebook {
        UpdateLorebook {
            name: self.name,
            description: self.description,
            config: self.config,
        }
    }
}
