// src/core/merge/mod.rs
//
// Chronological Log Merging Module for Tylogger

pub mod kway;
pub mod live;
pub mod provenance;
pub mod source;

pub use kway::{BoundedKWayMerger, MergeCandidate};
pub use live::{LiveLogMergeEngine, LiveMergeMetrics};
pub use provenance::{RecordProvenance, SourceAlias};
pub use source::MergedLogSource;
