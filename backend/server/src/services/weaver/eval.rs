//! Running a weaver body, and what comes back out of it.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use lorealm_storage::models::{Param, StateDelta};
use std::collections::BTreeMap;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use weaver_lang::{
    CompiledExpr, CompiledTemplate, EvalError, EvalOptions, InputDecl, Registry,
    Value as WeaverValue, ValueDoc, evaluate_value_doc_with_options, parse_value_doc,
};

use crate::{
    services::{
        chat::WidgetInstance,
        commands::CommandEffect,
        state::StatePool,
        tools::{EmittedEvent, ToolContext},
    },
    util::weaver::{ConversionError, describe_parse_errors, json_to_weaver, weaver_to_json},
};

/// Weaver-related ceiling config
#[derive(Clone)]
pub struct Limits {
    pub max_node_evaluations: Option<u64>,
    pub max_iterations: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_node_evaluations: Some(1_000_000),
            max_iterations: Some(10_000),
        }
    }
}

impl Limits {
    pub(crate) fn options(&self, stop: Option<Arc<AtomicBool>>) -> EvalOptions {
        let mut options = EvalOptions::new();
        if let Some(max) = self.max_node_evaluations {
            options = options.max_node_evaluations(max);
        }
        if let Some(max) = self.max_iterations {
            options = options.max_iterations(max);
        }
        if let Some(stop) = stop {
            options = options.cancellation_token(stop);
        }
        options
    }
}

/// Anything weaver can run.
pub trait Evaluable: Send + 'static {
    fn eval(
        &self,
        ctx: &mut ToolContext,
        registry: &Registry,
        limits: &Limits,
        stop: Option<Arc<AtomicBool>>,
    ) -> Result<WeaverValue, EvalError>;
}

impl Evaluable for CompiledTemplate {
    fn eval(
        &self,
        ctx: &mut ToolContext,
        registry: &Registry,
        limits: &Limits,
        stop: Option<Arc<AtomicBool>>,
    ) -> Result<WeaverValue, EvalError> {
        self.evaluate_value_with_options(ctx, registry, limits.options(stop))
    }
}

impl Evaluable for CompiledExpr {
    fn eval(
        &self,
        ctx: &mut ToolContext,
        registry: &Registry,
        _limits: &Limits,
        _stop: Option<Arc<AtomicBool>>,
    ) -> Result<WeaverValue, EvalError> {
        self.evaluate(ctx, registry)
    }
}

/// A parsed WTN document together with the inputs it will be bound to.
pub struct CompiledDoc {
    doc: ValueDoc,
    inputs: BTreeMap<String, WeaverValue>,
}

impl CompiledDoc {
    pub fn declarations(&self) -> &[InputDecl] {
        &self.doc.inputs
    }

    /// The parse tree, for reading a document's structure without running it.
    pub fn document(&self) -> &ValueDoc {
        &self.doc
    }

    pub fn with_inputs(mut self, inputs: BTreeMap<String, WeaverValue>) -> Self {
        self.inputs = inputs;
        self
    }
}

impl Evaluable for CompiledDoc {
    fn eval(
        &self,
        ctx: &mut ToolContext,
        registry: &Registry,
        limits: &Limits,
        stop: Option<Arc<AtomicBool>>,
    ) -> Result<WeaverValue, EvalError> {
        evaluate_value_doc_with_options(
            &self.doc,
            &self.inputs,
            ctx,
            registry,
            limits.options(stop),
        )
    }
}

/// Everything one evaluation produced.
#[derive(Debug)]
pub struct Outcome {
    /// What the body evaluated to.
    pub value: WeaverValue,
    /// The pool as the body left it.
    pub _pool: StatePool,
    /// What the body changed, against the pool it started from.
    pub delta: StateDelta,
    pub effects: Vec<CommandEffect>,
    pub events: Vec<EmittedEvent>,
    pub _widgets: Vec<WidgetInstance>,
}

impl Outcome {
    /// The body's output as text.
    pub fn to_output_string(&self) -> String {
        self.value.to_output_string()
    }

    /// The body's output as JSON.
    pub fn to_json(&self) -> Result<serde_json::Value, ConversionError> {
        weaver_to_json(&self.value)
    }

    /// Whether the body's output is truthy.
    pub fn is_truthy(&self) -> bool {
        self.value.is_truthy()
    }

    /// Whether the body touched state at all.
    pub fn wrote_state(&self) -> bool {
        !self.delta.writes.is_empty() || !self.delta.deletes.is_empty()
    }
}

#[derive(Debug, Error)]
pub enum WeaverError {
    /// The source did not compile. Already a sentence; See
    /// [`describe_parse_errors`].
    #[error("{0}")]
    Compile(String),
    #[error("{0}")]
    Eval(#[from] EvalError),
    /// The blocking thread panicked or was aborted mid-body.
    #[error("evaluation did not finish: {0}")]
    DidNotFinish(String),
    /// A required parameter was absent, or an argument would not convert.
    #[error("parameter '{name}': {reason}")]
    Parameter { name: String, reason: String },
    /// A host carrying [`Caps::GENERATION`] was evaluated inline. Those
    /// commands `block_on` a future, which panics on a runtime worker.
    ///
    /// [`Caps::GENERATION`]: super::Caps::GENERATION
    #[error("this host must be evaluated off the async runtime")]
    BlockingRequired,
}

/// Compile a template, reporting failure as one readable sentence.
pub fn compile_template(source: &str) -> Result<CompiledTemplate, WeaverError> {
    CompiledTemplate::compile(source)
        .map_err(|errors| WeaverError::Compile(describe_parse_errors(source, &errors)))
}

/// Parse a WTN document and bind its inputs, reporting failure as one
/// readable sentence.
pub fn compile_doc(source: &str) -> Result<CompiledDoc, WeaverError> {
    let doc = parse_value_doc(source)
        .map_err(|errors| WeaverError::Compile(describe_parse_errors(source, &errors)))?;
    Ok(CompiledDoc {
        doc,
        inputs: BTreeMap::new(),
    })
}

/// Compile an expression, reporting failure as one readable sentence.
pub fn compile_expr(source: &str) -> Result<CompiledExpr, WeaverError> {
    CompiledExpr::compile(source)
        .map_err(|errors| WeaverError::Compile(describe_parse_errors(source, &errors)))
}

/// Bind declared `params` from a JSON argument object into the `cmd:`
/// namespace a body reads.
pub fn bind_json_args(
    params: &[Param],
    args: &serde_json::Value,
) -> Result<std::collections::HashMap<String, WeaverValue>, WeaverError> {
    let args = args.as_object().cloned().unwrap_or_default();
    let mut bound = std::collections::HashMap::new();

    for param in params {
        match args.get(&param.name) {
            Some(value) => {
                let value = json_to_weaver(value).map_err(|e| WeaverError::Parameter {
                    name: param.name.clone(),
                    reason: e.to_string(),
                })?;
                bound.insert(param.name.clone(), value);
            }
            None if param.required => {
                return Err(WeaverError::Parameter {
                    name: param.name.clone(),
                    reason: "is required".into(),
                });
            }
            None => {
                bound.insert(param.name.clone(), WeaverValue::None);
            }
        }
    }

    Ok(bound)
}

pub(super) fn cancellation_flag(
    token: Option<CancellationToken>,
) -> (Option<Arc<AtomicBool>>, Option<tokio::task::JoinHandle<()>>) {
    let Some(token) = token else {
        return (None, None);
    };
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = tokio::spawn({
        let stop = Arc::clone(&stop);
        async move {
            token.cancelled().await;
            stop.store(true, Ordering::Relaxed);
        }
    });
    (Some(stop), Some(watcher))
}
