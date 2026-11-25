// src/core/correlation/mod.rs
//
// Source-Aware Log Correlation Module for Tylogger

pub mod candidate;
pub mod engine;

pub use candidate::{CorrelationCandidate, MatchReason};
pub use engine::{CorrelationEngine, CorrelationRule, CorrelationTargetRecord};
