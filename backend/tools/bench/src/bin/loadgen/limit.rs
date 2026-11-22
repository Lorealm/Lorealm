//! Finding the highest load that still meets a set of criteria.

use serde::Serialize;

use crate::{report::StepReport, run::Load};

/// What a step must meet to pass. A limit of `None` isn't checked.
#[derive(Debug, Clone, Serialize)]
pub struct Criteria {
    /// Largest share of turns allowed to fail.
    pub max_error_rate: f64,
    /// p99 of the time the server adds before the first token.
    pub max_added_ttft_p99_ms: Option<f64>,
    /// p99 of the gap between streamed text events.
    pub max_token_gap_p99_ms: Option<f64>,
    /// p99 of last token → `done`.
    pub max_after_stream_p99_ms: Option<f64>,
    /// Open mode: completed turns/s must reach this share of the arrival rate.
    pub min_throughput_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "verdict", content = "reasons", rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    /// The server missed one or more criteria.
    Fail(Vec<String>),
    /// The harness, not the server, limited the step, so it says nothing
    /// about the server.
    Inconclusive(String),
}

/// How far behind schedule loadgen may fall before its numbers stop being
/// about the server.
const MAX_SEND_LAG_MS: f64 = 1000.0;

impl Criteria {
    pub fn check(&self, r: &StepReport) -> Verdict {
        if r.skipped_all_busy > 0 {
            return Verdict::Inconclusive(format!(
                "{} arrivals found every conversation busy; seed more with --users or \
                 --convs-per-user",
                r.skipped_all_busy
            ));
        }
        if r.max_send_lag_ms > MAX_SEND_LAG_MS {
            return Verdict::Inconclusive(format!(
                "loadgen fell {:.0}ms behind schedule; run it on a less busy machine",
                r.max_send_lag_ms
            ));
        }
        if r.turns == 0 {
            return Verdict::Fail(vec!["no turns finished".into()]);
        }

        let mut reasons = Vec::new();
        let error_rate = (r.turns - r.ok) as f64 / r.turns as f64;
        if error_rate > self.max_error_rate {
            reasons.push(format!(
                "{:.1}% of turns failed (limit {:.1}%)",
                error_rate * 100.0,
                self.max_error_rate * 100.0
            ));
        }

        let mut p99 = |name: &str, limit: Option<f64>, value: Option<f64>| {
            let Some(limit) = limit else { return };
            match value {
                Some(v) if v > limit => {
                    reasons.push(format!("{name} p99 {v:.0}ms (limit {limit:.0}ms)"))
                }
                Some(_) => {}
                None if r.ok > 0 => reasons.push(format!("{name} couldn't be measured")),
                None => {}
            }
        };
        p99(
            "added ttft",
            self.max_added_ttft_p99_ms,
            r.overhead_ttft_ms.map(|s| s.p99),
        );
        p99(
            "token gap",
            self.max_token_gap_p99_ms,
            r.delta_gap_ms.map(|s| s.p99),
        );
        p99(
            "after stream",
            self.max_after_stream_p99_ms,
            r.persist_ms.map(|s| s.p99),
        );

        if let (Some(ratio), Some(rate)) = (self.min_throughput_ratio, r.arrival_rate)
            && r.throughput < rate * ratio
        {
            reasons.push(format!(
                "finished {:.1} turns/s of {rate} arriving (limit {:.0}%)",
                r.throughput,
                ratio * 100.0
            ));
        }

        if reasons.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Fail(reasons)
        }
    }
}

/// Where the search goes next.
#[derive(Debug, Clone)]
pub struct Search {
    pub growth: f64,
    pub max_level: Option<f64>,
    /// Bisection steps after the first failure.
    pub refine: u32,
    /// Closed mode counts conversations, which are whole.
    pub whole: bool,
    highest_pass: Option<f64>,
    lowest_fail: Option<f64>,
    refined: u32,
    next: Option<f64>,
}

impl Search {
    pub fn new(start: f64, growth: f64, max_level: Option<f64>, refine: u32, whole: bool) -> Self {
        let mut search = Self {
            growth,
            max_level,
            refine,
            whole,
            highest_pass: None,
            lowest_fail: None,
            refined: 0,
            next: None,
        };
        search.next = Some(search.clamp(start));
        search
    }

    fn clamp(&self, level: f64) -> f64 {
        let level = if self.whole {
            level.round().max(1.0)
        } else {
            level
        };
        match self.max_level {
            Some(max) => level.min(max),
            None => level,
        }
    }

    pub fn next_level(&self) -> Option<f64> {
        self.next
    }

    pub fn highest_pass(&self) -> Option<f64> {
        self.highest_pass
    }

    pub fn lowest_fail(&self) -> Option<f64> {
        self.lowest_fail
    }

    /// Records how `level` went and picks the next level, if any.
    pub fn record(&mut self, level: f64, passed: bool) {
        if passed {
            self.highest_pass = Some(self.highest_pass.map_or(level, |p| p.max(level)));
        } else {
            self.lowest_fail = Some(self.lowest_fail.map_or(level, |f| f.min(level)));
        }

        self.next = match (self.highest_pass, self.lowest_fail) {
            // Still growing.
            (_, None) => {
                let grown = if self.whole {
                    (level * self.growth).round().max(level + 1.0)
                } else {
                    level * self.growth
                };
                let grown = self.clamp(grown);
                (grown > level).then_some(grown)
            }
            // Failed at the very first level: nothing to bisect.
            (None, Some(_)) => None,
            (Some(lo), Some(hi)) => {
                if self.refined >= self.refine {
                    None
                } else {
                    self.refined += 1;
                    let mid = (lo + hi) / 2.0;
                    let mid = if self.whole { mid.round() } else { mid };
                    let close_enough = if self.whole {
                        mid <= lo || mid >= hi
                    } else {
                        (hi - lo) / lo < 0.02
                    };
                    (!close_enough).then_some(mid)
                }
            }
        };
    }

    pub fn load(&self, level: f64) -> Load {
        if self.whole {
            Load::Concurrency(level as usize)
        } else {
            Load::Rate(level)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a search against a server that passes every level up to `limit`.
    fn run(mut search: Search, limit: f64) -> (Vec<f64>, Option<f64>, Option<f64>) {
        let mut tried = Vec::new();
        while let Some(level) = search.next_level() {
            tried.push(level);
            search.record(level, level <= limit);
            assert!(tried.len() < 50, "search doesn't end: {tried:?}");
        }
        (tried, search.highest_pass(), search.lowest_fail())
    }

    #[test]
    fn grows_then_bisects_whole_levels() {
        let (tried, pass, fail) = run(Search::new(50.0, 2.0, None, 3, true), 300.0);
        assert_eq!(&tried[..4], &[50.0, 100.0, 200.0, 400.0]);
        assert_eq!(tried[4], 300.0); // midpoint of 200 and 400
        assert_eq!(pass, Some(300.0));
        assert!(fail.unwrap() > 300.0 && fail.unwrap() <= 400.0);
        assert_eq!(tried.len(), 7);
    }

    #[test]
    fn small_growth_still_advances_whole_levels() {
        let (tried, ..) = run(Search::new(1.0, 1.1, None, 0, true), 3.0);
        assert_eq!(tried, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn stops_at_max_level_without_failing() {
        let (tried, pass, fail) = run(Search::new(10.0, 2.0, Some(60.0), 3, true), 1e9);
        assert_eq!(tried, vec![10.0, 20.0, 40.0, 60.0]);
        assert_eq!((pass, fail), (Some(60.0), None));
    }

    #[test]
    fn failing_the_first_level_ends_the_search() {
        let (tried, pass, fail) = run(Search::new(10.0, 2.0, None, 3, true), 5.0);
        assert_eq!(tried, vec![10.0]);
        assert_eq!((pass, fail), (None, Some(10.0)));
    }

    #[test]
    fn rates_bisect_until_close() {
        let (tried, pass, _) = run(Search::new(10.0, 2.0, None, 10, false), 33.0);
        assert!(pass.unwrap() > 30.0 && pass.unwrap() <= 33.0, "{tried:?}");
    }
}
