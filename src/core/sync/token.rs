// src/core/sync/token.rs
//
// Recursion Guards & Loop-Free Origin Tokens for Tylogger Time Synchronization
// Prevents Doc A -> Doc B -> Doc A recursive infinite event update loops.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyncOriginToken {
    pub origin_doc_id: u32,
    pub sequence: u64,
}

impl SyncOriginToken {
    pub fn new(origin_doc_id: u32, sequence: u64) -> Self {
        Self {
            origin_doc_id,
            sequence,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SyncLoopGuard {
    seen_events: HashSet<(SyncOriginToken, u32)>,
    current_sequence: u64,
}

impl SyncLoopGuard {
    pub fn new() -> Self {
        Self {
            seen_events: HashSet::new(),
            current_sequence: 1,
        }
    }

    pub fn next_token(&mut self, origin_doc_id: u32) -> SyncOriginToken {
        self.current_sequence = self.current_sequence.wrapping_add(1);
        SyncOriginToken::new(origin_doc_id, self.current_sequence)
    }

    pub fn should_process(&mut self, token: SyncOriginToken, receiver_doc_id: u32) -> bool {
        // Reject event if receiver is the origin document itself
        if token.origin_doc_id == receiver_doc_id {
            return false;
        }

        // Keep bounded memory: retain at most 1024 recent tokens
        if self.seen_events.len() > 1024 {
            self.seen_events.clear();
        }

        // Accept and record token once
        self.seen_events.insert((token, receiver_doc_id))
    }

    pub fn clear(&mut self) {
        self.seen_events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_loop_guard_prevents_echo() {
        let mut guard = SyncLoopGuard::new();
        let tok = guard.next_token(1);

        // Origin document should reject its own echo
        assert!(!guard.should_process(tok, 1));

        // Receiver document 2 should process once
        assert!(guard.should_process(tok, 2));

        // Receiver document 2 duplicate should be rejected
        assert!(!guard.should_process(tok, 2));
    }
}
