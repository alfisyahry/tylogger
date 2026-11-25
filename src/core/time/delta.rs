// src/core/time/delta.rs
//
// Delta Time Analysis & Configurable Display Formats for Tylogger
// Supports Delta from Previous, Reference, Bookmark, and Custom pair with nanosecond precision.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
    HumanDuration,
}

#[derive(Debug, Clone)]
pub struct DeltaCalculator;

impl DeltaCalculator {
    /// Calculate signed delta in nanoseconds: to_nanos - from_nanos
    #[inline]
    pub fn calculate(from_nanos: i64, to_nanos: i64) -> i64 {
        to_nanos.saturating_sub(from_nanos)
    }

    /// Format delta nanoseconds into user-configured display unit
    pub fn format(delta_nanos: i64, unit: DeltaUnit) -> String {
        let sign = if delta_nanos >= 0 { "+" } else { "-" };
        let abs_nanos = delta_nanos.abs();

        match unit {
            DeltaUnit::Nanoseconds => format!("{}{}", sign, abs_nanos),
            DeltaUnit::Microseconds => {
                let us = abs_nanos as f64 / 1_000.0;
                format!("{}{:.3}us", sign, us)
            }
            DeltaUnit::Milliseconds => {
                let ms = abs_nanos as f64 / 1_000_000.0;
                format!("{}{:.3}ms", sign, ms)
            }
            DeltaUnit::Seconds => {
                let s = abs_nanos as f64 / 1_000_000_000.0;
                format!("{}{:.3}s", sign, s)
            }
            DeltaUnit::HumanDuration => {
                let total_secs = abs_nanos / 1_000_000_000;
                let rem_nanos = abs_nanos % 1_000_000_000;
                let rem_ms = rem_nanos / 1_000_000;
                let rem_us = (rem_nanos % 1_000_000) / 1_000;

                let hours = total_secs / 3600;
                let mins = (total_secs % 3600) / 60;
                let secs = total_secs % 60;

                if hours > 0 {
                    format!("{}{}h {}m {}s", sign, hours, mins, secs)
                } else if mins > 0 {
                    format!("{}{}m {}s {}ms", sign, mins, secs, rem_ms)
                } else if secs > 0 {
                    format!("{}{}s {}ms", sign, secs, rem_ms)
                } else if rem_ms > 0 {
                    format!("{}{}ms {}us", sign, rem_ms, rem_us)
                } else {
                    format!("{}{}us", sign, rem_us)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_calculations_and_formatting() {
        let d1 = DeltaCalculator::calculate(1_000_000_000, 1_500_000_000);
        assert_eq!(d1, 500_000_000);
        let s_ms = DeltaCalculator::format(d1, DeltaUnit::Milliseconds);
        assert_eq!(s_ms, "+500.000ms");

        // Negative delta (out of order event)
        let d2 = DeltaCalculator::calculate(2_000_000_000, 1_900_000_000);
        assert_eq!(d2, -100_000_000);
        let s_neg = DeltaCalculator::format(d2, DeltaUnit::HumanDuration);
        assert_eq!(s_neg, "-100ms 0us");

        // 1m 23s 450ms
        let big_delta = (83 * 1_000_000_000) + (450 * 1_000_000);
        let s_human = DeltaCalculator::format(big_delta, DeltaUnit::HumanDuration);
        assert_eq!(s_human, "+1m 23s 450ms");
    }
}
