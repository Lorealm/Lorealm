//! Running turns and load steps.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering::Relaxed},
    },
    time::{Duration, Instant},
};

use bench::{TAG_STREAM, TAG_TTFT, read_tag, rng::Rng};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio::{task::JoinSet, time::sleep_until};

use crate::api::{Api, Conversation};

/// How a turn ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Ok,
    /// The turn request itself was refused.
    Http(u16),
    /// The server sent an `error` event.
    StreamError(String),
    /// The stream ended without a `done` event.
    Incomplete,
    /// Gave up waiting; says how far the turn had got.
    Timeout(&'static str),
    Transport(String),
}

impl Outcome {
    pub fn label(&self) -> String {
        match self {
            Outcome::Ok => "ok".into(),
            Outcome::Http(status) => format!("http {status}"),
            Outcome::StreamError(code) => format!("stream error: {code}"),
            Outcome::Incomplete => "stream ended early".into(),
            Outcome::Timeout(stage) => format!("timeout {stage}"),
            Outcome::Transport(e) => format!("transport: {e}"),
        }
    }
}

/// Everything measured about one turn, in milliseconds.
#[derive(Debug, Clone)]
pub struct Turn {
    pub outcome: Outcome,
    /// Request sent → response headers.
    pub headers_ms: Option<f64>,
    /// Request sent (or scheduled, in open-loop mode) → first text.
    pub ttft_ms: Option<f64>,
    pub total_ms: f64,
    /// What the mock reports for the same span on its side.
    pub mock_ttft_ms: Option<f64>,
    pub mock_stream_ms: Option<f64>,
    /// First text → last text, as the client saw it.
    pub client_stream_ms: Option<f64>,
    /// Last text → `done`: the server's work after the model finishes.
    pub persist_ms: Option<f64>,
    /// Gaps between consecutive text events.
    pub gaps: Vec<f64>,
}

impl Turn {
    /// Time the server added before the first token.
    pub fn overhead_ttft_ms(&self) -> Option<f64> {
        Some(self.ttft_ms? - self.mock_ttft_ms?)
    }

    /// Delay the server's relay added across the whole stream.
    pub fn overhead_stream_ms(&self) -> Option<f64> {
        Some(self.client_stream_ms? - self.mock_stream_ms?)
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

pub async fn run_turn(
    api: &Api,
    conv: &Conversation,
    content: &str,
    scheduled: Instant,
    timeout: Duration,
) -> Turn {
    let mut turn = Turn {
        outcome: Outcome::Incomplete,
        headers_ms: None,
        ttft_ms: None,
        total_ms: 0.0,
        mock_ttft_ms: None,
        mock_stream_ms: None,
        client_stream_ms: None,
        persist_ms: None,
        gaps: Vec::new(),
    };
    let result =
        tokio::time::timeout(timeout, drive(api, conv, content, scheduled, &mut turn)).await;
    if result.is_err() {
        turn.outcome = Outcome::Timeout(if turn.headers_ms.is_none() {
            "before the response started"
        } else if turn.ttft_ms.is_none() {
            "before the first token"
        } else if turn.mock_stream_ms.is_some() {
            "after the last token"
        } else {
            "mid-stream"
        });
    }
    turn.total_ms = ms(scheduled.elapsed());
    turn
}

async fn drive(api: &Api, conv: &Conversation, content: &str, scheduled: Instant, turn: &mut Turn) {
    let response = api
        .http
        .post(api.url(&format!("/chat/{}/turn", conv.id)))
        .header("Authorization", conv.session.bearer())
        .json(&json!({ "content": content }))
        .send()
        .await;
    let response = match response {
        Ok(r) => r,
        Err(e) => {
            turn.outcome = Outcome::Transport(short_error(&e));
            return;
        }
    };
    conv.session.absorb(&response);
    turn.headers_ms = Some(ms(scheduled.elapsed()));
    if !response.status().is_success() {
        turn.outcome = Outcome::Http(response.status().as_u16());
        return;
    }

    let mut body = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    let mut text = String::new();
    let mut first_delta: Option<Instant> = None;
    let mut last_delta: Option<Instant> = None;

    while let Some(chunk) = body.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                turn.outcome = Outcome::Transport(short_error(&e));
                return;
            }
        };
        let now = Instant::now();
        buffer.extend_from_slice(&chunk);

        while let Some(end) = find_event_end(&buffer) {
            let event: Vec<u8> = buffer.drain(..end + 2).collect();
            let event = String::from_utf8_lossy(&event);
            let mut kind = "message";
            let mut data = String::new();
            for line in event.lines() {
                if let Some(v) = line.strip_prefix("event:") {
                    kind = v.trim();
                } else if let Some(v) = line.strip_prefix("data:") {
                    data.push_str(v.strip_prefix(' ').unwrap_or(v));
                }
            }
            if data.is_empty() {
                continue; // keep-alive comment
            }
            if kind == "error" {
                let code = serde_json::from_str::<Value>(&data)
                    .ok()
                    .and_then(|v| v["error_code"].as_str().map(str::to_string))
                    .unwrap_or_else(|| data.chars().take(80).collect());
                turn.outcome = Outcome::StreamError(code);
                return;
            }
            let Ok(value) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            match value["type"].as_str() {
                Some("delta") => {
                    if let Some(c) = value["content"].as_str() {
                        text.push_str(c);
                        // Noted as it arrives, so a turn that times out
                        // afterwards still shows the model had finished.
                        if let Some(us) = read_tag(c, TAG_STREAM) {
                            turn.mock_stream_ms = Some(us as f64 / 1000.0);
                        }
                    }
                    if first_delta.is_none() {
                        first_delta = Some(now);
                        turn.ttft_ms = Some(ms(now - scheduled));
                    }
                    if let Some(last) = last_delta {
                        turn.gaps.push(ms(now - last));
                    }
                    last_delta = Some(now);
                }
                Some("done") => {
                    if let Some(last) = last_delta {
                        turn.persist_ms = Some(ms(now - last));
                    }
                    turn.outcome = Outcome::Ok;
                }
                _ => {}
            }
        }
    }

    if let (Some(first), Some(last)) = (first_delta, last_delta) {
        turn.client_stream_ms = Some(ms(last - first));
    }
    turn.mock_ttft_ms = read_tag(&text, TAG_TTFT).map(|us| us as f64 / 1000.0);
    turn.mock_stream_ms = read_tag(&text, TAG_STREAM).map(|us| us as f64 / 1000.0);
}

fn find_event_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|w| w == b"\n\n")
}

fn short_error(e: &reqwest::Error) -> String {
    let kind = if e.is_connect() {
        "connect"
    } else if e.is_timeout() {
        "timeout"
    } else if e.is_body() || e.is_decode() {
        "body"
    } else if e.is_request() {
        "request"
    } else {
        "other"
    };
    kind.to_string()
}

/// Closed mode: how long a conversation waits after a failed turn.
const FAILURE_BACKOFF: Duration = Duration::from_millis(250);

/// Mixed into the seed so think times don't repeat the arrival times.
const THINK_STREAM: u64 = 0x7468_696e_6b00_0000;

#[derive(Debug, Clone, Copy)]
pub enum Load {
    Concurrency(usize),
    Rate(f64),
}

impl Load {
    pub fn describe(&self) -> String {
        match self {
            Load::Concurrency(n) => format!("{n} concurrent conversations"),
            Load::Rate(r) => format!("{r} turns/s arrival rate"),
        }
    }
}

pub struct StepConfig {
    pub load: Load,
    pub warmup: Duration,
    pub measure: Duration,
    pub think: Duration,
    pub timeout: Duration,
    pub content: String,
    pub seed: u64,
}

impl StepConfig {
    pub fn effective_warmup(&self) -> Duration {
        match self.load {
            Load::Concurrency(_) => self.warmup.max(self.think),
            Load::Rate(_) => self.warmup,
        }
    }
}

pub struct StepResult {
    /// When the measured window began (after warm-up).
    pub measured_from: Instant,
    /// Turns that started inside the measured window.
    pub turns: Vec<Turn>,
    pub peak_in_flight: u64,
    /// Open loop only: arrivals dropped because every conversation was busy.
    pub skipped: u64,
    /// Open loop only: how late the generator was in sending a turn.
    pub max_send_lag_ms: f64,
}

#[derive(Default)]
struct Shared {
    turns: Mutex<Vec<Turn>>,
    in_flight: AtomicU64,
    peak: AtomicU64,
}

impl Shared {
    fn begin(&self) {
        let now = self.in_flight.fetch_add(1, Relaxed) + 1;
        self.peak.fetch_max(now, Relaxed);
    }

    fn end(&self, turn: Turn, record: bool) {
        self.in_flight.fetch_sub(1, Relaxed);
        if record {
            self.turns.lock().unwrap().push(turn);
        }
    }
}

pub async fn run_step(api: &Api, convs: &[Conversation], config: &StepConfig) -> StepResult {
    let shared = Arc::new(Shared::default());
    let start = Instant::now();
    let measure_from = start + config.effective_warmup();
    let stop_at = measure_from + config.measure;
    let mut tasks = JoinSet::new();
    let mut skipped = 0;
    let mut max_lag = Duration::ZERO;

    match config.load {
        Load::Concurrency(n) => {
            for (index, conv) in convs.iter().take(n).enumerate() {
                let conv = conv.clone();
                let (api, shared) = (api.clone(), Arc::clone(&shared));

                let (content, think, timeout) =
                    (config.content.clone(), config.think, config.timeout);

                let mut rng = Rng::derive(config.seed ^ THINK_STREAM, index as u64);

                tasks.spawn(async move {
                    if !think.is_zero() {
                        tokio::time::sleep(think.mul_f64(rng.unit())).await;
                    }
                    while Instant::now() < stop_at {
                        let begun = Instant::now();

                        shared.begin();
                        let turn = run_turn(&api, &conv, &content, begun, timeout).await;
                        let failed = turn.outcome != Outcome::Ok;
                        shared.end(turn, begun >= measure_from);

                        let think = think.mul_f64(0.5 + rng.unit());

                        let pause = if failed {
                            think.max(FAILURE_BACKOFF)
                        } else {
                            think
                        };
                        if !pause.is_zero() {
                            tokio::time::sleep(pause).await;
                        }
                    }
                });
            }
        }
        Load::Rate(rate) => {
            let idle: Arc<Mutex<Vec<usize>>> =
                Arc::new(Mutex::new((0..convs.len()).rev().collect()));

            let mut rng = Rng::derive(config.seed, rate.to_bits());
            let mut next = start;

            loop {
                next += Duration::from_secs_f64(rng.exponential(1.0 / rate));
                if next >= stop_at {
                    break;
                }

                sleep_until(next.into()).await;

                max_lag = max_lag.max(Instant::now().saturating_duration_since(next));

                let Some(index) = idle.lock().unwrap().pop() else {
                    skipped += u64::from(next >= measure_from);
                    continue;
                };

                let conv = convs[index].clone();
                let (api, shared, idle) = (api.clone(), Arc::clone(&shared), Arc::clone(&idle));
                let (content, timeout, scheduled) = (config.content.clone(), config.timeout, next);

                shared.begin();
                tasks.spawn(async move {
                    let turn = run_turn(&api, &conv, &content, scheduled, timeout).await;
                    idle.lock().unwrap().push(index);
                    shared.end(turn, scheduled >= measure_from);
                });
            }
        }
    }

    while tasks.join_next().await.is_some() {}

    let turns = std::mem::take(&mut *shared.turns.lock().unwrap());
    StepResult {
        measured_from: measure_from,
        turns,
        peak_in_flight: shared.peak.load(Relaxed),
        skipped,
        max_send_lag_ms: ms(max_lag),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_event_boundaries() {
        assert_eq!(find_event_end(b"data: {}\n\nrest"), Some(8));
        assert_eq!(find_event_end(b"data: {}\n"), None);
    }

    #[test]
    fn overheads_need_both_sides() {
        let mut turn = Turn {
            outcome: Outcome::Ok,
            headers_ms: None,
            ttft_ms: Some(605.0),
            total_ms: 0.0,
            mock_ttft_ms: Some(600.0),
            mock_stream_ms: None,
            client_stream_ms: Some(10.0),
            persist_ms: None,
            gaps: vec![],
        };
        assert_eq!(turn.overhead_ttft_ms(), Some(5.0));
        assert_eq!(turn.overhead_stream_ms(), None);
        turn.mock_stream_ms = Some(9.0);
        assert_eq!(turn.overhead_stream_ms(), Some(1.0));
    }
}
