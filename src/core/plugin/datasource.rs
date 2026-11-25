use std::path::Path;

use crate::core::reader::UpdateEvent;
use crate::core::source::{
    allocate_source_id, LogSource, SourceCapabilities, SourceIdentity, SourceLineRef, SourceType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSourceIpcMessage {
    Open {
        uri: String,
        options: Vec<String>,
    },
    OpenAck {
        source_id: u32,
        initial_lines: usize,
        is_continuous: bool,
    },
    ReadLine {
        line_idx: usize,
    },
    ReadLineAck {
        line_idx: usize,
        text: String,
    },
    AppendNotification {
        new_lines: usize,
    },
    Close,
    Error {
        message: String,
    },
}

impl DataSourceIpcMessage {
    pub fn encode_frame(&self) -> Vec<u8> {
        let json = match self {
            Self::Open { uri, options } => {
                let opt_json: Vec<String> = options.iter().map(|o| format!("\"{}\"", o)).collect();
                format!(
                    "{{\"type\":\"ds_open\",\"uri\":\"{}\",\"options\":[{}]}}",
                    uri,
                    opt_json.join(",")
                )
            }
            Self::OpenAck {
                source_id,
                initial_lines,
                is_continuous,
            } => {
                format!(
                    "{{\"type\":\"ds_open_ack\",\"source_id\":{},\"initial_lines\":{},\"is_continuous\":{}}}",
                    source_id, initial_lines, is_continuous
                )
            }
            Self::ReadLine { line_idx } => {
                format!("{{\"type\":\"ds_read_line\",\"line_idx\":{}}}", line_idx)
            }
            Self::ReadLineAck { line_idx, text } => {
                format!(
                    "{{\"type\":\"ds_read_line_ack\",\"line_idx\":{},\"text\":\"{}\"}}",
                    line_idx,
                    text.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            Self::AppendNotification { new_lines } => {
                format!("{{\"type\":\"ds_append\",\"new_lines\":{}}}", new_lines)
            }
            Self::Close => "{\"type\":\"ds_close\"}".to_string(),
            Self::Error { message } => {
                format!("{{\"type\":\"ds_error\",\"message\":\"{}\"}}", message)
            }
        };

        let len = json.len() as u32;
        let mut frame = Vec::with_capacity(4 + json.len());
        frame.extend_from_slice(&len.to_le_bytes());
        frame.extend_from_slice(json.as_bytes());
        frame
    }

    pub fn decode_frame(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 {
            return Err("Frame header too short".to_string());
        }
        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if bytes.len() < 4 + len {
            return Err("Incomplete frame data".to_string());
        }
        let json = std::str::from_utf8(&bytes[4..4 + len])
            .map_err(|e| format!("Invalid UTF-8 frame: {}", e))?;

        if json.contains("\"ds_open_ack\"") {
            let id = extract_num(json, "\"source_id\":").unwrap_or(1) as u32;
            let lines = extract_num(json, "\"initial_lines\":").unwrap_or(0);
            let cont = json.contains("\"is_continuous\":true");
            Ok(Self::OpenAck {
                source_id: id,
                initial_lines: lines,
                is_continuous: cont,
            })
        } else if json.contains("\"ds_open\"") {
            let uri = extract_str(json, "\"uri\":\"").unwrap_or_default();
            let mut options = Vec::new();
            if let Some(opt_pos) = json.find("\"options\":[") {
                let rest = &json[opt_pos + "\"options\":[".len()..];
                if let Some(opt_end) = rest.find(']') {
                    for item in rest[..opt_end].split(',') {
                        let clean = item.trim().trim_matches('"');
                        if !clean.is_empty() {
                            options.push(clean.to_string());
                        }
                    }
                }
            }
            Ok(Self::Open { uri, options })
        } else if json.contains("\"ds_read_line_ack\"") {
            let idx = extract_num(json, "\"line_idx\":").unwrap_or(0);
            let text = extract_str(json, "\"text\":\"").unwrap_or_default();
            Ok(Self::ReadLineAck {
                line_idx: idx,
                text,
            })
        } else if json.contains("\"ds_read_line\"") {
            let idx = extract_num(json, "\"line_idx\":").unwrap_or(0);
            Ok(Self::ReadLine { line_idx: idx })
        } else if json.contains("\"ds_append\"") {
            let n = extract_num(json, "\"new_lines\":").unwrap_or(1);
            Ok(Self::AppendNotification { new_lines: n })
        } else if json.contains("\"ds_close\"") {
            Ok(Self::Close)
        } else if json.contains("\"ds_error\"") {
            let msg = extract_str(json, "\"message\":\"").unwrap_or_default();
            Ok(Self::Error { message: msg })
        } else {
            Err(format!("Unknown data source message: {}", json))
        }
    }
}

fn extract_num(s: &str, key: &str) -> Option<usize> {
    if let Some(pos) = s.find(key) {
        let rest = &s[pos + key.len()..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].trim().parse().ok()
    } else {
        None
    }
}

fn extract_str(s: &str, key: &str) -> Option<String> {
    if let Some(pos) = s.find(key) {
        let rest = &s[pos + key.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// In-memory virtual adapter for DataSourcePlugin testing and local execution.
pub struct MockDataSourcePlugin {
    id: u32,
    generation: u64,
    title: String,
    uri: String,
    lines: Vec<String>,
    bookmarks: Vec<usize>,
    is_continuous: bool,
}

impl MockDataSourcePlugin {
    pub fn new(title: &str, uri: &str, sample_lines: &[&str], is_continuous: bool) -> Self {
        Self {
            id: allocate_source_id(),
            generation: 1,
            title: title.to_string(),
            uri: uri.to_string(),
            lines: sample_lines.iter().map(|s| s.to_string()).collect(),
            bookmarks: Vec::new(),
            is_continuous,
        }
    }

    pub fn append_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }
}

impl LogSource for MockDataSourcePlugin {
    fn source_id(&self) -> u32 {
        self.id
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn display_name(&self) -> &str {
        &self.title
    }

    fn source_path(&self) -> Option<&Path> {
        None
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn file_size(&self) -> u64 {
        self.lines.iter().map(|l| l.len() as u64 + 1).sum()
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.lines
            .get(idx)
            .cloned()
            .ok_or_else(|| format!("Line {} out of bounds", idx))
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        SourceLineRef {
            source_id: self.id,
            generation: self.generation,
            line_idx: idx,
            byte_offset: 0,
        }
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        Ok(UpdateEvent::NoChange)
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        if idx >= self.lines.len() {
            return false;
        }
        if let Some(pos) = self.bookmarks.iter().position(|&b| b == idx) {
            self.bookmarks.remove(pos);
            false
        } else {
            self.bookmarks.push(idx);
            self.bookmarks.sort_unstable();
            true
        }
    }

    fn is_bookmarked(&self, idx: usize) -> bool {
        self.bookmarks.binary_search(&idx).is_ok()
    }

    fn bookmarks(&self) -> &[usize] {
        &self.bookmarks
    }

    fn source_type(&self) -> SourceType {
        SourceType::Plugin
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            can_read: true,
            can_random_access: true,
            can_monitor: self.is_continuous,
            can_discover_files: false,
            can_detect_rotation: false,
            can_reconnect: false,
            can_resume: false,
            has_stable_identity: false,
            is_live_append: self.is_continuous,
            is_end_of_stream: !self.is_continuous,
        }
    }

    fn identity(&self) -> SourceIdentity {
        SourceIdentity {
            source_type: SourceType::Plugin,
            profile_id: None,
            host_fingerprint: None,
            canonical_path: self.uri.clone(),
            generation: self.generation,
            file_id: None,
            mtime: None,
            size: self.file_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datasource_ipc_message_and_mock_plugin() {
        let msg = DataSourceIpcMessage::Open {
            uri: "custom://source".to_string(),
            options: vec!["opt1".to_string(), "opt2".to_string()],
        };
        let frame = msg.encode_frame();
        let decoded = DataSourceIpcMessage::decode_frame(&frame).expect("Decode frame");
        assert_eq!(msg, decoded);

        let mut plugin = MockDataSourcePlugin::new(
            "Test Plugin",
            "custom://source",
            &["line 1", "line 2"],
            false,
        );
        assert_eq!(plugin.line_count(), 2);
        assert_eq!(plugin.get_line(0).unwrap(), "line 1");
        assert_eq!(plugin.source_type(), SourceType::Plugin);
        assert!(!plugin.is_continuous());
    }
}
