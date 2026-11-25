// src/core/stats/collector.rs
//
// Streaming Event Statistics Aggregation for Tylogger
// Bounded time buckets (s/m/5m/h/d), severity counters, per-source frequency.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSeverity {
    Error,
    Warn,
    Info,
    Debug,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBucketDuration {
    Second,
    Minute,
    FiveMinutes,
    Hour,
    Day,
}

impl TimeBucketDuration {
    pub fn duration_secs(&self) -> i64 {
        match self {
            Self::Second => 1,
            Self::Minute => 60,
            Self::FiveMinutes => 300,
            Self::Hour => 3600,
            Self::Day => 86400,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimeBucketEntry {
    pub bucket_start_epoch_secs: i64,
    pub total_events: u64,
    pub errors: u64,
    pub warns: u64,
    pub infos: u64,
    pub debugs: u64,
}

#[derive(Debug, Clone)]
pub struct EventStatisticsCollector {
    pub bucket_duration: TimeBucketDuration,
    buckets: HashMap<i64, TimeBucketEntry>,
    pub total_observed: u64,
    pub total_errors: u64,
    pub total_warns: u64,
    pub total_infos: u64,
    pub total_debugs: u64,
    max_buckets: usize,
}

impl Default for EventStatisticsCollector {
    fn default() -> Self {
        Self::new(TimeBucketDuration::Minute, 1000)
    }
}

impl EventStatisticsCollector {
    pub fn new(bucket_duration: TimeBucketDuration, max_buckets: usize) -> Self {
        Self {
            bucket_duration,
            buckets: HashMap::new(),
            total_observed: 0,
            total_errors: 0,
            total_warns: 0,
            total_infos: 0,
            total_debugs: 0,
            max_buckets: max_buckets.max(100),
        }
    }

    pub fn record_event(&mut self, epoch_nanos: i64, severity: EventSeverity) {
        self.total_observed += 1;
        match severity {
            EventSeverity::Error => self.total_errors += 1,
            EventSeverity::Warn => self.total_warns += 1,
            EventSeverity::Info => self.total_infos += 1,
            EventSeverity::Debug => self.total_debugs += 1,
            EventSeverity::Other => {}
        }

        let epoch_secs = epoch_nanos / 1_000_000_000;
        let d = self.bucket_duration.duration_secs();
        let bucket_start = (epoch_secs / d) * d;

        if self.buckets.len() < self.max_buckets || self.buckets.contains_key(&bucket_start) {
            let entry = self
                .buckets
                .entry(bucket_start)
                .or_insert_with(|| TimeBucketEntry {
                    bucket_start_epoch_secs: bucket_start,
                    ..Default::default()
                });

            entry.total_events += 1;
            match severity {
                EventSeverity::Error => entry.errors += 1,
                EventSeverity::Warn => entry.warns += 1,
                EventSeverity::Info => entry.infos += 1,
                EventSeverity::Debug => entry.debugs += 1,
                EventSeverity::Other => {}
            }
        }
    }

    pub fn sorted_buckets(&self) -> Vec<TimeBucketEntry> {
        let mut list: Vec<TimeBucketEntry> = self.buckets.values().cloned().collect();
        list.sort_unstable_by_key(|b| b.bucket_start_epoch_secs);
        list
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.total_observed = 0;
        self.total_errors = 0;
        self.total_warns = 0;
        self.total_infos = 0;
        self.total_debugs = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_statistics_aggregation() {
        let mut collector = EventStatisticsCollector::new(TimeBucketDuration::Minute, 100);

        // 3 events at t = 10s, 20s, 30s (all in 0..60s bucket)
        collector.record_event(10_000_000_000, EventSeverity::Info);
        collector.record_event(20_000_000_000, EventSeverity::Error);
        collector.record_event(30_000_000_000, EventSeverity::Error);

        // 1 event at t = 90s (in 60..120s bucket)
        collector.record_event(90_000_000_000, EventSeverity::Warn);

        assert_eq!(collector.total_observed, 4);
        assert_eq!(collector.total_errors, 2);
        assert_eq!(collector.total_warns, 1);
        assert_eq!(collector.total_infos, 1);

        let buckets = collector.sorted_buckets();
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].total_events, 3);
        assert_eq!(buckets[0].errors, 2);
        assert_eq!(buckets[1].total_events, 1);
        assert_eq!(buckets[1].warns, 1);
    }
}
