// src/core/time/mod.rs
//
// Core Time Intelligence Module for Tylogger

pub mod delta;
pub mod index;
pub mod locator;
pub mod model;
pub mod parser;
pub mod spread;
pub mod timeshift;

pub use delta::{DeltaCalculator, DeltaUnit};
pub use index::{SparseTimestampCheckpoint, SparseTimestampIndex};
pub use locator::{LocatorResult, MatchMode, TimestampLocator};
pub use model::{
    days_to_gregorian, nanos_to_iso_string, ymd_hms_to_nanos, ParseStatus, TimePrecision,
    TimestampModel,
};
pub use parser::TimestampParserRegistry;
pub use spread::{TimeSpreadBucket, TimeSpreadModel};
pub use timeshift::{CalibrationResult, TimeshiftEngine};
