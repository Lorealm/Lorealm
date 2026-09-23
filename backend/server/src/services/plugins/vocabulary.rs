//! What a user's plugins add to Weaver and to context templates.

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
};

use async_trait::async_trait;
use lorealm_plugins::{
    CommandSignature, Param, ProcessorSignature, Role, SourceNode, ValueType as PluginValueType,
};
use tokio::runtime::Handle;
use weaver_lang::{
    EvalContext, EvalError, EvalErrorKind, Registry, Value, WeaverCommand, WeaverProcessor,
    registry::{
        CommandSignature as WeaverCommandSignature, ParamDef,
        ProcessorSignature as WeaverProcessorSignature, PropertyDef, ValueType,
    },
};

use super::Slot;
use crate::{
    errors::ContextError,
    services::context::{
        ContentRef, ContextSource, PromptRole, ResolveContext, ResolvedBlock, SourceDefinition,
        SourceNodeDefinition, SourceType,
    },
    util::weaver::{json_to_weaver, weaver_to_json},
};

/// Source ids of plugin sources start with this, followed by the handle.
pub const PLUGIN_SOURCE_PREFIX: &str = "plugin:";

/// A snapshot of what one user's loaded plugins registered, and the means to
/// call them.
#[derive(Clone)]
pub struct PluginVocabulary {
    slot: Slot,
    runtime: Handle,
    commands: Vec<(String, CommandSignature)>,
    processors: Vec<(String, ProcessorSignature)>,
    sources: Vec<PluginSourceDef>,
}

#[derive(Clone)]
struct PluginSourceDef {
    handle: String,
    name: String,
    description: String,
    nodes: Vec<SourceNode>,
}

impl PluginVocabulary {
    pub(super) fn new(
        slot: Slot,
        plugins: &[(String, lorealm_plugins::LoadedPlugin)],
        infos: &HashMap<String, (String, String)>,
    ) -> Self {
        let mut vocabulary = Self {
            slot,
            runtime: Handle::current(),
            commands: Vec::new(),
            processors: Vec::new(),
            sources: Vec::new(),
        };
        for (handle, plugin) in plugins {
            let registered = plugin.registrations();
            let command = |s: &CommandSignature| (handle.clone(), s.clone());
            let processor = |s: &ProcessorSignature| (handle.clone(), s.clone());
            vocabulary
                .commands
                .extend(registered.commands.iter().map(command));
            vocabulary
                .processors
                .extend(registered.processors.iter().map(processor));
            if !registered.source_nodes.is_empty() {
                let (name, description) = infos.get(handle).cloned().unwrap_or_default();
                vocabulary.sources.push(PluginSourceDef {
                    handle: handle.clone(),
                    name,
                    description,
                    nodes: registered.source_nodes.clone(),
                });
            }
        }
        vocabulary
    }

    pub fn register(&self, registry: &mut Registry) {
        let taken: HashSet<String> = registry
            .processor_signatures()
            .into_iter()
            .map(|s| s.namespace)
            .collect();

        for (handle, sig) in &self.commands {
            registry.register_command(PluginCommand {
                call: self.call_handle(handle),
                signature: command_signature(handle, sig),
            });
        }
        for (handle, sig) in &self.processors {
            if taken.contains(handle) {
                tracing::warn!(%handle, "plugin processors shadow a built-in namespace; skipped");
                continue;
            }
            registry.register_processor(PluginProcessor {
                call: self.call_handle(handle),
                signature: processor_signature(handle, sig),
            });
        }
    }

    /// One context source per plugin that registered nodes.
    pub fn sources(&self) -> impl Iterator<Item = Arc<dyn ContextSource>> + '_ {
        self.sources.iter().map(|def| {
            Arc::new(PluginSource {
                call: self.call_handle(&def.handle),
                def: def.clone(),
            }) as Arc<dyn ContextSource>
        })
    }

    fn call_handle(&self, handle: &str) -> Caller {
        Caller {
            slot: Arc::clone(&self.slot),
            runtime: self.runtime.clone(),
            handle: handle.to_owned(),
        }
    }
}

/// Lorebooks run Weaver through `context_weaver`, which takes its vocabulary
/// as plugins.
impl context_weaver::Plugin for PluginVocabulary {
    fn name(&self) -> &str {
        "lorealm_plugins"
    }

    fn register(&self, registry: &mut Registry) {
        PluginVocabulary::register(self, registry);
    }
}

/// Calls into one plugin of one user.
#[derive(Clone)]
struct Caller {
    slot: Slot,
    runtime: Handle,
    handle: String,
}

impl Caller {
    async fn call<T>(
        &self,
        op: impl AsyncFnOnce(
            &mut lorealm_plugins::LoadedPlugin,
        ) -> Result<T, lorealm_plugins::PluginError>,
    ) -> Result<T, String> {
        let mut guard = self.slot.lock().await;
        match guard.as_mut() {
            Some(mounted) => mounted.call(&self.handle, op).await,
            None => Err(format!("plugin `{}` is not loaded", self.handle)),
        }
    }

    /// Weaver callables are synchronous. They run on a blocking thread, or
    /// inline on a runtime worker, so this steps out of the runtime to wait.
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        tokio::task::block_in_place(|| self.runtime.block_on(future))
    }
}

struct PluginCommand {
    call: Caller,
    signature: WeaverCommandSignature,
}

impl WeaverCommand for PluginCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let name = &self.signature.name;
        let args = args.iter().map(to_json).collect::<Result<Vec<_>, _>>()?;
        let local = local_name(name).to_owned();
        let result = self
            .call
            .block_on(
                self.call
                    .call(async move |p| p.call_command(local, args).await),
            )
            .map_err(|e| host_error(name, e))?;
        result.map(|json| from_json(name, &json)).transpose()
    }

    fn signature(&self) -> WeaverCommandSignature {
        self.signature.clone()
    }
}

struct PluginProcessor {
    call: Caller,
    signature: WeaverProcessorSignature,
}

impl WeaverProcessor for PluginProcessor {
    fn call(&self, properties: HashMap<String, Value>) -> Result<Value, EvalError> {
        let name = format!("{}.{}", self.signature.namespace, self.signature.name);
        let properties = properties
            .iter()
            .map(|(k, v)| Ok((k.clone(), to_json(v)?)))
            .collect::<Result<Vec<_>, EvalError>>()?;
        let local = self.signature.name.clone();
        let json = self
            .call
            .block_on(
                self.call
                    .call(async move |p| p.call_processor(local, properties).await),
            )
            .map_err(|e| host_error(&name, e))?;
        from_json(&name, &json)
    }

    fn signature(&self) -> WeaverProcessorSignature {
        self.signature.clone()
    }
}

struct PluginSource {
    call: Caller,
    def: PluginSourceDef,
}

#[async_trait]
impl ContextSource for PluginSource {
    fn definition(&self) -> SourceDefinition {
        SourceDefinition {
            source_id: format!("{PLUGIN_SOURCE_PREFIX}{}", self.def.handle),
            name: self.def.name.clone(),
            description: self.def.description.clone(),
            source_type: SourceType::Extension {
                extension_id: self.def.handle.clone(),
            },
            nodes: self
                .def
                .nodes
                .iter()
                .map(|n| SourceNodeDefinition {
                    node_id: n.id.clone(),
                    title: n.title.clone(),
                    description: n.description.clone(),
                    icon: n.icon.clone(),
                    settings: Vec::new(),
                })
                .collect(),
        }
    }

    /// A plugin that fails contributes nothing; its log says why.
    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        _ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError> {
        let settings = serde_json::to_string(settings).unwrap_or_else(|_| "{}".into());
        let node = node_id.to_owned();
        let blocks = self
            .call
            .call(async move |p| p.resolve_source(node, settings).await)
            .await
            .unwrap_or_default();
        Ok(blocks
            .into_iter()
            .map(|b| ResolvedBlock::atomic(role(b.role), b.content, ContentRef::Verbatim))
            .collect())
    }
}

/// `handle.name`, the key a namespaced command is registered under.
fn command_signature(handle: &str, sig: &CommandSignature) -> WeaverCommandSignature {
    WeaverCommandSignature {
        name: format!("{handle}.{}", sig.name),
        description: sig.description.clone(),
        returns: value_type(sig.returns),
        params: sig
            .params
            .iter()
            .map(|p| ParamDef {
                name: p.name.clone(),
                description: p.description.clone(),
                expected_type: p.value_type.map(value_type),
                required: p.required,
            })
            .collect(),
        mutates_context: false,
    }
}

fn processor_signature(handle: &str, sig: &ProcessorSignature) -> WeaverProcessorSignature {
    WeaverProcessorSignature {
        namespace: handle.to_owned(),
        name: sig.name.clone(),
        description: sig.description.clone(),
        returns: value_type(sig.returns),
        properties: sig.properties.iter().map(property).collect(),
    }
}

fn property(p: &Param) -> PropertyDef {
    PropertyDef {
        key: p.name.clone(),
        description: p.description.clone(),
        expected_type: p.value_type.map(value_type),
        required: p.required,
    }
}

fn value_type(t: PluginValueType) -> ValueType {
    match t {
        PluginValueType::String => ValueType::String,
        PluginValueType::Number => ValueType::Number,
        PluginValueType::Bool => ValueType::Bool,
        PluginValueType::Array => ValueType::Array,
        PluginValueType::Object => ValueType::Object,
        PluginValueType::None => ValueType::None,
        PluginValueType::Any => ValueType::Any,
    }
}

fn role(role: Role) -> PromptRole {
    match role {
        Role::System => PromptRole::System,
        Role::User => PromptRole::User,
        Role::Assistant => PromptRole::Assistant,
    }
}

/// The name a plugin registered, without the handle.
fn local_name(key: &str) -> &str {
    key.split_once('.').map_or(key, |(_, name)| name)
}

fn to_json(value: &Value) -> Result<String, EvalError> {
    weaver_to_json(value)
        .map(|json| json.to_string())
        .map_err(|e| EvalError::new(EvalErrorKind::TypeError, e.to_string()))
}

fn from_json(name: &str, json: &str) -> Result<Value, EvalError> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| json_to_weaver(&v).ok())
        .ok_or_else(|| host_error(name, "returned a value that isn't valid JSON".into()))
}

fn host_error(name: &str, message: String) -> EvalError {
    EvalError::new(EvalErrorKind::HostError, format!("`{name}`: {message}"))
}
