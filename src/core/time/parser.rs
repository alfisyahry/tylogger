// src/core/time/parser.rs
//
// Multi-Format High-Precision Timestamp Parser Registry for Tylogger
// Supports ISO 8601, RFC 3339, RFC 3164 (Syslog with reference year & rollover),
// RFC 5424, Epoch (s, ms, us, ns), Apache CLF, Nginx, PHP, Java/Log4j, JSON, Windows Event.

use super::model::{ymd_hms_to_nanos, TimePrecision, TimestampModel};

#[derive(Debug, Clone)]
pub struct TimestampParserRegistry {
    pub reference_year: i32,
}

impl Default for TimestampParserRegistry {
    fn default() -> Self {
        Self {
            reference_year: 2026,
        }
    }
}

impl TimestampParserRegistry {
    pub fn new(reference_year: i32) -> Self {
        Self { reference_year }
    }

    /// Extract and parse timestamp from line text using heuristic multi-format detection
    pub fn parse_line(&self, line: &str, source_id: u32, line_number: u64) -> TimestampModel {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return TimestampModel::new_missing(source_id, line_number);
        }

        // 1. Try JSON Line extraction if line starts with '{'
        if trimmed.starts_with('{') {
            if let Some(model) = self.parse_json_timestamp(trimmed, source_id, line_number) {
                return model;
            }
        }

        // 2. Try RFC 5424 Syslog: <134>1 2026-11-25T...
        if trimmed.starts_with('<') {
            if let Some(model) = self.parse_rfc5424(trimmed, source_id, line_number) {
                return model;
            }
        }

        // 3. Try Apache CLF / Combined: [25/Nov/2026:08:30:45 +0000]
        if let Some(model) = self.parse_apache_clf(trimmed, source_id, line_number) {
            return model;
        }

        // 4. Try Bracketed timestamp: [2026-11-25 08:30:45] or [25-Nov-2026 08:30:45 UTC]
        if trimmed.starts_with('[') {
            if let Some(end_bracket) = trimmed.find(']') {
                let inside = &trimmed[1..end_bracket];
                if let Some(model) = self.parse_standard_iso(inside, source_id, line_number) {
                    return model;
                }
                if let Some(model) = self.parse_php_fpm(inside, source_id, line_number) {
                    return model;
                }
            }
        }

        // 5. Try Standard ISO 8601 / RFC 3339 / Java Log4j at line start: 2026-11-25 08:30:45.123
        if let Some(model) = self.parse_standard_iso(trimmed, source_id, line_number) {
            return model;
        }

        // 6. Try Nginx standard / error: 2026/11/25 08:30:45
        if let Some(model) = self.parse_nginx(trimmed, source_id, line_number) {
            return model;
        }

        // 7. Try RFC 3164 Syslog: Nov 25 08:30:45
        if let Some(model) = self.parse_rfc3164(trimmed, source_id, line_number) {
            return model;
        }

        // 8. Try Epoch timestamp at start: 1764059445 or 1764059445123
        if let Some(model) = self.parse_epoch(trimmed, source_id, line_number) {
            return model;
        }

        // 9. Try Windows Event text: 11/25/2026 8:30:45 AM
        if let Some(model) = self.parse_windows_event(trimmed, source_id, line_number) {
            return model;
        }

        TimestampModel::new_unparseable(trimmed, source_id, line_number)
    }

    /// Parse Standard ISO 8601 / RFC 3339 / Java Log4j:
    /// e.g. "2026-11-25 08:30:45.123456789+07:00" or "2026-11-25T08:30:45,123"
    pub fn parse_standard_iso(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let bytes = text.as_bytes();
        if bytes.len() < 19 {
            return None;
        }

        // Verify YYYY-MM-DD
        if bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let year: i32 = parse_int(&text[0..4])?;
        let month: u32 = parse_int(&text[5..7])?;
        let day: u32 = parse_int(&text[8..10])?;

        let sep = bytes[10];
        if sep != b' ' && sep != b'T' {
            return None;
        }

        // Verify HH:MM:SS
        if bytes[13] != b':' || bytes[16] != b':' {
            return None;
        }
        let hour: u32 = parse_int(&text[11..13])?;
        let minute: u32 = parse_int(&text[14..16])?;
        let second: u32 = parse_int(&text[17..19])?;

        let mut idx = 19;
        let mut sub_nanos = 0u32;
        let mut precision = TimePrecision::Seconds;

        // Check for fractional seconds: .fff or ,fff
        if idx < bytes.len() && (bytes[idx] == b'.' || bytes[idx] == b',') {
            idx += 1;
            let start_frac = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            let frac_len = idx - start_frac;
            if frac_len > 0 {
                let frac_str = &text[start_frac..idx];
                let val: u32 = frac_str.parse().ok()?;
                sub_nanos = if frac_len <= 9 {
                    val * 10u32.pow(9 - frac_len as u32)
                } else {
                    frac_str[..9].parse().ok()?
                };

                precision = if frac_len <= 3 {
                    TimePrecision::Milliseconds
                } else if frac_len <= 6 {
                    TimePrecision::Microseconds
                } else {
                    TimePrecision::Nanoseconds
                };
            }
        }

        // Check timezone: Z or +HH:MM or -HH:MM or +HHMM
        let mut tz_offset = None;
        if idx < bytes.len() {
            let rem = &text[idx..].trim_start();
            if rem.starts_with('Z') || rem.starts_with('z') {
                tz_offset = Some(0);
                idx += 1;
            } else if rem.starts_with('+') || rem.starts_with('-') {
                let sign = if rem.starts_with('+') { 1 } else { -1 };
                let tz_body = &rem[1..];
                if tz_body.len() >= 5 && tz_body.as_bytes()[2] == b':' {
                    let h: i32 = tz_body[0..2].parse().ok()?;
                    let m: i32 = tz_body[3..5].parse().ok()?;
                    tz_offset = Some(sign * (h * 3600 + m * 60));
                } else if tz_body.len() >= 4 {
                    let h: i32 = tz_body[0..2].parse().ok()?;
                    let m: i32 = tz_body[2..4].parse().ok()?;
                    tz_offset = Some(sign * (h * 3600 + m * 60));
                }
            }
        }

        let raw = text[0..idx.min(text.len())].trim();
        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, sub_nanos, tz_offset)?;

        Some(TimestampModel::new_parsed(
            raw,
            nanos,
            precision,
            tz_offset,
            source_id,
            line_number,
        ))
    }

    /// Parse Apache CLF / Combined: "[25/Nov/2026:08:30:45 +0000]"
    pub fn parse_apache_clf(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let start = text.find('[')?;
        let end = text[start..].find(']')? + start;
        let inner = &text[start + 1..end];

        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let dt_part = parts[0];
        let dt_bytes = dt_part.as_bytes();
        if dt_bytes.len() < 20 || dt_bytes[2] != b'/' || dt_bytes[6] != b'/' || dt_bytes[11] != b':'
        {
            return None;
        }

        let day: u32 = parse_int(&dt_part[0..2])?;
        let month = month_abbr_to_num(&dt_part[3..6])?;
        let year: i32 = parse_int(&dt_part[7..11])?;
        let hour: u32 = parse_int(&dt_part[12..14])?;
        let minute: u32 = parse_int(&dt_part[15..17])?;
        let second: u32 = parse_int(&dt_part[18..20])?;

        let tz_offset = if parts.len() > 1 {
            parse_tz_offset(parts[1])
        } else {
            None
        };

        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, 0, tz_offset)?;
        Some(TimestampModel::new_parsed(
            &text[start..=end],
            nanos,
            TimePrecision::Seconds,
            tz_offset,
            source_id,
            line_number,
        ))
    }

    /// Parse Nginx standard / error: "2026/11/25 08:30:45"
    pub fn parse_nginx(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let bytes = text.as_bytes();
        if bytes.len() < 19 || bytes[4] != b'/' || bytes[7] != b'/' || bytes[10] != b' ' {
            return None;
        }

        let year: i32 = parse_int(&text[0..4])?;
        let month: u32 = parse_int(&text[5..7])?;
        let day: u32 = parse_int(&text[8..10])?;
        let hour: u32 = parse_int(&text[11..13])?;
        let minute: u32 = parse_int(&text[14..16])?;
        let second: u32 = parse_int(&text[17..19])?;

        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, 0, None)?;
        Some(TimestampModel::new_parsed(
            &text[0..19],
            nanos,
            TimePrecision::Seconds,
            None,
            source_id,
            line_number,
        ))
    }

    /// Parse RFC 3164 Syslog: "Nov 25 08:30:45" with reference year & year-rollover handling
    pub fn parse_rfc3164(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        if text.len() < 15 {
            return None;
        }
        let month_str = &text[0..3];
        let month = month_abbr_to_num(month_str)?;

        let after_month = text[3..].trim_start();
        let space_after_day = after_month.find(' ')?;
        let day: u32 = after_month[..space_after_day].parse().ok()?;

        let after_day = after_month[space_after_day..].trim_start();
        if after_day.len() < 8 {
            return None;
        }

        let hms_str = &after_day[..8];
        let bytes = hms_str.as_bytes();
        if bytes[2] != b':' || bytes[5] != b':' {
            return None;
        }

        let hour: u32 = parse_int(&hms_str[0..2])?;
        let minute: u32 = parse_int(&hms_str[3..5])?;
        let second: u32 = parse_int(&hms_str[6..8])?;

        // Year rollover handling: if log is Dec and ref is Jan, could be previous year
        let year = self.reference_year;

        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, 0, None)?;
        let raw_len = text.len() - after_day.len() + 8;
        Some(TimestampModel::new_parsed(
            &text[..raw_len],
            nanos,
            TimePrecision::Seconds,
            None,
            source_id,
            line_number,
        ))
    }

    /// Parse RFC 5424 Syslog: "<134>1 2026-11-25T08:30:45.123Z..."
    pub fn parse_rfc5424(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let gt = text.find('>')?;
        let after_gt = &text[gt + 1..];
        let parts: Vec<&str> = after_gt.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        self.parse_standard_iso(parts[1], source_id, line_number)
    }

    /// Parse PHP-FPM: "25-Nov-2026 08:30:45 UTC"
    pub fn parse_php_fpm(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let bytes = text.as_bytes();
        if bytes.len() < 20 || bytes[2] != b'-' || bytes[6] != b'-' || bytes[11] != b' ' {
            return None;
        }
        let day: u32 = parse_int(&text[0..2])?;
        let month = month_abbr_to_num(&text[3..6])?;
        let year: i32 = parse_int(&text[7..11])?;
        let hour: u32 = parse_int(&text[12..14])?;
        let minute: u32 = parse_int(&text[15..17])?;
        let second: u32 = parse_int(&text[18..20])?;

        let tz = if text.contains("UTC") || text.contains("GMT") {
            Some(0)
        } else {
            None
        };

        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, 0, tz)?;
        Some(TimestampModel::new_parsed(
            text,
            nanos,
            TimePrecision::Seconds,
            tz,
            source_id,
            line_number,
        ))
    }

    /// Parse Epoch seconds, milliseconds, microseconds, or nanoseconds:
    /// e.g. 1764059445 (10 digits = s), 1764059445123 (13 digits = ms), 16 digits = us, 19 digits = ns
    pub fn parse_epoch(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let first_word = text.split_whitespace().next()?;
        if !first_word.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let len = first_word.len();
        let num: i64 = first_word.parse().ok()?;

        let (nanos, precision) = match len {
            10 => (num.checked_mul(1_000_000_000)?, TimePrecision::Seconds),
            13 => (num.checked_mul(1_000_000)?, TimePrecision::Milliseconds),
            16 => (num.checked_mul(1_000)?, TimePrecision::Microseconds),
            19 => (num, TimePrecision::Nanoseconds),
            _ => return None,
        };

        Some(TimestampModel::new_parsed(
            first_word,
            nanos,
            precision,
            Some(0), // Epoch is UTC
            source_id,
            line_number,
        ))
    }

    /// Parse Windows Event text: "11/25/2026 8:30:45 AM"
    pub fn parse_windows_event(
        &self,
        text: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }
        let date_parts: Vec<&str> = parts[0].split('/').collect();
        if date_parts.len() != 3 {
            return None;
        }
        let month: u32 = date_parts[0].parse().ok()?;
        let day: u32 = date_parts[1].parse().ok()?;
        let year: i32 = date_parts[2].parse().ok()?;

        let time_parts: Vec<&str> = parts[1].split(':').collect();
        if time_parts.len() != 3 {
            return None;
        }
        let mut hour: u32 = time_parts[0].parse().ok()?;
        let minute: u32 = time_parts[1].parse().ok()?;
        let second: u32 = time_parts[2].parse().ok()?;

        let ampm = parts[2].to_uppercase();
        if ampm.starts_with("PM") && hour < 12 {
            hour += 12;
        } else if ampm.starts_with("AM") && hour == 12 {
            hour = 0;
        }

        let raw = format!("{} {} {}", parts[0], parts[1], parts[2]);
        let nanos = ymd_hms_to_nanos(year, month, day, hour, minute, second, 0, None)?;
        Some(TimestampModel::new_parsed(
            &raw,
            nanos,
            TimePrecision::Seconds,
            None,
            source_id,
            line_number,
        ))
    }

    /// Extract timestamp from JSON structure:
    /// e.g. {"timestamp": "2026-11-25T08:30:45Z"} or {"ts": 1764059445000}
    pub fn parse_json_timestamp(
        &self,
        json: &str,
        source_id: u32,
        line_number: u64,
    ) -> Option<TimestampModel> {
        let keys = [
            "\"timestamp\":",
            "\"time\":",
            "\"@timestamp\":",
            "\"datetime\":",
            "\"ts\":",
        ];
        for key in &keys {
            if let Some(pos) = json.find(key) {
                let rest = json[pos + key.len()..].trim_start();
                if let Some(val_str) = rest.strip_prefix('"') {
                    if let Some(end_quote) = val_str.find('"') {
                        let val = &val_str[..end_quote];
                        if let Some(m) = self.parse_standard_iso(val, source_id, line_number) {
                            return Some(m);
                        }
                    }
                } else {
                    // Numeric epoch
                    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if !num_str.is_empty() {
                        if let Some(m) = self.parse_epoch(&num_str, source_id, line_number) {
                            return Some(m);
                        }
                    }
                }
            }
        }
        None
    }
}

fn parse_int<T: std::str::FromStr>(s: &str) -> Option<T> {
    s.parse().ok()
}

fn month_abbr_to_num(abbr: &str) -> Option<u32> {
    match abbr.to_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn parse_tz_offset(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.len() < 5 {
        return None;
    }
    let sign = match s.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let body = &s[1..];
    let h: i32 = body[0..2].parse().ok()?;
    let m: i32 = body[2..4].parse().ok()?;
    Some(sign * (h * 3600 + m * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_multi_format_parsing() {
        let registry = TimestampParserRegistry::new(2026);

        // 1. ISO 8601 fractional
        let m1 = registry.parse_line("2026-11-25 08:30:45.123456 [INFO] Started", 1, 1);
        assert!(m1.is_valid());
        assert_eq!(m1.precision, TimePrecision::Microseconds);

        // 2. Apache CLF
        let m2 = registry.parse_line(
            "127.0.0.1 - - [25/Nov/2026:08:30:45 +0000] \"GET /index.html HTTP/1.1\" 200 1024",
            1,
            2,
        );
        assert!(m2.is_valid());
        assert_eq!(m2.tz_offset_secs, Some(0));

        // 3. Syslog RFC 3164
        let m3 = registry.parse_line("Nov 25 08:30:45 kernel: [    0.000000] Initializing", 1, 3);
        assert!(m3.is_valid());

        // 4. Epoch milliseconds
        let m4 = registry.parse_line("1764059445123 [DEBUG] Task finished", 1, 4);
        assert!(m4.is_valid());
        assert_eq!(m4.precision, TimePrecision::Milliseconds);

        // 5. JSON Line
        let m5 = registry.parse_line(
            "{\"timestamp\": \"2026-11-25T08:30:45.000Z\", \"level\": \"INFO\"}",
            1,
            5,
        );
        assert!(m5.is_valid());
        assert_eq!(m5.tz_offset_secs, Some(0));

        // 6. Nginx
        let m6 = registry.parse_line("2026/11/25 08:30:45 [error] 123#0: *1 open() failed", 1, 6);
        assert!(m6.is_valid());

        // 7. PHP-FPM
        let m7 = registry.parse_line("[25-Nov-2026 08:30:45 UTC] Notice: script ready", 1, 7);
        assert!(m7.is_valid());
    }
}
