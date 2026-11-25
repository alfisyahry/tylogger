// src/core/time/locator.rs
//
// Timestamp Locator & Time-Window Navigation for Tylogger
// Reusable time locator supporting Exact, Nearest, Preceding, Following, and Range queries.

use super::model::TimestampModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Exact,
    Nearest,
    Previous,
    Next,
}

#[derive(Debug, Clone)]
pub struct LocatorResult {
    pub line_number: u64,
    pub actual_nanos: i64,
    pub delta_nanos: i64,
    pub is_exact: bool,
}

#[derive(Debug, Clone)]
pub struct TimestampLocator;

impl TimestampLocator {
    /// Find a record matching target_nanos in a slice of TimestampModel items
    pub fn locate(
        records: &[TimestampModel],
        target_nanos: i64,
        mode: MatchMode,
        use_adjusted: bool,
    ) -> Option<LocatorResult> {
        let valid_records: Vec<(usize, i64)> = records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_valid())
            .map(|(i, r)| {
                let nanos = if use_adjusted {
                    r.effective_nanos()
                } else {
                    r.epoch_nanos
                };
                (i, nanos)
            })
            .collect();

        if valid_records.is_empty() {
            return None;
        }

        match mode {
            MatchMode::Exact => {
                for &(idx, nanos) in &valid_records {
                    if nanos == target_nanos {
                        return Some(LocatorResult {
                            line_number: records[idx].line_number,
                            actual_nanos: nanos,
                            delta_nanos: 0,
                            is_exact: true,
                        });
                    }
                }
                None
            }
            MatchMode::Nearest => {
                let mut best_idx = valid_records[0].0;
                let mut best_nanos = valid_records[0].1;
                let mut min_diff = (best_nanos - target_nanos).abs();

                for &(idx, nanos) in &valid_records[1..] {
                    let diff = (nanos - target_nanos).abs();
                    if diff < min_diff {
                        min_diff = diff;
                        best_idx = idx;
                        best_nanos = nanos;
                    }
                }

                Some(LocatorResult {
                    line_number: records[best_idx].line_number,
                    actual_nanos: best_nanos,
                    delta_nanos: best_nanos - target_nanos,
                    is_exact: min_diff == 0,
                })
            }
            MatchMode::Previous => {
                let mut candidate = None;
                for &(idx, nanos) in &valid_records {
                    if nanos <= target_nanos {
                        match candidate {
                            None => candidate = Some((idx, nanos)),
                            Some((_, cnanos)) if nanos >= cnanos => candidate = Some((idx, nanos)),
                            _ => {}
                        }
                    }
                }
                candidate.map(|(idx, nanos)| LocatorResult {
                    line_number: records[idx].line_number,
                    actual_nanos: nanos,
                    delta_nanos: nanos - target_nanos,
                    is_exact: nanos == target_nanos,
                })
            }
            MatchMode::Next => {
                let mut candidate = None;
                for &(idx, nanos) in &valid_records {
                    if nanos >= target_nanos {
                        match candidate {
                            None => candidate = Some((idx, nanos)),
                            Some((_, cnanos)) if nanos <= cnanos => candidate = Some((idx, nanos)),
                            _ => {}
                        }
                    }
                }
                candidate.map(|(idx, nanos)| LocatorResult {
                    line_number: records[idx].line_number,
                    actual_nanos: nanos,
                    delta_nanos: nanos - target_nanos,
                    is_exact: nanos == target_nanos,
                })
            }
        }
    }

    /// Find boundary line numbers (first_line, last_line) within [start_nanos, end_nanos]
    pub fn find_range(
        records: &[TimestampModel],
        start_nanos: Option<i64>,
        end_nanos: Option<i64>,
        use_adjusted: bool,
    ) -> (Option<u64>, Option<u64>) {
        let mut first_line = None;
        let mut last_line = None;

        for r in records {
            if !r.is_valid() {
                continue;
            }
            let nanos = if use_adjusted {
                r.effective_nanos()
            } else {
                r.epoch_nanos
            };

            let matches_start = match start_nanos {
                Some(s) => nanos >= s,
                None => true,
            };
            let matches_end = match end_nanos {
                Some(e) => nanos <= e,
                None => true,
            };

            if matches_start && matches_end {
                if first_line.is_none() {
                    first_line = Some(r.line_number);
                }
                last_line = Some(r.line_number);
            }
        }

        (first_line, last_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::time::model::TimePrecision;

    #[test]
    fn test_locator_modes() {
        let records = vec![
            TimestampModel::new_parsed("t1", 1000, TimePrecision::Seconds, None, 1, 0),
            TimestampModel::new_parsed("t2", 2000, TimePrecision::Seconds, None, 1, 1),
            TimestampModel::new_parsed("t3", 3000, TimePrecision::Seconds, None, 1, 2),
            TimestampModel::new_parsed("t4", 4000, TimePrecision::Seconds, None, 1, 3),
        ];

        let exact = TimestampLocator::locate(&records, 2000, MatchMode::Exact, false).unwrap();
        assert_eq!(exact.line_number, 1);
        assert!(exact.is_exact);

        let nearest = TimestampLocator::locate(&records, 2400, MatchMode::Nearest, false).unwrap();
        assert_eq!(nearest.line_number, 1);
        assert_eq!(nearest.actual_nanos, 2000);
        assert_eq!(nearest.delta_nanos, -400);

        let prev = TimestampLocator::locate(&records, 2400, MatchMode::Previous, false).unwrap();
        assert_eq!(prev.line_number, 1);

        let next = TimestampLocator::locate(&records, 2400, MatchMode::Next, false).unwrap();
        assert_eq!(next.line_number, 2);
    }
}
