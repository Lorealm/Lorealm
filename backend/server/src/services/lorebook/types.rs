use context_weaver::LorebookConfig;
use serde::Deserialize;

#[derive(Deserialize)]
pub enum CreationPreset {
    Default,
}

#[derive(Deserialize)]
pub struct CreateLorebook {
    pub name: String,
    pub description: Option<String>,
    pub preset: Option<CreationPreset>,
}

#[derive(Deserialize)]
pub struct UpdateLorebook {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<LorebookConfig>,
}
