use std::collections::HashMap;

use serde::Deserialize;

/// A WTN document and the inputs to expand it with.
#[derive(Deserialize)]
pub struct ImportRealmRequest {
    pub source: String,
    /// Values for the document's `#inputs`.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
}
