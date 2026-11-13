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

impl From<CreateLorebookRequest> for CreateLorebook {
    fn from(val: CreateLorebookRequest) -> Self {
        CreateLorebook {
            name: val.name.clone(),
            description: val.description.clone(),
            _preset: None,
        }
    }
}

impl From<UpdateLorebookRequest> for UpdateLorebook {
    fn from(val: UpdateLorebookRequest) -> Self {
        UpdateLorebook {
            name: val.name,
            description: val.description,
            config: val.config,
        }
    }
}
