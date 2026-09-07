use std::sync::{Arc, Mutex};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use weaver_lang::{
    EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

use lorealm_storage::models::UserWidget;

use crate::services::commands::CommandEffect;
use crate::util::weaver::{ConversionError, weaver_to_json};

pub type WidgetSink = Arc<Mutex<Vec<WidgetInstance>>>;

#[derive(Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub round: usize,
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub is_error: bool,
    pub duration_ms: u64,
}

/// Event produced by the inference tool loop.
pub enum InferEvent {
    Text(String),
    Reasoning(String),
    Widget(WidgetInstance),
    ToolCall(ToolCallRecord),
    /// Something a tool body asked the host to do
    Effect(CommandEffect),
}

/// A widget the model attached to its reply.
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

/// One entry in the catalog the model searches. Named "attachment" because
/// it attaches inline to a reply, as opposed to a [`UserWidget`], which is
/// always-on and reads weaver state.
///
/// [`UserWidget`]: lorealm_storage::models::UserWidget
#[derive(Debug, Clone, Serialize)]
pub struct AttachmentSpec {
    pub id: String,
    pub desc: String,
    pub props: Vec<String>,
}

impl From<&UserWidget> for AttachmentSpec {
    fn from(w: &UserWidget) -> Self {
        Self {
            id: w.id.to_string(),
            desc: w.description.clone(),
            props: w.props.clone(),
        }
    }
}

/// `$[search_attachments(keywords)]`
pub struct SearchAttachmentsCommand {
    catalog: Arc<Vec<AttachmentSpec>>,
}

impl WeaverCommand for SearchAttachmentsCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> std::result::Result<Option<Value>, EvalError> {
        let needles: Vec<String> = match args.into_iter().next() {
            Some(Value::Array(items)) => items
                .iter()
                .map(|v| v.to_output_string().to_lowercase())
                .collect(),
            Some(Value::String(s)) => vec![s.to_lowercase()],
            Some(Value::None) | None => Vec::new(),
            Some(other) => {
                return Err(EvalError::host_error(format!(
                    "search_attachments: keywords must be an array of strings, got {}",
                    other.type_name()
                )));
            }
        };

        tracing::info!("The model searched attachments for: {needles:?}");
        let hits: Vec<&AttachmentSpec> = self
            .catalog
            .iter()
            .filter(|spec| {
                needles
                    .iter()
                    .any(|n| spec.desc.to_lowercase().contains(n.as_str()))
            })
            .collect();

        let shown: Vec<&AttachmentSpec> = if hits.is_empty() {
            self.catalog.iter().collect()
        } else {
            hits
        };

        let matches = shown
            .into_iter()
            .map(|spec| {
                Value::object([
                    ("id", Value::String(spec.id.clone())),
                    ("description", Value::String(spec.desc.clone())),
                    (
                        "props",
                        Value::Array(
                            spec.props
                                .iter()
                                .map(|p| Value::String(p.clone()))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect();

        Ok(Some(Value::object([("matches", Value::Array(matches))])))
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "search_attachments".into(),
            description: "Search the attachment catalog by keyword".into(),
            returns: ValueType::Object,
            params: vec![ParamDef {
                name: "keywords".into(),
                description: "Words to match against ids, descriptions and keywords".into(),
                expected_type: Some(ValueType::Array),
                required: true,
            }],
            mutates_context: false,
        }
    }
}

/// `$[attach(id, props)]`
pub struct AttachCommand {
    sink: WidgetSink,
    catalog: Arc<Vec<AttachmentSpec>>,
}

impl WeaverCommand for AttachCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> std::result::Result<Option<Value>, EvalError> {
        let mut args = args.into_iter();
        let id = match args.next() {
            Some(Value::String(s)) => s,
            other => {
                return Err(EvalError::host_error(format!(
                    "attach: id must be a string, got {}",
                    other.map_or("none", |v| v.type_name())
                )));
            }
        };
        let spec = self.catalog.iter().find(|w| w.id == id).ok_or_else(|| {
            EvalError::host_error(format!(
                "attach: unknown attachment id '{id}'; call search_attachments for valid ids"
            ))
        })?;

        let props = match args.next() {
            Some(Value::Object(fields)) => fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), weaver_to_json(v)?)))
                .collect::<std::result::Result<serde_json::Map<_, _>, ConversionError>>()
                .map_err(|e| EvalError::host_error(e.to_string()))?,
            Some(Value::None) | None => serde_json::Map::new(),
            Some(other) => {
                return Err(EvalError::host_error(format!(
                    "attach: props must be an object, got {}",
                    other.type_name()
                )));
            }
        };

        tracing::info!("The model attached: {}", spec.id);
        self.sink.lock().unwrap().push(WidgetInstance {
            widget_id: spec.id.clone(),
            props,
        });
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "attach".into(),
            description: "Attach a catalog entry inline at this point in the reply".into(),
            returns: ValueType::None,
            params: vec![
                ParamDef {
                    name: "id".into(),
                    description: "An attachment id from search_attachments".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                ParamDef {
                    name: "props".into(),
                    description: "Property values, keyed by prop name".into(),
                    expected_type: Some(ValueType::Object),
                    required: false,
                },
            ],
            mutates_context: true,
        }
    }
}

/// Marker embedded into message content at the position a widget was
/// attached.
///
/// A model should never see these
pub fn widget_marker(index: usize) -> String {
    format!("\u{E000}widget:{index}\u{E001}")
}

/// Replaces [`widget_marker`] spans with a plain-text stand-in
/// (`[widget: <id>]`, or `[widget]` when the index can't be resolved
/// against `widgets`).
///
/// If not applied, models can and will start imitating this pattern.
///
/// TODO: In the future, this will be changed to return whatever the widget
/// returns, if anything.
pub fn sanitize_widget_markers(content: &str, widgets: &[WidgetInstance]) -> String {
    const START: char = '\u{E000}';
    const END: char = '\u{E001}';

    if !content.contains(START) {
        return content.to_string();
    }
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(pos) = rest.find(START) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + START.len_utf8()..];
        let Some(end) = after.find(END) else {
            rest = after;
            continue;
        };
        let index = after[..end]
            .strip_prefix("widget:")
            .and_then(|n| n.parse::<usize>().ok());
        if let Some(i) = index {
            match widgets.get(i) {
                Some(w) => {
                    out.push_str("[widget: ");
                    out.push_str(&w.widget_id);
                    out.push(']');
                }
                None => out.push_str("[widget]"),
            }
        }
        rest = &after[end + END.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// Register the attachment commands onto a weaver registry, returning the
/// sink [`AttachCommand`] buffers into.
pub fn register_attachment_commands(
    registry: &mut Registry,
    catalog: Vec<AttachmentSpec>,
) -> WidgetSink {
    let sink: WidgetSink = Arc::new(Mutex::new(Vec::new()));
    let catalog = Arc::new(catalog);
    registry.register_command(SearchAttachmentsCommand {
        catalog: Arc::clone(&catalog),
    });
    registry.register_command(AttachCommand {
        sink: Arc::clone(&sink),
        catalog,
    });
    sink
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tools::{ToolContext, ToolStatePool};
    use crate::services::weaver::WeaverHost;
    use crate::util::weaver::json_to_weaver;
    use std::collections::HashMap;
    use weaver_lang::CompiledTemplate;

    fn widget(id: &str) -> WidgetInstance {
        WidgetInstance {
            widget_id: id.into(),
            props: serde_json::Map::new(),
        }
    }

    fn spec(id: &str, desc: &str, props: &[&str]) -> AttachmentSpec {
        AttachmentSpec {
            id: id.into(),
            desc: desc.into(),
            props: props.iter().map(|p| (*p).to_string()).collect(),
        }
    }

    #[test]
    fn spec_id_matches_the_serialized_widget_id() {
        let w = UserWidget {
            id: lorealm_storage::models::Snowflake(123456789),
            user_id: lorealm_storage::models::Snowflake(1),
            name: "Note".into(),
            description: "A handwritten note".into(),
            source: String::new(),
            prompt: String::new(),
            tools: Vec::new(),
            enabled: true,
            placement: lorealm_storage::models::WidgetPlacement::Attachment,
            mode: Default::default(),
            props: vec!["content".into()],
            created_at: chrono::DateTime::UNIX_EPOCH,
            updated_at: chrono::DateTime::UNIX_EPOCH,
        };
        assert_eq!(
            serde_json::to_value(w.id).unwrap(),
            serde_json::Value::String(AttachmentSpec::from(&w).id)
        );
    }

    fn catalog() -> Vec<AttachmentSpec> {
        vec![
            spec("101", "A handwritten paper note", &["content"]),
            spec("102", "A glowing neon sign", &["text", "color"]),
        ]
    }

    fn run(
        source: &str,
        cmd: &[(&str, serde_json::Value)],
    ) -> Result<(String, Vec<WidgetInstance>), EvalError> {
        let mut registry = WeaverHost::base().into_registry();
        let sink = register_attachment_commands(&mut registry, catalog());

        let bound: HashMap<String, Value> = cmd
            .iter()
            .map(|(k, v)| ((*k).to_string(), json_to_weaver(v).unwrap()))
            .collect();
        let mut ctx = ToolContext::new(bound, Arc::new(Mutex::new(ToolStatePool::default())));

        let out = CompiledTemplate::compile(source)
            .map_err(|e| EvalError::host_error(format!("{e:?}")))?
            .evaluate(&mut ctx, &registry)?;
        let attached = std::mem::take(&mut *sink.lock().unwrap());
        Ok((out, attached))
    }

    const ATTACH_BODY: &str = "$[attach({{cmd:id}}, {{cmd:props}})]";

    #[test]
    fn attach_buffers_the_instance_with_its_props() {
        let (out, attached) = run(
            ATTACH_BODY,
            &[
                ("id", serde_json::json!("101")),
                (
                    "props",
                    serde_json::json!({ "content": "Meet me at dusk." }),
                ),
            ],
        )
        .unwrap();

        assert_eq!(out, "");
        assert_eq!(attached.len(), 1);
        assert_eq!(attached[0].widget_id, "101");
        assert_eq!(attached[0].props["content"], "Meet me at dusk.");
    }

    #[test]
    fn attach_rejects_an_id_outside_the_catalog() {
        let err = run(
            ATTACH_BODY,
            &[
                ("id", serde_json::json!("no-such-thing")),
                ("props", serde_json::json!({})),
            ],
        )
        .unwrap_err();
        assert!(err.to_string().contains("no-such-thing"), "{err}");
    }

    #[test]
    fn attach_survives_a_missing_props_object() {
        let (_, attached) = run(
            ATTACH_BODY,
            &[
                ("id", serde_json::json!("101")),
                ("props", serde_json::json!(null)),
            ],
        )
        .unwrap();
        assert!(attached[0].props.is_empty());
    }

    #[test]
    fn search_renders_as_parseable_json() {
        let (out, _) = run(
            "$[search_attachments({{cmd:keywords}})]",
            &[("keywords", serde_json::json!(["neon"]))],
        )
        .unwrap();

        let parsed: serde_json::Value =
            serde_json::from_str(&out).expect("search output must be JSON the model can read");
        let matches = parsed["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1, "only the neon sign matches: {out}");
        // Props travel with each hit so the model can fill them in.
        assert_eq!(matches[0]["id"], "102");
        assert_eq!(matches[0]["props"], serde_json::json!(["text", "color"]));
    }

    #[test]
    fn search_falls_back_to_the_whole_catalog_when_nothing_matches() {
        let (out, _) = run(
            "$[search_attachments({{cmd:keywords}})]",
            &[("keywords", serde_json::json!(["zzzz-no-match"]))],
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["matches"].as_array().unwrap().len(), catalog().len());
    }

    #[test]
    fn marker_resolves_to_widget_id() {
        let text = format!("read this: {}", widget_marker(0));
        assert_eq!(
            sanitize_widget_markers(&text, &[widget("note")]),
            "read this: [widget: note]"
        );
    }

    #[test]
    fn unresolvable_index_degrades_to_generic_stand_in() {
        let text = format!("a {} b", widget_marker(3));
        assert_eq!(sanitize_widget_markers(&text, &[]), "a [widget] b");
    }

    #[test]
    fn multiple_markers_resolve_independently() {
        let text = format!("{}mid{}", widget_marker(0), widget_marker(1));
        assert_eq!(
            sanitize_widget_markers(&text, &[widget("note"), widget("sign-neon")]),
            "[widget: note]mid[widget: sign-neon]"
        );
    }

    #[test]
    fn non_widget_pua_spans_are_stripped() {
        assert_eq!(
            sanitize_widget_markers("x\u{E000}bogus\u{E001}y", &[]),
            "xy"
        );
    }

    #[test]
    fn unpaired_start_marker_drops_only_the_sentinel() {
        assert_eq!(
            sanitize_widget_markers("x\u{E000}widget:1", &[]),
            "xwidget:1"
        );
    }

    #[test]
    fn text_without_markers_is_unchanged() {
        assert_eq!(sanitize_widget_markers("plain text", &[]), "plain text");
    }
}
