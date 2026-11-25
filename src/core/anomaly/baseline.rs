#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyPoint {
    pub timestamp_nanos: i64,
    pub metric_name: String,
    pub observed_value: f64,
    pub baseline_mean: f64,
    pub z_score: f64,
    pub explanation: String,
}

pub struct RollingBaselineDetector {
    window_size: usize,
    z_threshold: f64,
    history: Vec<f64>,
}

impl RollingBaselineDetector {
    pub fn new(window_size: usize, z_threshold: f64) -> Self {
        Self {
            window_size: window_size.max(3),
            z_threshold: z_threshold.max(1.0),
            history: Vec::with_capacity(window_size.max(3)),
        }
    }

    /// Evaluates a new observation and returns an AnomalyPoint if it significantly deviates from baseline.
    pub fn evaluate(
        &mut self,
        timestamp_nanos: i64,
        metric_name: &str,
        value: f64,
    ) -> Option<AnomalyPoint> {
        let anomaly = if self.history.len() >= 3 {
            let n = self.history.len() as f64;
            let mean = self.history.iter().sum::<f64>() / n;
            let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            let std_dev = variance.sqrt();
            let z_score = if std_dev > 0.001 {
                (value - mean) / std_dev
            } else if (value - mean).abs() > 0.001 {
                (value - mean).abs() / 0.001
            } else {
                0.0
            };

            if z_score >= self.z_threshold {
                let ratio = if mean > 0.0 { value / mean } else { value };
                let explanation = format!(
                    "Statistical deviation: {} reached {:.1}, exceeding rolling baseline by {:.1}x (z-score: {:.2}, threshold: {:.1})",
                    metric_name, value, ratio, z_score, self.z_threshold
                );
                Some(AnomalyPoint {
                    timestamp_nanos,
                    metric_name: metric_name.to_string(),
                    observed_value: value,
                    baseline_mean: mean,
                    z_score,
                    explanation,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Update sliding window
        if self.history.len() >= self.window_size {
            self.history.remove(0);
        }
        self.history.push(value);

        anomaly
    }
}
