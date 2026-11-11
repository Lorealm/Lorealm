mod service;

use std::{collections::HashMap, sync::Arc};

use lorealm_storage::{Storage, models::Snowflake};

use weaver_lang::{
    CompiledModule, EvalError, Export, Registry, SimpleContext, Value, WeaverProcessor,
    registry::{ProcessorSignature, PropertyDef, ValueType},
};

pub use service::ModuleService;

use crate::{services::weaver::WeaverHost, util::weaver::summarize_parser_text};

pub const RESERVED_SCOPES: &[&str] = &[
    "cmd", "props", "local", "input", "self", "actor", "agents", "cast", "turn", "scenario",
    "user", "char", "realm", "state",
];

/// Exported values of every loaded module
pub type ModuleExports = Arc<HashMap<String, Value>>;

/// An enabled module, parsed, with its exported values evaluated.
#[derive(Clone)]
pub struct LoadedModule {
    name: String,
    source: Arc<str>,
    procs: Vec<ProcessorSignature>,
    values: Value,
}

impl LoadedModule {
    /// Parse `source` and evaluate its exported values.
    pub fn load(name: &str, source: &str) -> Result<Self, String> {
        let compiled = CompiledModule::compile(source).map_err(|errors| {
            errors
                .first()
                .map(|e| summarize_parser_text(&e.message))
                .unwrap_or_else(|| "the module does not parse".to_string())
        })?;

        let host = WeaverHost::builder().build();
        let values = compiled
            .values(&mut SimpleContext::new(), host.registry())
            .map_err(|e| e.to_string())?;

        let procs = compiled
            .ast()
            .exports
            .iter()
            .filter_map(|export| match export {
                Export::Processor(p) => Some(ProcessorSignature {
                    namespace: name.to_string(),
                    name: p.name.clone(),
                    description: String::new(),
                    returns: ValueType::Any,
                    properties: p
                        .params
                        .iter()
                        .map(|param| PropertyDef {
                            key: param.name.clone(),
                            description: param.doc.clone().unwrap_or_default(),
                            expected_type: None,
                            required: param.is_required(),
                        })
                        .collect(),
                }),
                Export::Value { .. } => None,
            })
            .collect();

        Ok(Self {
            name: name.to_string(),
            source: source.into(),
            procs,
            values: Value::Object(values),
        })
    }
}

pub async fn load_active(
    storage: &dyn Storage,
    user_id: Snowflake,
    imported: &[Snowflake],
) -> Vec<LoadedModule> {
    let rows = match storage.list_modules(user_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("listing modules failed: {e}");
            return Vec::new();
        }
    };
    rows.into_iter()
        .filter(|m| m.enabled || imported.contains(&m.id))
        .filter_map(|m| match LoadedModule::load(&m.name, &m.source) {
            Ok(loaded) => Some(loaded),
            Err(e) => {
                tracing::warn!(module = %m.name, "module failed to load: {e}");
                None
            }
        })
        .collect()
}

/// Register `modules` into `registry`, skipping any whose name is already a
/// processor namespace or listed in `reserved`.
pub(crate) fn register(modules: &[LoadedModule], reserved: &[&str], registry: &mut Registry) {
    let taken: Vec<String> = registry
        .processor_signatures()
        .into_iter()
        .map(|s| s.namespace)
        .collect();

    for module in modules {
        let name = module.name.as_str();
        if reserved.contains(&name) || taken.iter().any(|t| t == name) {
            tracing::warn!(
                module = name,
                "module shadows a registered namespace; skipped"
            );
            continue;
        }
        // Compiled once already in `load`; this cannot fail on the same source.
        let Ok(compiled) = CompiledModule::compile(&module.source) else {
            continue;
        };
        registry.add_module(name, compiled);
        for signature in &module.procs {
            registry.register_processor(Declaration(signature.clone()));
        }
    }
}

/// Every module's exported values, each read as the module's own scope.
pub(crate) fn values(modules: &[LoadedModule]) -> ModuleExports {
    Arc::new(
        modules
            .iter()
            .map(|m| (m.name.clone(), m.values.clone()))
            .collect(),
    )
}

/// A module processor's signature, registered for name resolution only.
struct Declaration(ProcessorSignature);

impl WeaverProcessor for Declaration {
    fn call(&self, _: HashMap<String, Value>) -> Result<Value, EvalError> {
        Err(EvalError::host_error(format!(
            "module processor `{}.{}` is not loaded",
            self.0.namespace, self.0.name
        )))
    }

    fn signature(&self) -> ProcessorSignature {
        self.0.clone()
    }
}
