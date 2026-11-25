// src/core/correlation/candidate.rs
//
// Explainable Correlation Candidates & Match Reasons for Tylogger
// Transparent determinism: no fake causality or black-box conclusions.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchReason {
    TimeProximityOnly {
        window_ms: i64,
        delta_ms: i64,
    },
    ExactFieldMatch {
        field_name: String,
        field_value: String,
    },
    TimeAndFieldMatch {
        delta_ms: i64,
        field_name: String,
        field_value: String,
    },
}

impl fmt::Display for MatchReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimeProximityOnly {
                window_ms,
                delta_ms,
            } => {
                let sign = if *delta_ms >= 0 { "+" } else { "" };
                write!(
                    f,
                    "Time Proximity +/-{}ms (Delta: {}{}ms)",
                    window_ms, sign, delta_ms
                )
            }
            Self::ExactFieldMatch {
                field_name,
                field_value,
            } => {
                write!(f, "Exact Match: {} == \"{}\"", field_name, field_value)
            }
            Self::TimeAndFieldMatch {
                delta_ms,
                field_name,
                field_value,
            } => {
                let sign = if *delta_ms >= 0 { "+" } else { "" };
                write!(
                    f,
                    "Time Delta: {}{}ms AND {} == \"{}\"",
                    sign, delta_ms, field_name, field_value
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationCandidate {
    pub source_id: u32,
    pub source_alias: String,
    pub physical_line_number: u64,
    pub timestamp_nanos: i64,
    pub delta_nanos: i64,
    pub match_reason: MatchReason,
    pub message: String,
}
