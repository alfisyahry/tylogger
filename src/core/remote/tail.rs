use std::sync::Arc;

use crate::core::reader::UpdateEvent;
use crate::core::remote::client::RemoteTransport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteRotationType {
    None,
    Append { bytes_added: u64 },
    Truncation { old_size: u64, new_size: u64 },
    CopyTruncate { old_size: u64, new_size: u64 },
}

pub struct SftpLiveTailEngine {
    transport: Arc<dyn RemoteTransport>,
    remote_path: String,
    last_size: u64,
    last_mtime: u64,
    poll_interval_ms: u64,
}

impl SftpLiveTailEngine {
    pub fn new(
        transport: Arc<dyn RemoteTransport>,
        remote_path: &str,
        initial_size: u64,
        initial_mtime: u64,
    ) -> Self {
        Self {
            transport,
            remote_path: remote_path.to_string(),
            last_size: initial_size,
            last_mtime: initial_mtime,
            poll_interval_ms: 1000,
        }
    }

    pub fn set_poll_interval(&mut self, interval_ms: u64) {
        self.poll_interval_ms = interval_ms;
    }

    pub fn poll_rotation(&mut self) -> Result<RemoteRotationType, String> {
        let stat = self.transport.stat(&self.remote_path)?;
        if stat.size > self.last_size {
            let added = stat.size - self.last_size;
            self.last_size = stat.size;
            self.last_mtime = stat.mtime;
            Ok(RemoteRotationType::Append { bytes_added: added })
        } else if stat.size < self.last_size {
            let old = self.last_size;
            self.last_size = stat.size;
            self.last_mtime = stat.mtime;
            Ok(RemoteRotationType::Truncation {
                old_size: old,
                new_size: stat.size,
            })
        } else if stat.mtime != self.last_mtime && stat.size == self.last_size {
            // Timestamp changed but size equal: possible copy-truncate with identical payload
            self.last_mtime = stat.mtime;
            Ok(RemoteRotationType::None)
        } else {
            Ok(RemoteRotationType::None)
        }
    }

    pub fn poll_update(&mut self) -> Result<UpdateEvent, String> {
        match self.poll_rotation()? {
            RemoteRotationType::Append { .. } => Ok(UpdateEvent::Appended { new_lines: 1 }),
            RemoteRotationType::Truncation { .. } | RemoteRotationType::CopyTruncate { .. } => {
                Ok(UpdateEvent::Truncated)
            }
            RemoteRotationType::None => Ok(UpdateEvent::NoChange),
        }
    }
}
