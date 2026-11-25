#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerStreamType {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerStreamRecord {
    pub stream_type: DockerStreamType,
    pub timestamp: Option<String>,
    pub message: String,
}

impl DockerStreamRecord {
    pub fn format_display(&self) -> String {
        let channel = match self.stream_type {
            DockerStreamType::Stdout => "STDOUT",
            DockerStreamType::Stderr => "STDERR",
        };
        if let Some(ref ts) = self.timestamp {
            format!("{} [{}] {}", ts, channel, self.message)
        } else {
            format!("[{}] {}", channel, self.message)
        }
    }
}

pub struct DockerLogParser;

impl DockerLogParser {
    /// Parses a raw line or 8-byte multiplexed frame emitted by `docker logs`.
    pub fn parse_line(raw: &str) -> DockerStreamRecord {
        let trimmed = raw.trim_end();
        let bytes = trimmed.as_bytes();

        let mut stream_type = DockerStreamType::Stdout;
        let mut text_start = 0;

        // Check for Docker 8-byte stream multiplex header
        if bytes.len() >= 8
            && (bytes[0] == 1 || bytes[0] == 2)
            && bytes[1] == 0
            && bytes[2] == 0
            && bytes[3] == 0
        {
            if bytes[0] == 2 {
                stream_type = DockerStreamType::Stderr;
            }
            text_start = 8;
        }

        let payload = if text_start < bytes.len() {
            String::from_utf8_lossy(&bytes[text_start..]).to_string()
        } else {
            String::new()
        };

        // Check for timestamp prefix (RFC3339) e.g. 2026-11-25T10:00:00.123456789Z
        if payload.len() > 30 && payload.as_bytes()[10] == b'T' && payload.contains('Z') {
            if let Some((ts, msg)) = payload.split_once(' ') {
                return DockerStreamRecord {
                    stream_type,
                    timestamp: Some(ts.to_string()),
                    message: msg.to_string(),
                };
            }
        }

        DockerStreamRecord {
            stream_type,
            timestamp: None,
            message: payload,
        }
    }
}
