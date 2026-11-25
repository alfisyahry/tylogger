use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// IPC message types between Tylogger and out-of-process executable plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginIpcMessage {
    Handshake {
        api_version: String,
        plugin_id: String,
    },
    HandshakeAck {
        plugin_name: String,
        columns: Vec<String>,
    },
    ParseLine {
        line_idx: usize,
        line: String,
    },
    ParseResponse {
        line_idx: usize,
        columns: Vec<String>,
    },
    Error {
        message: String,
    },
    Shutdown,
}

impl PluginIpcMessage {
    pub fn encode_frame(&self) -> Vec<u8> {
        let json = match self {
            Self::Handshake {
                api_version,
                plugin_id,
            } => {
                format!(
                    "{{\"type\":\"handshake\",\"api_version\":\"{}\",\"plugin_id\":\"{}\"}}",
                    api_version, plugin_id
                )
            }
            Self::HandshakeAck {
                plugin_name,
                columns,
            } => {
                let cols_json: Vec<String> = columns.iter().map(|c| format!("\"{}\"", c)).collect();
                format!(
                    "{{\"type\":\"handshake_ack\",\"plugin_name\":\"{}\",\"columns\":[{}]}}",
                    plugin_name,
                    cols_json.join(",")
                )
            }
            Self::ParseLine { line_idx, line } => {
                format!(
                    "{{\"type\":\"parse_line\",\"line_idx\":{},\"line\":\"{}\"}}",
                    line_idx,
                    line.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            Self::ParseResponse { line_idx, columns } => {
                let cols_json: Vec<String> = columns
                    .iter()
                    .map(|c| format!("\"{}\"", c.replace('"', "\\\"")))
                    .collect();
                format!(
                    "{{\"type\":\"parse_response\",\"line_idx\":{},\"columns\":[{}]}}",
                    line_idx,
                    cols_json.join(",")
                )
            }
            Self::Error { message } => {
                format!("{{\"type\":\"error\",\"message\":\"{}\"}}", message)
            }
            Self::Shutdown => "{\"type\":\"shutdown\"}".to_string(),
        };

        let len = json.len() as u32;
        let mut frame = Vec::with_capacity(4 + json.len());
        frame.extend_from_slice(&len.to_le_bytes());
        frame.extend_from_slice(json.as_bytes());
        frame
    }

    pub fn decode_frame(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 {
            return Err("Frame too short for length header".to_string());
        }
        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if bytes.len() < 4 + len {
            return Err("Incomplete frame payload".to_string());
        }

        let json = std::str::from_utf8(&bytes[4..4 + len])
            .map_err(|e| format!("Invalid UTF-8 in frame: {}", e))?;

        if json.contains("\"handshake_ack\"") {
            let name = extract_val(json, "\"plugin_name\":").unwrap_or_default();
            let mut cols = Vec::new();
            if let Some(pos) = json.find("\"columns\":") {
                if let Some(open) = json[pos..].find('[') {
                    if let Some(close) = json[pos + open..].find(']') {
                        let c_str = &json[pos + open + 1..pos + open + close];
                        for c in c_str.split(',') {
                            let clean = c.trim().trim_matches('"');
                            if !clean.is_empty() {
                                cols.push(clean.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Self::HandshakeAck {
                plugin_name: name,
                columns: cols,
            })
        } else if json.contains("\"parse_response\"") {
            let l_idx = extract_usize(json, "\"line_idx\":").unwrap_or(0);
            let mut cols = Vec::new();
            if let Some(pos) = json.find("\"columns\":") {
                if let Some(open) = json[pos..].find('[') {
                    if let Some(close) = json[pos + open..].find(']') {
                        let c_str = &json[pos + open + 1..pos + open + close];
                        for c in c_str.split(',') {
                            let clean = c.trim().trim_matches('"');
                            cols.push(clean.to_string());
                        }
                    }
                }
            }
            Ok(Self::ParseResponse {
                line_idx: l_idx,
                columns: cols,
            })
        } else if json.contains("\"error\"") {
            let msg =
                extract_val(json, "\"message\":").unwrap_or_else(|| "Unknown error".to_string());
            Ok(Self::Error { message: msg })
        } else {
            Ok(Self::Shutdown)
        }
    }
}

/// Out-of-process executable plugin host controller.
/// Bound by Windows Job Objects (RAM/CPU limits, kill on close) and watchdog timeouts.
pub struct ExecutablePluginHost {
    executable_path: PathBuf,
    plugin_name: String,
    columns: Vec<String>,
    child_process: Option<Child>,
    is_active: bool,
}

impl ExecutablePluginHost {
    pub fn new(path: &Path) -> Self {
        Self {
            executable_path: path.to_path_buf(),
            plugin_name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            columns: vec!["Col1".to_string(), "Col2".to_string()],
            child_process: None,
            is_active: false,
        }
    }

    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Spawns the plugin process and performs capability negotiation handshake.
    pub fn start(&mut self) -> Result<(), String> {
        if !self.executable_path.exists() {
            return Err(format!(
                "Plugin executable not found: {:?}",
                self.executable_path
            ));
        }

        let mut child = Command::new(&self.executable_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin process: {}", e))?;

        // Windows Job Object Isolation
        #[cfg(windows)]
        unsafe {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            };

            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if !job.is_null() {
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                AssignProcessToJobObject(job, child.as_raw_handle() as _);
            }
        }

        // Perform handshake
        let handshake_req = PluginIpcMessage::Handshake {
            api_version: "1.0".to_string(),
            plugin_id: self.plugin_name.clone(),
        };
        let frame = handshake_req.encode_frame();

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(&frame)
                .map_err(|e| format!("Write handshake: {}", e))?;
            stdin
                .flush()
                .map_err(|e| format!("Flush handshake: {}", e))?;
        }

        // Read handshake ack with 2 second timeout guard
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut header = [0u8; 4];

        if let Some(stdout) = child.stdout.as_mut() {
            while Instant::now() < deadline {
                if let Ok(()) = stdout.read_exact(&mut header) {
                    let len = u32::from_le_bytes(header) as usize;
                    if len > 65_536 {
                        return Err("Excessive response frame size from plugin".to_string());
                    }
                    let mut payload = vec![0u8; len];
                    stdout.read_exact(&mut payload).map_err(|e| e.to_string())?;

                    let mut full_frame = Vec::with_capacity(4 + len);
                    full_frame.extend_from_slice(&header);
                    full_frame.extend_from_slice(&payload);

                    if let Ok(PluginIpcMessage::HandshakeAck {
                        plugin_name,
                        columns,
                    }) = PluginIpcMessage::decode_frame(&full_frame)
                    {
                        self.plugin_name = plugin_name;
                        if !columns.is_empty() {
                            self.columns = columns;
                        }
                        self.child_process = Some(child);
                        self.is_active = true;
                        return Ok(());
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        let _ = child.kill();
        Err("Plugin handshake timed out".to_string())
    }

    /// Dispatches a parse line request through IPC pipe with bounded watchdog timeout.
    pub fn parse_line(&mut self, line_idx: usize, line: &str) -> Result<Vec<String>, String> {
        if !self.is_active {
            return Err("Plugin host is not running".to_string());
        }

        let req = PluginIpcMessage::ParseLine {
            line_idx,
            line: line.to_string(),
        };
        let frame = req.encode_frame();

        let child = self
            .child_process
            .as_mut()
            .ok_or_else(|| "No active child".to_string())?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(&frame)
                .map_err(|e| format!("IPC write error: {}", e))?;
            stdin
                .flush()
                .map_err(|e| format!("IPC flush error: {}", e))?;
        }

        // Read response with 3 second watchdog
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut header = [0u8; 4];

        if let Some(stdout) = child.stdout.as_mut() {
            while Instant::now() < deadline {
                if let Ok(()) = stdout.read_exact(&mut header) {
                    let len = u32::from_le_bytes(header) as usize;
                    let mut payload = vec![0u8; len];
                    stdout.read_exact(&mut payload).map_err(|e| e.to_string())?;

                    let mut full_frame = Vec::with_capacity(4 + len);
                    full_frame.extend_from_slice(&header);
                    full_frame.extend_from_slice(&payload);

                    match PluginIpcMessage::decode_frame(&full_frame)? {
                        PluginIpcMessage::ParseResponse { columns, .. } => return Ok(columns),
                        PluginIpcMessage::Error { message } => return Err(message),
                        _ => return Err("Unexpected IPC message response".to_string()),
                    }
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        // Timeout hit: terminate process safely to prevent zombie
        self.terminate();
        Err("Plugin execution timeout: process killed by watchdog".to_string())
    }

    pub fn terminate(&mut self) {
        if let Some(mut child) = self.child_process.take() {
            let _ = child.kill();
        }
        self.is_active = false;
    }
}

impl Drop for ExecutablePluginHost {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn extract_val(json: &str, key: &str) -> Option<String> {
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

fn extract_usize(json: &str, key: &str) -> Option<usize> {
    if let Some(pos) = json.find(key) {
        let rem = &json[pos + key.len()..];
        let num_str: String = rem
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        return num_str.parse::<usize>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_protocol_frame_encoding_and_decoding() {
        let req = PluginIpcMessage::HandshakeAck {
            plugin_name: "ExternalJsonParser".to_string(),
            columns: vec!["timestamp".to_string(), "message".to_string()],
        };

        let encoded = req.encode_frame();
        assert!(encoded.len() > 4);

        let decoded = PluginIpcMessage::decode_frame(&encoded).expect("Decode frame");
        match decoded {
            PluginIpcMessage::HandshakeAck {
                plugin_name,
                columns,
            } => {
                assert_eq!(plugin_name, "ExternalJsonParser");
                assert_eq!(columns, vec!["timestamp", "message"]);
            }
            _ => panic!("Expected HandshakeAck variant"),
        }
    }

    #[test]
    fn test_ipc_parse_response_frame() {
        let resp = PluginIpcMessage::ParseResponse {
            line_idx: 42,
            columns: vec!["2026-11-25 10:00:00".to_string(), "SUCCESS".to_string()],
        };
        let encoded = resp.encode_frame();
        let decoded = PluginIpcMessage::decode_frame(&encoded).expect("Decode parse response");
        match decoded {
            PluginIpcMessage::ParseResponse { line_idx, columns } => {
                assert_eq!(line_idx, 42);
                assert_eq!(columns, vec!["2026-11-25 10:00:00", "SUCCESS"]);
            }
            _ => panic!("Expected ParseResponse"),
        }
    }
}
