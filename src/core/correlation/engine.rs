// src/core/correlation/engine.rs
//
// Deterministic Source-Aware Correlation Engine for Tylogger
// Evaluates Time Window Proximity and Field Equality without black-box AI.

use super::candidate::{CorrelationCandidate, MatchReason};

#[derive(Debug, Clone)]
pub struct CorrelationRule {
    pub window_before_nanos: i64,
    pub window_after_nanos: i64,
    pub match_field_name: Option<String>,
    pub match_field_value: Option<String>,
    pub require_both: bool,
}

impl Default for CorrelationRule {
    fn default() -> Self {
        Self {
            window_before_nanos: 5_000_000_000, // 5s
            window_after_nanos: 5_000_000_000,
            match_field_name: None,
            match_field_value: None,
            require_both: false,
        }
    }
}

pub struct CorrelationEngine;

impl CorrelationEngine {
    pub fn correlate_record<F>(
        anchor_nanos: i64,
        rule: &CorrelationRule,
        target_records: &[CorrelationTargetRecord],
        max_candidates: usize,
        mut line_field_extractor: F,
    ) -> Vec<CorrelationCandidate>
    where
        F: FnMut(&str, &str) -> Option<String>,
    {
        let mut candidates = Vec::new();
        let min_t = anchor_nanos.saturating_sub(rule.window_before_nanos);
        let max_t = anchor_nanos.saturating_add(rule.window_after_nanos);

        for rec in target_records {
            let delta = rec.timestamp_nanos.saturating_sub(anchor_nanos);
            let in_window = rec.timestamp_nanos >= min_t && rec.timestamp_nanos <= max_t;

            let field_match = if let (Some(fname), Some(fval)) =
                (&rule.match_field_name, &rule.match_field_value)
            {
                line_field_extractor(&rec.message, fname).is_some_and(|v| v == *fval)
            } else {
                false
            };

            let reason = if rule.require_both {
                if in_window && field_match {
                    Some(MatchReason::TimeAndFieldMatch {
                        delta_ms: delta / 1_000_000,
                        field_name: rule.match_field_name.clone().unwrap(),
                        field_value: rule.match_field_value.clone().unwrap(),
                    })
                } else {
                    None
                }
            } else if field_match && in_window {
                Some(MatchReason::TimeAndFieldMatch {
                    delta_ms: delta / 1_000_000,
                    field_name: rule.match_field_name.clone().unwrap(),
                    field_value: rule.match_field_value.clone().unwrap(),
                })
            } else if field_match {
                Some(MatchReason::ExactFieldMatch {
                    field_name: rule.match_field_name.clone().unwrap(),
                    field_value: rule.match_field_value.clone().unwrap(),
                })
            } else if in_window && rule.match_field_name.is_none() {
                Some(MatchReason::TimeProximityOnly {
                    window_ms: (rule.window_before_nanos + rule.window_after_nanos) / 2_000_000,
                    delta_ms: delta / 1_000_000,
                })
            } else {
                None
            };

            if let Some(match_reason) = reason {
                candidates.push(CorrelationCandidate {
                    source_id: rec.source_id,
                    source_alias: rec.source_alias.clone(),
                    physical_line_number: rec.physical_line_number,
                    timestamp_nanos: rec.timestamp_nanos,
                    delta_nanos: delta,
                    match_reason,
                    message: rec.message.clone(),
                });

                if candidates.len() >= max_candidates {
                    break;
                }
            }
        }

        candidates
    }
}

#[derive(Debug, Clone)]
pub struct CorrelationTargetRecord {
    pub source_id: u32,
    pub source_alias: String,
    pub physical_line_number: u64,
    pub timestamp_nanos: i64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explainable_correlation_matching() {
        let targets = vec![
            CorrelationTargetRecord {
                source_id: 2,
                source_alias: "App".to_string(),
                physical_line_number: 10,
                timestamp_nanos: 10_012_000_000, // +12ms
                message: "Exception processing request req-abc-99".to_string(),
            },
            CorrelationTargetRecord {
                source_id: 3,
                source_alias: "DB".to_string(),
                physical_line_number: 55,
                timestamp_nanos: 15_000_000_000, // +5s (out of tight window)
                message: "Deadlock detected".to_string(),
            },
        ];

        let rule = CorrelationRule {
            window_before_nanos: 1_000_000_000,
            window_after_nanos: 1_000_000_000,
            match_field_name: Some("req_id".to_string()),
            match_field_value: Some("req-abc-99".to_string()),
            require_both: true,
        };

        let extractor = |msg: &str, field: &str| {
            if field == "req_id" && msg.contains("req-abc-99") {
                Some("req-abc-99".to_string())
            } else {
                None
            }
        };

        let res =
            CorrelationEngine::correlate_record(10_000_000_000, &rule, &targets, 50, extractor);

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].source_alias, "App");
        let reason_str = format!("{}", res[0].match_reason);
        assert!(reason_str.contains("req-abc-99"));
        assert!(reason_str.contains("Time Delta: +12ms"));
    }
}
