// src/core/stats/mod.rs
//
// Event Statistics & Native GDI Timeline Module for Tylogger

pub mod collector;
pub mod timeline;

pub use collector::{EventSeverity, EventStatisticsCollector, TimeBucketDuration, TimeBucketEntry};
pub use timeline::{EventTimelineModel, TimelineBar};
