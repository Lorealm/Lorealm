use lorealm_storage::models::{AGENTS_SCOPE, MAIN_CHANNEL, is_identifier};
use weaver_lang::{
    EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

/// `$[channel.set(name, handle?)]` and `$[channel.unset(name, handle?)]
pub struct ChannelCommand {
    join: bool,
}

impl ChannelCommand {
    pub const SET: Self = Self { join: true };
    pub const UNSET: Self = Self { join: false };

    fn name(&self) -> &'static str {
        if self.join {
            "channel.set"
        } else {
            "channel.unset"
        }
    }
}

impl WeaverCommand for ChannelCommand {
    fn call(
        &self,
        args: Vec<Value>,
        ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let mut args = args.into_iter();
        let channel = match args.next() {
            Some(Value::String(c)) if is_identifier(&c) => c,
            other => {
                return Err(EvalError::host_error(format!(
                    "{}: channel must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*), got {other:?}",
                    self.name()
                )));
            }
        };
        let (scope, var) = match args.next() {
            None | Some(Value::None) => ("self".to_string(), "channels".to_string()),
            Some(Value::String(h)) if is_identifier(&h) => {
                (AGENTS_SCOPE.to_string(), format!("{h}.channels"))
            }
            other => {
                return Err(EvalError::host_error(format!(
                    "{}: handle must be an agent handle, got {other:?}",
                    self.name()
                )));
            }
        };

        let mut channels = match ctx.resolve_variable(&scope, &var)? {
            Some(Value::Array(items)) => items,
            _ => vec![Value::String(MAIN_CHANNEL.into())],
        };
        let is_this = |v: &Value| matches!(v, Value::String(c) if *c == channel);
        if !self.join {
            channels.retain(|v| !is_this(v));
        } else if !channels.iter().any(is_this) {
            channels.push(Value::String(channel));
        }
        ctx.set_variable(&scope, &var, Value::Array(channels))?;
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: self.name().into(),
            description: if self.join {
                "Assign a channel to the given agent"
            } else {
                "Remove a channel from the given agent"
            }
            .into(),
            returns: ValueType::None,
            params: vec![
                ParamDef {
                    name: "channel".into(),
                    description: "The channel, as a bare identifier".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                ParamDef {
                    name: "handle".into(),
                    description: "The agent to change. Defaults to the one running this".into(),
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
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::services::tools::{Actor, ToolContext, ToolStatePool};

    fn ctx() -> ToolContext {
        ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        )
        .with_actor(Some(Actor {
            handle: "oda".into(),
            name: "Oda".into(),
            kind: "llm",
        }))
    }

    fn run(c: &mut ToolContext, cmd: ChannelCommand, args: &[&str]) -> Result<(), EvalError> {
        let args = args.iter().map(|a| Value::String(a.to_string())).collect();
        cmd.call(args, c, &Registry::new()).map(|_| ())
    }

    fn channels(c: &ToolContext, handle: &str) -> Option<Value> {
        c.resolve_variable(AGENTS_SCOPE, &format!("{handle}.channels"))
            .unwrap()
    }

    fn list(names: &[&str]) -> Option<Value> {
        Some(Value::Array(
            names.iter().map(|n| Value::String(n.to_string())).collect(),
        ))
    }

    #[test]
    fn an_agent_starts_on_main_and_joins_beside_it() {
        let mut c = ctx();
        run(&mut c, ChannelCommand::SET, &["cell_3_4"]).unwrap();
        run(&mut c, ChannelCommand::SET, &["cell_3_4"]).unwrap();
        assert_eq!(channels(&c, "oda"), list(&["main", "cell_3_4"]));
    }

    #[test]
    fn unsetting_main_takes_an_agent_off_the_broadcast() {
        let mut c = ctx();
        run(&mut c, ChannelCommand::UNSET, &["main"]).unwrap();
        assert_eq!(channels(&c, "oda"), list(&[]));
    }

    #[test]
    fn a_handle_moves_someone_else() {
        let mut c = ctx();
        run(&mut c, ChannelCommand::SET, &["court", "mori"]).unwrap();
        assert_eq!(channels(&c, "mori"), list(&["main", "court"]));
        assert_eq!(channels(&c, "oda"), None);
    }

    #[test]
    fn channels_and_handles_must_be_identifiers() {
        let mut c = ctx();
        assert!(run(&mut c, ChannelCommand::SET, &["cell 3"]).is_err());
        assert!(run(&mut c, ChannelCommand::SET, &["court", "a.b"]).is_err());
    }
}
