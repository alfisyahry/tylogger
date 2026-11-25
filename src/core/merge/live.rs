// src/core/merge/live.rs
//
// Live Log Merging with Bounded Reorder Window & Watermark Lateness Policy
// Guarantees zero silent record loss, explicit late arrival flag, and live telemetry metrics.

use super::provenance::RecordProvenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LiveMergeMetrics {
    pub received_records: u64,
    pub buffered_records: u64,
    pub late_records: u64,
    pub invalid_timestamps: u64,
    pub dropped_records: u64, // Always 0 (zero silent dropping guarantee)
}

#[derive(Debug, Clone)]
pub struct LiveLogMergeEngine {
    pub reorder_window_nanos: i64,
    pub watermark_nanos: i64,
    pub max_observed_nanos: i64,
    pub buffer: Vec<RecordProvenance>,
    pub metrics: LiveMergeMetrics,
}

impl LiveLogMergeEngine {
    pub fn new(reorder_window_nanos: i64) -> Self {
        Self {
            reorder_window_nanos: reorder_window_nanos.max(0),
            watermark_nanos: i64::MIN,
            max_observed_nanos: i64::MIN,
            buffer: Vec::new(),
            metrics: LiveMergeMetrics::default(),
        }
    }

    /// Ingest a newly arrived record during live tailing
    pub fn ingest_record(&mut self, mut record: RecordProvenance) {
        self.metrics.received_records += 1;

        if record.effective_nanos == 0 {
            self.metrics.invalid_timestamps += 1;
        }

        self.max_observed_nanos = self.max_observed_nanos.max(record.effective_nanos);
        self.watermark_nanos = self
            .max_observed_nanos
            .saturating_sub(self.reorder_window_nanos);

        if record.effective_nanos < self.watermark_nanos {
            // Late arrival: arrived past the watermark window
            record.is_late_arrival = true;
            self.metrics.late_records += 1;
        }

        // Insert into chronological buffer
        let pos = match self
            .buffer
            .binary_search_by_key(&record.effective_nanos, |r| r.effective_nanos)
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        self.buffer.insert(pos, record);
        self.metrics.buffered_records = self.buffer.len() as u64;
    }

    /// Flush records older than watermark that are ready to be safely emitted
    pub fn drain_ready_records(&mut self) -> Vec<RecordProvenance> {
        let split_idx = self
            .buffer
            .partition_point(|r| r.effective_nanos < self.watermark_nanos);

        let ready = self.buffer.drain(0..split_idx).collect();
        self.metrics.buffered_records = self.buffer.len() as u64;
        ready
    }

    /// Flush all remaining buffered records (e.g. on shutdown / stream end)
    pub fn drain_all(&mut self) -> Vec<RecordProvenance> {
        let all = std::mem::take(&mut self.buffer);
        self.metrics.buffered_records = 0;
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_merge_watermark_and_late_arrivals() {
        // 3-second reorder window (3_000_000_000 ns)
        let mut engine = LiveLogMergeEngine::new(3_000_000_000);

        // Record at t = 10s
        engine.ingest_record(RecordProvenance::new(1, 0, 0, 10_000_000_000, "SrcA"));
        assert_eq!(engine.watermark_nanos, 7_000_000_000);

        // Record at t = 8s (arrives late but within 3s window -> valid, not flagged)
        engine.ingest_record(RecordProvenance::new(2, 0, 0, 8_000_000_000, "SrcB"));
        assert_eq!(engine.metrics.late_records, 0);

        // Record at t = 5s (arrives past watermark 7s -> flagged late, zero dropped)
        engine.ingest_record(RecordProvenance::new(2, 1, 0, 5_000_000_000, "SrcB"));
        assert_eq!(engine.metrics.late_records, 1);
        assert_eq!(engine.metrics.dropped_records, 0);
        assert_eq!(engine.metrics.received_records, 3);
    }
}
