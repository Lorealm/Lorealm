use std::sync::{Arc, Mutex};

use serde::Serialize;
use weaver_lang::{
    EvalContext, EvalError, Registry, Value, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

/// Something a command body asked the host to do with its output.
///
/// An effect is produced by an explicit weaver command in the tool
/// body. Note that this differs from the tool's return value in the
/// sense that effects are purely visual.
/// 
/// NOTE: [`CommandEffect::Sys`] should be consumed in the server and
/// should never reach the client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandEffect {
    /// Text destined for a system message in the conversation.
    Sys { content: String },
    /// A transient notice, executed by the client.
    Toast { content: String, variant: String },
}

pub type EffectSink = Arc<Mutex<Vec<CommandEffect>>>;

/// Registers `$[sys(...)]` and `$[toast(...)]` against a fresh sink, and hands
/// the sink back for the caller to drain after evaluation.
pub fn register_effect_commands(registry: &mut Registry) -> EffectSink {
    let sink: EffectSink = Arc::new(Mutex::new(Vec::new()));
    registry.register_command(SysCommand {
        sink: Arc::clone(&sink),
    });
    registry.register_command(ToastCommand {
        sink: Arc::clone(&sink),
    });
    sink
}

/// The text of one argument, rendered the way weaver renders output.
fn text_arg(command: &str, args: &mut std::vec::IntoIter<Value>) -> Result<String, EvalError> {
    match args.next() {
        Some(Value::String(s)) => Ok(s),
        Some(Value::None) | None => Err(EvalError::host_error(format!(
            "{command}: content is required"
        ))),
        Some(other) => Ok(other.to_output_string()),
    }
}

/// `$[sys(content)]`: write `content` into the conversation as a system
/// message. Several calls in one run append to the same message.
struct SysCommand {
    sink: EffectSink,
}

impl WeaverCommand for SysCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let mut args = args.into_iter();
        let content = text_arg("sys", &mut args)?;
        self.sink
            .lock()
            .unwrap()
            .push(CommandEffect::Sys { content });
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "sys".into(),
            description: "Post text into the conversation as a system message".into(),
            returns: ValueType::None,
            params: vec![ParamDef {
                name: "content".into(),
                description: "What the message says".into(),
                expected_type: Some(ValueType::String),
                required: true,
            }],
            mutates_context: true,
        }
    }
}

const VARIANTS: [&str; 4] = ["info", "success", "warning", "error"];

/// `$[toast(content, variant?)]`: raise a transient notice in the client.
struct ToastCommand {
    sink: EffectSink,
}

impl WeaverCommand for ToastCommand {
    fn call(
        &self,
        args: Vec<Value>,
        _ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<Value>, EvalError> {
        let mut args = args.into_iter();
        let content = text_arg("toast", &mut args)?;
        let variant = match args.next() {
            None | Some(Value::None) => "info".to_string(),
            Some(Value::String(v)) if VARIANTS.contains(&v.as_str()) => v,
            Some(other) => {
                return Err(EvalError::host_error(format!(
                    "toast: variant must be one of {}, got {other:?}",
                    VARIANTS.join(", ")
                )));
            }
        };
        self.sink
            .lock()
            .unwrap()
            .push(CommandEffect::Toast { content, variant });
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "toast".into(),
            description: "Raise a transient notice in the client".into(),
            returns: ValueType::None,
            params: vec![
                ParamDef {
                    name: "content".into(),
                    description: "What the notice says".into(),
                    expected_type: Some(ValueType::String),
                    required: true,
                },
                ParamDef {
                    name: "variant".into(),
                    description: "\"info\" (default), \"success\", \"warning\" or \"error\"".into(),
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
    use crate::services::tools::{ToolContext, ToolStatePool, build_tool_registry};
    use std::collections::HashMap;
    use weaver_lang::CompiledTemplate;

    fn run(source: &str) -> Result<(String, Vec<CommandEffect>), EvalError> {
        let mut registry = build_tool_registry();
        let sink = register_effect_commands(&mut registry);
        let mut ctx = ToolContext::new(
            HashMap::new(),
            Arc::new(Mutex::new(ToolStatePool::default())),
        );
        let out = CompiledTemplate::compile(source)
            .map_err(|e| EvalError::host_error(format!("{e:?}")))?
            .evaluate(&mut ctx, &registry)?;
        let effects = std::mem::take(&mut *sink.lock().unwrap());
        Ok((out, effects))
    }

    #[test]
    fn sys_buffers_its_content_and_renders_nothing() {
        let (out, effects) = run(r#"$[sys("Rolled 17.")]"#).unwrap();
        assert_eq!(out, "");
        assert!(matches!(
            effects.as_slice(),
            [CommandEffect::Sys { content }] if content == "Rolled 17."
        ));
    }

    #[test]
    fn toast_defaults_to_info_and_accepts_a_variant() {
        let (_, effects) = run(r#"$[toast("Saved.")]$[toast("Gone.", "error")]"#).unwrap();
        assert!(matches!(
            effects.as_slice(),
            [
                CommandEffect::Toast { variant: a, .. },
                CommandEffect::Toast { variant: b, .. }
            ] if a == "info" && b == "error"
        ));
    }

    #[test]
    fn an_unknown_toast_variant_is_rejected() {
        assert!(run(r#"$[toast("x", "catastrophe")]"#).is_err());
    }

    #[test]
    fn a_non_string_argument_renders_the_way_output_does() {
        let (_, effects) = run(r#"$[sys([1, 2])]"#).unwrap();
        assert!(matches!(
            effects.as_slice(),
            [CommandEffect::Sys { content }] if !content.is_empty()
        ));
    }

    #[test]
    fn several_calls_keep_their_order_for_the_anchor_message() {
        let (_, effects) = run(r#"$[sys("first")]$[toast("noise")]$[sys("second")]"#).unwrap();
        let sys: Vec<&str> = effects
            .iter()
            .filter_map(|e| match e {
                CommandEffect::Sys { content } => Some(content.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(sys, ["first", "second"]);
    }
}
