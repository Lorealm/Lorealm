//! A fake OpenAI-compatible chat-completions endpoint with tunable latency.

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering::Relaxed},
    },
    time::{Duration, Instant},
};

use argh::FromArgs;
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
    serve::ListenerExt,
};
use bench::{TAG_STREAM, TAG_TTFT, fdlimit, rng::Rng};
use serde_json::{Value, json};
use tokio::{sync::mpsc, time::sleep_until};

#[derive(FromArgs)]
/// Fake OpenAI-compatible LLM endpoint with tunable latency and faults.
struct Args {
    /// address to listen on; repeat to listen on several (e.g. 127.0.0.2:9100)
    /// and spread connections over more ephemeral ports. Default 127.0.0.1:9100
    #[argh(option)]
    listen: Vec<String>,

    /// median time to first token, in ms (default 600)
    #[argh(option, default = "600.0")]
    ttft_ms: f64,
    /// log-normal spread of the time to first token; 0 makes it fixed (default 0.5)
    #[argh(option, default = "0.5")]
    ttft_sigma: f64,
    /// tokens per second once streaming; 0 streams as fast as possible (default 60)
    #[argh(option, default = "60.0")]
    tps: f64,
    /// per-request tokens-per-second spread, as a fraction (default 0.25)
    #[argh(option, default = "0.25")]
    tps_jitter: f64,
    /// fewest tokens per response (default 200)
    #[argh(option, default = "200")]
    min_tokens: u64,
    /// most tokens per response (default 600)
    #[argh(option, default = "600")]
    max_tokens: u64,
    /// tokens per streamed chunk (default 1)
    #[argh(option, default = "1")]
    chunk_tokens: u64,

    /// fraction of requests answered with an error status (default 0)
    #[argh(option, default = "0.0")]
    error_rate: f64,
    /// status code for injected errors (default 500; 429 is also useful)
    #[argh(option, default = "500")]
    error_status: u16,
    /// fraction of streams that pause once partway through (default 0)
    #[argh(option, default = "0.0")]
    stall_rate: f64,
    /// length of an injected pause, in ms (default 5000)
    #[argh(option, default = "5000.0")]
    stall_ms: f64,
    /// fraction of streams cut off partway through (default 0)
    #[argh(option, default = "0.0")]
    drop_rate: f64,

    /// seed for repeatable runs (default: from the clock)
    #[argh(option)]
    seed: Option<u64>,
    /// leave the timing markers out of the generated text
    #[argh(switch)]
    no_tags: bool,
    /// soft open-file limit to raise to: a number, or `max` (default max)
    #[argh(option, default = "String::from(\"max\")")]
    max_open_files: String,
    /// seconds between status lines; 0 turns them off (default 5)
    #[argh(option, default = "5")]
    status_secs: u64,
}

/// What one response looks like. Built from the flags, then adjusted by any
/// overrides in the request's model name.
#[derive(Debug, Clone)]
struct Profile {
    ttft_ms: f64,
    ttft_sigma: f64,
    tps: f64,
    tps_jitter: f64,
    min_tokens: u64,
    max_tokens: u64,
    chunk_tokens: u64,
    error_rate: f64,
    error_status: u16,
    stall_rate: f64,
    stall_ms: f64,
    drop_rate: f64,
    tags: bool,
}

impl Profile {
    fn from_args(args: &Args) -> Self {
        Self {
            ttft_ms: args.ttft_ms,
            ttft_sigma: args.ttft_sigma,
            tps: args.tps,
            tps_jitter: args.tps_jitter,
            min_tokens: args.min_tokens,
            max_tokens: args.max_tokens,
            chunk_tokens: args.chunk_tokens.max(1),
            error_rate: args.error_rate,
            error_status: args.error_status,
            stall_rate: args.stall_rate,
            stall_ms: args.stall_ms,
            drop_rate: args.drop_rate,
            tags: !args.no_tags,
        }
    }

    /// Applies `key=value&...` overrides from the part of the model name
    /// after `?`.
    fn with_overrides(&self, model: &str) -> Result<Self, String> {
        let mut p = self.clone();
        let Some((_, query)) = model.split_once('?') else {
            return Ok(p);
        };
        for pair in query.split('&').filter(|s| !s.is_empty()) {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("override `{pair}` has no value"))?;
            let float = || {
                value
                    .parse::<f64>()
                    .map_err(|_| format!("`{key}` needs a number"))
            };
            let int = || {
                value
                    .parse::<u64>()
                    .map_err(|_| format!("`{key}` needs a whole number"))
            };
            match key {
                "ttft_ms" => p.ttft_ms = float()?,
                "ttft_sigma" => p.ttft_sigma = float()?,
                "tps" => p.tps = float()?,
                "tps_jitter" => p.tps_jitter = float()?,
                "min_tokens" => p.min_tokens = int()?,
                "max_tokens" => p.max_tokens = int()?,
                "tokens" => {
                    p.min_tokens = int()?;
                    p.max_tokens = p.min_tokens;
                }
                "chunk_tokens" => p.chunk_tokens = int()?.max(1),
                "error_rate" => p.error_rate = float()?,
                "error_status" => p.error_status = int()? as u16,
                "stall_rate" => p.stall_rate = float()?,
                "stall_ms" => p.stall_ms = float()?,
                "drop_rate" => p.drop_rate = float()?,
                "tags" => p.tags = value != "0" && value != "false",
                _ => return Err(format!("unknown override `{key}`")),
            }
        }
        Ok(p)
    }
}

/// The dice rolled for one request.
struct Plan {
    ttft: Duration,
    tokens: u64,
    /// Time per chunk; zero streams as fast as the socket allows.
    chunk_interval: Duration,
    /// Chunk index to pause before, and for how long.
    stall: Option<(u64, Duration)>,
    /// Chunk index at which the connection is cut.
    drop_at: Option<u64>,
}

impl Plan {
    fn roll(p: &Profile, rng: &mut Rng) -> Self {
        let tokens = rng
            .range(p.min_tokens, p.max_tokens.max(p.min_tokens))
            .max(1);
        let chunks = tokens.div_ceil(p.chunk_tokens);
        let tps = p.tps * (1.0 + p.tps_jitter * (2.0 * rng.unit() - 1.0));
        let chunk_interval = if tps > 0.0 {
            Duration::from_secs_f64(p.chunk_tokens as f64 / tps)
        } else {
            Duration::ZERO
        };
        let stall = rng
            .chance(p.stall_rate)
            .then(|| (rng.range(1, chunks), ms(p.stall_ms)));
        let drop_at = rng.chance(p.drop_rate).then(|| rng.range(1, chunks));
        Self {
            ttft: ms(rng.lognormal(p.ttft_ms, p.ttft_sigma).max(0.0)),
            tokens,
            chunk_interval,
            stall,
            drop_at,
        }
    }
}

fn ms(v: f64) -> Duration {
    Duration::from_secs_f64(v.max(0.0) / 1000.0)
}

#[derive(Default)]
struct Counters {
    requests: AtomicU64,
    in_flight: AtomicU64,
    completed: AtomicU64,
    errors_injected: AtomicU64,
    stalls_injected: AtomicU64,
    drops_injected: AtomicU64,
    client_disconnects: AtomicU64,
    tokens_sent: AtomicU64,
    prompt_bytes: AtomicU64,
    bad_requests: AtomicU64,
}

impl Counters {
    fn snapshot(&self) -> Value {
        json!({
            "requests": self.requests.load(Relaxed),
            "in_flight": self.in_flight.load(Relaxed),
            "completed": self.completed.load(Relaxed),
            "errors_injected": self.errors_injected.load(Relaxed),
            "stalls_injected": self.stalls_injected.load(Relaxed),
            "drops_injected": self.drops_injected.load(Relaxed),
            "client_disconnects": self.client_disconnects.load(Relaxed),
            "tokens_sent": self.tokens_sent.load(Relaxed),
            "prompt_bytes": self.prompt_bytes.load(Relaxed),
            "bad_requests": self.bad_requests.load(Relaxed),
        })
    }

    fn reset(&self) {
        for c in [
            &self.requests,
            &self.completed,
            &self.errors_injected,
            &self.stalls_injected,
            &self.drops_injected,
            &self.client_disconnects,
            &self.tokens_sent,
            &self.prompt_bytes,
            &self.bad_requests,
        ] {
            c.store(0, Relaxed);
        }
    }
}

struct Mock {
    profile: Profile,
    seed: u64,
    counters: Counters,
}

/// Decrements the in-flight gauge however the request ends.
struct InFlight(Arc<Mock>);

impl InFlight {
    fn new(mock: Arc<Mock>) -> Self {
        mock.counters.in_flight.fetch_add(1, Relaxed);
        Self(mock)
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.counters.in_flight.fetch_sub(1, Relaxed);
    }
}

const WORDS: &[&str] = &[
    "the",
    "lantern",
    "flickered",
    "as",
    "she",
    "stepped",
    "into",
    "hall",
    "of",
    "quiet",
    "stone",
    "and",
    "old",
    "maps",
    "rustled",
    "on",
    "a",
    "table",
    "where",
    "candles",
    "had",
    "burned",
    "low",
    "voices",
    "drifted",
    "from",
    "somewhere",
    "beyond",
    "door",
    "rain",
    "tapped",
    "against",
    "glass",
    "he",
    "turned",
    "slowly",
    "smiled",
    "said",
    "nothing",
    "for",
    "long",
    "moment",
    "then",
    "river",
    "below",
    "carried",
    "lights",
    "toward",
    "sea",
    "wind",
    "through",
    "pines",
    "story",
    "began",
    "again",
    "with",
    "promise",
    "kept",
    "night",
    "xylophone",
    "i",
    "guess",
];

fn words(rng: &mut Rng, count: u64) -> String {
    let mut out = String::with_capacity(count as usize * 7);
    for _ in 0..count {
        out.push_str(WORDS[rng.range(0, WORDS.len() as u64 - 1) as usize]);
        out.push(' ');
    }
    out
}

fn chunk_frame(model: &str, content: Option<&str>, finish: Option<&str>) -> Bytes {
    let delta = match content {
        Some(c) => json!({ "role": "assistant", "content": c }),
        None => json!({}),
    };
    let body = json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }],
    });
    Bytes::from(format!("data: {body}\n\n"))
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({ "error": { "message": message, "type": "mock_error" } })),
    )
        .into_response()
}

async fn chat_completions(State(mock): State<Arc<Mock>>, body: Bytes) -> Response {
    let arrived = Instant::now();
    let c = &mock.counters;
    let index = c.requests.fetch_add(1, Relaxed);
    c.prompt_bytes.fetch_add(body.len() as u64, Relaxed);

    let request: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            c.bad_requests.fetch_add(1, Relaxed);
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid JSON: {e}"));
        }
    };
    let model = request["model"].as_str().unwrap_or("mock").to_string();
    let stream = request["stream"].as_bool().unwrap_or(false);
    let profile = match mock.profile.with_overrides(&model) {
        Ok(p) => p,
        Err(e) => {
            c.bad_requests.fetch_add(1, Relaxed);
            return error_response(StatusCode::BAD_REQUEST, &format!("model name: {e}"));
        }
    };

    let mut rng = Rng::derive(mock.seed, index);
    if rng.chance(profile.error_rate) {
        c.errors_injected.fetch_add(1, Relaxed);
        let status =
            StatusCode::from_u16(profile.error_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        return error_response(status, "injected failure");
    }
    let plan = Plan::roll(&profile, &mut rng);

    if !stream {
        let guard = InFlight::new(Arc::clone(&mock));
        let total =
            plan.ttft + plan.chunk_interval * plan.tokens.div_ceil(profile.chunk_tokens) as u32;
        sleep_until((arrived + total).into()).await;
        let text = words(&mut rng, plan.tokens);
        c.tokens_sent.fetch_add(plan.tokens, Relaxed);
        c.completed.fetch_add(1, Relaxed);
        drop(guard);
        return Json(json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "model": model,
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": text },
                "finish_reason": "stop",
            }],
            "usage": {
                "prompt_tokens": body.len() / 4,
                "completion_tokens": plan.tokens,
                "total_tokens": body.len() as u64 / 4 + plan.tokens,
            },
        }))
        .into_response();
    }

    // Small buffer: a slow reader should slow the producer, like a real one.
    let (tx, mut rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(8);
    let guard = InFlight::new(Arc::clone(&mock));
    tokio::spawn(produce(guard, profile, plan, rng, model, arrived, tx));

    let body = Body::from_stream(futures_util::stream::poll_fn(move |cx| rx.poll_recv(cx)));
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(body)
        .unwrap()
}

/// Streams one response on its own schedule. Chunk `i` goes out at
/// `first + i * interval`, so a late wakeup doesn't push the rest back.
async fn produce(
    guard: InFlight,
    profile: Profile,
    plan: Plan,
    mut rng: Rng,
    model: String,
    arrived: Instant,
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
) {
    let c = &guard.0.counters;
    let chunks = plan.tokens.div_ceil(profile.chunk_tokens);

    sleep_until((arrived + plan.ttft).into()).await;
    let first_sent = Instant::now();
    let mut offset = Duration::ZERO;
    let mut sent_tokens = 0;

    for i in 0..chunks {
        if plan.drop_at == Some(i) {
            c.drops_injected.fetch_add(1, Relaxed);
            let cut = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "injected drop");
            let _ = tx.send(Err(cut)).await;
            return;
        }
        if let Some((at, pause)) = plan.stall
            && at == i
        {
            c.stalls_injected.fetch_add(1, Relaxed);
            offset += pause;
        }
        if i > 0 {
            offset += plan.chunk_interval;
            if !offset.is_zero() {
                sleep_until((first_sent + offset).into()).await;
            }
        }

        let n = profile.chunk_tokens.min(plan.tokens - sent_tokens);
        let mut text = words(&mut rng, n);
        if profile.tags && i == 0 {
            let ttft_us = arrived.elapsed().as_micros();
            text = format!("{TAG_TTFT}{ttft_us}]] {text}");
        }
        if profile.tags && i + 1 == chunks {
            let stream_us = first_sent.elapsed().as_micros();
            text.push_str(&format!("{TAG_STREAM}{stream_us}]]"));
        }
        if tx
            .send(Ok(chunk_frame(&model, Some(&text), None)))
            .await
            .is_err()
        {
            c.client_disconnects.fetch_add(1, Relaxed);
            return;
        }
        sent_tokens += n;
        c.tokens_sent.fetch_add(n, Relaxed);
    }

    let _ = tx.send(Ok(chunk_frame(&model, None, Some("stop")))).await;
    let _ = tx.send(Ok(Bytes::from_static(b"data: [DONE]\n\n"))).await;
    c.completed.fetch_add(1, Relaxed);
}

async fn models() -> Json<Value> {
    Json(json!({ "object": "list", "data": [{ "id": "mock", "object": "model" }] }))
}

async fn stats(State(mock): State<Arc<Mock>>) -> Json<Value> {
    Json(mock.counters.snapshot())
}

async fn reset_stats(State(mock): State<Arc<Mock>>) -> Result<Json<Value>, Infallible> {
    mock.counters.reset();
    Ok(Json(mock.counters.snapshot()))
}

async fn report_status(mock: Arc<Mock>, every: Duration) {
    let c = &mock.counters;
    let mut last = (Instant::now(), 0u64, 0u64);
    loop {
        tokio::time::sleep(every).await;
        let now = Instant::now();
        let (requests, tokens) = (c.requests.load(Relaxed), c.tokens_sent.load(Relaxed));
        let secs = now.duration_since(last.0).as_secs_f64();
        eprintln!(
            "mock-llm: {:>7.1} req/s  {:>9.0} tok/s  {:>6} in flight  ({} done, {} injected errors, {} client disconnects)",
            requests.saturating_sub(last.1) as f64 / secs,
            tokens.saturating_sub(last.2) as f64 / secs,
            c.in_flight.load(Relaxed),
            c.completed.load(Relaxed),
            c.errors_injected.load(Relaxed),
            c.client_disconnects.load(Relaxed),
        );
        last = (now, requests, tokens);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();
    fdlimit::raise_and_report("mock-llm", &args.max_open_files);

    let seed = args.seed.unwrap_or_else(|| Rng::from_time().next_u64());
    let mock = Arc::new(Mock {
        profile: Profile::from_args(&args),
        seed,
        counters: Counters::default(),
    });
    eprintln!("mock-llm: seed {seed}, {:?}", mock.profile);

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(models))
        .route("/stats", get(stats).delete(reset_stats))
        .with_state(Arc::clone(&mock));

    let listen = if args.listen.is_empty() {
        vec!["127.0.0.1:9100".to_string()]
    } else {
        args.listen
    };

    let mut servers = Vec::new();
    for addr in listen {
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("binding {addr}: {e}"))?
            .tap_io(|tcp| {
                let _ = tcp.set_nodelay(true);
            });
        eprintln!("mock-llm: listening on http://{addr}/v1");
        let app = app.clone();
        servers.push(tokio::spawn(
            async move { axum::serve(listener, app).await },
        ));
    }

    if args.status_secs > 0 {
        tokio::spawn(report_status(mock, Duration::from_secs(args.status_secs)));
    }

    for server in servers {
        server.await??;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Profile {
        Profile {
            ttft_ms: 600.0,
            ttft_sigma: 0.5,
            tps: 60.0,
            tps_jitter: 0.0,
            min_tokens: 10,
            max_tokens: 20,
            chunk_tokens: 1,
            error_rate: 0.0,
            error_status: 500,
            stall_rate: 0.0,
            stall_ms: 0.0,
            drop_rate: 0.0,
            tags: true,
        }
    }

    #[test]
    fn model_name_overrides_apply() {
        let p = base()
            .with_overrides("mock?ttft_ms=0&tps=0&tokens=5")
            .unwrap();
        assert_eq!(p.ttft_ms, 0.0);
        assert_eq!(p.tps, 0.0);
        assert_eq!((p.min_tokens, p.max_tokens), (5, 5));
        assert_eq!(base().with_overrides("mock").unwrap().ttft_ms, 600.0);
    }

    #[test]
    fn bad_overrides_are_rejected() {
        assert!(base().with_overrides("mock?nope=1").is_err());
        assert!(base().with_overrides("mock?tps=fast").is_err());
        assert!(base().with_overrides("mock?tps").is_err());
    }

    #[test]
    fn plan_respects_token_bounds_and_rate() {
        let mut rng = Rng::new(9);
        for _ in 0..100 {
            let plan = Plan::roll(&base(), &mut rng);
            assert!((10..=20).contains(&plan.tokens));
            assert_eq!(plan.chunk_interval, Duration::from_secs_f64(1.0 / 60.0));
        }
        let mut p = base();
        p.tps = 0.0;
        assert_eq!(Plan::roll(&p, &mut rng).chunk_interval, Duration::ZERO);
    }
}
