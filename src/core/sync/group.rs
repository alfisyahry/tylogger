// src/core/sync/group.rs
//
// Multi-Document Time Synchronization Groups & Event Dispatcher for Tylogger
// Supports Group Creation, Membership, Status (Synchronized, OutsideRange, Missing), Split View.

use super::token::{SyncLoopGuard, SyncOriginToken};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMemberStatus {
    Synchronized,
    Indexing,
    OutsideRange,
    MissingTimestamp,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct SyncMember {
    pub doc_id: u32,
    pub status: SyncMemberStatus,
    pub current_line: u64,
    pub current_nanos: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SyncGroup {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub members: Vec<SyncMember>,
}

impl SyncGroup {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            is_enabled: true,
            members: Vec::new(),
        }
    }

    pub fn add_member(&mut self, doc_id: u32) {
        if !self.has_member(doc_id) {
            self.members.push(SyncMember {
                doc_id,
                status: SyncMemberStatus::Synchronized,
                current_line: 0,
                current_nanos: None,
            });
        }
    }

    pub fn remove_member(&mut self, doc_id: u32) {
        self.members.retain(|m| m.doc_id != doc_id);
    }

    pub fn has_member(&self, doc_id: u32) -> bool {
        self.members.iter().any(|m| m.doc_id == doc_id)
    }

    pub fn update_member_state(
        &mut self,
        doc_id: u32,
        status: SyncMemberStatus,
        line: u64,
        nanos: Option<i64>,
    ) {
        if let Some(m) = self.members.iter_mut().find(|m| m.doc_id == doc_id) {
            m.status = status;
            m.current_line = line;
            m.current_nanos = nanos;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitViewOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct SplitViewState {
    pub is_split: bool,
    pub orientation: SplitViewOrientation,
    pub primary_doc_id: u32,
    pub secondary_doc_id: Option<u32>,
    pub splitter_ratio: f32, // 0.1 to 0.9 (default 0.5)
    pub is_sync_enabled: bool,
}

impl Default for SplitViewState {
    fn default() -> Self {
        Self {
            is_split: false,
            orientation: SplitViewOrientation::Horizontal,
            primary_doc_id: 0,
            secondary_doc_id: None,
            splitter_ratio: 0.5,
            is_sync_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SyncManager {
    groups: HashMap<String, SyncGroup>,
    loop_guard: SyncLoopGuard,
    pub split_view: SplitViewState,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            loop_guard: SyncLoopGuard::new(),
            split_view: SplitViewState::default(),
        }
    }

    pub fn create_group(&mut self, id: &str, name: &str) -> &mut SyncGroup {
        self.groups
            .entry(id.to_string())
            .or_insert_with(|| SyncGroup::new(id, name))
    }

    pub fn get_group(&self, id: &str) -> Option<&SyncGroup> {
        self.groups.get(id)
    }

    pub fn get_group_mut(&mut self, id: &str) -> Option<&mut SyncGroup> {
        self.groups.get_mut(id)
    }

    pub fn join_group(&mut self, group_id: &str, doc_id: u32) {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.add_member(doc_id);
        }
    }

    pub fn leave_all_groups(&mut self, doc_id: u32) {
        for group in self.groups.values_mut() {
            group.remove_member(doc_id);
        }
    }

    pub fn dispatch_time_navigation(
        &mut self,
        origin_doc_id: u32,
        target_nanos: i64,
    ) -> Vec<(u32, i64, SyncOriginToken)> {
        let mut actions = Vec::new();

        // Find all groups where origin_doc_id is a member
        for group in self.groups.values_mut() {
            if !group.is_enabled || !group.has_member(origin_doc_id) {
                continue;
            }

            let token = self.loop_guard.next_token(origin_doc_id);

            for member in &group.members {
                if member.doc_id != origin_doc_id
                    && self.loop_guard.should_process(token, member.doc_id)
                {
                    actions.push((member.doc_id, target_nanos, token));
                }
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_manager_membership_and_dispatch() {
        let mut mgr = SyncManager::new();
        mgr.create_group("grp1", "Production Incident");
        mgr.join_group("grp1", 1);
        mgr.join_group("grp1", 2);
        mgr.join_group("grp1", 3);

        let actions = mgr.dispatch_time_navigation(1, 1_700_000_000_000);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].0, 2);
        assert_eq!(actions[1].0, 3);
        assert_eq!(actions[0].1, 1_700_000_000_000);
    }
}
