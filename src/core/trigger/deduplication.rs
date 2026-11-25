// src/core/trigger/deduplication.rs
//
// Global Multi-View Trigger Deduplication for Tylogger
// Binds trigger execution to (source_file_id, physical_line_number, rule_id)
// Prevents duplicate alerts when the same line appears in Main Tab, Filter-to-Tab, and Merged Tab.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TriggerExecutionKey {
    pub source_file_id: u32,
    pub physical_line_number: u64,
    pub rule_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalTriggerDeduplicator {
    executed_keys: HashSet<TriggerExecutionKey>,
}

impl GlobalTriggerDeduplicator {
    pub fn new() -> Self {
        Self {
            executed_keys: HashSet::new(),
        }
    }

    /// Try to acquire execution token. Returns true if first time, false if already executed.
    pub fn try_claim(
        &mut self,
        source_file_id: u32,
        physical_line_number: u64,
        rule_id: u32,
    ) -> bool {
        let key = TriggerExecutionKey {
            source_file_id,
            physical_line_number,
            rule_id,
        };

        let is_new = self.executed_keys.insert(key);

        // Keep memory bounded: retain at most 10,000 keys
        if self.executed_keys.len() > 10_000 {
            self.executed_keys.clear();
            self.executed_keys.insert(key);
        }

        is_new
    }

    pub fn clear(&mut self) {
        self.executed_keys.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_view_deduplication() {
        let mut dedup = GlobalTriggerDeduplicator::new();

        // 1. First trigger in Main Tab -> Claimed
        assert!(dedup.try_claim(1, 100, 42));

        // 2. Same line observed in Filter-to-Tab -> Deduplicated!
        assert!(!dedup.try_claim(1, 100, 42));

        // 3. Same line observed in Merged Tab -> Deduplicated!
        assert!(!dedup.try_claim(1, 100, 42));

        // Different line -> Claimed
        assert!(dedup.try_claim(1, 101, 42));
    }
}
