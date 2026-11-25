use std::path::Path;

/// Supported types of custom user-defined columnizers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomParserType {
    RegexNamedCaptures,
    Delimited { delimiter: char },
    FixedWidth { widths: Vec<usize> },
    JsonMapping { fields: Vec<String> },
}

/// Custom Columnizer configuration created via Builder dialog or imported from JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomColumnizer {
    pub name: String,
    pub parser_type: CustomParserType,
    pub pattern: String,
    pub column_names: Vec<String>,
    pub timestamp_column_idx: Option<usize>,
}

impl CustomColumnizer {
    /// Creates a new regex-based custom columnizer with named captures.
    /// E.g. pattern: `(?P<time>\S+)\s+\[(?P<level>\w+)\]\s+(?P<msg>.*)`
    pub fn new_regex(name: &str, pattern: &str) -> Self {
        let cols = Self::extract_named_captures(pattern);
        Self {
            name: name.to_string(),
            parser_type: CustomParserType::RegexNamedCaptures,
            pattern: pattern.to_string(),
            column_names: cols,
            timestamp_column_idx: Some(0),
        }
    }

    /// Creates a delimited custom columnizer (e.g. pipe '|' or semicolon ';').
    pub fn new_delimited(name: &str, delimiter: char, column_names: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            parser_type: CustomParserType::Delimited { delimiter },
            pattern: delimiter.to_string(),
            column_names,
            timestamp_column_idx: None,
        }
    }

    /// Extracts named captures `(?P<name>...)` from a regex pattern string.
    pub fn extract_named_captures(pattern: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = pattern;
        while let Some(start) = rest.find("(?P<") {
            let slice = &rest[start + 4..];
            if let Some(end) = slice.find('>') {
                let name = &slice[..end];
                names.push(name.to_string());
                rest = &slice[end + 1..];
            } else {
                break;
            }
        }
        if names.is_empty() {
            vec!["Message".to_string()]
        } else {
            names
        }
    }

    /// Parses a raw line using this custom columnizer specification.
    pub fn parse_line(&self, line: &str) -> Vec<String> {
        match &self.parser_type {
            CustomParserType::Delimited { delimiter } => {
                line.split(*delimiter).map(|s| s.to_string()).collect()
            }
            CustomParserType::FixedWidth { widths } => {
                let mut cols = Vec::new();
                let mut start = 0;
                for &w in widths {
                    if start >= line.len() {
                        cols.push(String::new());
                    } else {
                        let end = (start + w).min(line.len());
                        cols.push(line[start..end].trim().to_string());
                        start = end;
                    }
                }
                cols
            }
            CustomParserType::RegexNamedCaptures => {
                // Lightweight named-capture pattern extraction without external regex dependencies
                // Uses pattern token anchors: extracts text between pattern delimiters
                Self::extract_by_pattern(&self.pattern, line, &self.column_names)
            }
            CustomParserType::JsonMapping { fields } => {
                let mut cols = Vec::new();
                for f in fields {
                    let key_pat = format!("\"{}\":", f);
                    if let Some(pos) = line.find(&key_pat) {
                        let after = line[pos + key_pat.len()..].trim_start();
                        if let Some(stripped) = after.strip_prefix('"') {
                            let end = stripped.find('"').unwrap_or(stripped.len());
                            cols.push(stripped[..end].to_string());
                        } else {
                            let end = after.find([',', '}', ']']).unwrap_or(after.len());
                            cols.push(after[..end].trim().to_string());
                        }
                    } else {
                        cols.push(String::new());
                    }
                }
                cols
            }
        }
    }

    fn extract_by_pattern(pattern: &str, line: &str, col_names: &[String]) -> Vec<String> {
        if col_names.is_empty() {
            return vec![line.to_string()];
        }

        // For simple patterns like `^\[(?P<time>[^\]]+)\] \[(?P<module>[^\]]+)\] (?P<msg>.*)$`
        // We match literal delimiters outside of `(?P<...>)`
        let mut cols = Vec::new();
        let mut cur_line = line;

        // Split pattern by named capture blocks `(?P<...>)`
        let mut delimiters = Vec::new();
        let mut rest = pattern.trim_start_matches('^').trim_end_matches('$');

        while let Some(start) = rest.find("(?P<") {
            let prefix = &rest[..start];
            delimiters.push(prefix);
            let slice = &rest[start + 4..];
            if let Some(end) = slice.find('>') {
                // Find closing paren of group
                let group_slice = &slice[end + 1..];
                if let Some(close_paren) = group_slice.find(')') {
                    rest = &group_slice[close_paren + 1..];
                } else {
                    rest = "";
                }
            } else {
                break;
            }
        }
        delimiters.push(rest);

        // Apply delimiters to slice cur_line
        for i in 0..col_names.len() {
            let raw_left = delimiters.get(i).copied().unwrap_or("");
            let raw_right = delimiters.get(i + 1).copied().unwrap_or("");

            let left_delim = raw_left.replace('\\', "");
            let right_delim = raw_right.replace('\\', "");

            if !left_delim.is_empty() {
                if let Some(pos) = cur_line.find(&left_delim) {
                    cur_line = &cur_line[pos + left_delim.len()..];
                }
            }

            if !right_delim.is_empty() {
                if let Some(pos) = cur_line.find(&right_delim) {
                    let val = &cur_line[..pos];
                    cols.push(val.to_string());
                    cur_line = &cur_line[pos + right_delim.len()..];
                } else {
                    cols.push(cur_line.to_string());
                    cur_line = "";
                }
            } else {
                cols.push(cur_line.to_string());
                cur_line = "";
            }
        }

        cols
    }

    /// Serializes configuration to JSON string.
    pub fn to_json(&self) -> String {
        let cols_json = self
            .column_names
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let type_str = match &self.parser_type {
            CustomParserType::RegexNamedCaptures => "regex",
            CustomParserType::Delimited { delimiter } => {
                return format!(
                    r#"{{"name":"{}","type":"delimited","delimiter":"{}","columns":[{}]}}"#,
                    self.name, delimiter, cols_json
                )
            }
            CustomParserType::FixedWidth { .. } => "fixed_width",
            CustomParserType::JsonMapping { .. } => "json",
        };

        format!(
            r#"{{"name":"{}","type":"{}","pattern":"{}","columns":[{}]}}"#,
            self.name,
            type_str,
            self.pattern.replace('\\', "\\\\").replace('"', "\\\""),
            cols_json
        )
    }

    /// Saves custom columnizer to target file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = self.to_json();
        std::fs::write(path, json).map_err(|e| format!("Failed to write custom columnizer: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_regex_named_captures() {
        let pat = r#"^\[(?P<time>[^\]]+)\] \[(?P<module>[^\]]+)\] (?P<msg>.*)$"#;
        let col = CustomColumnizer::new_regex("CustomApp", pat);

        assert_eq!(col.column_names, vec!["time", "module", "msg"]);

        let sample = "[2026-11-25 10:00:00] [AUTH] User login successful";
        let parsed = col.parse_line(sample);

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "2026-11-25 10:00:00");
        assert_eq!(parsed[1], "AUTH");
        assert_eq!(parsed[2], "User login successful");
    }

    #[test]
    fn test_custom_delimited() {
        let col = CustomColumnizer::new_delimited(
            "PipeDelimited",
            '|',
            vec!["Date".to_string(), "Level".to_string(), "Msg".to_string()],
        );
        let sample = "2026-11-25|WARN|Disk space low";
        let parsed = col.parse_line(sample);

        assert_eq!(parsed, vec!["2026-11-25", "WARN", "Disk space low"]);
    }
}
