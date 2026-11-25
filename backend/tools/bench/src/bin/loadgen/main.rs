mod api;
mod limit;
mod report;
mod run;

use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use argh::FromArgs;
use bench::{fdlimit, rng::Rng};

use crate::{
    api::{Api, BenchUser, Conversation, SeedPlan},
    limit::{Criteria, Search, Verdict},
    report::{MockCounts, ProcSampler, StepReport},
    run::{Load, StepConfig, run_step},
};

#[derive(FromArgs)]
/// Load generator for Lorealm chat turns.
struct Args {
    /// server root URL (default http://127.0.0.1:1551)
    #[argh(option, default = "String::from(\"http://127.0.0.1:1551\")")]
    server: String,
    /// mock-llm base URL as the server should reach it (default http://127.0.0.1:9100/v1)
    #[argh(option)]
    mock_url: Vec<String>,
    /// model name for the profiles (default mock)
    #[argh(option, default = "String::from(\"mock\")")]
    model: String,
    /// admin username, used to create bench users (default admin)
    #[argh(option, default = "String::from(\"admin\")")]
    admin_user: String,
    /// admin password (default change-me)
    #[argh(option, default = "String::from(\"change-me\")")]
    admin_password: String,
    /// prefix for bench usernames; existing users are reused (default bench)
    #[argh(option, default = "String::from(\"bench\")")]
    user_prefix: String,
    /// users to seed up front; more are added when a step needs them (default 10)
    #[argh(option, default = "10")]
    users: usize,
    /// conversations each user carries; (default 3)
    #[argh(option, default = "3")]
    convs_per_user: usize,
    /// keep conversations from one step to the next
    #[argh(switch)]
    reuse_conversations: bool,
    /// size of the character description in bytes, to grow the prompt (default 2000)
    #[argh(option, default = "2000")]
    character_bytes: usize,
    /// tokenizer source installed (public) and activated for bench users
    /// (default hf:google/gemma-4-12B)
    #[argh(option, default = "String::from(\"hf:google/gemma-4-12B\")")]
    tokenizer: String,
    /// give each user a private copy of the tokenizer
    #[argh(switch)]
    tokenizer_per_user: bool,
    /// parallel requests while seeding (default 16)
    #[argh(option, default = "16")]
    setup_concurrency: usize,

    /// load model: `closed` keeps a fixed number of conversations busy, `open`
    /// starts turns at a fixed rate (default closed)
    #[argh(option, default = "String::from(\"closed\")")]
    mode: String,
    /// comma-separated load levels (default 10,50,100)
    #[argh(option, default = "String::from(\"10,50,100\")")]
    steps: String,
    /// instead of --steps, raise the load until a step misses the criteria
    /// below, then narrow down the highest load that passes
    #[argh(switch)]
    find_limit: bool,
    /// first load level for --find-limit (default 50)
    #[argh(option, default = "50.0")]
    start: f64,
    /// how much --find-limit multiplies the load by after each pass (default 2)
    #[argh(option, default = "2.0")]
    growth: f64,
    /// highest load --find-limit will try
    #[argh(option)]
    max_level: Option<f64>,
    /// steps --find-limit spends narrowing down after the first failure (default 3)
    #[argh(option, default = "3")]
    refine: u32,
    /// pass criterion: largest share of failed turns (default 0.01)
    #[argh(option, default = "0.01")]
    max_error_rate: f64,
    /// pass criterion: p99 time the server adds before the first token, in ms;
    /// 0 turns it off (default 250)
    #[argh(option, default = "250.0")]
    max_added_ttft_ms: f64,
    /// pass criterion: p99 gap between streamed tokens, in ms; 0 turns it off
    /// (default 0)
    #[argh(option, default = "0.0")]
    max_token_gap_ms: f64,
    /// pass criterion: p99 time from the last token to the turn finishing, in
    /// ms; 0 turns it off (default 0)
    #[argh(option, default = "0.0")]
    max_after_stream_ms: f64,
    /// pass criterion, open mode: finished turns/s as a share of the arrival
    /// rate; 0 turns it off (default 0.95)
    #[argh(option, default = "0.95")]
    min_throughput_ratio: f64,

    /// seconds measured per step (default 30)
    #[argh(option, default = "30")]
    step_secs: u64,
    /// seconds of unmeasured warm-up before each step (default 5)
    #[argh(option, default = "5")]
    warmup_secs: u64,
    /// closed mode: average pause between a conversation's turns, in ms; each pause is 0.5–1.5× this, and warm-up lasts at least this long (default 0)
    #[argh(option, default = "0")]
    think_ms: u64,
    /// give up on a turn after this many seconds (default 120)
    #[argh(option, default = "120")]
    turn_timeout_secs: u64,
    /// what the player says each turn
    #[argh(
        option,
        default = "String::from(\"I pull up a chair and ask about the road north.\")"
    )]
    message: String,

    /// server process id, to report its CPU, memory and open files (Linux)
    #[argh(option)]
    server_pid: Option<u32>,
    /// also write the results as JSON to this path
    #[argh(option)]
    json: Option<String>,
    /// seed for arrival times (default: from the clock)
    #[argh(option)]
    seed: Option<u64>,
    /// soft open-file limit to raise to: a number, or `max` (default max)
    #[argh(option, default = "String::from(\"max\")")]
    max_open_files: String,
}

fn parse_steps(mode: &str, steps: &str) -> anyhow::Result<Vec<Load>> {
    steps
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| match mode {
            "closed" => s
                .parse::<usize>()
                .ok()
                .filter(|&n| n > 0)
                .map(Load::Concurrency)
                .with_context(|| format!("step `{s}`: expected a positive whole number")),
            "open" => s
                .parse::<f64>()
                .ok()
                .filter(|&r| r > 0.0)
                .map(Load::Rate)
                .with_context(|| format!("step `{s}`: expected a positive rate")),
            other => bail!("--mode must be `closed` or `open`, not `{other}`"),
        })
        .collect()
}

/// `http://host:9100/v1` → `http://host:9100`, where the mock serves `/stats`.
fn mock_root(url: &str) -> String {
    let url = url.trim_end_matches('/');
    url.strip_suffix("/v1").unwrap_or(url).to_string()
}

async fn reset_mocks(http: &reqwest::Client, urls: &[String]) {
    for url in urls {
        let _ = http
            .delete(format!("{}/stats", mock_root(url)))
            .send()
            .await;
    }
}

/// Sums the mocks' counters, or `None` if none of them answered (for
/// example, because the loadgen can't reach them directly).
async fn read_mocks(http: &reqwest::Client, urls: &[String]) -> Option<MockCounts> {
    let mut counts = MockCounts::default();
    let mut any = false;
    for url in urls {
        let response = http.get(format!("{}/stats", mock_root(url))).send().await;
        if let Ok(stats) = async { response?.json::<serde_json::Value>().await }.await {
            counts.add(&stats);
            any = true;
        }
    }
    any.then_some(counts)
}

/// Everything a step needs, plus the users and conversations so far.
struct Runner<'a> {
    args: &'a Args,
    api: Api,
    plan: SeedPlan<'a>,
    users: Vec<BenchUser>,
    convs: Vec<Conversation>,
    seed: u64,
    mock_urls: &'a [String],
    server_sampler: Option<ProcSampler>,
    self_sampler: ProcSampler,
    reports: Vec<StepReport>,
}

impl Runner<'_> {
    /// Seeds more users until there are at least `wanted`.
    async fn ensure_users(&mut self, wanted: usize) -> anyhow::Result<()> {
        if self.users.len() >= wanted {
            return Ok(());
        }
        let range = self.users.len()..wanted;
        eprintln!("loadgen: seeding users {}–{}…", range.start, range.end - 1);
        let started = Instant::now();
        let more = api::seed_users(&self.api, &self.plan, range).await?;
        self.users.extend(more);
        eprintln!(
            "loadgen: {} users ready ({:.1}s)",
            self.users.len(),
            started.elapsed().as_secs_f64()
        );
        Ok(())
    }

    /// Gets `wanted` conversations ready for the next step: new ones, unless
    /// `--reuse-conversations` is set. A conversation's history grows with
    /// every turn and makes later turns cost more, so reusing them makes later
    /// steps heavier than earlier ones.
    async fn prepare(&mut self, wanted: usize) -> anyhow::Result<()> {
        let per_user = self.plan.convs_per_user.max(1);
        self.ensure_users(wanted.div_ceil(per_user)).await?;
        let users = &self.users[..wanted.div_ceil(per_user).min(self.users.len())];
        let concurrency = self.plan.concurrency;
        if self.args.reuse_conversations {
            if self.convs.len() < wanted {
                let more = api::start_conversations(
                    &self.api,
                    users,
                    wanted - self.convs.len(),
                    concurrency,
                )
                .await?;
                self.convs.extend(more);
            }
        } else {
            self.convs = api::start_conversations(&self.api, users, wanted, concurrency).await?;
        }
        Ok(())
    }

    /// Runs one step at `load`, prints it, and keeps the report.
    async fn measure(&mut self, load: Load) -> &StepReport {
        let index = self.reports.len();
        eprintln!("loadgen: step {}: {}", index + 1, load.describe());
        let config = StepConfig {
            load,
            warmup: Duration::from_secs(self.args.warmup_secs),
            measure: Duration::from_secs(self.args.step_secs),
            think: Duration::from_millis(self.args.think_ms),
            timeout: Duration::from_secs(self.args.turn_timeout_secs),
            content: self.args.message.clone(),
            seed: self.seed,
        };

        let result = run_step(&self.api, &self.convs, &config).await;
        let from = result.measured_from;
        let until = from + config.measure;
        // Read the mocks before resetting them for the next step; their counts
        // cover warm-up too, so they are a sanity check rather than a metric.
        let mock = read_mocks(&self.api.http, self.mock_urls).await;
        reset_mocks(&self.api.http, self.mock_urls).await;

        let report = StepReport::build(
            load,
            config.measure,
            result,
            self.server_sampler
                .as_ref()
                .and_then(|s| s.usage_between(from, until)),
            self.self_sampler.usage_between(from, until),
            mock,
        );
        report.print(index);
        self.reports.push(report);
        &self.reports[index]
    }

    /// Conversations a level needs: one per concurrent turn in closed mode.
    /// In open mode, enough for the arrival rate times how long turns took
    /// last time, with room to spare.
    fn conversations_for(&self, load: Load) -> usize {
        match load {
            Load::Concurrency(n) => n,
            Load::Rate(rate) => {
                let turn_secs = self
                    .reports
                    .iter()
                    .rev()
                    .find_map(|r| r.total_ms.map(|s| s.p99 / 1000.0))
                    .unwrap_or(10.0);
                (rate * turn_secs * 1.5).ceil() as usize + 1
            }
        }
    }
}

fn opt(limit: f64) -> Option<f64> {
    (limit > 0.0).then_some(limit)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();
    fdlimit::raise_and_report("loadgen", &args.max_open_files);

    let whole = match args.mode.as_str() {
        "closed" => true,
        "open" => false,
        other => bail!("--mode must be `closed` or `open`, not `{other}`"),
    };
    let steps = if args.find_limit {
        if args.start <= 0.0 || args.growth <= 1.0 {
            bail!("--find-limit needs --start above 0 and --growth above 1");
        }
        Vec::new()
    } else {
        parse_steps(&args.mode, &args.steps)?
    };
    let mock_urls = if args.mock_url.is_empty() {
        vec!["http://127.0.0.1:9100/v1".to_string()]
    } else {
        args.mock_url.clone()
    };

    let api = Api::new(&args.server, args.turn_timeout_secs)?;
    api.health()
        .await
        .with_context(|| format!("is the server running at {}?", args.server))?;

    let mut runner = Runner {
        args: &args,
        api,
        plan: SeedPlan {
            admin_user: &args.admin_user,
            admin_password: &args.admin_password,
            user_prefix: &args.user_prefix,
            convs_per_user: args.convs_per_user,
            model: &args.model,
            mock_urls: &mock_urls,
            character_bytes: args.character_bytes,
            tokenizer: &args.tokenizer,
            tokenizer_per_user: args.tokenizer_per_user,
            concurrency: args.setup_concurrency,
        },
        users: Vec::new(),
        convs: Vec::new(),
        seed: args.seed.unwrap_or_else(|| Rng::from_time().next_u64()),
        mock_urls: &mock_urls,
        server_sampler: args.server_pid.map(|pid| ProcSampler::start(Some(pid))),
        self_sampler: ProcSampler::start(None),
        reports: Vec::new(),
    };
    runner.ensure_users(args.users).await?;

    let mut limit_summary = None;
    if args.find_limit {
        let criteria = Criteria {
            max_error_rate: args.max_error_rate,
            max_added_ttft_p99_ms: opt(args.max_added_ttft_ms),
            max_token_gap_p99_ms: opt(args.max_token_gap_ms),
            max_after_stream_p99_ms: opt(args.max_after_stream_ms),
            min_throughput_ratio: opt(args.min_throughput_ratio),
        };
        let mut search = Search::new(args.start, args.growth, args.max_level, args.refine, whole);
        let mut steps = Vec::new();
        let mut stopped = None;

        while let Some(level) = search.next_level() {
            let load = search.load(level);
            runner.prepare(runner.conversations_for(load)).await?;
            let verdict = criteria.check(runner.measure(load).await);
            match &verdict {
                Verdict::Pass => println!("               → pass"),
                Verdict::Fail(reasons) => println!("               → fail: {}", reasons.join("; ")),
                Verdict::Inconclusive(why) => println!("               → inconclusive: {why}"),
            }
            steps.push(serde_json::json!({ "level": level, "result": verdict }));
            if let Verdict::Inconclusive(why) = verdict {
                stopped = Some(why);
                break;
            }
            search.record(level, verdict == Verdict::Pass);
        }

        let unit = if whole {
            "concurrent conversations"
        } else {
            "turns/s"
        };
        println!();
        println!("══ limit ══");
        match search.highest_pass() {
            Some(level) => println!("highest passing   {level} {unit}"),
            None => println!("highest passing   none: the first level already failed"),
        }
        match search.lowest_fail() {
            Some(level) => println!("lowest failing    {level} {unit}"),
            None if stopped.is_none() => println!(
                "lowest failing    none reached{}",
                if args.max_level.is_some() {
                    " (stopped at --max-level)"
                } else {
                    ""
                }
            ),
            None => {}
        }
        if let Some(why) = &stopped {
            println!("stopped early     {why}");
        }
        limit_summary = Some(serde_json::json!({
            "criteria": criteria,
            "highest_passing": search.highest_pass(),
            "lowest_failing": search.lowest_fail(),
            "stopped_early": stopped,
            "steps": steps,
        }));
    } else {
        for load in steps {
            runner.prepare(runner.conversations_for(load)).await?;
            runner.measure(load).await;
        }
    }

    if let Some(path) = &args.json {
        let out = serde_json::json!({
            "server": args.server,
            "model": args.model,
            "mode": args.mode,
            "users": runner.users.len(),
            "conversations": runner.convs.len(),
            "character_bytes": args.character_bytes,
            "seed": runner.seed,
            "steps": runner.reports,
            "limit": limit_summary,
        });
        std::fs::write(path, serde_json::to_string_pretty(&out)?)
            .with_context(|| format!("writing {path}"))?;
        eprintln!("loadgen: wrote {path}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_parse_per_mode() {
        assert!(matches!(
            parse_steps("closed", "10, 50").unwrap()[..],
            [Load::Concurrency(10), Load::Concurrency(50)]
        ));
        assert!(matches!(parse_steps("open", "2.5").unwrap()[..], [Load::Rate(r)] if r == 2.5));
        assert!(parse_steps("closed", "0").is_err());
        assert!(parse_steps("sideways", "1").is_err());
    }

    #[test]
    fn mock_root_strips_version() {
        assert_eq!(mock_root("http://a:1/v1/"), "http://a:1");
        assert_eq!(mock_root("http://a:1"), "http://a:1");
    }
}
