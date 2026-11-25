// src/core/stats/timeline.rs
//
// Native GDI Event Timeline Layout & Navigation for Tylogger
// Renders event frequency bars against time axis without heavy web chart dependencies.

use super::collector::EventStatisticsCollector;

#[derive(Debug, Clone)]
pub struct TimelineBar {
    pub bucket_start_secs: i64,
    pub total_count: u64,
    pub error_ratio: f32,
    pub warn_ratio: f32,
    pub info_ratio: f32,
    pub norm_x: f32,      // [0.0, 1.0]
    pub norm_height: f32, // [0.0, 1.0]
}

#[derive(Debug, Clone, Default)]
pub struct EventTimelineModel {
    pub bars: Vec<TimelineBar>,
    pub max_count_in_bucket: u64,
}

impl EventTimelineModel {
    pub fn build_from_collector(collector: &EventStatisticsCollector) -> Self {
        let buckets = collector.sorted_buckets();
        if buckets.is_empty() {
            return Self::default();
        }

        let max_count = buckets
            .iter()
            .map(|b| b.total_events)
            .max()
            .unwrap_or(1)
            .max(1);
        let n = buckets.len() as f32;

        let bars = buckets
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let total = b.total_events as f32;
                let norm_h = (total / max_count as f32).clamp(0.05, 1.0);
                TimelineBar {
                    bucket_start_secs: b.bucket_start_epoch_secs,
                    total_count: b.total_events,
                    error_ratio: b.errors as f32 / total,
                    warn_ratio: b.warns as f32 / total,
                    info_ratio: b.infos as f32 / total,
                    norm_x: (i as f32 + 0.5) / n,
                    norm_height: norm_h,
                }
            })
            .collect();

        Self {
            bars,
            max_count_in_bucket: max_count,
        }
    }

    /// Locate timestamp from click ratio [0.0, 1.0]
    pub fn timestamp_at_ratio(&self, ratio: f32) -> Option<i64> {
        if self.bars.is_empty() {
            return None;
        }
        let clamped = ratio.clamp(0.0, 1.0);
        let idx = ((clamped * self.bars.len() as f32) as usize).min(self.bars.len() - 1);
        Some(self.bars[idx].bucket_start_secs * 1_000_000_000)
    }
}
