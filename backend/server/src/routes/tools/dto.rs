use serde::Deserialize;

#[derive(Deserialize)]
pub struct ToolEnabledRequest {
    pub enabled: bool,
}
