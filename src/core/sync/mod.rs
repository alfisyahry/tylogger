// src/core/sync/mod.rs
//
// Multi-Document Time Synchronization Module for Tylogger

pub mod group;
pub mod token;

pub use group::{
    SplitViewOrientation, SplitViewState, SyncGroup, SyncManager, SyncMember, SyncMemberStatus,
};
pub use token::{SyncLoopGuard, SyncOriginToken};
