use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileStat {
    pub size: u64,
    pub mtime: u64,
    pub is_dir: bool,
    pub permissions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: u64,
    pub permissions: u32,
}

/// Abstract transport layer for SFTP operations.
/// Enables full testability via MockRemoteTransport and production TCP/SSH clients.
pub trait RemoteTransport: Send + Sync {
    fn stat(&self, path: &str) -> Result<RemoteFileStat, String>;
    fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, String>;
    fn read_chunk(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String>;
}

/// In-memory mock transport simulating remote SSH/SFTP server.
/// Supports 64-bit sparse files (>4GB), latency simulation, and rotation events.
pub struct MockRemoteTransport {
    files: Mutex<HashMap<String, MockRemoteFile>>,
}

struct MockRemoteFile {
    content: Vec<u8>,
    sparse_size: Option<u64>,
    mtime: u64,
    is_dir: bool,
    permissions: u32,
}

impl MockRemoteTransport {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_file(&self, path: &str, content: &[u8], mtime: u64) {
        if let Ok(mut lock) = self.files.lock() {
            lock.insert(
                path.to_string(),
                MockRemoteFile {
                    content: content.to_vec(),
                    sparse_size: None,
                    mtime,
                    is_dir: false,
                    permissions: 0o644,
                },
            );
        }
    }

    pub fn add_sparse_file(&self, path: &str, virtual_size: u64, mtime: u64) {
        if let Ok(mut lock) = self.files.lock() {
            lock.insert(
                path.to_string(),
                MockRemoteFile {
                    content: Vec::new(),
                    sparse_size: Some(virtual_size),
                    mtime,
                    is_dir: false,
                    permissions: 0o644,
                },
            );
        }
    }

    pub fn add_dir(&self, path: &str, mtime: u64) {
        if let Ok(mut lock) = self.files.lock() {
            lock.insert(
                path.to_string(),
                MockRemoteFile {
                    content: Vec::new(),
                    sparse_size: None,
                    mtime,
                    is_dir: true,
                    permissions: 0o755,
                },
            );
        }
    }

    pub fn append_file(&self, path: &str, additional_content: &[u8], new_mtime: u64) {
        if let Ok(mut lock) = self.files.lock() {
            if let Some(f) = lock.get_mut(path) {
                f.content.extend_from_slice(additional_content);
                f.mtime = new_mtime;
            }
        }
    }

    pub fn truncate_file(&self, path: &str, new_content: &[u8], new_mtime: u64) {
        if let Ok(mut lock) = self.files.lock() {
            if let Some(f) = lock.get_mut(path) {
                f.content.clear();
                f.content.extend_from_slice(new_content);
                f.mtime = new_mtime;
            }
        }
    }
}

impl Default for MockRemoteTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteTransport for MockRemoteTransport {
    fn stat(&self, path: &str) -> Result<RemoteFileStat, String> {
        let lock = self.files.lock().map_err(|e| e.to_string())?;
        let f = lock
            .get(path)
            .ok_or_else(|| format!("Remote file '{}' not found", path))?;
        let size = f.sparse_size.unwrap_or(f.content.len() as u64);
        Ok(RemoteFileStat {
            size,
            mtime: f.mtime,
            is_dir: f.is_dir,
            permissions: f.permissions,
        })
    }

    fn list_dir(&self, dir_path: &str) -> Result<Vec<RemoteDirEntry>, String> {
        let lock = self.files.lock().map_err(|e| e.to_string())?;
        let clean_prefix = if dir_path.ends_with('/') {
            dir_path.to_string()
        } else {
            format!("{}/", dir_path)
        };

        let mut entries = Vec::new();
        for (path, f) in lock.iter() {
            if path.starts_with(&clean_prefix) && path != dir_path {
                let sub = &path[clean_prefix.len()..];
                if !sub.is_empty() && !sub.contains('/') {
                    entries.push(RemoteDirEntry {
                        name: sub.to_string(),
                        path: path.clone(),
                        is_dir: f.is_dir,
                        size: f.sparse_size.unwrap_or(f.content.len() as u64),
                        mtime: f.mtime,
                        permissions: f.permissions,
                    });
                }
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_chunk(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let lock = self.files.lock().map_err(|e| e.to_string())?;
        let f = lock
            .get(path)
            .ok_or_else(|| format!("Remote file '{}' not found", path))?;

        if let Some(sparse_size) = f.sparse_size {
            // Simulated 64-bit sparse synthetic data
            if offset >= sparse_size {
                return Ok(Vec::new());
            }
            let avail = (sparse_size - offset).min(length as u64) as usize;
            let mut chunk = Vec::with_capacity(avail);
            for i in 0..avail {
                let byte_pos = offset + i as u64;
                if byte_pos % 80 == 79 {
                    chunk.push(b'\n');
                } else {
                    chunk.push(b'A' + ((byte_pos % 26) as u8));
                }
            }
            return Ok(chunk);
        }

        if offset >= f.content.len() as u64 {
            return Ok(Vec::new());
        }
        let start = offset as usize;
        let end = (start + length).min(f.content.len());
        Ok(f.content[start..end].to_vec())
    }
}
