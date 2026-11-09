use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::services::{
    cards::{
        extension::{ToolData, WidgetData},
        import::AttachmentReport,
    },
    context_template::ContextNode,
};

/// Bumped when a file written by an older version would read differently.
pub const TRANSFER_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Template,
    Persona,
    Tool,
    Widget,
}

/// An exported template, persona, tool or widget.
#[derive(Serialize, Deserialize)]
pub struct TransferFile {
    pub lorealm: u32,
    #[serde(flatten)]
    pub item: Item,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    Template {
        name: String,
        #[serde(default)]
        description: String,
        nodes: Vec<ContextNode>,
    },
    Persona {
        name: String,
        display_name: Option<String>,
        pseudonym: Option<String>,
        description: Option<String>,
    },
    Tool { tools: Vec<ToolData> },
    Widget {
        tools: Vec<ToolData>,
        widgets: Vec<WidgetData>,
    },
}

#[derive(Serialize)]
pub struct ImportResponse {
    pub kind: Kind,
    /// None when a tool or widget was left out; the report says why.
    pub id: Option<Snowflake>,
    /// Present for tools and widgets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<AttachmentReport>,
}
