use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateBlueprintRequest {
    pub name: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Deserialize)]
pub struct UpdateBlueprintRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
}

/// Values for the blueprint's declared inputs.
/// 
/// Note: A `Ref<..>` is always represented by IDs.
#[derive(Deserialize)]
pub struct ExpandBlueprintRequest {
    #[serde(default)]
    pub inputs: serde_json::Map<String, serde_json::Value>,
}
