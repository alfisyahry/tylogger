// src/core/trigger/queue.rs
//
// Bounded Action Queue & Per-Rule Cooldown Rate Limiter for Tylogger
// Protects CPU and RAM against action flooding / storms during high-frequency log tailing.

use super::action::TriggerAction;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct QueuedAction {
    pub rule_id: u32,
    pub source_id: u32,
    pub line_number: u64,
    pub action: TriggerAction,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BoundedActionQueue {
    queue: VecDeque<QueuedAction>,
    capacity: usize,
    pub dropped_count: u64,
}

impl Default for BoundedActionQueue {
    fn default() -> Self {
        Self::new(256)
    }
}

impl BoundedActionQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity.min(1024)),
            capacity: capacity.max(1),
            dropped_count: 0,
        }
    }

    /// Enqueue an action. If queue is full, increments dropped_count and drops oldest or rejects.
    pub fn enqueue(&mut self, action: QueuedAction) -> bool {
        if self.queue.len() >= self.capacity {
            self.dropped_count += 1;
            false
        } else {
            self.queue.push_back(action);
            true
        }
    }

    pub fn dequeue(&mut self) -> Option<QueuedAction> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_queue_overflow_protection() {
        let mut queue = BoundedActionQueue::new(2);

        assert!(queue.enqueue(QueuedAction {
            rule_id: 1,
            source_id: 1,
            line_number: 10,
            action: TriggerAction::StopFollowTail,
            timestamp_ms: 1000,
        }));

        assert!(queue.enqueue(QueuedAction {
            rule_id: 1,
            source_id: 1,
            line_number: 11,
            action: TriggerAction::StopFollowTail,
            timestamp_ms: 1001,
        }));

        // 3rd action overflows bounded queue
        assert!(!queue.enqueue(QueuedAction {
            rule_id: 1,
            source_id: 1,
            line_number: 12,
            action: TriggerAction::StopFollowTail,
            timestamp_ms: 1002,
        }));

        assert_eq!(queue.dropped_count, 1);
        assert_eq!(queue.len(), 2);
    }
}
