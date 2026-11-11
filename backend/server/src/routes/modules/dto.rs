use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateModuleRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Deserialize)]
pub struct UpdateModuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
    pub enabled: Option<bool>,
}
