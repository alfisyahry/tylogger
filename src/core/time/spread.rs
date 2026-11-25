// src/core/time/spread.rs
//
// Time Spread Aggregation & Viewport Navigation for Tylogger
// Distributes event timestamps into discrete visual time buckets with direct line navigation.

use super::model::TimestampModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSpreadBucket {
    pub bucket_index: usize,
    pub start_nanos: i64,
    pub end_nanos: i64,
    pub count: u64,
    pub first_line: u64,
}

#[derive(Debug, Clone)]
pub struct TimeSpreadModel {
    pub buckets: Vec<TimeSpreadBucket>,
    pub min_nanos: i64,
    pub max_nanos: i64,
    pub total_records: u64,
    pub viewport_start_nanos: Option<i64>,
    pub viewport_end_nanos: Option<i64>,
}

impl TimeSpreadModel {
    pub fn build(records: &[TimestampModel], num_buckets: usize, use_adjusted: bool) -> Self {
        let num_buckets = num_buckets.max(1);
        let mut min_t = i64::MAX;
        let mut max_t = i64::MIN;
        let mut valid_count = 0u64;

        for r in records {
            if !r.is_valid() {
                continue;
            }
            let t = if use_adjusted {
                r.effective_nanos()
            } else {
                r.epoch_nanos
            };
            min_t = min_t.min(t);
            max_t = max_t.max(t);
            valid_count += 1;
        }

        if valid_count == 0 || min_t > max_t {
            return Self {
                buckets: Vec::new(),
                min_nanos: 0,
                max_nanos: 0,
                total_records: 0,
                viewport_start_nanos: None,
                viewport_end_nanos: None,
            };
        }

        let range = (max_t - min_t).max(1);
        let bucket_duration = (range / (num_buckets as i64)).max(1);

        let mut buckets = Vec::with_capacity(num_buckets);
        for i in 0..num_buckets {
            let b_start = min_t + (i as i64) * bucket_duration;
            let b_end = if i == num_buckets - 1 {
                max_t
            } else {
                b_start + bucket_duration
            };
            buckets.push(TimeSpreadBucket {
                bucket_index: i,
                start_nanos: b_start,
                end_nanos: b_end,
                count: 0,
                first_line: 0,
            });
        }

        for r in records {
            if !r.is_valid() {
                continue;
            }
            let t = if use_adjusted {
                r.effective_nanos()
            } else {
                r.epoch_nanos
            };
            let mut idx = ((t - min_t) / bucket_duration) as usize;
            if idx >= num_buckets {
                idx = num_buckets - 1;
            }

            let b = &mut buckets[idx];
            if b.count == 0 {
                b.first_line = r.line_number;
            }
            b.count += 1;
        }

        Self {
            buckets,
            min_nanos: min_t,
            max_nanos: max_t,
            total_records: valid_count,
            viewport_start_nanos: None,
            viewport_end_nanos: None,
        }
    }

    pub fn set_viewport(&mut self, start_nanos: i64, end_nanos: i64) {
        self.viewport_start_nanos = Some(start_nanos);
        self.viewport_end_nanos = Some(end_nanos);
    }

    /// Find which bucket was clicked based on a normalized coordinate [0.0, 1.0]
    pub fn bucket_at_ratio(&self, ratio: f32) -> Option<&TimeSpreadBucket> {
        if self.buckets.is_empty() {
            return None;
        }
        let clamped = ratio.clamp(0.0, 1.0);
        let idx = ((clamped * self.buckets.len() as f32) as usize).min(self.buckets.len() - 1);
        Some(&self.buckets[idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::time::model::TimePrecision;

    #[test]
    fn test_time_spread_aggregation() {
        let records = vec![
            TimestampModel::new_parsed("t1", 1_000_000_000, TimePrecision::Seconds, None, 1, 0),
            TimestampModel::new_parsed("t2", 1_500_000_000, TimePrecision::Seconds, None, 1, 1),
            TimestampModel::new_parsed("t3", 2_000_000_000, TimePrecision::Seconds, None, 1, 2),
            TimestampModel::new_parsed("t4", 3_000_000_000, TimePrecision::Seconds, None, 1, 3),
        ];

        let spread = TimeSpreadModel::build(&records, 2, false);
        assert_eq!(spread.buckets.len(), 2);
        assert_eq!(spread.total_records, 4);
        assert_eq!(spread.buckets[0].count, 2); // 1.0s, 1.5s
        assert_eq!(spread.buckets[1].count, 2); // 2.0s, 3.0s
    }
}
