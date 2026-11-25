// src/core/time/model.rs
//
// Unified High-Precision Timestamp Model for Tylogger (LogExpert Rust Reimplementation)
// Strict integer checked arithmetic, nanosecond precision, zero float drift.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimePrecision {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl fmt::Display for TimePrecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seconds => write!(f, "s"),
            Self::Milliseconds => write!(f, "ms"),
            Self::Microseconds => write!(f, "us"),
            Self::Nanoseconds => write!(f, "ns"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseStatus {
    Parsed,
    Unparseable,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampModel {
    pub raw_text: String,
    pub epoch_nanos: i64,
    pub precision: TimePrecision,
    pub tz_offset_secs: Option<i32>,
    pub parse_status: ParseStatus,
    pub timeshift_offset_nanos: i64,
    pub source_id: u32,
    pub line_number: u64,
}

impl TimestampModel {
    pub fn new_missing(source_id: u32, line_number: u64) -> Self {
        Self {
            raw_text: String::new(),
            epoch_nanos: 0,
            precision: TimePrecision::Seconds,
            tz_offset_secs: None,
            parse_status: ParseStatus::Missing,
            timeshift_offset_nanos: 0,
            source_id,
            line_number,
        }
    }

    pub fn new_unparseable(raw: &str, source_id: u32, line_number: u64) -> Self {
        Self {
            raw_text: raw.to_string(),
            epoch_nanos: 0,
            precision: TimePrecision::Seconds,
            tz_offset_secs: None,
            parse_status: ParseStatus::Unparseable,
            timeshift_offset_nanos: 0,
            source_id,
            line_number,
        }
    }

    pub fn new_parsed(
        raw: &str,
        epoch_nanos: i64,
        precision: TimePrecision,
        tz_offset_secs: Option<i32>,
        source_id: u32,
        line_number: u64,
    ) -> Self {
        Self {
            raw_text: raw.to_string(),
            epoch_nanos,
            precision,
            tz_offset_secs,
            parse_status: ParseStatus::Parsed,
            timeshift_offset_nanos: 0,
            source_id,
            line_number,
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.parse_status == ParseStatus::Parsed
    }

    #[inline]
    pub fn effective_nanos(&self) -> i64 {
        self.epoch_nanos.saturating_add(self.timeshift_offset_nanos)
    }

    pub fn set_timeshift_nanos(&mut self, offset_nanos: i64) {
        self.timeshift_offset_nanos = offset_nanos;
    }

    /// Format to ISO 8601 string representation: YYYY-MM-DDTHH:MM:SS.fff
    pub fn format_iso(&self, use_adjusted: bool) -> String {
        if !self.is_valid() {
            return if self.raw_text.is_empty() {
                "-".to_string()
            } else {
                self.raw_text.clone()
            };
        }

        let nanos = if use_adjusted {
            self.effective_nanos()
        } else {
            self.epoch_nanos
        };

        nanos_to_iso_string(nanos, self.precision, self.tz_offset_secs)
    }
}

/// Convert Gregorian components to Unix epoch nanoseconds (checked integer math)
pub fn ymd_hms_to_nanos(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
    tz_offset_secs: Option<i32>,
) -> Option<i64> {
    if month < 1
        || month > 12
        || day < 1
        || day > 31
        || hour > 23
        || minute > 59
        || second > 60
        || nanos >= 1_000_000_000
    {
        return None;
    }

    // Days since Unix Epoch (1970-01-01) using integer arithmetic
    let days = gregorian_to_days(year, month, day)?;
    let secs_in_day = (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64);
    let total_secs = days.checked_mul(86400)?.checked_add(secs_in_day)?;

    // Adjust for explicit timezone offset
    let adjusted_secs = match tz_offset_secs {
        Some(offset) => total_secs.checked_sub(offset as i64)?,
        None => total_secs,
    };

    let base_nanos = adjusted_secs.checked_mul(1_000_000_000)?;
    base_nanos.checked_add(nanos as i64)
}

/// Convert Unix epoch nanoseconds to ISO 8601 string
pub fn nanos_to_iso_string(nanos: i64, precision: TimePrecision, tz_offset: Option<i32>) -> String {
    let (secs, sub_nanos) = if nanos >= 0 {
        (nanos / 1_000_000_000, (nanos % 1_000_000_000) as u32)
    } else {
        let s = (nanos - 999_999_999) / 1_000_000_000;
        let rem = (nanos - s * 1_000_000_000) as u32;
        (s, rem)
    };

    let (year, month, day, hour, minute, second) = days_to_gregorian(secs);

    let frac_part = match precision {
        TimePrecision::Seconds => String::new(),
        TimePrecision::Milliseconds => format!(".{:03}", sub_nanos / 1_000_000),
        TimePrecision::Microseconds => format!(".{:06}", sub_nanos / 1_000),
        TimePrecision::Nanoseconds => format!(".{:09}", sub_nanos),
    };

    let tz_part = match tz_offset {
        Some(0) => "Z".to_string(),
        Some(off) => {
            let sign = if off >= 0 { '+' } else { '-' };
            let abs_off = off.abs();
            format!("{}{:02}:{:02}", sign, abs_off / 3600, (abs_off % 3600) / 60)
        }
        None => String::new(),
    };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}{}{}",
        year, month, day, hour, minute, second, frac_part, tz_part
    )
}

fn gregorian_to_days(y: i32, m: u32, d: u32) -> Option<i64> {
    let mut y = y as i64;
    let mut m = m as i64;
    if m <= 2 {
        y -= 1;
        m += 12;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + (d as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days)
}

pub fn days_to_gregorian(total_secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = if total_secs >= 0 {
        total_secs / 86400
    } else {
        (total_secs - 86399) / 86400
    };
    let rem_secs = (total_secs - days * 86400) as u32;

    let hour = rem_secs / 3600;
    let minute = (rem_secs % 3600) / 60;
    let second = rem_secs % 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }

    (y as i32, m, d, hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_model_conversion_roundtrip() {
        let nanos = ymd_hms_to_nanos(2026, 11, 25, 8, 30, 45, 123_456_789, Some(0)).unwrap();
        let formatted = nanos_to_iso_string(nanos, TimePrecision::Nanoseconds, Some(0));
        assert_eq!(formatted, "2026-11-25 08:30:45.123456789Z");
    }

    #[test]
    fn test_timeshift_effective_nanos() {
        let mut model = TimestampModel::new_parsed(
            "2026-11-25 08:00:00",
            1_000_000_000,
            TimePrecision::Seconds,
            None,
            1,
            10,
        );
        assert_eq!(model.effective_nanos(), 1_000_000_000);
        model.set_timeshift_nanos(500_000_000);
        assert_eq!(model.effective_nanos(), 1_500_000_000);
    }
}
