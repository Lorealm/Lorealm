use serde::{Deserialize, Serialize};

use crate::services::{
    cards::extension::{LorebookData, ToolData, WidgetData},
    context_template::{ContextNode, NodeContent},
    snowflake::SnowflakeService,
};

pub const TRANSFER_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Template,
    Persona,
    Tool,
    Widget,
    Lorebook,
    Module,
}

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
    Tool {
        tools: Vec<ToolData>,
    },
    Widget {
        tools: Vec<ToolData>,
        widgets: Vec<WidgetData>,
    },
    Lorebook {
        lorebook: LorebookData,
    },
    Module {
        name: String,
        #[serde(default)]
        description: String,
        source: String,
    },
}

pub fn fresh_ids(nodes: &mut [ContextNode], snowflake: &SnowflakeService) {
    for node in nodes {
        node.id = snowflake.generate();
        if let NodeContent::Group { children, .. } = &mut node.content {
            fresh_ids(children, snowflake);
        }
    }
}
