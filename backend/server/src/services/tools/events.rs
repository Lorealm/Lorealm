use std::sync::{Arc, Mutex};

use lorealm_storage::models::{EventKind, is_identifier};
use weaver_lang::{
    EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

/// An event drafted by a weaver body
#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub kind: EventKind,
    pub scope: String,
    pub content: String,
}

/// A shared interior-mutable reference to a vector of type [`EmittedEvent`]
pub type EventSink = Arc<Mutex<Vec<EmittedEvent>>>;

/// `$[emit_event(scope, content, kind?)]` — draft a realm event on channel
/// `scope`. `kind` is `"spoke"` (default) or `"system"`. Only agents whose
/// grants cover the channel observe it.
pub struct EmitEventCommand {
    sink: EventSink,
}

impl EmitEventCommand {
    pub fn new(sink: EventSink) -> Self {
        Self { sink }
    }
}

impl WeaverCommand for EmitEventCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let mut args = args.into_iter();
        let scope = match args.next() {
            Some(Value::String(s)) => s,
            other => {
                return Err(EvalError::host_error(format!(
                    "emit_event: scope must be a string, got {other:?}"
                )));
            }
        };
        if !is_identifier(&scope) {
            return Err(EvalError::host_error(format!(
                "emit_event: scope '{scope}' must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
            )));
        }
        let content = match args.next() {
            Some(Value::String(s)) => s,
            Some(other) => other.to_string(),
            None => {
                return Err(EvalError::host_error("emit_event: content is required"));
            }
        };
        let kind = match args.next() {
            None | Some(Value::None) => EventKind::Spoke,
            Some(Value::String(k)) if k == "spoke" => EventKind::Spoke,
            Some(Value::String(k)) if k == "system" => EventKind::System,
            Some(other) => {
                return Err(EvalError::host_error(format!(
                    "emit_event: kind must be \"spoke\" or \"system\", got {other:?}"
                )));
            }
        };

        self.sink.lock().unwrap().push(EmittedEvent {
            kind,
            scope,
            content,
        });
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "emit_event".into(),
            description:
                "Draft a realm event on a channel; only agents granted that channel observe it"
                    .into(),
            returns: ValueType::None,
            params: vec![
                ParamDef {
                    name: "scope".into(),
                    description: "The channel the event carries (a bare identifier)".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                ParamDef {
                    name: "content".into(),
                    description: "The event's content".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                ParamDef {
                    name: "kind".into(),
                    description: "\"spoke\" (default) or \"system\"".into(),
                    expected_type: Some(ValueType::String),
                    required: false,
                },
            ],
            mutates_context: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tools::{ToolContext, ToolStatePool};
    use crate::services::weaver::WeaverHost;
    use std::collections::HashMap;
    use weaver_lang::CompiledTemplate;

    fn run(source: &str) -> Result<Vec<EmittedEvent>, EvalError> {
        let sink: EventSink = Arc::new(Mutex::new(Vec::new()));
        let mut registry = WeaverHost::base().into_registry();
        registry.register_command(EmitEventCommand::new(Arc::clone(&sink)));

        let mut ctx = ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        );
        CompiledTemplate::compile(source)
            .map_err(|e| EvalError::host_error(format!("{e:?}")))?
            .evaluate(&mut ctx, &registry)?;
        let events = std::mem::take(&mut *sink.lock().unwrap());
        Ok(events)
    }

    #[test]
    fn emit_event_buffers_a_spoke_draft() {
        let events = run(r#"$[emit_event("court", "The king stirs.")]"#).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].kind, EventKind::Spoke));
        assert_eq!(events[0].scope, "court");
        assert_eq!(events[0].content, "The king stirs.");
    }

    #[test]
    fn emit_event_accepts_the_system_kind() {
        let events = run(r#"$[emit_event("main", "Night falls.", "system")]"#).unwrap();
        assert!(matches!(events[0].kind, EventKind::System));
    }

    #[test]
    fn emit_event_rejects_bad_scope_and_kind() {
        assert!(run(r#"$[emit_event("not an identifier", "x")]"#).is_err());
        assert!(run(r#"$[emit_event("main", "x", "acted")]"#).is_err());
    }
}
