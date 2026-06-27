use crate::error::{Error, Result};
use crate::types::{ContentBlock, Delta};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait DeltaStreamExt: Stream {
    fn collect_string(self) -> CollectString<Self>
    where
        Self: Sized + Stream<Item = Result<crate::types::Delta>>,
    {
        CollectString {
            stream: self,
            accumulated: String::new(),
        }
    }
}

impl<T: Stream> DeltaStreamExt for T {}

pub struct CollectString<S> {
    stream: S,
    accumulated: String,
}

impl<S> std::future::Future for CollectString<S>
where
    S: Stream<Item = Result<crate::types::Delta>> + Unpin,
{
    type Output = Result<String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(delta))) => {
                    self.accumulated.push_str(&delta.content);
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                Poll::Ready(None) => return Poll::Ready(Ok(self.accumulated.clone())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Drain a delta stream into content blocks: one `Thinking` block for any
/// accumulated reasoning, one `Text` block for the accumulated text (if any),
/// followed by one `ToolUse` block per completed tool call, with argument
/// fragments reassembled by index. Also returns the final finish reason.
/// Push the blocks as an assistant message to continue the conversation.
pub async fn collect_blocks<S>(mut stream: S) -> Result<(Vec<ContentBlock>, Option<String>)>
where
    S: Stream<Item = Result<Delta>> + Unpin,
{
    let mut text = String::new();
    let mut reasoning = String::new();
    // (id, name, accumulated arguments) per tool call index
    let mut calls: Vec<(Option<String>, Option<String>, String)> = Vec::new();
    let mut finish_reason = None;

    while let Some(delta) = stream.next().await {
        let delta = delta?;
        text.push_str(&delta.content);
        reasoning.push_str(&delta.reasoning);
        for tc in delta.tool_calls {
            if calls.len() <= tc.index {
                calls.resize(tc.index + 1, (None, None, String::new()));
            }
            let slot = &mut calls[tc.index];
            if tc.id.is_some() {
                slot.0 = tc.id;
            }
            if tc.name.is_some() {
                slot.1 = tc.name;
            }
            slot.2.push_str(&tc.arguments);
        }
        if delta.finish_reason.is_some() {
            finish_reason = delta.finish_reason;
        }
    }

    let mut blocks = Vec::new();
    if !reasoning.is_empty() {
        blocks.push(ContentBlock::Thinking { text: reasoning });
    }
    if !text.is_empty() {
        blocks.push(ContentBlock::Text { text });
    }
    for (id, name, arguments) in calls {
        let input = if arguments.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&arguments)?
        };
        blocks.push(ContentBlock::ToolUse {
            id: id.unwrap_or_default(),
            name: name.unwrap_or_default(),
            input,
        });
    }
    Ok((blocks, finish_reason))
}

pub fn parse_sse_stream<S, F, T>(
    byte_stream: S,
    parser: F,
) -> impl Stream<Item = Result<T>> + Unpin + Send
where
    S: Stream<Item = reqwest::Result<Bytes>> + Unpin + Send + 'static,
    F: Fn(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    Box::pin(futures::stream::unfold(
        (byte_stream, parser, String::new()),
        |(mut stream, parser, mut buffer)| async move {
            loop {
                // Drain complete events (separated by double newlines) already
                // in the buffer before reading more bytes, so events left over
                // from a previous chunk aren't delayed or dropped.
                while let Some(event_end) = buffer.find("\n\n") {
                    let event = buffer[..event_end].to_string();
                    buffer = buffer[event_end + 2..].to_string();
                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            match parser(data) {
                                Ok(Some(item)) => {
                                    return Some((Ok(item), (stream, parser, buffer)));
                                }
                                Ok(None) => continue,
                                Err(e) => return Some((Err(e), (stream, parser, buffer))),
                            }
                        }
                    }
                }

                match stream.next().await {
                    Some(Ok(chunk)) => match std::str::from_utf8(&chunk) {
                        Ok(s) => buffer.push_str(s),
                        Err(e) => {
                            return Some((
                                Err(Error::StreamError(format!("Invalid UTF-8: {}", e))),
                                (stream, parser, buffer),
                            ));
                        }
                    },
                    Some(Err(e)) => return Some((Err(Error::Http(e)), (stream, parser, buffer))),
                    None => return None,
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolCallDelta;
    use futures::executor::block_on;

    #[test]
    fn sse_handles_split_and_batched_events() {
        // One event split across two chunks, plus two complete events in one chunk.
        let chunks: Vec<reqwest::Result<Bytes>> = vec![
            Ok(Bytes::from("data: \"a\"\n\ndata: \"b")),
            Ok(Bytes::from("\"\n\ndata: \"c\"\n\ndata: [DONE]\n\n")),
        ];
        let stream = parse_sse_stream(futures::stream::iter(chunks), |data| {
            if data == "[DONE]" {
                return Ok(None);
            }
            Ok(Some(serde_json::from_str::<String>(data)?))
        });
        let items: Vec<String> = block_on(stream.map(|r| r.unwrap()).collect());
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn collect_blocks_reassembles_tool_calls() {
        let deltas = vec![
            Ok(Delta {
                content: "Hi ".into(),
                reasoning: "Let me think.".into(),
                tool_calls: vec![],
                finish_reason: None,
            }),
            Ok(Delta {
                content: "there".into(),
                reasoning: String::new(),
                tool_calls: vec![ToolCallDelta {
                    index: 0,
                    id: Some("call_1".into()),
                    name: Some("add".into()),
                    arguments: "{\"a\":".into(),
                }],
                finish_reason: None,
            }),
            Ok(Delta {
                content: String::new(),
                reasoning: String::new(),
                tool_calls: vec![ToolCallDelta {
                    index: 0,
                    id: None,
                    name: None,
                    arguments: "1,\"b\":2}".into(),
                }],
                finish_reason: Some("tool_calls".into()),
            }),
        ];
        let (blocks, finish) = block_on(collect_blocks(futures::stream::iter(deltas))).unwrap();
        assert_eq!(finish.as_deref(), Some("tool_calls"));
        assert_eq!(blocks.len(), 3);
        match &blocks[0] {
            ContentBlock::Thinking { text } => assert_eq!(text, "Let me think."),
            _ => panic!("expected thinking block"),
        }
        match &blocks[1] {
            ContentBlock::Text { text } => assert_eq!(text, "Hi there"),
            _ => panic!("expected text block"),
        }
        match &blocks[2] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "add");
                assert_eq!(input, &serde_json::json!({"a": 1, "b": 2}));
            }
            _ => panic!("expected tool use block"),
        }
    }
}
