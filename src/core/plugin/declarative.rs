use std::fs;
use std::path::Path;

use crate::core::columnizer::Columnizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarativeParserType {
    Delimited { delimiter: char, quote: char },
    Regex { pattern: String },
    JsonLines { fields: Vec<String> },
}

/// Declarative Columnizer loaded from external package (columnizer.json).
/// Operates 100% in-process with zero arbitrary code execution.
pub struct DeclarativeColumnizer {
    id: String,
    name: String,
    columns: Vec<String>,
    parser_type: DeclarativeParserType,
}

impl DeclarativeColumnizer {
    pub fn load_from_package(package_dir: &Path, config_rel_path: &str) -> Result<Self, String> {
        let config_path = package_dir.join(config_rel_path);
        if !config_path.exists() {
            return Err(format!(
                "Columnizer config file not found: {:?}",
                config_path
            ));
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read columnizer config: {}", e))?;

        if content.len() > 262_144 {
            return Err("Columnizer config exceeds maximum size limit of 256KB".to_string());
        }

        Self::parse_config(&content)
    }

    pub fn parse_config(json: &str) -> Result<Self, String> {
        let id = extract_field(json, "\"id\":").unwrap_or_else(|| "custom_declarative".to_string());
        let name = extract_field(json, "\"name\":")
            .unwrap_or_else(|| "Declarative Columnizer".to_string());

        // Extract column names array
        let mut columns = Vec::new();
        if let Some(pos) = json.find("\"columns\":") {
            if let Some(open) = json[pos..].find('[') {
                if let Some(close) = json[pos + open..].find(']') {
                    let cols_str = &json[pos + open + 1..pos + open + close];
                    for col in cols_str.split(',') {
                        let c_trim = col.trim().trim_matches('"');
                        if !c_trim.is_empty() {
                            columns.push(c_trim.to_string());
                        }
                    }
                }
            }
        }

        let format_type =
            extract_field(json, "\"format\":").unwrap_or_else(|| "delimited".to_string());
        let parser_type = match format_type.to_lowercase().as_str() {
            "regex" => {
                let pat = extract_field(json, "\"pattern\":")
                    .ok_or_else(|| "Missing 'pattern' for regex columnizer".to_string())?;
                DeclarativeParserType::Regex { pattern: pat }
            }
            "jsonl" | "json" => DeclarativeParserType::JsonLines {
                fields: columns.clone(),
            },
            _ => {
                let delim = extract_field(json, "\"delimiter\":")
                    .and_then(|s| s.chars().next())
                    .unwrap_or(',');
                DeclarativeParserType::Delimited {
                    delimiter: delim,
                    quote: '"',
                }
            }
        };

        if columns.is_empty() {
            columns = (1..=5).map(|i| format!("Col{}", i)).collect();
        }

        Ok(Self {
            id,
            name,
            columns,
            parser_type,
        })
    }
}

impl Columnizer for DeclarativeColumnizer {
    fn id(&self) -> &'static str {
        Box::leak(self.id.clone().into_boxed_str())
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn column_names(&self) -> Vec<String> {
        self.columns.clone()
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        match &self.parser_type {
            DeclarativeParserType::Delimited { delimiter, quote } => {
                let mut fields = Vec::new();
                let mut current = String::new();
                let mut in_quotes = false;
                let mut chars = line.chars().peekable();

                while let Some(ch) = chars.next() {
                    if ch == *quote {
                        if in_quotes && chars.peek() == Some(quote) {
                            chars.next();
                            current.push(*quote);
                        } else {
                            in_quotes = !in_quotes;
                        }
                    } else if ch == *delimiter && !in_quotes {
                        fields.push(current);
                        current = String::new();
                    } else {
                        current.push(ch);
                    }
                }
                fields.push(current);
                fields
            }
            DeclarativeParserType::Regex { pattern } => {
                let custom = crate::core::custom_columnizer::CustomColumnizer::new_regex(
                    &self.name, pattern,
                );
                custom.parse_line(line)
            }
            DeclarativeParserType::JsonLines { fields } => {
                let json_pairs = crate::core::inspector::LogInspector::extract_json_pairs(line);
                let mut row = Vec::with_capacity(fields.len());
                for f in fields {
                    let val = json_pairs
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(f))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    row.push(val);
                }
                row
            }
        }
    }

    fn detect_confidence(&self, sample_lines: &[&str]) -> f32 {
        if sample_lines.is_empty() {
            return 0.0;
        }

        let mut hits = 0;
        for line in sample_lines {
            let cols = self.parse_line(line);
            if cols.iter().any(|c| !c.is_empty()) {
                hits += 1;
            }
        }
        hits as f32 / sample_lines.len() as f32
    }
}

fn extract_field(json: &str, key: &str) -> Option<String> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        if let Some(first_quote) = rem.find('"') {
            if let Some(end_quote) = rem[first_quote + 1..].find('"') {
                return Some(
                    rem[first_quote + 1..first_quote + 1 + end_quote].replace("\\\\", "\\"),
                );
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declarative_delimited_parsing() {
        let config = r#"{
  "id": "semicolon_parser",
  "name": "Semicolon Log Format",
  "format": "delimited",
  "delimiter": ";",
  "columns": ["Time", "Level", "Message"]
}"#;

        let col = DeclarativeColumnizer::parse_config(config).unwrap();
        assert_eq!(col.column_names(), vec!["Time", "Level", "Message"]);

        let line = "2026-11-25 10:00:00;WARN;Disk space low";
        let parsed = col.parse_line(line);
        assert_eq!(
            parsed,
            vec!["2026-11-25 10:00:00", "WARN", "Disk space low"]
        );
    }

    #[test]
    fn test_declarative_json_lines_parsing() {
        let config = r#"{
  "id": "audit_json_parser",
  "name": "Audit JSON Format",
  "format": "jsonl",
  "columns": ["action", "user", "status"]
}"#;

        let col = DeclarativeColumnizer::parse_config(config).unwrap();
        let line = r#"{"action":"login","user":"admin","status":"success"}"#;
        let parsed = col.parse_line(line);
        assert_eq!(parsed, vec!["login", "admin", "success"]);
    }
}
