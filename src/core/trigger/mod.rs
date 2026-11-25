// src/core/trigger/mod.rs
//
// Keyword Triggers & Action Automation Module for Tylogger

pub mod action;
pub mod deduplication;
pub mod engine;
pub mod history;
pub mod queue;

pub use action::TriggerAction;
pub use deduplication::{GlobalTriggerDeduplicator, TriggerExecutionKey};
pub use engine::{KeywordTriggerRule, TriggerEngine};
pub use history::{ActionHistoryEntry, BoundedActionHistory};
pub use queue::{BoundedActionQueue, QueuedAction};
