use std::collections::HashMap;

/// Trait representing a modular log columnizer / parser.
pub trait Columnizer: Send + Sync {
    /// Stable internal identifier (e.g. "plain", "clf", "csv", "jsonl").
    fn id(&self) -> &'static str;

    /// User-friendly display name.
    fn name(&self) -> &'static str;

    /// List of column names defined by this parser.
    fn column_names(&self) -> Vec<String>;

    /// Parses a single raw log line into a list of column values.
    fn parse_line(&self, line: &str) -> Vec<String>;

    /// Extracts timestamp string if present, for time sorting or filtering.
    fn extract_timestamp(&self, line: &str) -> Option<String> {
        let cols = self.parse_line(line);
        cols.first().cloned()
    }

    /// Evaluates sample lines and returns confidence level (0.0 to 1.0).
    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        if sample_lines.is_empty() {
            return 0.0;
        }
        let mut matches = 0;
        for line in sample_lines {
            let cols = self.parse_line(line);
            if cols.len() > 1 && cols.iter().any(|c| !c.is_empty()) {
                matches += 1;
            }
        }
        matches as f32 / sample_lines.len() as f32
    }
}

// ============================================================================
// Upstream Built-in Columnizers
// ============================================================================

/// 1. Plain Text / Default Columnizer
#[derive(Default)]
pub struct PlainTextColumnizer;

impl Columnizer for PlainTextColumnizer {
    fn id(&self) -> &'static str {
        "plain"
    }

    fn name(&self) -> &'static str {
        "Plain Text / Default"
    }

    fn column_names(&self) -> Vec<String> {
        vec!["Message".to_string()]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        vec![line.to_string()]
    }

    fn detect_confidence(&self, _sample_lines: &[&str]) -> f32 {
        0.1 // Lowest fallback confidence
    }
}

/// 2. Timestamp Columnizer (Timestamp + Message)
#[derive(Default)]
pub struct TimestampColumnizer;

impl Columnizer for TimestampColumnizer {
    fn id(&self) -> &'static str {
        "timestamp"
    }

    fn name(&self) -> &'static str {
        "Timestamp Columnizer"
    }

    fn column_names(&self) -> Vec<String> {
        vec!["Timestamp".to_string(), "Message".to_string()]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let trimmed = line.trim();
        // Check for ISO (2026-11-25 10:00:00) or bracketed [2026-11-25 ...] or EU (25.11.2026 ...)
        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some((ts, msg)) = rest.split_once(']') {
                return vec![ts.to_string(), msg.trim_start().to_string()];
            }
        }

        // Try standard space split for ISO date (YYYY-MM-DD HH:MM:SS.mmm)
        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let p0 = parts[0];
            let p1 = parts[1];
            // Check if p0 looks like a date (contains '-' or '/' or '.')
            if (p0.contains('-') || p0.contains('/') || p0.contains('.')) && p1.contains(':') {
                let ts = format!("{} {}", p0, p1);
                let msg = if parts.len() == 3 { parts[2] } else { "" };
                return vec![ts, msg.to_string()];
            }
        }

        vec!["".to_string(), line.to_string()]
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let res = self.parse_line(line);
            if !res[0].is_empty() {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

/// 3. Webserver Common Log Format (CLF)
///
/// Format: 127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
#[derive(Default)]
pub struct WebserverClfColumnizer;

impl Columnizer for WebserverClfColumnizer {
    fn id(&self) -> &'static str {
        "clf"
    }

    fn name(&self) -> &'static str {
        "Webserver Common Log Format (CLF)"
    }

    fn column_names(&self) -> Vec<String> {
        vec![
            "Host".to_string(),
            "Ident".to_string(),
            "User".to_string(),
            "Timestamp".to_string(),
            "Method".to_string(),
            "Path".to_string(),
            "Protocol".to_string(),
            "Status".to_string(),
            "Bytes".to_string(),
        ]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut cols = vec![String::new(); 9];
        let trimmed = line.trim();

        // 1. Host
        let Some((host, rest)) = trimmed.split_once(' ') else {
            cols[0] = trimmed.to_string();
            return cols;
        };
        cols[0] = host.to_string();

        // 2. Ident
        let Some((ident, rest)) = rest.split_once(' ') else {
            return cols;
        };
        cols[1] = ident.to_string();

        // 3. User
        let Some((user, rest)) = rest.split_once(' ') else {
            return cols;
        };
        cols[2] = user.to_string();

        // 4. Timestamp in brackets [...]
        let Some(rest_after_bracket) = rest.strip_prefix('[') else {
            return cols;
        };
        let Some((ts, rest)) = rest_after_bracket.split_once(']') else {
            return cols;
        };
        cols[3] = ts.to_string();

        // 5. Request in quotes "METHOD PATH PROTOCOL"
        let rest = rest.trim_start();
        let Some(rest_after_quote) = rest.strip_prefix('"') else {
            return cols;
        };
        let Some((req, rest)) = rest_after_quote.split_once('"') else {
            return cols;
        };

        let req_parts: Vec<&str> = req.splitn(3, ' ').collect();
        if !req_parts.is_empty() {
            cols[4] = req_parts[0].to_string();
        }
        if req_parts.len() > 1 {
            cols[5] = req_parts[1].to_string();
        }
        if req_parts.len() > 2 {
            cols[6] = req_parts[2].to_string();
        }

        // 6. Status and Bytes
        let rest_parts: Vec<&str> = rest.split_whitespace().collect();
        if !rest_parts.is_empty() {
            cols[7] = rest_parts[0].to_string();
        }
        if rest_parts.len() > 1 {
            cols[8] = rest_parts[1].to_string();
        }

        cols
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                continue;
            }
            let cols = self.parse_line(line);
            if !cols[0].is_empty()
                && !cols[3].is_empty()
                && !cols[4].is_empty()
                && !cols[7].is_empty()
            {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

/// 4. CSV Columnizer (RFC 4180 with quotes, escapes)
pub struct CsvColumnizer {
    pub delimiter: char,
}

impl Default for CsvColumnizer {
    fn default() -> Self {
        Self { delimiter: ',' }
    }
}

impl Columnizer for CsvColumnizer {
    fn id(&self) -> &'static str {
        "csv"
    }

    fn name(&self) -> &'static str {
        "CSV Columnizer (RFC 4180)"
    }

    fn column_names(&self) -> Vec<String> {
        (1..=10).map(|i| format!("Col{}", i)).collect()
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '"' {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Escaped quote: "" -> "
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
            } else if ch == self.delimiter && !in_quotes {
                fields.push(current);
                current = String::new();
            } else {
                current.push(ch);
            }
        }
        fields.push(current);
        fields
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let trimmed = line.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('<') {
                continue;
            }
            if line.contains(self.delimiter) {
                let cols = self.parse_line(line);
                if cols.len() >= 3 {
                    hits += 1;
                }
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

/// 5. Log4j XML Columnizer (parses XML event attributes safely without DTD/XXE)
#[derive(Default)]
pub struct Log4jXmlColumnizer;

impl Columnizer for Log4jXmlColumnizer {
    fn id(&self) -> &'static str {
        "log4j_xml"
    }

    fn name(&self) -> &'static str {
        "Log4j XML Columnizer"
    }

    fn column_names(&self) -> Vec<String> {
        vec![
            "Timestamp".to_string(),
            "Level".to_string(),
            "Logger".to_string(),
            "Thread".to_string(),
            "Message".to_string(),
            "Location".to_string(),
        ]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut cols = vec![String::new(); 6];
        // Parse attributes from <log4j:event ...>
        if let Some(pos) = line.find("timestamp=\"") {
            let slice = &line[pos + 11..];
            if let Some(end) = slice.find('"') {
                cols[0] = slice[..end].to_string();
            }
        }
        if let Some(pos) = line.find("level=\"") {
            let slice = &line[pos + 7..];
            if let Some(end) = slice.find('"') {
                cols[1] = slice[..end].to_string();
            }
        }
        if let Some(pos) = line.find("logger=\"") {
            let slice = &line[pos + 8..];
            if let Some(end) = slice.find('"') {
                cols[2] = slice[..end].to_string();
            }
        }
        if let Some(pos) = line.find("thread=\"") {
            let slice = &line[pos + 8..];
            if let Some(end) = slice.find('"') {
                cols[3] = slice[..end].to_string();
            }
        }

        // Message inside <log4j:message> or <![CDATA[...]]>
        if let Some(msg_start) = line.find("<log4j:message>") {
            let slice = &line[msg_start + 15..];
            if let Some(msg_end) = slice.find("</log4j:message>") {
                let msg_content = &slice[..msg_end];
                let unwrap_cdata = if let Some(cdata_start) = msg_content.find("<![CDATA[") {
                    let cslice = &msg_content[cdata_start + 9..];
                    cslice.strip_suffix("]]>").unwrap_or(cslice)
                } else {
                    msg_content
                };
                cols[4] = unwrap_cdata.trim().to_string();
            }
        } else if cols[0].is_empty() && cols[1].is_empty() {
            // Raw text fallback
            cols[4] = line.to_string();
        }

        cols
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            if line.contains("<log4j:event") || line.contains("log4j:message") {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

// ============================================================================
// Extended Tylogger Columnizers
// ============================================================================

/// 6. Apache Combined Log Format (CLF + Referer + User-Agent)
#[derive(Default)]
pub struct ApacheCombinedColumnizer;

impl Columnizer for ApacheCombinedColumnizer {
    fn id(&self) -> &'static str {
        "apache_combined"
    }

    fn name(&self) -> &'static str {
        "Apache Combined Log Format"
    }

    fn column_names(&self) -> Vec<String> {
        vec![
            "Host".to_string(),
            "Ident".to_string(),
            "User".to_string(),
            "Timestamp".to_string(),
            "Method".to_string(),
            "Path".to_string(),
            "Protocol".to_string(),
            "Status".to_string(),
            "Bytes".to_string(),
            "Referer".to_string(),
            "User-Agent".to_string(),
        ]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut base_cols = WebserverClfColumnizer.parse_line(line);
        while base_cols.len() < 11 {
            base_cols.push(String::new());
        }

        // Look for trailing "Referer" "User-Agent"
        let mut quotes = Vec::new();
        let mut in_quote = false;
        let mut start = 0;

        for (i, ch) in line.char_indices() {
            if ch == '"' {
                if in_quote {
                    quotes.push(&line[start..i]);
                    in_quote = false;
                } else {
                    start = i + 1;
                    in_quote = true;
                }
            }
        }

        if quotes.len() >= 3 {
            base_cols[9] = quotes[1].to_string(); // Referer
            base_cols[10] = quotes[2].to_string(); // User-Agent
        }

        base_cols
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                continue;
            }
            let cols = self.parse_line(line);
            if !cols[0].is_empty()
                && !cols[3].is_empty()
                && !cols[9].is_empty()
                && !cols[10].is_empty()
            {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

/// 7. Nginx Columnizer (Nginx Common/Combined format)
#[derive(Default)]
pub struct NginxColumnizer;

impl Columnizer for NginxColumnizer {
    fn id(&self) -> &'static str {
        "nginx"
    }

    fn name(&self) -> &'static str {
        "Nginx Access Log Format"
    }

    fn column_names(&self) -> Vec<String> {
        ApacheCombinedColumnizer.column_names()
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        ApacheCombinedColumnizer.parse_line(line)
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        ApacheCombinedColumnizer.detect_confidence(sample_lines) * 0.95
    }
}

/// 8. JSON Lines / NDJSON Columnizer
#[derive(Default)]
pub struct JsonLinesColumnizer;

impl Columnizer for JsonLinesColumnizer {
    fn id(&self) -> &'static str {
        "jsonl"
    }

    fn name(&self) -> &'static str {
        "JSON Lines / NDJSON"
    }

    fn column_names(&self) -> Vec<String> {
        vec![
            "Timestamp".to_string(),
            "Level".to_string(),
            "Logger".to_string(),
            "Message".to_string(),
            "Data".to_string(),
        ]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut cols = vec![String::new(); 5];
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            cols[3] = line.to_string();
            return cols;
        }

        // Lightweight resilient field extractor without heavy external dependencies
        cols[0] = extract_json_field(trimmed, &["time", "timestamp", "ts", "@timestamp"])
            .unwrap_or_default();
        cols[1] = extract_json_field(trimmed, &["level", "severity", "lvl"]).unwrap_or_default();
        cols[2] = extract_json_field(trimmed, &["logger", "module", "name", "caller"])
            .unwrap_or_default();
        cols[3] = extract_json_field(trimmed, &["msg", "message", "log", "text"])
            .unwrap_or_else(|| trimmed.to_string());
        cols[4] = trimmed.to_string(); // Raw payload

        cols
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.contains(':') {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

fn extract_json_field(json: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        let search_pattern = format!("\"{}\":", key);
        if let Some(pos) = json.find(&search_pattern) {
            let after = json[pos + search_pattern.len()..].trim_start();
            if let Some(stripped) = after.strip_prefix('"') {
                // String value
                let mut val = String::new();
                let mut escape = false;
                for ch in stripped.chars() {
                    if escape {
                        val.push(ch);
                        escape = false;
                    } else if ch == '\\' {
                        escape = true;
                    } else if ch == '"' {
                        break;
                    } else {
                        val.push(ch);
                    }
                }
                return Some(val);
            } else {
                // Non-string value (number, bool, object)
                let end = after.find([',', '}', ']', '\n']).unwrap_or(after.len());
                let val = after[..end].trim();
                return Some(val.to_string());
            }
        }
    }
    None
}

/// 9. TSV Columnizer (Tab-Separated Values)
#[derive(Default)]
pub struct TsvColumnizer;

impl Columnizer for TsvColumnizer {
    fn id(&self) -> &'static str {
        "tsv"
    }

    fn name(&self) -> &'static str {
        "TSV Columnizer"
    }

    fn column_names(&self) -> Vec<String> {
        (1..=10).map(|i| format!("Col{}", i)).collect()
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        line.split('\t').map(|s| s.to_string()).collect()
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            if line.contains('\t') {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

/// 10. Syslog Columnizer (RFC 3164 and RFC 5424)
#[derive(Default)]
pub struct SyslogColumnizer;

impl Columnizer for SyslogColumnizer {
    fn id(&self) -> &'static str {
        "syslog"
    }

    fn name(&self) -> &'static str {
        "Syslog Columnizer (RFC 3164/5424)"
    }

    fn column_names(&self) -> Vec<String> {
        vec![
            "Pri".to_string(),
            "Timestamp".to_string(),
            "Hostname".to_string(),
            "App".to_string(),
            "Pid".to_string(),
            "Message".to_string(),
        ]
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut cols = vec![String::new(); 6];
        let trimmed = line.trim();

        let mut rest = trimmed;
        // Priority <PRI>
        if let Some(stripped) = rest.strip_prefix('<') {
            if let Some((pri, r)) = stripped.split_once('>') {
                cols[0] = pri.to_string();
                rest = r.trim_start();
            }
        }

        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            return cols;
        }

        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];

        if tokens.len() >= 4 && MONTHS.contains(&tokens[0]) {
            // RFC 3164 format: Mmm dd hh:mm:ss hostname app[pid]: msg
            cols[1] = format!("{} {} {}", tokens[0], tokens[1], tokens[2]);
            cols[2] = tokens[3].to_string();
            if tokens.len() > 4 {
                let app_pid = tokens[4];
                if let Some((app, pid_bracket)) = app_pid.split_once('[') {
                    cols[3] = app.to_string();
                    cols[4] = pid_bracket.trim_end_matches([']', ':']).to_string();
                } else {
                    cols[3] = app_pid.trim_end_matches(':').to_string();
                }
            }
            if tokens.len() > 5 {
                cols[5] = tokens[5..].join(" ");
            }
        } else if tokens.len() >= 3 && tokens[0].chars().all(|c| c.is_ascii_digit()) {
            // RFC 5424 format: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID ...
            if tokens.len() > 1 {
                cols[1] = tokens[1].to_string();
            }
            if tokens.len() > 2 {
                cols[2] = tokens[2].to_string();
            }
            if tokens.len() > 3 {
                cols[3] = tokens[3].to_string();
            }
            if tokens.len() > 4 {
                cols[4] = tokens[4].to_string();
            }
            if tokens.len() > 5 {
                cols[5] = tokens[5..].join(" ");
            }
        } else {
            // Fallback generic split
            if !tokens.is_empty() {
                cols[1] = tokens[0].to_string();
            }
            if tokens.len() > 1 {
                cols[2] = tokens[1].to_string();
            }
            if tokens.len() > 2 {
                cols[3] = tokens[2].to_string();
            }
            if tokens.len() > 3 {
                cols[5] = tokens[3..].join(" ");
            }
        }

        cols
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        let mut hits = 0;
        for line in sample_lines {
            let trimmed = line.trim();
            if trimmed.starts_with('<') && trimmed.contains('>') {
                hits += 1;
            }
        }
        if sample_lines.is_empty() {
            0.0
        } else {
            hits as f32 / sample_lines.len() as f32
        }
    }
}

// ============================================================================
// Columnizer Registry & Format Auto-Detection
// ============================================================================

pub struct ColumnizerRegistry {
    columnizers: HashMap<&'static str, Box<dyn Columnizer>>,
}

impl Default for ColumnizerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnizerRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            columnizers: HashMap::new(),
        };
        reg.register(Box::new(PlainTextColumnizer));
        reg.register(Box::new(TimestampColumnizer));
        reg.register(Box::new(WebserverClfColumnizer));
        reg.register(Box::new(CsvColumnizer::default()));
        reg.register(Box::new(Log4jXmlColumnizer));
        reg.register(Box::new(ApacheCombinedColumnizer));
        reg.register(Box::new(NginxColumnizer));
        reg.register(Box::new(JsonLinesColumnizer));
        reg.register(Box::new(TsvColumnizer));
        reg.register(Box::new(SyslogColumnizer));
        reg
    }

    pub fn register(&mut self, col: Box<dyn Columnizer>) {
        self.columnizers.insert(col.id(), col);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Columnizer> {
        self.columnizers.get(id).map(|b| b.as_ref())
    }

    pub fn list_all(&self) -> Vec<(&'static str, &'static str)> {
        let mut list: Vec<_> = self
            .columnizers
            .values()
            .map(|c| (c.id(), c.name()))
            .collect();
        list.sort_by_key(|(_, name)| *name);
        list
    }

    /// Auto-detects the most suitable columnizer for the given sample lines.
    /// Falls back to "plain" if confidence is low (< 0.4).
    pub fn detect(&self, sample_lines: &[&str]) -> &'static str {
        if sample_lines.is_empty() {
            return "plain";
        }

        // Deterministic priority ordering so structured formats override generic delimiters
        let priority_order = [
            "jsonl",
            "log4j_xml",
            "apache_combined",
            "nginx",
            "clf",
            "syslog",
            "timestamp",
            "tsv",
            "csv",
        ];

        let mut best_id = "plain";
        let mut best_score = 0.35f32; // Threshold for override

        for id in priority_order {
            if let Some(col) = self.columnizers.get(id) {
                let score = col.detect_confidence(sample_lines);
                if score > best_score {
                    best_score = score;
                    best_id = col.id();
                }
            }
        }

        best_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webserver_clf_parsing() {
        let line = r#"192.168.1.1 - admin [25/Nov/2026:10:00:00 +0700] "GET /index.html HTTP/1.1" 200 4096"#;
        let col = WebserverClfColumnizer;
        let parsed = col.parse_line(line);

        assert_eq!(parsed[0], "192.168.1.1");
        assert_eq!(parsed[1], "-");
        assert_eq!(parsed[2], "admin");
        assert_eq!(parsed[3], "25/Nov/2026:10:00:00 +0700");
        assert_eq!(parsed[4], "GET");
        assert_eq!(parsed[5], "/index.html");
        assert_eq!(parsed[6], "HTTP/1.1");
        assert_eq!(parsed[7], "200");
        assert_eq!(parsed[8], "4096");
    }

    #[test]
    fn test_csv_rfc4180_parsing() {
        let col = CsvColumnizer::default();
        let line = r#"123,"Hello, world","Line with ""escaped quotes""",456"#;
        let parsed = col.parse_line(line);

        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0], "123");
        assert_eq!(parsed[1], "Hello, world");
        assert_eq!(parsed[2], r#"Line with "escaped quotes""#);
        assert_eq!(parsed[3], "456");
    }

    #[test]
    fn test_json_lines_parsing() {
        let col = JsonLinesColumnizer;
        let line = r#"{"time":"2026-11-25T10:00:00Z","level":"error","logger":"auth","msg":"Login failed"}"#;
        let parsed = col.parse_line(line);

        assert_eq!(parsed[0], "2026-11-25T10:00:00Z");
        assert_eq!(parsed[1], "error");
        assert_eq!(parsed[2], "auth");
        assert_eq!(parsed[3], "Login failed");
    }

    #[test]
    fn test_auto_detection() {
        let reg = ColumnizerRegistry::new();
        let clf_sample = [
            r#"10.0.0.1 - - [25/Nov/2026:10:01:00 +0000] "GET /api HTTP/1.1" 200 120"#,
            r#"10.0.0.2 - - [25/Nov/2026:10:02:00 +0000] "POST /api HTTP/1.1" 201 45"#,
        ];
        assert_eq!(reg.detect(&clf_sample), "clf");

        let json_sample = [
            r#"{"level":"info","msg":"Server started"}"#,
            r#"{"level":"warn","msg":"High memory usage"}"#,
        ];
        assert_eq!(reg.detect(&json_sample), "jsonl");
    }
}
