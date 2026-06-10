use crate::error::{Error, Result};
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
                match stream.next().await {
                    Some(Ok(chunk)) => {
                        let chunk_str = match std::str::from_utf8(&chunk) {
                            Ok(s) => s,
                            Err(e) => {
                                return Some((
                                    Err(Error::StreamError(format!("Invalid UTF-8: {}", e))),
                                    (stream, parser, buffer),
                                ));
                            }
                        };

                        buffer.push_str(chunk_str);

                        // Process complete events (separated by double newlines)
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
                            Some(Err(e)) => {
                                return Some((Err(Error::Http(e)), (stream, parser, buffer)));
                            }
                            None => return None,
                        }
                    }
                    Some(Err(e)) => return Some((Err(Error::Http(e)), (stream, parser, buffer))),
                    None => return None,
                }
            }
        },
    ))
}
