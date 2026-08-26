//! Drives `OpenAiProvider` against a stub HTTP server that answers the way a
//! local KoboldCPP-style endpoint does: no authentication, SSE streaming, and
//! a bare `/models` listing.
//! 
//! This particular test was generated with Claude Opus 5

use futures::StreamExt;
use lorealm_providers::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn stub_server() -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        let mut seen = Vec::new();
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            let body = if request.starts_with("GET /v1/models") {
                let json = r#"{"object":"list","data":[{"id":"koboldcpp/L3","object":"model"}]}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    json.len(),
                    json
                )
            } else {
                let sse = concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
                    "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n"
                );
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
                    sse.len(),
                    sse
                )
            };

            socket.write_all(body.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
            seen.push(request);
        }
        seen
    });

    (format!("http://{}/v1", addr), handle)
}

#[tokio::test]
async fn streams_and_lists_models_without_an_api_key() {
    let (base_url, server) = stub_server().await;

    let provider = OpenAiProvider::new(
        Arc::new(reqwest::Client::new()),
        "koboldcpp/L3",
        Some(base_url),
        None,
    );
    let client = Client::new(provider);

    let mut stream = client
        .request()
        .user("hi")
        .temperature(0.7)
        .stream()
        .await
        .unwrap();

    let mut text = String::new();
    while let Some(delta) = stream.next().await {
        text.push_str(&delta.unwrap().content);
    }
    assert_eq!(text, "Hello");

    let models = client.list_models().await.unwrap().unwrap();
    assert_eq!(models[0].id, "koboldcpp/L3");

    let requests = server.await.unwrap();
    // No key configured => no Authorization header on either request.
    assert!(!requests.iter().any(|r| r.contains("authorization")));
    assert!(requests[0].contains("POST /v1/chat/completions"));
    assert!(requests[0].contains(r#""stream":true"#));
    assert!(requests[0].contains(r#""temperature":0.7"#));
}
