use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use bytes::Bytes;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::p2::{OutputStream, Pollable, StreamError};

use crate::host::lorealm::plugin::types::Level;

/// Longer messages are truncated.
const MAX_MESSAGE: usize = 4096;
/// Entries past this in a single call are dropped and counted.
const MAX_ENTRIES_PER_CALL: usize = 100;
/// Stdout or stderr past this in a single call is dropped.
const MAX_CAPTURE: usize = 16 * 1024;

/// Who wrote a log entry. Only the host sets this, so a plugin can't pass its
/// own lines off as the host's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Host,
    Guest,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    pub source: LogSource,
    pub message: String,
}

/// Entries collected during one call into a plugin, handed to the embedder
/// when the call returns.
#[derive(Default)]
pub(crate) struct LogBuffer {
    entries: Vec<LogEntry>,
    dropped: usize,
    pub stdout: Capture,
    pub stderr: Capture,
}

impl LogBuffer {
    pub fn record(&mut self, level: Level, source: LogSource, mut message: String) {
        if self.entries.len() >= MAX_ENTRIES_PER_CALL {
            self.dropped += 1;
            return;
        }
        if message.len() > MAX_MESSAGE {
            let mut end = MAX_MESSAGE;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
            message.push('…');
        }
        self.entries.push(LogEntry {
            level,
            source,
            message,
        });
    }

    /// Everything recorded since the last drain, captured output included,
    /// followed by `error` (the call's outcome) as a host entry.
    pub fn drain(&mut self, error: Option<String>) -> Vec<LogEntry> {
        for (output, level) in [
            (self.stdout.take(), Level::Info),
            (self.stderr.take(), Level::Warn),
        ] {
            for line in String::from_utf8_lossy(&output).lines() {
                if !line.trim().is_empty() {
                    self.record(level, LogSource::Guest, line.to_owned());
                }
            }
        }

        if let Some(error) = error {
            self.record(Level::Error, LogSource::Host, error);
        }

        let mut entries = std::mem::take(&mut self.entries);
        if self.dropped > 0 {
            entries.push(LogEntry {
                level: Level::Warn,
                source: LogSource::Host,
                message: format!(
                    "{} log entries dropped: at most {MAX_ENTRIES_PER_CALL} are kept per call",
                    self.dropped
                ),
            });
            self.dropped = 0;
        }
        entries
    }
}

/// A WASI stdout/stderr that buffers what the plugin writes. Never blocks or
/// fails; output past [`MAX_CAPTURE`] is dropped.
#[derive(Clone, Default)]
pub(crate) struct Capture(Arc<Mutex<Vec<u8>>>);

impl Capture {
    fn append(&self, bytes: &[u8]) {
        let mut buf = self.0.lock().unwrap();
        let room = MAX_CAPTURE.saturating_sub(buf.len());
        buf.extend_from_slice(&bytes[..bytes.len().min(room)]);
    }

    fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl IsTerminal for Capture {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for Capture {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(self.clone())
    }

    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl OutputStream for Capture {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        self.append(&bytes);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        Ok(())
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        Ok(MAX_CAPTURE)
    }
}

#[async_trait::async_trait]
impl Pollable for Capture {
    async fn ready(&mut self) {}
}

impl tokio::io::AsyncWrite for Capture {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.append(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_messages_are_truncated_on_a_char_boundary() {
        let mut logs = LogBuffer::default();
        logs.record(Level::Info, LogSource::Guest, "é".repeat(MAX_MESSAGE));

        let entries = logs.drain(None);

        assert!(entries[0].message.len() <= MAX_MESSAGE + '…'.len_utf8());
        assert!(entries[0].message.ends_with('…'));
    }

    #[test]
    fn entries_past_the_cap_are_counted() {
        let mut logs = LogBuffer::default();
        for i in 0..MAX_ENTRIES_PER_CALL + 5 {
            logs.record(Level::Info, LogSource::Guest, i.to_string());
        }

        let entries = logs.drain(None);

        assert_eq!(entries.len(), MAX_ENTRIES_PER_CALL + 1);
        let note = entries.last().unwrap();
        assert_eq!(note.source, LogSource::Host);
        assert!(note.message.starts_with("5 log entries dropped"));
    }

    #[test]
    fn output_comes_before_the_calls_outcome() {
        let mut logs = LogBuffer::default();
        logs.stderr.append(b"traceback\n\n");

        let entries = logs.drain(Some("trapped".into()));

        let order: Vec<_> = entries
            .iter()
            .map(|e| (e.source, e.message.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                (LogSource::Guest, "traceback"),
                (LogSource::Host, "trapped")
            ]
        );
    }
}
