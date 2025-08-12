use std::{pin::Pin, task::{Context, Poll}};

use bytes::Bytes;
use futures::Stream;

pub struct SseStream<S> {
    inner: S,
    buffer: String,
}

impl<S> SseStream<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<SseEvent, Box<dyn std::error::Error + Send + Sync>>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Try to parse an event from the buffer first
            if let Some(event) = self.try_parse_event() {
                return Poll::Ready(Some(Ok(event)));
            }
            
            // Need more data
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(Box::new(e))));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S> SseStream<S> {
    fn try_parse_event(&mut self) -> Option<SseEvent> {
        // Look for complete event (double newline)
        if let Some(pos) = self.buffer.find("\n\n") {
            let event_text = self.buffer.drain(..=pos + 1).collect::<String>();
            
            let mut data = None;
            for line in event_text.lines() {
                if let Some(content) = line.strip_prefix("data: ") {
                    data = Some(content.to_string());
                }
            }
            
            data.map(|d| SseEvent { data: d })
        } else {
            None
        }
    }
}

pub struct SseEvent {
    pub data: String,
}