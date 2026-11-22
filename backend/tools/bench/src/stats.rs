use serde::Serialize;

#[derive(Debug, Default, Clone)]
pub struct Samples(Vec<f64>);

impl Samples {
    pub fn push(&mut self, value: f64) {
        if value.is_finite() {
            self.0.push(value);
        }
    }

    pub fn extend(&mut self, values: impl IntoIterator<Item = f64>) {
        for v in values {
            self.push(v);
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn summary(&self) -> Option<Summary> {
        if self.0.is_empty() {
            return None;
        }
        let mut sorted = self.0.clone();
        sorted.sort_by(f64::total_cmp);
        let at = |q: f64| {
            let rank = (q * (sorted.len() - 1) as f64).round() as usize;
            sorted[rank.min(sorted.len() - 1)]
        };
        Some(Summary {
            count: sorted.len(),
            mean: sorted.iter().sum::<f64>() / sorted.len() as f64,
            p50: at(0.50),
            p90: at(0.90),
            p99: at(0.99),
            p999: at(0.999),
            max: sorted[sorted.len() - 1],
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Summary {
    pub count: usize,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
    pub p999: f64,
    pub max: f64,
}

impl Summary {
    /// `p50 1.2  p90 3.4  p99 5.6  max 7.8`
    pub fn line(&self) -> String {
        format!(
            "p50 {:>8}  p90 {:>8}  p99 {:>8}  max {:>8}",
            fmt(self.p50),
            fmt(self.p90),
            fmt(self.p99),
            fmt(self.max)
        )
    }
}

fn fmt(v: f64) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_of_one_to_hundred() {
        let mut s = Samples::default();
        s.extend((1..=100).map(f64::from));
        let sum = s.summary().unwrap();
        assert_eq!(sum.count, 100);
        assert_eq!(sum.max, 100.0);
        assert!((sum.p50 - 50.0).abs() <= 1.0);
        assert!((sum.p99 - 99.0).abs() <= 1.0);
    }

    #[test]
    fn empty_has_no_summary_and_nan_is_ignored() {
        let mut s = Samples::default();
        assert!(s.summary().is_none());
        s.push(f64::NAN);
        assert!(s.is_empty());
    }
}
