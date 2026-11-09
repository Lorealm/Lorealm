//! `extensions.lorealm`: everything a card needs to come back into Lorealm
//! exactly as it left, whatever the standard CCv3 fields could hold.

use lorealm_storage::models::{
    Lorebook, Param, ToolSurface, UserTool, UserWidget, WidgetMode, WidgetPlacement,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const EXTENSION_KEY: &str = "lorealm";
pub const EXTENSION_VERSION: u32 = 1;

/// Which syntax the card's standard text fields are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDialect {
    /// SillyTavern macros (`{{char}}`). Also what a card without this
    /// extension is assumed to use.
    #[default]
    Sillytavern,
    /// Native Weaver, used as is.
    Weaver,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LorealmCardExtension {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub dialect: TextDialect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_injection: Option<String>,
    /// Native text for fields the standard ones couldn't carry exactly.
    #[serde(default, skip_serializing_if = "NativeFields::is_empty")]
    pub fields: NativeFields,
    /// Every attached lorebook, main first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lorebooks: Vec<LorebookData>,
    /// Attached tools, plus tools that attached widgets depend on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widgets: Vec<WidgetData>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NativeFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_history_instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub greetings: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
}

impl NativeFields {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LorebookData {
    pub hash: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub config_yaml: String,
    pub entries: Vec<EntryData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryData {
    pub entry_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolData {
    pub hash: String,
    /// False for a tool that's only here because a widget needs it.
    pub attached: bool,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub params: Vec<Param>,
    pub body: String,
    #[serde(default)]
    pub surface: ToolSurface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetData {
    pub hash: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub render: String,
    #[serde(default)]
    pub props: Vec<String>,
    #[serde(default)]
    pub placement: WidgetPlacement,
    #[serde(default)]
    pub mode: WidgetMode,
    /// The hashes of the tools it uses, which appear in `tools`.
    #[serde(default)]
    pub tools: Vec<String>,
}

fn digest(kind: &str, value: &impl Serialize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\n");
    hasher.update(serde_json::to_vec(value).expect("hash inputs always serialize"));
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub fn tool_hash(
    name: &str,
    description: &str,
    params: &[Param],
    body: &str,
    surface: ToolSurface,
) -> String {
    digest(
        "lorealm-tool-v1",
        &(name, description, params, body, surface),
    )
}

pub fn widget_hash(
    source: &str,
    prompt: &str,
    render: &str,
    props: &[String],
    placement: WidgetPlacement,
    mode: WidgetMode,
    tool_hashes: &[String],
) -> String {
    digest(
        "lorealm-widget-v1",
        &(source, prompt, render, props, placement, mode, tool_hashes),
    )
}

/// Covers the config (which includes the name) and every entry, in order.
pub fn lorebook_hash(config_yaml: &str, entries: &[(&str, &str)]) -> String {
    digest("lorealm-lorebook-v1", &(config_yaml, entries))
}

impl ToolData {
    pub fn from_tool(tool: &UserTool, attached: bool) -> Self {
        Self {
            hash: tool_hash(
                &tool.name,
                &tool.description,
                &tool.params,
                &tool.body,
                tool.surface,
            ),
            attached,
            name: tool.name.clone(),
            display_name: tool.display_name.clone(),
            description: tool.description.clone(),
            params: tool.params.clone(),
            body: tool.body.clone(),
            surface: tool.surface,
        }
    }

    /// The hash of this content, recomputed.
    pub fn content_hash(&self) -> String {
        tool_hash(
            &self.name,
            &self.description,
            &self.params,
            &self.body,
            self.surface,
        )
    }
}

impl WidgetData {
    pub fn from_widget(widget: &UserWidget, tool_hashes: Vec<String>) -> Self {
        Self {
            hash: widget_hash(
                &widget.source,
                &widget.prompt,
                &widget.render,
                &widget.props,
                widget.placement,
                widget.mode,
                &tool_hashes,
            ),
            name: widget.name.clone(),
            description: widget.description.clone(),
            source: widget.source.clone(),
            prompt: widget.prompt.clone(),
            render: widget.render.clone(),
            props: widget.props.clone(),
            placement: widget.placement,
            mode: widget.mode,
            tools: tool_hashes,
        }
    }

    pub fn content_hash(&self) -> String {
        widget_hash(
            &self.source,
            &self.prompt,
            &self.render,
            &self.props,
            self.placement,
            self.mode,
            &self.tools,
        )
    }
}

impl LorebookData {
    pub fn from_lorebook(lorebook: &Lorebook) -> Self {
        let entries: Vec<EntryData> = lorebook
            .entries
            .iter()
            .map(|e| EntryData {
                entry_id: e.entry_id.clone(),
                source: e.source.clone(),
            })
            .collect();
        let mut data = Self {
            hash: String::new(),
            name: lorebook.name.clone(),
            description: lorebook.description.clone(),
            config_yaml: lorebook.config_yaml.clone(),
            entries,
        };
        data.hash = data.content_hash();
        data
    }

    pub fn content_hash(&self) -> String {
        let entries: Vec<(&str, &str)> = self
            .entries
            .iter()
            .map(|e| (e.entry_id.as_str(), e.source.as_str()))
            .collect();
        lorebook_hash(&self.config_yaml, &entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_legacy_extension_reads_as_version_zero() {
        let legacy: LorealmCardExtension = serde_json::from_value(serde_json::json!({
            "display_name": "Ali",
            "template_injection": "{{template}}"
        }))
        .unwrap();
        assert_eq!(legacy.version, 0);
        assert_eq!(legacy.dialect, TextDialect::Sillytavern);
        assert_eq!(legacy.display_name.as_deref(), Some("Ali"));
        assert!(legacy.tools.is_empty());
    }

    #[test]
    fn hashes_ignore_identity_but_not_behaviour() {
        let a = tool_hash(
            "roll",
            "Roll a die",
            &[],
            "{# return 1 #}",
            ToolSurface::Tool,
        );
        let b = tool_hash(
            "roll",
            "Roll a die",
            &[],
            "{# return 1 #}",
            ToolSurface::Tool,
        );
        let c = tool_hash(
            "roll",
            "Roll a die",
            &[],
            "{# return 2 #}",
            ToolSurface::Tool,
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("sha256:"));
    }
}
