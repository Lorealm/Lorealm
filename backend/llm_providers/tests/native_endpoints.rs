//! Drives `AnthropicProvider` and `VertexProvider` against a stub HTTP
//! server replaying their SSE formats, checking paths, auth headers and that
//! a streamed tool call with signed reasoning comes back as ordered blocks.
//!
//! Written with the help of Claude Opus 5.5.

use lorealm_providers::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serve one request with `sse` as the body, returning the raw request.
async fn stub_server(sse: &'static str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        // Read until the full body has arrived; it may span several reads.
        let mut request = Vec::new();
        let mut buf = vec![0u8; 8192];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            request.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&request);
            if let Some(head_end) = text.find("\r\n\r\n") {
                let length = text[..head_end]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + length {
                    break;
                }
            }
        }

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
            sse.len(),
            sse
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();
        String::from_utf8_lossy(&request).to_string()
    });

    (format!("http://{}", addr), handle)
}

fn add_tool() -> ToolDef {
    ToolDef {
        name: "add".into(),
        description: "Add two numbers".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

#[tokio::test]
async fn anthropic_streams_signed_thinking_and_tool_calls() {
    let sse = concat!(
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\"}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Sum it.\"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"add\",\"input\":{}}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"a\\\":2}\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":9}}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );
    let (base_url, server) = stub_server(sse).await;

    let provider = AnthropicProvider::new(
        Arc::new(reqwest::Client::new()),
        "claude-opus-5",
        Some(format!("{}/v1", base_url)),
        "test-key",
    );
    let client = Client::new(provider);
    let stream = client
        .request()
        .user("What is 2 + 0?")
        .tools(vec![add_tool()])
        .stream()
        .await
        .unwrap();
    let (blocks, finish) = collect_blocks(stream).await.unwrap();

    assert_eq!(finish.as_deref(), Some("tool_use"));
    assert!(matches!(
        &blocks[0],
        ContentBlock::Thinking { text, signature: Some(s) } if text == "Sum it." && s == "sig"
    ));
    assert!(matches!(
        &blocks[1],
        ContentBlock::ToolUse { id, name, input }
            if id == "toolu_1" && name == "add" && input == &serde_json::json!({"a": 2})
    ));

    let request = server.await.unwrap();
    assert!(request.starts_with("POST /v1/messages "));
    assert!(request.contains("x-api-key: test-key"));
    assert!(request.contains("anthropic-version: 2023-06-01"));
    assert!(request.contains(r#""stream":true"#));
    assert!(request.contains(r#""input_schema":{"type":"object"}"#));
}

#[tokio::test]
async fn vertex_streams_crlf_events_with_thought_signatures() {
    // Google separates SSE events with CRLF pairs.
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Sum it.\",\"thought\":true}]}}]}\r\n\r\n",
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"name\":\"add\",\"args\":{\"a\":2}},\"thoughtSignature\":\"sig\"}]},\"finishReason\":\"STOP\"}],\"modelVersion\":\"gemini-3-pro\"}\r\n\r\n",
    );
    let (base_url, server) = stub_server(sse).await;

    let provider = VertexProvider::new(
        Arc::new(reqwest::Client::new()),
        "gemini-3-pro",
        VertexAuth::AccessToken("test-token".into()),
        Some("proj".into()),
        None,
    )
    .with_base_url(format!(
        "{}/v1/projects/proj/locations/global/publishers/google",
        base_url
    ));
    let client = Client::new(provider);
    let stream = client
        .request()
        .user("What is 2 + 0?")
        .tools(vec![add_tool()])
        .stream()
        .await
        .unwrap();
    let (blocks, finish) = collect_blocks(stream).await.unwrap();

    assert_eq!(finish.as_deref(), Some("STOP"));
    assert!(matches!(
        &blocks[0],
        ContentBlock::Thinking { text, signature: Some(s) } if text == "Sum it." && s == "sig"
    ));
    assert!(matches!(
        &blocks[1],
        ContentBlock::ToolUse { name, input, .. }
            if name == "add" && input == &serde_json::json!({"a": 2})
    ));

    let request = server.await.unwrap();
    assert!(request.starts_with(
        "POST /v1/projects/proj/locations/global/publishers/google/models/gemini-3-pro:streamGenerateContent?alt=sse "
    ));
    assert!(request.contains("authorization: Bearer test-token"));
    assert!(request.contains(r#""functionDeclarations""#));
}
