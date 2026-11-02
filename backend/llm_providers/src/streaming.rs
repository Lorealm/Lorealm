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

/// Drain a delta stream into content blocks, in the order they arrived:
/// consecutive reasoning becomes one `Thinking` block (closed early by a
/// signature), consecutive text one `Text` block, and each tool call one
/// `ToolUse` block with argument fragments reassembled by index. Also
/// returns the final finish reason. Push the blocks as an assistant message
/// to continue the conversation.
pub async fn collect_blocks<S>(mut stream: S) -> Result<(Vec<ContentBlock>, Option<String>)>
where
    S: Stream<Item = Result<Delta>> + Unpin,
{
    let mut acc = BlockAccumulator::default();
    while let Some(delta) = stream.next().await {
        acc.push(delta?);
    }
    acc.finish()
}

/// The synchronous core of [`collect_blocks`], for callers that need to act
/// on each delta as it arrives, and for providers whose non-streaming
/// responses are easiest to build from deltas.
#[derive(Default)]
pub struct BlockAccumulator {
    blocks: Vec<ContentBlock>,
    /// Per tool call index: (position in `blocks`, accumulated arguments).
    calls: Vec<Option<(usize, String)>>,
    finish_reason: Option<String>,
}

impl BlockAccumulator {
    pub fn push(&mut self, delta: Delta) {
        if !delta.reasoning.is_empty() {
            match self.blocks.last_mut() {
                Some(ContentBlock::Thinking {
                    text,
                    signature: None,
                }) => text.push_str(&delta.reasoning),
                _ => self.blocks.push(ContentBlock::Thinking {
                    text: delta.reasoning,
                    signature: None,
                }),
            }
        }
        if let Some(sig) = delta.reasoning_signature {
            match self.blocks.last_mut() {
                Some(ContentBlock::Thinking { signature, .. }) if signature.is_none() => {
                    *signature = Some(sig)
                }
                _ => self.blocks.push(ContentBlock::Thinking {
                    text: String::new(),
                    signature: Some(sig),
                }),
            }
        }
        if !delta.content.is_empty() {
            match self.blocks.last_mut() {
                Some(ContentBlock::Text { text }) => text.push_str(&delta.content),
                _ => self.blocks.push(ContentBlock::Text {
                    text: delta.content,
                }),
            }
        }
        for tc in delta.tool_calls {
            if self.calls.len() <= tc.index {
                self.calls.resize(tc.index + 1, None);
            }
            let (pos, arguments) = self.calls[tc.index].get_or_insert_with(|| {
                self.blocks.push(ContentBlock::ToolUse {
                    id: String::new(),
                    name: String::new(),
                    input: serde_json::Value::Null,
                });
                (self.blocks.len() - 1, String::new())
            });
            if let ContentBlock::ToolUse { id, name, .. } = &mut self.blocks[*pos] {
                if let Some(new_id) = tc.id {
                    *id = new_id;
                }
                if let Some(new_name) = tc.name {
                    *name = new_name;
                }
            }
            arguments.push_str(&tc.arguments);
        }
        if delta.finish_reason.is_some() {
            self.finish_reason = delta.finish_reason;
        }
    }

    pub fn finish(mut self) -> Result<(Vec<ContentBlock>, Option<String>)> {
        for (pos, arguments) in self.calls.into_iter().flatten() {
            if arguments.is_empty() {
                continue;
            }
            if let ContentBlock::ToolUse { input, .. } = &mut self.blocks[pos] {
                *input = serde_json::from_str(&arguments)?;
            }
        }
        Ok((self.blocks, self.finish_reason))
    }
}

/// Split a byte stream into Server-Sent Events and run `parser` on each
/// `data:` payload. `Ok(None)` from the parser skips the event. The parser
/// may keep state across events (e.g. to map provider block indices).
pub fn parse_sse_stream<S, F, T>(
    byte_stream: S,
    parser: F,
) -> impl Stream<Item = Result<T>> + Unpin + Send
where
    S: Stream<Item = reqwest::Result<Bytes>> + Unpin + Send + 'static,
    F: FnMut(&str) -> Result<Option<T>> + Send + 'static,
    T: Send + 'static,
{
    Box::pin(futures::stream::unfold(
        (byte_stream, parser, String::new(), Vec::new()),
        |(mut stream, mut parser, mut buffer, mut partial)| async move {
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
                                    return Some((Ok(item), (stream, parser, buffer, partial)));
                                }
                                Ok(None) => continue,
                                Err(e) => {
                                    return Some((Err(e), (stream, parser, buffer, partial)));
                                }
                            }
                        }
                    }
                }

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        // A multi-byte character may straddle two chunks; keep
                        // its leading bytes in `partial` until the rest arrives.
                        partial.extend_from_slice(&chunk);
                        let valid = match std::str::from_utf8(&partial) {
                            Ok(s) => s.len(),
                            Err(e) if e.error_len().is_none() => e.valid_up_to(),
                            Err(e) => {
                                return Some((
                                    Err(Error::StreamError(format!("Invalid UTF-8: {}", e))),
                                    (stream, parser, buffer, partial),
                                ));
                            }
                        };
                        let text = std::str::from_utf8(&partial[..valid]).expect("validated above");
                        buffer.push_str(text);
                        partial.drain(..valid);
                        // Google's endpoints separate events with CRLF pairs.
                        if buffer.contains('\r') {
                            buffer = buffer.replace("\r\n", "\n");
                        }
                    }
                    Some(Err(e)) => {
                        return Some((Err(Error::Http(e)), (stream, parser, buffer, partial)));
                    }
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
    fn sse_handles_crlf_and_split_multibyte_characters() {
        // "é" is 0xC3 0xA9; split it across chunks, with CRLF event endings.
        let chunks: Vec<reqwest::Result<Bytes>> = vec![
            Ok(Bytes::from_static(b"data: \"caf\xC3")),
            Ok(Bytes::from_static(b"\xA9\"\r\n\r")),
            Ok(Bytes::from_static(b"\ndata: \"x\"\r\n\r\n")),
        ];
        let stream = parse_sse_stream(futures::stream::iter(chunks), |data| {
            Ok(Some(serde_json::from_str::<String>(data)?))
        });
        let items: Vec<String> = block_on(stream.map(|r| r.unwrap()).collect());
        assert_eq!(items, vec!["café", "x"]);
    }

    #[test]
    fn collect_blocks_reassembles_tool_calls() {
        let deltas = vec![
            Ok(Delta {
                content: "Hi ".into(),
                reasoning: "Let me think.".into(),
                ..Delta::default()
            }),
            Ok(Delta {
                content: "there".into(),
                tool_calls: vec![ToolCallDelta {
                    index: 0,
                    id: Some("call_1".into()),
                    name: Some("add".into()),
                    arguments: "{\"a\":".into(),
                }],
                ..Delta::default()
            }),
            Ok(Delta {
                tool_calls: vec![ToolCallDelta {
                    index: 0,
                    id: None,
                    name: None,
                    arguments: "1,\"b\":2}".into(),
                }],
                finish_reason: Some("tool_calls".into()),
                ..Delta::default()
            }),
        ];
        let (blocks, finish) = block_on(collect_blocks(futures::stream::iter(deltas))).unwrap();
        assert_eq!(finish.as_deref(), Some("tool_calls"));
        assert_eq!(blocks.len(), 3);
        match &blocks[0] {
            ContentBlock::Thinking { text, signature } => {
                assert_eq!(text, "Let me think.");
                assert!(signature.is_none());
            }
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

    #[test]
    fn collect_blocks_keeps_signed_blocks_separate_and_in_order() {
        let reasoning = |r: &str| Delta {
            reasoning: r.into(),
            ..Delta::default()
        };
        let signature = |s: &str| Delta {
            reasoning_signature: Some(s.into()),
            ..Delta::default()
        };
        let deltas = vec![
            Ok(reasoning("first ")),
            Ok(reasoning("thought")),
            Ok(signature("sig1")),
            Ok(Delta {
                content: "answer".into(),
                ..Delta::default()
            }),
            Ok(reasoning("second")),
            Ok(signature("sig2")),
            // A signature with no reasoning before it stands alone.
            Ok(signature("sig3")),
        ];
        let (blocks, _) = block_on(collect_blocks(futures::stream::iter(deltas))).unwrap();
        let shape: Vec<(String, Option<String>)> = blocks
            .into_iter()
            .map(|b| match b {
                ContentBlock::Thinking { text, signature } => (format!("think:{text}"), signature),
                ContentBlock::Text { text } => (format!("text:{text}"), None),
                other => panic!("unexpected block {other:?}"),
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                ("think:first thought".into(), Some("sig1".into())),
                ("text:answer".into(), None),
                ("think:second".into(), Some("sig2".into())),
                ("think:".into(), Some("sig3".into())),
            ]
        );
    }
}
