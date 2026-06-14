use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lorealm_providers::{Error, Result, ToolRegistry, TypedTool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub type WidgetSink = Arc<Mutex<Vec<WidgetInstance>>>;

/// Event produced by the inference tool loop.
pub enum InferEvent {
    Text(String),
    Reasoning(String),
    Widget(WidgetInstance),
}

/// A widget the model attached to its reply. Streamed to the client as a
/// `widget` event and persisted in message metadata under `"widgets"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WidgetInstance {
    pub widget_id: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

impl WidgetInstance {
    pub fn filtered(mut self, f: impl Fn(&str) -> String) -> Self {
        fn walk(value: &mut serde_json::Value, f: &impl Fn(&str) -> String) {
            match value {
                serde_json::Value::String(s) => *s = f(s),
                serde_json::Value::Array(items) => items.iter_mut().for_each(|v| walk(v, f)),
                serde_json::Value::Object(map) => map.values_mut().for_each(|v| walk(v, f)),
                _ => {}
            }
        }
        for value in self.props.values_mut() {
            walk(value, &f);
        }
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WidgetSpec {
    pub id: &'static str,
    pub desc: &'static str,
    pub props: &'static [&'static str],
    #[serde(skip)]
    keywords: &'static [&'static str],
}

/// Hardcoded PoC catalog. Endgame: populated from user-defined weaver_lang
/// commands.
pub const WIDGET_CATALOG: &[WidgetSpec] = &[
    WidgetSpec {
        id: "sms-outgoing",
        desc: "A classic SMS chat bubble, as if a text message was sent on a phone",
        props: &["sender", "content"],
        keywords: &["sms", "text", "phone", "message", "bubble", "dm", "texting"],
    },
    WidgetSpec {
        id: "sms-incoming",
        desc: "A classic SMS chat bubble, as if a text message was received on a phone",
        props: &["sender", "content"],
        keywords: &["sms", "text", "phone", "message", "bubble", "dm", "texting"],
    },
    WidgetSpec {
        id: "note",
        desc: "A handwritten paper note",
        props: &["content"],
        keywords: &[
            "note",
            "letter",
            "paper",
            "handwritten",
            "scribble",
            "written",
        ],
    },
    WidgetSpec {
        id: "sign-medieval",
        desc: "A carved wooden tavern sign with engraved text and iron studs",
        props: &["title", "subtitle"],
        keywords: &[
            "sign", "medieval", "tavern", "wood", "wooden", "inn", "fantasy", "shingle", "placard",
        ],
    },
    WidgetSpec {
        id: "note-parchment",
        desc: "An aged parchment note sealed with wax, for proclamations and quests",
        props: &["title", "content"],
        keywords: &[
            "note",
            "parchment",
            "scroll",
            "medieval",
            "letter",
            "proclamation",
            "quest",
            "notice",
            "aged",
            "wax",
            "fantasy",
        ],
    },
    WidgetSpec {
        id: "sign-steampunk",
        desc: "A polished brass plaque with riveted corners and embossed lettering",
        props: &["title", "subtitle"],
        keywords: &[
            "sign",
            "steampunk",
            "brass",
            "plaque",
            "metal",
            "rivets",
            "industrial",
            "engraved",
            "victorian",
            "placard",
        ],
    },
    WidgetSpec {
        id: "sign-neon",
        desc: "A glowing neon sign with a configurable tube color",
        props: &["text", "color"],
        keywords: &[
            "sign",
            "neon",
            "glow",
            "glowing",
            "light",
            "cyberpunk",
            "bar",
            "city",
            "night",
            "storefront",
        ],
    },
    WidgetSpec {
        id: "note-arcane",
        desc: "A glowing arcane note with a pulsing sigil, for spells and incantations",
        props: &["title", "content"],
        keywords: &[
            "note",
            "arcane",
            "magic",
            "magical",
            "spell",
            "rune",
            "sigil",
            "incantation",
            "mystic",
            "enchanted",
            "fantasy",
        ],
    },
];

#[derive(Deserialize, JsonSchema)]
pub struct SearchWidgetsInput {
    pub keywords: Vec<String>,
}

pub struct SearchWidgets;

#[async_trait]
impl TypedTool for SearchWidgets {
    type Input = SearchWidgetsInput;
    type Output = Vec<WidgetSpec>;

    fn name(&self) -> &str {
        "search_widgets"
    }

    fn description(&self) -> &str {
        "Search the catalog of visual widgets you can attach to your reply, e.g. an SMS \
             bubble when a character sends a text message. Returns widget ids along with the \
             props each widget accepts. You may use the IDs to render widgets through the start_widget command."
    }

    async fn call(&self, input: SearchWidgetsInput) -> Result<Vec<WidgetSpec>> {
        let needles: Vec<String> = input.keywords.iter().map(|k| k.to_lowercase()).collect();
        tracing::info!("The model searched for: {needles:?}");
        let hits: Vec<WidgetSpec> = WIDGET_CATALOG
            .iter()
            .filter(|spec| {
                needles.iter().any(|n| {
                    spec.id.contains(n.as_str())
                        || spec.desc.to_lowercase().contains(n.as_str())
                        || spec
                            .keywords
                            .iter()
                            .any(|k| k.contains(n.as_str()) || n.contains(k))
                })
            })
            .cloned()
            .collect();

        // The catalog is tiny for now; an empty result helps nobody, so fall
        // back to returning everything. Revisit once the catalog grows.
        Ok(if hits.is_empty() {
            WIDGET_CATALOG.to_vec()
        } else {
            hits
        })
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct StartWidgetInput {
    /// A widget id obtained from search_widgets.
    pub id: String,
    /// Property values for the widget, keyed by prop name.
    pub props: serde_json::Map<String, serde_json::Value>,
}

pub struct StartWidget {
    sink: Arc<Mutex<Vec<WidgetInstance>>>,
}

#[async_trait]
impl TypedTool for StartWidget {
    type Input = StartWidgetInput;
    type Output = serde_json::Value;

    fn name(&self) -> &str {
        "start_widget"
    }

    fn description(&self) -> &str {
        r#"Render a visual widget inline, at the point in your reply where it's most relevant.
            This tool returns no content you need to reason about — it only emits UI, rendered
            exactly at the position where the call appears in your message.

            The text immediately before the call must be a natural in-context lead-in that the
            widget continues — NOT an announcement of the tool or a description of showing it.
            Never narrate the act of rendering (no "I will now show you...", "here is...",
            "let me hand you..."). The widget should land as if it is simply the next beat of
            your message. Write that lead-in text first, then call this tool immediately after.

            Use search_widgets first to get the id and props."#
    }

    async fn call(&self, input: StartWidgetInput) -> Result<serde_json::Value> {
        let spec = WIDGET_CATALOG
            .iter()
            .find(|w| w.id == input.id)
            .ok_or_else(|| {
                Error::Tool(format!(
                    "unknown widget id '{}'; call search_widgets to discover valid ids",
                    input.id
                ))
            })?;

        tracing::info!("The model attached a widget: {}", input.id);

        self.sink.lock().unwrap().push(WidgetInstance {
            widget_id: spec.id.to_string(),
            props: input.props,
        });

        Ok(json!({ "status": "attached", "id": spec.id }))
    }
}

/// Marker embedded into message content at the position a widget was
/// attached. References an index into the message's `metadata.widgets`.
/// Private-use-area characters are used so model-generated text can't
/// collide with the marker syntax.
pub fn widget_marker(index: usize) -> String {
    format!("\u{E000}widget:{index}\u{E001}")
}

pub fn widget_registry() -> (ToolRegistry, WidgetSink) {
    let sink = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ToolRegistry::new();
    registry.register_typed(SearchWidgets);
    registry.register_typed(StartWidget {
        sink: Arc::clone(&sink),
    });
    (registry, sink)
}
