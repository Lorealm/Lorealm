use crate::{
    error::{Error, Result},
    types::ToolDef,
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;

/// An executable tool the model can invoke. This is the untyped core that the
/// registry stores; prefer implementing [`TypedTool`] instead, which derives
/// the schema and parses the input for you.
#[async_trait]
pub trait Tool: Send + Sync {
    fn def(&self) -> ToolDef;
    async fn call(&self, input: serde_json::Value) -> Result<serde_json::Value>;
}

/// A tool with a typed, serde-derived input. The JSON Schema is generated
/// from `Input` via schemars, so the schema the model sees and the struct
/// your code receives can never drift apart. Derive both on one struct:
///
/// ```ignore
/// #[derive(Deserialize, JsonSchema)]
/// struct AddInput {
///     /// The first number.
///     a: f64,
///     /// The second number.
///     b: f64,
/// }
/// ```
///
/// Doc comments become field descriptions in the schema.
#[async_trait]
pub trait TypedTool: Send + Sync {
    type Input: DeserializeOwned + JsonSchema + Send;
    type Output: Serialize;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn call(&self, input: Self::Input) -> Result<Self::Output>;
}

/// Adapter that erases a [`TypedTool`] into a [`Tool`].
struct Typed<T>(T);

#[async_trait]
impl<T: TypedTool> Tool for Typed<T> {
    fn def(&self) -> ToolDef {
        // Inline subschemas: LLM providers don't reliably resolve $ref.
        let mut settings = schemars::generate::SchemaSettings::draft07();
        settings.inline_subschemas = true;
        let schema = settings.into_generator().into_root_schema_for::<T::Input>();
        let mut input_schema =
            serde_json::to_value(schema).expect("schema serialization cannot fail");
        if let Some(obj) = input_schema.as_object_mut() {
            obj.remove("$schema");
            obj.remove("title");
        }
        ToolDef {
            name: self.0.name().to_string(),
            description: self.0.description().to_string(),
            input_schema,
        }
    }

    async fn call(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let input: T::Input = serde_json::from_value(input)
            .map_err(|e| Error::Tool(format!("invalid input for {}: {}", self.0.name(), e)))?;
        Ok(serde_json::to_value(self.0.call(input).await?)?)
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.def().name, Box::new(tool));
    }

    pub fn register_typed(&mut self, tool: impl TypedTool + 'static) {
        self.register(Typed(tool));
    }

    pub fn defs(&self) -> Vec<ToolDef> {
        self.tools.values().map(|t| t.def()).collect()
    }

    pub async fn call(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value> {
        match self.tools.get(name) {
            Some(tool) => tool.call(input).await,
            None => Err(Error::Tool(format!("unknown tool: {}", name))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use serde::Deserialize;

    #[derive(Deserialize, JsonSchema)]
    struct AddInput {
        /// The first number.
        a: f64,
        /// The second number.
        b: f64,
    }

    struct Adder;

    #[async_trait]
    impl TypedTool for Adder {
        type Input = AddInput;
        type Output = f64;

        fn name(&self) -> &str {
            "add"
        }
        fn description(&self) -> &str {
            "Add two numbers"
        }
        async fn call(&self, input: AddInput) -> Result<f64> {
            Ok(input.a + input.b)
        }
    }

    #[test]
    fn typed_tool_derives_schema_and_parses_input() {
        let mut registry = ToolRegistry::new();
        registry.register_typed(Adder);

        let def = &registry.defs()[0];
        assert_eq!(def.name, "add");
        assert_eq!(def.input_schema["type"], "object");
        assert_eq!(def.input_schema["properties"]["a"]["type"], "number");
        assert_eq!(
            def.input_schema["properties"]["a"]["description"],
            "The first number."
        );
        assert_eq!(def.input_schema["required"], serde_json::json!(["a", "b"]));

        let result = block_on(registry.call("add", serde_json::json!({"a": 2, "b": 3}))).unwrap();
        assert_eq!(result, serde_json::json!(5.0));

        // Malformed input becomes a Tool error (which send_with_tools feeds
        // back to the model) instead of a panic.
        let err = block_on(registry.call("add", serde_json::json!({"a": "x"}))).unwrap_err();
        assert!(matches!(err, Error::Tool(_)));
    }
}
