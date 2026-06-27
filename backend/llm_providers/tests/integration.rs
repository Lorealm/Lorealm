use futures::executor::block_on;
use lorealm_providers::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Adder;

#[async_trait::async_trait]
impl Tool for Adder {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "add".into(),
            description: "Add two numbers".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"a": {"type": "number"}, "b": {"type": "number"}}
            }),
        }
    }
    async fn call(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!(
            input["a"].as_f64().unwrap() + input["b"].as_f64().unwrap()
        ))
    }
}

struct MockProvider {
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    type ProviderParams = ();

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        _: &Self::ProviderParams,
    ) -> Result<Response> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(params.tools.is_some(), "tool defs should be advertised");
        if n == 0 {
            Ok(Response {
                content: vec![ContentBlock::ToolUse {
                    id: "call_1".into(),
                    name: "add".into(),
                    input: serde_json::json!({"a": 2, "b": 3}),
                }],
                model: "mock".into(),
                usage: None,
                finish_reason: Some("tool_calls".into()),
            })
        } else {
            let last = messages.last().unwrap();
            match (&last.role, &last.content[0]) {
                (
                    Role::User,
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    },
                ) => {
                    assert_eq!(tool_use_id, "call_1");
                    assert_eq!(content, "5.0");
                    assert!(!is_error);
                }
                other => panic!("expected tool result message, got {:?}", other),
            }
            Ok(Response {
                content: vec![ContentBlock::Text {
                    text: "The answer is 5".into(),
                }],
                model: "mock".into(),
                usage: None,
                finish_reason: Some("stop".into()),
            })
        }
    }

    async fn stream(
        &self,
        _: Vec<Message>,
        _: &CommonParams,
        _: &Self::ProviderParams,
    ) -> Result<Box<dyn futures::Stream<Item = Result<Delta>> + Unpin + Send>> {
        unimplemented!()
    }
}

#[test]
fn tool_loop_executes_and_feeds_back_results() {
    let client = Client::new(MockProvider {
        calls: AtomicUsize::new(0),
    });
    let mut registry = ToolRegistry::new();
    registry.register(Adder);

    let response = block_on(
        client
            .request()
            .user("What is 2 + 3?")
            .send_with_tools(&registry, 5),
    )
    .unwrap();

    assert_eq!(response.text(), "The answer is 5");
}
