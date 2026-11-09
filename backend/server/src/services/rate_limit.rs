//! In-memory token-bucket rate limiting.

use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use dashmap::DashMap;

use crate::{
    config::{ConfigProvider, Limit, RateLimitConfig},
    core::CleanableService,
    errors::AuthError,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LimitKey {
    LoginIp(IpAddr),
    /// A username or email, lowercased.
    LoginAccount(String),
    RegisterIp(IpAddr),
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    updated: Instant,
    /// The refill period last used
    period: Duration,
}

#[derive(Default)]
pub struct RateLimiter {
    buckets: DashMap<LimitKey, Bucket>,
}

impl RateLimiter {
    /// Take one token. On refusal, returns how long until one is available.
    pub fn acquire(&self, key: LimitKey, limit: Limit) -> Result<(), Duration> {
        self.acquire_at(key, limit, Instant::now())
    }

    /// Like [`acquire`](Self::acquire), but only looks: nothing is consumed.
    pub fn check(&self, key: &LimitKey, limit: Limit) -> Result<(), Duration> {
        self.check_at(key, limit, Instant::now())
    }

    fn acquire_at(&self, key: LimitKey, limit: Limit, now: Instant) -> Result<(), Duration> {
        if limit.burst == 0 {
            return Err(period(limit));
        }
        let mut entry = self.buckets.entry(key).or_insert_with(|| full(limit, now));
        let bucket = entry.value_mut();
        refill(bucket, limit, now);
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(wait_for_token(bucket, limit))
        }
    }

    fn check_at(&self, key: &LimitKey, limit: Limit, now: Instant) -> Result<(), Duration> {
        if limit.burst == 0 {
            return Err(period(limit));
        }
        let Some(mut entry) = self.buckets.get_mut(key) else {
            return Ok(());
        };
        let bucket = entry.value_mut();
        refill(bucket, limit, now);
        if bucket.tokens >= 1.0 {
            Ok(())
        } else {
            Err(wait_for_token(bucket, limit))
        }
    }

    fn sweep_at(&self, now: Instant) -> usize {
        let before = self.buckets.len();
        self.buckets
            .retain(|_, b| now.saturating_duration_since(b.updated) < b.period);
        before - self.buckets.len()
    }
}

fn period(limit: Limit) -> Duration {
    Duration::from_secs(limit.period_secs.max(1))
}

fn full(limit: Limit, now: Instant) -> Bucket {
    Bucket {
        tokens: limit.burst as f64,
        updated: now,
        period: period(limit),
    }
}

fn rate_per_sec(limit: Limit) -> f64 {
    limit.burst as f64 / period(limit).as_secs_f64()
}

fn refill(bucket: &mut Bucket, limit: Limit, now: Instant) {
    let elapsed = now.saturating_duration_since(bucket.updated).as_secs_f64();
    bucket.tokens = (bucket.tokens + elapsed * rate_per_sec(limit)).min(limit.burst as f64);
    bucket.updated = now;
    bucket.period = period(limit);
}

fn wait_for_token(bucket: &Bucket, limit: Limit) -> Duration {
    Duration::from_secs_f64((1.0 - bucket.tokens) / rate_per_sec(limit))
}

/// The limits on login and registration
pub struct AuthRateLimits {
    limiter: RateLimiter,
    config: Arc<ConfigProvider>,
}

impl AuthRateLimits {
    pub fn new(config: Arc<ConfigProvider>) -> Self {
        Self {
            limiter: RateLimiter::default(),
            config,
        }
    }

    fn config(&self) -> Option<RateLimitConfig> {
        let config = self.config.get::<RateLimitConfig>();
        config.enabled.then_some(config)
    }

    pub fn login_attempt(&self, ip: Option<IpAddr>, account: &str) -> Result<(), AuthError> {
        let Some(config) = self.config() else {
            return Ok(());
        };
        if let Some(ip) = ip {
            self.limiter
                .acquire(LimitKey::LoginIp(ip), config.login_per_ip)
                .map_err(limited)?;
        }
        self.limiter
            .check(&account_key(account), config.login_failures_per_account)
            .map_err(limited)
    }

    pub fn login_failed(&self, account: &str) {
        if let Some(config) = self.config() {
            let _ = self
                .limiter
                .acquire(account_key(account), config.login_failures_per_account);
        }
    }

    pub fn register_attempt(&self, ip: Option<IpAddr>) -> Result<(), AuthError> {
        let (Some(config), Some(ip)) = (self.config(), ip) else {
            return Ok(());
        };
        self.limiter
            .acquire(LimitKey::RegisterIp(ip), config.register_per_ip)
            .map_err(limited)
    }
}

fn account_key(account: &str) -> LimitKey {
    LimitKey::LoginAccount(account.trim().to_lowercase())
}

fn limited(wait: Duration) -> AuthError {
    AuthError::RateLimited {
        retry_after_secs: wait.as_secs_f64().ceil().max(1.0) as u64,
    }
}

#[async_trait]
impl CleanableService for AuthRateLimits {
    fn name(&self) -> &'static str {
        "AuthRateLimits"
    }

    async fn clean(&self) -> usize {
        self.limiter.sweep_at(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip() -> LimitKey {
        LimitKey::LoginIp("203.0.113.7".parse().unwrap())
    }

    #[test]
    fn allows_a_burst_then_refuses_until_a_token_refills() {
        let limiter = RateLimiter::default();
        let limit = Limit::new(3, 60); // one token every 20s
        let t0 = Instant::now();

        for _ in 0..3 {
            limiter.acquire_at(ip(), limit, t0).unwrap();
        }
        let wait = limiter.acquire_at(ip(), limit, t0).unwrap_err();
        assert_eq!(wait.as_secs(), 20);

        assert!(
            limiter
                .acquire_at(ip(), limit, t0 + Duration::from_secs(19))
                .is_err()
        );
        limiter
            .acquire_at(ip(), limit, t0 + Duration::from_secs(21))
            .unwrap();
    }

    #[test]
    fn check_does_not_consume() {
        let limiter = RateLimiter::default();
        let limit = Limit::new(1, 60);
        let t0 = Instant::now();

        assert!(limiter.check_at(&ip(), limit, t0).is_ok());
        assert!(limiter.check_at(&ip(), limit, t0).is_ok());
        limiter.acquire_at(ip(), limit, t0).unwrap();
        assert!(limiter.check_at(&ip(), limit, t0).is_err());
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::default();
        let limit = Limit::new(1, 60);
        let t0 = Instant::now();

        limiter.acquire_at(ip(), limit, t0).unwrap();
        limiter
            .acquire_at(LimitKey::LoginAccount("alice".into()), limit, t0)
            .unwrap();
        assert!(limiter.acquire_at(ip(), limit, t0).is_err());
    }

    #[test]
    fn sweep_drops_only_refilled_buckets() {
        let limiter = RateLimiter::default();
        let t0 = Instant::now();
        limiter.acquire_at(ip(), Limit::new(5, 60), t0).unwrap();
        limiter
            .acquire_at(LimitKey::LoginAccount("bob".into()), Limit::new(5, 600), t0)
            .unwrap();

        assert_eq!(limiter.sweep_at(t0 + Duration::from_secs(61)), 1);
        assert_eq!(limiter.buckets.len(), 1);
    }

    #[test]
    fn a_zero_burst_refuses_everything() {
        let limiter = RateLimiter::default();
        assert!(limiter.acquire(ip(), Limit::new(0, 60)).is_err());
    }
}
