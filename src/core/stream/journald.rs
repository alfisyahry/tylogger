#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournaldRecord {
    pub message: String,
    pub priority: u8,
    pub unit: Option<String>,
    pub pid: Option<u32>,
    pub realtime_timestamp_micros: Option<u64>,
    pub cursor: Option<String>,
}

impl JournaldRecord {
    pub fn format_display(&self) -> String {
        let ts_str = if let Some(ts) = self.realtime_timestamp_micros {
            let secs = ts / 1_000_000;
            let micros = ts % 1_000_000;
            format!("{}.{:06}", secs, micros)
        } else {
            "-".to_string()
        };

        let pri_str = match self.priority {
            0 => "EMERG",
            1 => "ALERT",
            2 => "CRIT",
            3 => "ERR",
            4 => "WARN",
            5 => "NOTICE",
            6 => "INFO",
            7 => "DEBUG",
            _ => "UNKNOWN",
        };

        let unit_str = self.unit.as_deref().unwrap_or("-");
        let pid_str = self
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());

        format!(
            "{} [{}] [{}] [PID:{}] {}",
            ts_str, pri_str, unit_str, pid_str, self.message
        )
    }
}

pub struct JournaldJsonParser;

impl JournaldJsonParser {
    pub fn parse_line(line: &str) -> Option<JournaldRecord> {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return None;
        }

        let mut message = String::new();
        let mut priority = 6; // Default INFO
        let mut unit = None;
        let mut pid = None;
        let mut timestamp = None;
        let mut cursor = None;

        // Simple JSON tokenizer extracting top-level keys
        let content = &trimmed[1..trimmed.len() - 1];
        for pair in content.split("\",") {
            let clean = pair.trim();
            if let Some((k, v)) = clean.split_once(':') {
                let key = k.trim().trim_matches('"');
                let val = v.trim().trim_matches('"');
                match key {
                    "MESSAGE" => message = val.to_string(),
                    "PRIORITY" => priority = val.parse().unwrap_or(6),
                    "_SYSTEMD_UNIT" => unit = Some(val.to_string()),
                    "_PID" => pid = val.parse().ok(),
                    "__REALTIME_TIMESTAMP" => timestamp = val.parse().ok(),
                    "__CURSOR" => cursor = Some(val.to_string()),
                    _ => {}
                }
            }
        }

        if message.is_empty() {
            message = trimmed.to_string();
        }

        Some(JournaldRecord {
            message,
            priority,
            unit,
            pid,
            realtime_timestamp_micros: timestamp,
            cursor,
        })
    }
}
