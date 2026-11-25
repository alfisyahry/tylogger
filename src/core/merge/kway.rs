// src/core/merge/kway.rs
//
// Deterministic Bounded K-Way Merge for Multi-Source Log Merging
// Priority queue (Min-Heap) with zero disk concatenation & bounded RAM overhead.
// Tie-breaking: (1) Effective timestamp, (2) Configured source order, (3) Source physical line.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::provenance::RecordProvenance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeCandidate {
    pub effective_nanos: i64,
    pub source_order: usize,
    pub source_id: u32,
    pub physical_line_number: u64,
    pub file_offset: u64,
    pub source_alias: String,
}

impl Ord for MergeCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for Min-Heap in Rust BinaryHeap
        other
            .effective_nanos
            .cmp(&self.effective_nanos)
            .then_with(|| other.source_order.cmp(&self.source_order))
            .then_with(|| other.physical_line_number.cmp(&self.physical_line_number))
            .then_with(|| other.source_id.cmp(&self.source_id))
    }
}

impl PartialOrd for MergeCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct BoundedKWayMerger {
    heap: BinaryHeap<MergeCandidate>,
}

impl Default for BoundedKWayMerger {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundedKWayMerger {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    /// Push candidate from source into min-heap
    pub fn push(&mut self, candidate: MergeCandidate) {
        self.heap.push(candidate);
    }

    /// Pop next chronologically earliest candidate (O(log K))
    pub fn pop_next(&mut self) -> Option<RecordProvenance> {
        self.heap.pop().map(|cand| RecordProvenance {
            source_id: cand.source_id,
            physical_line_number: cand.physical_line_number,
            file_offset: cand.file_offset,
            effective_nanos: cand.effective_nanos,
            source_alias: cand.source_alias,
            is_late_arrival: false,
        })
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_tie_breaking() {
        let mut merger = BoundedKWayMerger::new();

        // 3 candidates with identical timestamp (1000)
        merger.push(MergeCandidate {
            effective_nanos: 1000,
            source_order: 1, // Second source
            source_id: 102,
            physical_line_number: 5,
            file_offset: 50,
            source_alias: "ErrorLog".to_string(),
        });

        merger.push(MergeCandidate {
            effective_nanos: 1000,
            source_order: 0, // First source
            source_id: 101,
            physical_line_number: 10,
            file_offset: 100,
            source_alias: "AccessLog".to_string(),
        });

        merger.push(MergeCandidate {
            effective_nanos: 1000,
            source_order: 0, // First source, lower line number
            source_id: 101,
            physical_line_number: 4,
            file_offset: 40,
            source_alias: "AccessLog".to_string(),
        });

        // Earliest source_order and lowest line should pop first
        let p1 = merger.pop_next().unwrap();
        assert_eq!(p1.source_id, 101);
        assert_eq!(p1.physical_line_number, 4);

        let p2 = merger.pop_next().unwrap();
        assert_eq!(p2.source_id, 101);
        assert_eq!(p2.physical_line_number, 10);

        let p3 = merger.pop_next().unwrap();
        assert_eq!(p3.source_id, 102);
        assert_eq!(p3.physical_line_number, 5);
    }
}
