// src/core/time/timeshift.rs
//
// Non-Destructive Timeshift Engine & Calibration for Tylogger
// Per-document clock drift correction, multi-sample calibration, integer checked math.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CalibrationResult {
    pub suggested_offset_nanos: i64,
    pub min_offset_nanos: i64,
    pub max_offset_nanos: i64,
    pub sample_count: usize,
    pub is_consistent: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TimeshiftEngine {
    offsets: HashMap<u32, i64>,
}

impl TimeshiftEngine {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    pub fn set_offset_nanos(&mut self, doc_id: u32, offset_nanos: i64) {
        if offset_nanos == 0 {
            self.offsets.remove(&doc_id);
        } else {
            self.offsets.insert(doc_id, offset_nanos);
        }
    }

    pub fn set_offset_components(
        &mut self,
        doc_id: u32,
        days: i32,
        hours: i32,
        minutes: i32,
        seconds: i32,
        millis: i32,
        micros: i32,
        nanos: i32,
    ) {
        let total_nanos = (days as i64) * 86_400_000_000_000
            + (hours as i64) * 3_600_000_000_000
            + (minutes as i64) * 60_000_000_000
            + (seconds as i64) * 1_000_000_000
            + (millis as i64) * 1_000_000
            + (micros as i64) * 1_000
            + (nanos as i64);

        self.set_offset_nanos(doc_id, total_nanos);
    }

    pub fn get_offset_nanos(&self, doc_id: u32) -> i64 {
        self.offsets.get(&doc_id).copied().unwrap_or(0)
    }

    pub fn clear_offset(&mut self, doc_id: u32) {
        self.offsets.remove(&doc_id);
    }

    pub fn all_offsets(&self) -> &HashMap<u32, i64> {
        &self.offsets
    }

    /// Calculate single-sample calibration offset so that:
    /// target_nanos + offset == reference_nanos
    pub fn calibrate_sample(reference_nanos: i64, target_nanos: i64) -> i64 {
        reference_nanos.saturating_sub(target_nanos)
    }

    /// Multi-sample calibration workflow to detect consistent or erratic clock drift
    pub fn calibrate_multi_samples(samples: &[(i64, i64)]) -> Option<CalibrationResult> {
        if samples.is_empty() {
            return None;
        }

        let mut diffs: Vec<i64> = samples
            .iter()
            .map(|&(ref_t, tgt_t)| ref_t.saturating_sub(tgt_t))
            .collect();
        diffs.sort_unstable();

        let count = diffs.len();
        let min_offset = diffs[0];
        let max_offset = diffs[count - 1];
        let median = diffs[count / 2];

        // If spread is within 50ms, considered consistent
        let spread = max_offset.saturating_sub(min_offset);
        let is_consistent = spread < 50_000_000; // 50 ms

        Some(CalibrationResult {
            suggested_offset_nanos: median,
            min_offset_nanos: min_offset,
            max_offset_nanos: max_offset,
            sample_count: count,
            is_consistent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeshift_components_and_calibration() {
        let mut engine = TimeshiftEngine::new();
        // Server B is 2.5s ahead; correction is -2.5s
        engine.set_offset_components(2, 0, 0, 0, -2, -500, 0, 0);
        assert_eq!(engine.get_offset_nanos(2), -2_500_000_000);

        let samples = vec![
            (10_000_000_000, 12_500_000_000),
            (20_000_000_000, 22_500_000_000),
            (30_000_000_000, 32_501_000_000), // 1ms jitter
        ];

        let cal = TimeshiftEngine::calibrate_multi_samples(&samples).unwrap();
        assert_eq!(cal.suggested_offset_nanos, -2_500_000_000);
        assert!(cal.is_consistent);
    }
}
