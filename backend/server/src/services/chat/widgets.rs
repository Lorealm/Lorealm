use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use weaver_lang::{
    CompiledTemplate, EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

use lorealm_storage::models::{UserWidget, WidgetMode};

use crate::services::commands::CommandEffect;
use crate::services::plugins::PluginVocabulary;
use crate::services::state::StatePool;
use crate::services::weaver::{WeaverHost, WeaverProfile};
use crate::util::weaver::{ConversionError, json_to_weaver, weaver_to_json};

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
    /// Weaver template standing in for the widget in the prompt.
    pub render: String,
    /// Under [`WidgetMode::Latest`] only the newest instance renders.
    pub mode: WidgetMode,
}

impl From<&UserWidget> for AttachmentSpec {
    fn from(w: &UserWidget) -> Self {
        Self {
            id: w.id.to_string(),
            desc: w.description.clone(),
            props: w.props.clone(),
            render: w.render.clone(),
            mode: w.mode,
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

/// Walks [`widget_marker`] spans, replacing each with `f(index)`. A span that
/// is not a widget marker is dropped: if not applied, models can and will
/// start imitating this pattern.
fn replace_markers(content: &str, mut f: impl FnMut(usize) -> String) -> String {
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
        if let Some(index) = after[..end]
            .strip_prefix("widget:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            out.push_str(&f(index));
        }
        rest = &after[end + END.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// What the model reads in place of a widget it cannot be shown.
fn stand_in(widget: Option<&WidgetInstance>) -> String {
    match widget {
        Some(w) => format!("[widget: {}]", w.widget_id),
        None => "[widget]".to_string(),
    }
}

/// Replaces [`widget_marker`] spans with a plain-text stand-in.
pub fn sanitize_widget_markers(content: &str, widgets: &[WidgetInstance]) -> String {
    replace_markers(content, |i| stand_in(widgets.get(i)))
}

/// Turns attached widgets into the text the model reads, by evaluating each
/// one's `render` template over the props it was attached with.
#[derive(Default)]
pub struct WidgetRenderer {
    specs: HashMap<String, AttachmentSpec>,
    pool: Arc<Mutex<StatePool>>,
    rendered: HashSet<String>,
    plugins: Option<PluginVocabulary>,
}

impl WidgetRenderer {
    pub fn new(catalog: &[AttachmentSpec], pool: StatePool) -> Self {
        Self {
            specs: catalog.iter().map(|s| (s.id.clone(), s.clone())).collect(),
            pool: Arc::new(Mutex::new(pool)),
            rendered: HashSet::new(),
            plugins: None,
        }
    }

    /// Lets `render` templates call the user's plugin commands and processors.
    pub fn with_plugins(mut self, plugins: Option<PluginVocabulary>) -> Self {
        self.plugins = plugins;
        self
    }

    /// One message's content, with every marker resolved.
    pub fn apply(&mut self, content: &str, widgets: &[WidgetInstance]) -> String {
        replace_markers(content, |i| match widgets.get(i) {
            Some(w) => self.render(w).unwrap_or_else(|| stand_in(Some(w))),
            None => stand_in(None),
        })
    }

    fn render(&mut self, widget: &WidgetInstance) -> Option<String> {
        let spec = self.specs.get(&widget.widget_id)?;
        if spec.mode == WidgetMode::Latest && self.rendered.contains(&widget.widget_id) {
            return None;
        }
        let mut props: HashMap<String, Value> = widget
            .props
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), json_to_weaver(v).ok()?)))
            .collect();
        for name in &spec.props {
            props.entry(name.clone()).or_insert(Value::None);
        }
        let text = eval_render(
            &spec.id,
            &spec.render,
            Arc::clone(&self.pool),
            props,
            self.plugins.as_ref(),
        )?;

        self.rendered.insert(widget.widget_id.clone());
        Some(text)
    }
}

/// Evaluate one widget `render` template.
fn eval_render(
    id: &str,
    template: &str,
    pool: Arc<Mutex<StatePool>>,
    props: HashMap<String, Value>,
    plugins: Option<&PluginVocabulary>,
) -> Option<String> {
    if template.trim().is_empty() {
        return None;
    }
    let body = CompiledTemplate::compile(template)
        .inspect_err(|e| tracing::warn!("widget {id} has an invalid render: {e:?}"))
        .ok()?;
    let text = WeaverHost::builder()
        .profile(WeaverProfile::WidgetRender)
        .pool(pool)
        .args(props)
        .plugins(plugins.cloned())
        .build()
        .evaluate_inline(body)
        .inspect_err(|e| tracing::warn!("widget {id} failed to render: {e}"))
        .ok()?
        .to_output_string();
    Some(text)
}

pub fn render_widgets(
    templates: &[String],
    pool: &StatePool,
    plugins: Option<&PluginVocabulary>,
) -> Vec<String> {
    if templates.is_empty() {
        return Vec::new();
    }
    let pool = Arc::new(Mutex::new(pool.clone()));
    templates
        .iter()
        .filter_map(|t| eval_render("", t, Arc::clone(&pool), HashMap::new(), plugins))
        .filter(|text| !text.trim().is_empty())
        .collect()
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
            render: String::new(),
            mode: WidgetMode::Every,
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
            render: String::new(),
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
        run_with(catalog(), source, cmd)
    }

    fn run_with(
        catalog: Vec<AttachmentSpec>,
        source: &str,
        cmd: &[(&str, serde_json::Value)],
    ) -> Result<(String, Vec<WidgetInstance>), EvalError> {
        let mut registry = WeaverHost::base().into_registry();
        let sink = register_attachment_commands(&mut registry, catalog);

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
    fn builtins_can_be_searched_attached_and_rendered() {
        use crate::services::tools::{ATTACH_ID, SEARCH_ATTACHMENTS_ID, builtin_tool};
        use crate::services::user_widgets::builtin_widgets;

        let catalog: Vec<AttachmentSpec> =
            builtin_widgets().iter().map(AttachmentSpec::from).collect();
        let search = builtin_tool(SEARCH_ATTACHMENTS_ID).unwrap().body;
        let attach = builtin_tool(ATTACH_ID).unwrap().body;

        let (found, _) = run_with(
            catalog.clone(),
            &search,
            &[("keywords", serde_json::json!(["texting"]))],
        )
        .unwrap();
        let sms = builtin_widgets()
            .iter()
            .find(|w| w.name == "SMS (received)")
            .unwrap();
        let id = serde_json::to_value(sms.id).unwrap();
        assert!(found.contains(id.as_str().unwrap()), "{found}");

        let (_, attached) = run_with(
            catalog.clone(),
            &attach,
            &[
                ("id", id),
                ("props", serde_json::json!({ "content": "on my way" })),
            ],
        )
        .unwrap();
        let text =
            WidgetRenderer::new(&catalog, StatePool::new()).apply(&widget_marker(0), &attached);
        assert_eq!(text, "[Text message received: on my way]");
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

    fn rendering(render: &str, mode: WidgetMode) -> AttachmentSpec {
        AttachmentSpec {
            render: render.into(),
            mode,
            ..spec("101", "a tracker", &["level"])
        }
    }

    fn instance(id: &str, level: i64) -> WidgetInstance {
        WidgetInstance {
            widget_id: id.into(),
            props: serde_json::Map::from_iter([("level".into(), serde_json::json!(level))]),
        }
    }

    #[test]
    fn a_marker_renders_from_the_props_it_was_attached_with() {
        let mut r = WidgetRenderer::new(
            &[rendering(
                "Affection {{props:level}}/10.",
                WidgetMode::Every,
            )],
            StatePool::new(),
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 4)]),
            "Affection 4/10."
        );
    }

    #[test]
    fn a_render_reads_the_state_pool() {
        let pool = StatePool::from_iter([(
            "relationship".to_string(),
            HashMap::from_iter([("trust".to_string(), serde_json::json!(7))]),
        )]);
        let mut r = WidgetRenderer::new(
            &[rendering(
                "Trust {{relationship:trust}}.",
                WidgetMode::Every,
            )],
            pool,
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 0)]),
            "Trust 7."
        );
    }

    #[test]
    fn latest_mode_renders_only_the_first_message_it_is_fed() {
        let mut r = WidgetRenderer::new(
            &[rendering("Affection {{props:level}}.", WidgetMode::Latest)],
            StatePool::new(),
        );
        // Fed newest first, so the newer instance is the one that renders.
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 9)]),
            "Affection 9."
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 4)]),
            "[widget: 101]"
        );
    }

    #[test]
    fn every_mode_renders_the_whole_history() {
        let mut r = WidgetRenderer::new(
            &[rendering("Affection {{props:level}}.", WidgetMode::Every)],
            StatePool::new(),
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 9)]),
            "Affection 9."
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 4)]),
            "Affection 4."
        );
    }

    #[test]
    fn a_widget_without_a_render_falls_back_to_the_stand_in() {
        let mut r = WidgetRenderer::new(&[rendering("", WidgetMode::Every)], StatePool::new());
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 4)]),
            "[widget: 101]"
        );
    }

    #[test]
    fn a_broken_render_falls_back_rather_than_leaking_a_sentinel() {
        let mut r = WidgetRenderer::new(
            &[rendering("$[no_such_command()]", WidgetMode::Every)],
            StatePool::new(),
        );
        let out = r.apply(&widget_marker(0), &[instance("101", 4)]);
        assert_eq!(out, "[widget: 101]");
        assert!(!out.contains('\u{E000}'));
    }

    #[test]
    fn a_render_cannot_write_the_pool() {
        let mut r = WidgetRenderer::new(
            &[rendering(
                "$[set_var(\"ns\", \"x\", 1)]ok",
                WidgetMode::Every,
            )],
            StatePool::new(),
        );
        assert_eq!(
            r.apply(&widget_marker(0), &[instance("101", 4)]),
            "[widget: 101]"
        );
    }

    #[test]
    fn always_on_renders_drop_the_ones_that_produce_nothing() {
        let pool = StatePool::from_iter([(
            "n".to_string(),
            HashMap::from_iter([("gold".to_string(), serde_json::json!(3))]),
        )]);
        assert_eq!(
            render_widgets(
                &["Gold: {{n:gold}}".into(), String::new(), "   ".into()],
                &pool,
                None
            ),
            vec!["Gold: 3".to_string()]
        );
    }
}
