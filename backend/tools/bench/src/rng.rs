use std::time::{SystemTime, UNIX_EPOCH};

/// SplitMix64: fast, tiny, and plenty for sampling latencies.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self(nanos)
    }

    pub fn derive(seed: u64, index: u64) -> Self {
        let mut rng = Self(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        rng.next_u64();
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + self.next_u64() % (hi - lo + 1)
    }

    pub fn chance(&mut self, p: f64) -> bool {
        p > 0.0 && self.unit() < p
    }

    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.unit(); // (0, 1], keeps ln finite
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    pub fn lognormal(&mut self, median: f64, sigma: f64) -> f64 {
        if sigma <= 0.0 {
            return median;
        }
        median * (sigma * self.normal()).exp()
    }

    pub fn exponential(&mut self, mean: f64) -> f64 {
        -mean * (1.0 - self.unit()).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let xs: Vec<u64> = (0..5).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..5).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs[0], xs[1]);
    }

    #[test]
    fn lognormal_median_is_roughly_right() {
        let mut rng = Rng::new(1);
        let mut xs: Vec<f64> = (0..20_001).map(|_| rng.lognormal(600.0, 0.5)).collect();
        xs.sort_by(f64::total_cmp);
        let median = xs[xs.len() / 2];
        assert!((550.0..650.0).contains(&median), "median {median}");
    }

    #[test]
    fn range_is_inclusive_and_bounded() {
        let mut rng = Rng::new(3);
        for _ in 0..1000 {
            let x = rng.range(2, 4);
            assert!((2..=4).contains(&x));
        }
        assert_eq!(rng.range(5, 5), 5);
    }
}
