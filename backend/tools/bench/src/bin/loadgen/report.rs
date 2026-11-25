//! Process sampling and the per-step report.

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    time::{Duration, Instant},
};

use bench::stats::{Samples, Summary};
use serde::Serialize;
use serde_json::Value;

use crate::run::{Load, Outcome, StepResult};

/// CPU and memory of one process over the measured window.
#[derive(Debug, Clone, Serialize)]
pub struct ProcUsage {
    /// Average CPU use, in cores (1.0 = one core fully busy).
    pub cpu_cores: f64,
    pub peak_rss_mb: f64,
    pub peak_fds: u64,
}

#[derive(Clone, Copy)]
struct Sample {
    at: Instant,
    cpu_ticks: u64,
    rss_kb: u64,
    fds: u64,
}

/// Polls `/proc` for a process every 250ms. Linux only; elsewhere it
/// samples nothing.
pub struct ProcSampler {
    samples: Arc<Mutex<Vec<Sample>>>,
    stop: Arc<AtomicBool>,
}

const SAMPLE_EVERY: Duration = Duration::from_millis(250);

impl ProcSampler {
    /// `pid` of `None` samples this process.
    pub fn start(pid: Option<u32>) -> Self {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (sink, halt) = (Arc::clone(&samples), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !halt.load(Relaxed) {
                if let Some(s) = read_proc(pid) {
                    sink.lock().unwrap().push(s);
                }
                std::thread::sleep(SAMPLE_EVERY);
            }
        });
        Self { samples, stop }
    }

    pub fn usage_between(&self, from: Instant, until: Instant) -> Option<ProcUsage> {
        let samples = self.samples.lock().unwrap();
        // Widen to the samples just outside the span
        let start = samples.iter().rposition(|s| s.at <= from).unwrap_or(0);
        let end = samples
            .iter()
            .position(|s| s.at >= until)
            .unwrap_or(samples.len().saturating_sub(1));
        let window = samples.get(start..=end)?;
        let (first, last) = (window.first()?, window.last()?);
        let secs = (last.at - first.at).as_secs_f64();
        let wanted = until.saturating_duration_since(from).as_secs_f64();
        // Gaps would mean the sampler stalled
        let largest_gap = window
            .windows(2)
            .map(|w| (w[1].at - w[0].at).as_secs_f64())
            .fold(0.0, f64::max);
        if secs <= 0.0 || secs < wanted * 0.8 || largest_gap > 2.0 {
            return None;
        }
        let ticks_per_sec = clock_ticks_per_sec()?;
        Some(ProcUsage {
            cpu_cores: last.cpu_ticks.saturating_sub(first.cpu_ticks) as f64 / ticks_per_sec / secs,
            peak_rss_mb: window.iter().map(|s| s.rss_kb).max()? as f64 / 1024.0,
            peak_fds: window.iter().map(|s| s.fds).max()?,
        })
    }
}

impl Drop for ProcSampler {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
    }
}

#[cfg(target_os = "linux")]
fn read_proc(pid: Option<u32>) -> Option<Sample> {
    let dir = match pid {
        Some(pid) => format!("/proc/{pid}"),
        None => "/proc/self".to_string(),
    };
    let stat = std::fs::read_to_string(format!("{dir}/stat")).ok()?;
    // The command name may contain spaces; fields resume after its ')'.
    let fields: Vec<&str> = stat[stat.rfind(')')? + 2..].split_whitespace().collect();
    // utime and stime are fields 14 and 15 of the full line.
    let cpu_ticks = fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
    let status = std::fs::read_to_string(format!("{dir}/status")).ok()?;
    let rss_kb = status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let fds = std::fs::read_dir(format!("{dir}/fd"))
        .map(|d| d.count() as u64)
        .unwrap_or(0);
    Some(Sample {
        at: Instant::now(),
        cpu_ticks,
        rss_kb,
        fds,
    })
}

#[cfg(not(target_os = "linux"))]
fn read_proc(_pid: Option<u32>) -> Option<Sample> {
    None
}

#[cfg(unix)]
fn clock_ticks_per_sec() -> Option<f64> {
    // SAFETY: sysconf has no preconditions.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    (ticks > 0).then_some(ticks as f64)
}

#[cfg(not(unix))]
fn clock_ticks_per_sec() -> Option<f64> {
    None
}

/// Requests the mocks saw during a step, summed over all of them.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MockCounts {
    pub requests: u64,
    pub errors_injected: u64,
    pub client_disconnects: u64,
    pub bad_requests: u64,
}

impl MockCounts {
    pub fn add(&mut self, stats: &Value) {
        let get = |k: &str| stats[k].as_u64().unwrap_or(0);
        self.requests += get("requests");
        self.errors_injected += get("errors_injected");
        self.client_disconnects += get("client_disconnects");
        self.bad_requests += get("bad_requests");
    }
}

#[derive(Debug, Serialize)]
pub struct StepReport {
    pub load: String,
    pub concurrency: Option<usize>,
    pub arrival_rate: Option<f64>,
    pub measured_secs: f64,
    pub turns: usize,
    pub ok: usize,
    pub failures: BTreeMap<String, usize>,
    pub throughput: f64,
    pub peak_in_flight: u64,
    pub skipped_all_busy: u64,
    pub max_send_lag_ms: f64,
    pub headers_ms: Option<Summary>,
    pub ttft_ms: Option<Summary>,
    pub overhead_ttft_ms: Option<Summary>,
    pub overhead_stream_ms: Option<Summary>,
    pub persist_ms: Option<Summary>,
    pub delta_gap_ms: Option<Summary>,
    pub total_ms: Option<Summary>,
    pub server: Option<ProcUsage>,
    pub loadgen: Option<ProcUsage>,
    pub mock: Option<MockCounts>,
    pub tagged: usize,
}

impl StepReport {
    pub fn build(
        load: Load,
        measured: Duration,
        result: StepResult,
        server: Option<ProcUsage>,
        loadgen: Option<ProcUsage>,
        mock: Option<MockCounts>,
    ) -> Self {
        let mut failures = BTreeMap::new();
        let (mut headers, mut ttft, mut over_ttft, mut over_stream) = (
            Samples::default(),
            Samples::default(),
            Samples::default(),
            Samples::default(),
        );
        let (mut persist, mut gaps, mut total) =
            (Samples::default(), Samples::default(), Samples::default());
        let mut ok = 0;
        let mut tagged = 0;

        for turn in &result.turns {
            if turn.outcome != Outcome::Ok {
                *failures.entry(turn.outcome.label()).or_insert(0) += 1;
                continue;
            }
            ok += 1;
            tagged += usize::from(turn.mock_ttft_ms.is_some());
            headers.extend(turn.headers_ms);
            ttft.extend(turn.ttft_ms);
            over_ttft.extend(turn.overhead_ttft_ms());
            over_stream.extend(turn.overhead_stream_ms());
            persist.extend(turn.persist_ms);
            gaps.extend(turn.gaps.iter().copied());
            total.push(turn.total_ms);
        }

        let secs = measured.as_secs_f64();
        let (concurrency, arrival_rate) = match load {
            Load::Concurrency(n) => (Some(n), None),
            Load::Rate(r) => (None, Some(r)),
        };
        Self {
            load: load.describe(),
            concurrency,
            arrival_rate,
            measured_secs: secs,
            turns: result.turns.len(),
            ok,
            failures,
            throughput: ok as f64 / secs,
            peak_in_flight: result.peak_in_flight,
            skipped_all_busy: result.skipped,
            max_send_lag_ms: result.max_send_lag_ms,
            headers_ms: headers.summary(),
            ttft_ms: ttft.summary(),
            overhead_ttft_ms: over_ttft.summary(),
            overhead_stream_ms: over_stream.summary(),
            persist_ms: persist.summary(),
            delta_gap_ms: gaps.summary(),
            total_ms: total.summary(),
            server,
            loadgen,
            mock,
            tagged,
        }
    }

    pub fn print(&self, index: usize) {
        println!();
        println!(
            "── step {}: {} ({:.0}s measured) ──",
            index + 1,
            self.load,
            self.measured_secs
        );
        let failed = self.turns - self.ok;
        print!(
            "turns          {} ok, {} failed   {:.1} turns/s   peak {} in flight",
            self.ok, failed, self.throughput, self.peak_in_flight
        );
        if self.skipped_all_busy > 0 {
            print!(
                "   {} arrivals skipped (all conversations busy)",
                self.skipped_all_busy
            );
        }
        println!();
        for (label, count) in &self.failures {
            println!("               {count} × {label}");
        }
        if self
            .failures
            .keys()
            .any(|l| l.contains("tokenizer.forbidden"))
        {
            println!(
                "               (the server only lets a tokenizer's owner use it in turns; \
                 rerun with --tokenizer-per-user)"
            );
        }
        let row = |name: &str, s: &Option<Summary>| {
            if let Some(s) = s {
                println!("{name:<15}{}  ms", s.line());
            }
        };
        row("first token", &self.ttft_ms);
        row("added: ttft", &self.overhead_ttft_ms);
        row("added: stream", &self.overhead_stream_ms);
        row("after stream", &self.persist_ms);
        row("token gap", &self.delta_gap_ms);
        row("whole turn", &self.total_ms);
        if self.ok > 0 && self.tagged == 0 {
            println!(
                "               (no mock timing markers seen, so the server's added time \
                 can't be computed; is the server pointed at mock-llm?)"
            );
        }
        if let Some(s) = &self.server {
            println!(
                "server         {:.2} cores avg   {:.0} MB peak RSS   {} fds peak",
                s.cpu_cores, s.peak_rss_mb, s.peak_fds
            );
        }
        if let Some(s) = &self.loadgen {
            println!(
                "loadgen        {:.2} cores avg   {:.0} MB peak RSS",
                s.cpu_cores, s.peak_rss_mb
            );
        }
        if self.max_send_lag_ms > 50.0 {
            println!(
                "               warning: loadgen fell up to {:.0}ms behind schedule; \
                 its numbers include that lag",
                self.max_send_lag_ms
            );
        }
        if let Some(m) = &self.mock
            && (m.bad_requests > 0 || m.client_disconnects > 0)
        {
            println!(
                "mock           {} requests, {} rejected as malformed, {} disconnected mid-stream",
                m.requests, m.bad_requests, m.client_disconnects
            );
        }
    }
}
