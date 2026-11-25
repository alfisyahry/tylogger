// src/core/time/index.rs
//
// Generation-Aware Sparse Timestamp Index for Tylogger
// Bounded RAM overhead (< 10MB for 500k lines), 64-bit offsets, stride checkpoints.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparseTimestampCheckpoint {
    pub line_number: u64,
    pub epoch_nanos: i64,
    pub file_offset: u64,
}

#[derive(Debug, Clone)]
pub struct SparseTimestampIndex {
    pub checkpoints: Vec<SparseTimestampCheckpoint>,
    pub generation: u64,
    pub is_out_of_order: bool,
    pub total_lines_indexed: u64,
    pub stride: u64,
    pub min_nanos: Option<i64>,
    pub max_nanos: Option<i64>,
}

impl Default for SparseTimestampIndex {
    fn default() -> Self {
        Self::new(1, 500)
    }
}

impl SparseTimestampIndex {
    pub fn new(generation: u64, stride: u64) -> Self {
        Self {
            checkpoints: Vec::new(),
            generation,
            is_out_of_order: false,
            total_lines_indexed: 0,
            stride: stride.max(1),
            min_nanos: None,
            max_nanos: None,
        }
    }

    pub fn record_line(&mut self, line_number: u64, epoch_nanos: i64, file_offset: u64) {
        self.total_lines_indexed = self.total_lines_indexed.max(line_number + 1);

        // Update bounds
        self.min_nanos = Some(match self.min_nanos {
            Some(min) => min.min(epoch_nanos),
            None => epoch_nanos,
        });
        self.max_nanos = Some(match self.max_nanos {
            Some(max) => max.max(epoch_nanos),
            None => epoch_nanos,
        });

        // Detect non-monotonic out-of-order records
        if let Some(last) = self.checkpoints.last() {
            if epoch_nanos < last.epoch_nanos {
                self.is_out_of_order = true;
            }
        }

        // Add checkpoint on stride boundary
        if line_number.is_multiple_of(self.stride) {
            self.checkpoints.push(SparseTimestampCheckpoint {
                line_number,
                epoch_nanos,
                file_offset,
            });
        }
    }

    pub fn find_bracket(
        &self,
        target_nanos: i64,
    ) -> (
        Option<SparseTimestampCheckpoint>,
        Option<SparseTimestampCheckpoint>,
    ) {
        if self.checkpoints.is_empty() {
            return (None, None);
        }

        if !self.is_out_of_order {
            // Binary search in monotonic checkpoints
            match self
                .checkpoints
                .binary_search_by_key(&target_nanos, |cp| cp.epoch_nanos)
            {
                Ok(idx) => (Some(self.checkpoints[idx]), Some(self.checkpoints[idx])),
                Err(idx) => {
                    let prev = if idx > 0 {
                        Some(self.checkpoints[idx - 1])
                    } else {
                        None
                    };
                    let next = if idx < self.checkpoints.len() {
                        Some(self.checkpoints[idx])
                    } else {
                        None
                    };
                    (prev, next)
                }
            }
        } else {
            // Linear search across bounded checkpoints for out-of-order sources
            let mut best_prev: Option<SparseTimestampCheckpoint> = None;
            let mut best_next: Option<SparseTimestampCheckpoint> = None;

            for cp in &self.checkpoints {
                if cp.epoch_nanos <= target_nanos
                    && (best_prev.is_none() || cp.epoch_nanos > best_prev.unwrap().epoch_nanos)
                {
                    best_prev = Some(*cp);
                }
                if cp.epoch_nanos >= target_nanos
                    && (best_next.is_none() || cp.epoch_nanos < best_next.unwrap().epoch_nanos)
                {
                    best_next = Some(*cp);
                }
            }
            (best_prev, best_next)
        }
    }

    pub fn invalidate(&mut self, new_generation: u64) {
        self.checkpoints.clear();
        self.generation = new_generation;
        self.is_out_of_order = false;
        self.total_lines_indexed = 0;
        self.min_nanos = None;
        self.max_nanos = None;
    }
}
