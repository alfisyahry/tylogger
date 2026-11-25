#[derive(Debug, Clone, Default, PartialEq)]
pub struct LatencyQuantiles {
    pub count: u64,
    pub min_nanos: u64,
    pub max_nanos: u64,
    pub avg_nanos: u64,
    pub p50_nanos: u64,
    pub p90_nanos: u64,
    pub p95_nanos: u64,
    pub p99_nanos: u64,
    pub sla_breaches: u64,
}

pub struct LatencyAnalytics {
    sla_threshold_nanos: u64,
    durations: Vec<u64>,
}

impl LatencyAnalytics {
    pub fn new(sla_threshold_nanos: u64) -> Self {
        Self {
            sla_threshold_nanos,
            durations: Vec::new(),
        }
    }

    /// Records an explicit duration field from a log record.
    /// Strictly forbids inferring latency from timestamp differences of consecutive lines (RSK-06).
    pub fn record_explicit_duration(&mut self, duration_nanos: u64) {
        self.durations.push(duration_nanos);
    }

    pub fn compute_quantiles(&mut self) -> LatencyQuantiles {
        if self.durations.is_empty() {
            return LatencyQuantiles::default();
        }

        self.durations.sort_unstable();
        let count = self.durations.len();
        let sum: u64 = self.durations.iter().sum();
        let avg = sum / (count as u64);
        let min = self.durations[0];
        let max = self.durations[count - 1];

        let p50 = self.durations[((count as f64) * 0.50) as usize];
        let p90 = self.durations[(((count as f64) * 0.90) as usize).min(count - 1)];
        let p95 = self.durations[(((count as f64) * 0.95) as usize).min(count - 1)];
        let p99 = self.durations[(((count as f64) * 0.99) as usize).min(count - 1)];

        let sla_breaches = self
            .durations
            .iter()
            .filter(|&&d| d > self.sla_threshold_nanos)
            .count() as u64;

        LatencyQuantiles {
            count: count as u64,
            min_nanos: min,
            max_nanos: max,
            avg_nanos: avg,
            p50_nanos: p50,
            p90_nanos: p90,
            p95_nanos: p95,
            p99_nanos: p99,
            sla_breaches,
        }
    }
}
