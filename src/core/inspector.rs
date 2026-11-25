use crate::core::columnizer::Columnizer;
use crate::core::source::LogSource;

/// Parsed field pair in structured log inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorField {
    pub name: String,
    pub value: String,
}

/// Detailed structured analysis of a selected log line or record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedRecord {
    pub line_idx: usize,
    pub raw_content: String,
    pub constituent_filename: Option<String>,
    pub fields: Vec<InspectorField>,
    pub timestamp: Option<String>,
    pub is_multiline: bool,
    pub physical_line_range: (usize, usize),
}

pub struct LogInspector;

impl LogInspector {
    /// Inspects line `idx` of `source` using `columnizer`.
    pub fn inspect_line(
        source: &mut dyn LogSource,
        columnizer: &dyn Columnizer,
        idx: usize,
    ) -> Result<InspectedRecord, String> {
        let raw = source.get_line(idx)?;
        let constituent = source.get_constituent_filename(idx).map(|s| s.to_string());
        let col_names = columnizer.column_names();
        let values = columnizer.parse_line(&raw);

        let mut fields = Vec::new();
        for (i, name) in col_names.into_iter().enumerate() {
            let val = values.get(i).cloned().unwrap_or_default();
            fields.push(InspectorField { name, value: val });
        }

        // Additional JSON inspection if line looks like JSON
        let trimmed = raw.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            let extracted_pairs = Self::extract_json_pairs(trimmed);
            for (k, v) in extracted_pairs {
                if !fields.iter().any(|f| f.name == k) {
                    fields.push(InspectorField { name: k, value: v });
                }
            }
        }

        let timestamp = columnizer.extract_timestamp(&raw);

        Ok(InspectedRecord {
            line_idx: idx,
            raw_content: raw,
            constituent_filename: constituent,
            fields,
            timestamp,
            is_multiline: false,
            physical_line_range: (idx, idx),
        })
    }

    /// Extracts key-value pairs from a flat JSON object string.
    pub fn extract_json_pairs(json: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let trimmed = json.trim_matches(|c| c == '{' || c == '}').trim();
        let mut rest = trimmed;

        while let Some(quote_start) = rest.find('"') {
            let after_q1 = &rest[quote_start + 1..];
            let Some(quote_end) = after_q1.find('"') else {
                break;
            };
            let key = &after_q1[..quote_end];
            let after_key = &after_q1[quote_end + 1..].trim_start();

            let Some(colon_pos) = after_key.strip_prefix(':') else {
                break;
            };
            let after_colon = colon_pos.trim_start();

            if let Some(val_str) = after_colon.strip_prefix('"') {
                // String value
                let mut val = String::new();
                let mut escape = false;
                let mut advance = 0;
                for (i, ch) in val_str.char_indices() {
                    advance = i + 1;
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
                pairs.push((key.to_string(), val));
                rest = if advance < val_str.len() {
                    &val_str[advance..]
                } else {
                    ""
                };
            } else {
                // Number / bool / null / object
                let end = after_colon
                    .find([',', '}', ']'])
                    .unwrap_or(after_colon.len());
                let val = after_colon[..end].trim();
                pairs.push((key.to_string(), val.to_string()));
                rest = if end < after_colon.len() {
                    &after_colon[end + 1..]
                } else {
                    ""
                };
            }
        }

        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::columnizer::WebserverClfColumnizer;
    use crate::core::source::MockSource;

    #[test]
    fn test_inspector_field_extraction() {
        let sample = [
            r#"192.168.1.1 - admin [25/Nov/2026:10:00:00 +0700] "GET /api/v1/users HTTP/1.1" 200 1024"#,
        ];
        let mut source = MockSource::new("access.log", &sample);
        let col = WebserverClfColumnizer;

        let record = LogInspector::inspect_line(&mut source, &col, 0).unwrap();
        assert_eq!(record.line_idx, 0);
        assert_eq!(record.fields.len(), 9);
        assert_eq!(record.fields[0].name, "Host");
        assert_eq!(record.fields[0].value, "192.168.1.1");
        assert_eq!(record.fields[2].name, "User");
        assert_eq!(record.fields[2].value, "admin");
        assert_eq!(record.fields[4].name, "Method");
        assert_eq!(record.fields[4].value, "GET");
        assert_eq!(record.fields[7].name, "Status");
        assert_eq!(record.fields[7].value, "200");
    }

    #[test]
    fn test_json_pair_extraction() {
        let json = r#"{"user":"alice","action":"delete","code":403,"active":true}"#;
        let pairs = LogInspector::extract_json_pairs(json);
        assert_eq!(pairs.len(), 4);
        assert_eq!(pairs[0], ("user".to_string(), "alice".to_string()));
        assert_eq!(pairs[1], ("action".to_string(), "delete".to_string()));
        assert_eq!(pairs[2], ("code".to_string(), "403".to_string()));
        assert_eq!(pairs[3], ("active".to_string(), "true".to_string()));
    }
}
