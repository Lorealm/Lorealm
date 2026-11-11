mod service;

use std::{collections::HashMap, sync::Arc};

use lorealm_storage::{Storage, models::Snowflake};

use weaver_lang::{CompiledModule, Export, Registry, SimpleContext, Value, registry::ValueType};

pub use service::ModuleService;

use crate::{
    services::weaver::{Limits, MemberDef, NamespaceDef, WeaverHost},
    util::weaver::summarize_parser_text,
};

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
    compiled: CompiledModule,
    values: Value,
}

impl LoadedModule {
    pub fn load(name: &str, source: &str) -> Result<Self, String> {
        let compiled = CompiledModule::compile(source).map_err(|errors| {
            errors
                .first()
                .map(|e| summarize_parser_text(&e.message))
                .unwrap_or_else(|| "the module does not parse".to_string())
        })?;

        let host = WeaverHost::builder().build();
        let values = compiled
            .values_with_options(
                &mut SimpleContext::new(),
                host.registry(),
                Limits::default().options(None),
            )
            .map_err(|e| e.to_string())?;

        Ok(Self {
            name: name.to_string(),
            compiled,
            values: Value::Object(values),
        })
    }

    /// What the editor completes under this module's scope.
    fn namespace(&self) -> NamespaceDef {
        let Value::Object(values) = &self.values else {
            unreachable!("values are always an object");
        };
        let members = self
            .compiled
            .ast()
            .exports
            .iter()
            .filter_map(|export| match export {
                Export::Value { name, doc, .. } => Some(MemberDef {
                    name: name.clone(),
                    returns: values.get(name).map_or(ValueType::Any, value_type),
                    description: doc.clone().unwrap_or_default(),
                }),
                Export::Processor(_) => None,
            })
            .collect();
        NamespaceDef {
            scope: self.name.clone(),
            description: format!("Values exported by the `{}` module.", self.name),
            members,
            profiles: Vec::new(),
        }
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
        registry.add_module(name, module.compiled.clone());
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

/// One completion namespace per module, in name order.
pub(crate) fn namespaces(modules: &[LoadedModule]) -> Vec<NamespaceDef> {
    let mut out: Vec<NamespaceDef> = modules.iter().map(LoadedModule::namespace).collect();
    out.sort_by(|a, b| a.scope.cmp(&b.scope));
    out
}

fn value_type(value: &Value) -> ValueType {
    match value {
        Value::String(_) => ValueType::String,
        Value::Number(_) => ValueType::Number,
        Value::Bool(_) => ValueType::Bool,
        Value::Array(_) => ValueType::Array,
        Value::Object(_) => ValueType::Object,
        Value::None => ValueType::None,
    }
}
