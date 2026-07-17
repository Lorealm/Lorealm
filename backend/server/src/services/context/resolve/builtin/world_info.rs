use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use context_weaver::{
    ActivationState as CwActivationState, AssembledBlock, BookId, ChatMessage as CwMessage,
    ContextWeaver, EngineConfig, GuesstimationTokenizer, NamespaceAccess, Slot,
};
use kitoken::Kitoken;
use lorealm_storage::models::{ActivationState, Snowflake};
use serde_json::Value as JsonValue;
use weaver_lang::Value as WeaverValue;

use crate::{
    errors::ContextError,
    services::{
        context::{
            ContentRef, PromptTurn, SettingDefinition, SettingType, SourceDefinition,
            SourceNodeDefinition, SourceType, to_weaver_lorebook,
            weaver::{LorealmWeaverPlugin, stdlib::StdlibPlugin},
        },
        state::{CapturedState, StatePool, StateService},
    },
};

use super::super::{ContextSource, PromptRole, ResolveContext, ResolvedBlock};

pub struct WorldInfoResolver {
    state: Arc<StateService>,
}

impl WorldInfoResolver {
    pub fn new(state: Arc<StateService>) -> Self {
        Self { state }
    }
}

fn world_info_node(id: &str, title: &str, description: &str) -> SourceNodeDefinition {
    SourceNodeDefinition {
        node_id: id.into(),
        title: title.into(),
        description: description.into(),
        icon: Some("book".into()),
        settings: vec![
            SettingDefinition {
                key: "separator".into(),
                label: "Entry Separator".into(),
                description: "Text to insert between entries".into(),
                setting_type: SettingType::String {
                    max_length: Some(100),
                },
                default_value: serde_json::json!("\n---\n"),
                required: false,
            },
            SettingDefinition {
                key: "max_entries".into(),
                label: "Maximum Entries".into(),
                description: "Maximum number of entries to include (0 = unlimited)".into(),
                setting_type: SettingType::Integer {
                    min: Some(0),
                    max: Some(100),
                },
                default_value: serde_json::json!(0),
                required: false,
            },
        ],
    }
}

#[async_trait]
impl ContextSource for WorldInfoResolver {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: "world_info".into(),
            name: "World Info / Lorebook".into(),
            description: "Dynamic world information entries that activate based on keywords".into(),
            source_type: SourceType::BuiltIn,
            nodes: vec![
                world_info_node(
                    "prelude",
                    "World Info (Prelude)",
                    "World info entries placed at the deepest layer of context",
                ),
                world_info_node(
                    "preamble",
                    "World Info (Preamble)",
                    "World info entries placed near the base of context, above the prelude",
                ),
                world_info_node(
                    "backdrop",
                    "World Info (Backdrop)",
                    "World info entries providing background context",
                ),
                world_info_node(
                    "setting",
                    "World Info (Setting)",
                    "World info entries placed in the middle of context",
                ),
                world_info_node(
                    "foreground",
                    "World Info (Foreground)",
                    "World info entries placed near the front of context, closer to recent content",
                ),
                world_info_node(
                    "coda",
                    "World Info (Coda)",
                    "World info entries placed at the most recent layer of context",
                ),
            ],
        }
    }

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let slot = slot_for_node(node_id)?;

        let (blocks, persistent) = self.run_pipeline(ctx).await?;

        let separator = settings
            .get("separator")
            .and_then(|v| v.as_str())
            .unwrap_or("\n---\n");
        let max_entries = settings
            .get("max_entries")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let mut entries: Vec<&str> = blocks
            .iter()
            .filter(|b| b.slot == slot)
            .map(|b| b.content.as_str())
            .collect();

        if let Some(n) = max_entries {
            if n > 0 {
                entries.truncate(n); // 0 = unlimited
            }
        }

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        *ctx.lorebook_state.lock().await = persistent;

        Ok(vec![ResolvedBlock {
            role: PromptRole::System,
            content: entries.join(separator),
            content_ref: ContentRef::Verbatim,
        }])
    }
}

impl WorldInfoResolver {
    async fn run_pipeline(
        &self,
        ctx: &ResolveContext<'_>,
    ) -> Result<(Vec<AssembledBlock>, Option<CapturedState>), ContextError> {
        let lorebooks = match ctx.lorebook_ids {
            Some(ids) => self.state.get_lorebooks_by_ids(ctx.user_id, ids).await?,
            None => self
                .state
                .get_active_lorebooks(ctx.user_id)
                .await?
                .unwrap_or_default(),
        };

        // Init with the first book, stack the rest. No active books -> every slot empty.
        let mut iter = lorebooks.into_iter();
        let Some(first) = iter.next() else {
            return Ok((Vec::new(), None));
        };

        let tokenizer = WeaverTokenizer {
            inner: Arc::clone(ctx.tokenizer),
        };

        let mut book_ids: HashMap<Snowflake, BookId> = HashMap::new();

        let first_sf = first.id;
        let mut weaver = ContextWeaver::new(to_weaver_lorebook(first)?).with_config(EngineConfig {
            lenient: false,
            ..Default::default()
        });

        weaver.set_tokenizer(Box::new(tokenizer));
        weaver.register_plugin(StdlibPlugin)?;
        weaver.register_plugin(LorealmWeaverPlugin)?;

        book_ids.insert(first_sf, BookId(0)); // The first BookId is always 0

        for book in iter {
            let sf = book.id;
            let bid = weaver.add_lorebook(to_weaver_lorebook(book)?);
            book_ids.insert(sf, bid);
        }

        let state = ctx.state;

        for (scope, _) in ctx.variables.iter() {
            weaver.reserve_namespace(scope, NamespaceAccess::ReadOnly);
        }

        weaver.restore_persistent(to_weaver_snapshot(&state.persistent)?);

        for (sf, act) in &state.activation {
            if let Some(&bid) = book_ids.get(sf) {
                let activation = storage_activation_to_weaver(act.clone())?;
                weaver.restore_activation_state(bid, activation);
            }
        }

        for (scope, vars) in ctx.variables.iter() {
            for (name, value) in vars {
                weaver.set_variable(scope, name, value.clone());
            }
        }

        let cw_messages = convert_messages(ctx.messages);
        let blocks = weaver.assemble(&cw_messages)?;

        let exported = weaver.export_persistent();
        let output_pool = to_state_pool(&exported)?;
        let persistent = self.state.diff(&state.persistent, &output_pool);

        let states = weaver.activation_states();
        let mut activation = HashMap::with_capacity(book_ids.len());
        for (&sf, &bid) in &book_ids {
            let weaver_act = &states[bid.0];
            activation.insert(sf, weaver_activation_to_storage(weaver_act)?);
        }

        let captured = CapturedState {
            activation,
            persistent,
        };

        Ok((blocks, Some(captured)))
    }
}

pub struct WeaverTokenizer {
    inner: Arc<Kitoken>,
}

impl context_weaver::Tokenizer for WeaverTokenizer {
    fn estimate_tokens(&self, text: &str) -> usize {
        match self.inner.encode(text, true) {
            Ok(tokens) => tokens.len(),
            Err(_) => GuesstimationTokenizer.estimate_tokens(text),
        }
    }
}

fn slot_for_node(node_id: &str) -> Result<Slot, ContextError> {
    let slot = match node_id {
        "prelude" => Slot::Prelude,
        "preamble" => Slot::Preamble,
        "backdrop" => Slot::Backdrop,
        "setting" => Slot::Setting,
        "foreground" => Slot::Foreground,
        "coda" => Slot::Coda,
        other => {
            return Err(ContextError::UnknownSourceNode {
                source_id: "world_info".into(),
                node_id: other.into(),
            });
        }
    };
    Ok(slot)
}

fn convert_messages(messages: &[PromptTurn]) -> Vec<CwMessage> {
    messages
        .iter()
        .map(|t| match t.role {
            PromptRole::User => CwMessage::user(&t.content),
            PromptRole::Assistant => CwMessage::assistant(&t.content),
            PromptRole::System => CwMessage::system(&t.content),
        })
        .collect()
}

fn storage_activation_to_weaver(
    act: ActivationState,
) -> Result<CwActivationState, serde_json::Error> {
    let json = serde_json::to_string(&act)?;
    serde_json::from_str(&json)
}

fn weaver_activation_to_storage(
    act: &CwActivationState,
) -> Result<ActivationState, serde_json::Error> {
    let json = serde_json::to_string(act)?;
    serde_json::from_str(&json)
}

fn to_weaver_value(value: &JsonValue) -> Result<WeaverValue, ContextError> {
    Ok(match value {
        JsonValue::Null => WeaverValue::None,
        JsonValue::Bool(b) => WeaverValue::Bool(*b),
        JsonValue::String(s) => WeaverValue::String(s.clone()),
        JsonValue::Number(n) => match n.as_f64() {
            Some(f) => WeaverValue::Number(f),
            None => {
                return Err(ContextError::StateConversion(format!(
                    "number not representable as f64: {n}"
                )));
            }
        },
        JsonValue::Array(items) => WeaverValue::Array(
            items
                .iter()
                .map(to_weaver_value)
                .collect::<Result<_, _>>()?,
        ),
        JsonValue::Object(_) => {
            return Err(ContextError::StateConversion(
                "object has no weaver-lang representation".into(),
            ));
        }
    })
}

fn weaver_value_to_json(value: &WeaverValue) -> Result<JsonValue, ContextError> {
    Ok(match value {
        WeaverValue::None => JsonValue::Null,
        WeaverValue::Bool(b) => JsonValue::Bool(*b),
        WeaverValue::String(s) => JsonValue::String(s.clone()),
        WeaverValue::Number(n) => {
            if !n.is_finite() {
                return Err(ContextError::StateConversion(format!(
                    "non-finite number cannot be stored as JSON: {n}"
                )));
            }
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                JsonValue::Number((*n as i64).into())
            } else {
                JsonValue::Number(
                    serde_json::Number::from_f64(*n).expect("finite f64 is representable"),
                )
            }
        }
        WeaverValue::Array(items) => JsonValue::Array(
            items
                .iter()
                .map(weaver_value_to_json)
                .collect::<Result<_, _>>()?,
        ),
    })
}

fn to_state_pool(
    snapshot: &HashMap<String, HashMap<String, WeaverValue>>,
) -> Result<StatePool, ContextError> {
    snapshot
        .iter()
        .map(|(ns, vars)| {
            let converted = vars
                .iter()
                .map(|(name, v)| Ok((name.clone(), weaver_value_to_json(v)?)))
                .collect::<Result<HashMap<_, _>, ContextError>>()?;
            Ok((ns.clone(), converted))
        })
        .collect()
}

fn to_weaver_snapshot(
    pool: &StatePool,
) -> Result<HashMap<String, HashMap<String, WeaverValue>>, ContextError> {
    pool.iter()
        .map(|(ns, vars)| {
            let converted = vars
                .iter()
                .map(|(name, v)| Ok((name.clone(), to_weaver_value(v)?)))
                .collect::<Result<HashMap<_, _>, ContextError>>()?;
            Ok((ns.clone(), converted))
        })
        .collect()
}
