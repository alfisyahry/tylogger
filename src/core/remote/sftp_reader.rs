use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::core::reader::UpdateEvent;
use crate::core::remote::client::RemoteTransport;
use crate::core::source::{
    allocate_source_id, LogSource, SourceCapabilities, SourceIdentity, SourceLineRef, SourceStatus,
    SourceType,
};

const CHUNK_SIZE: usize = 65536; // 64 KB chunk size for SFTP block caching
const SPARSE_CHECKPOINT_STRIDE: usize = 64;

pub struct SftpLogReader {
    id: u32,
    generation: u64,
    title: String,
    path: String,
    profile_id: String,
    host_fingerprint: String,
    transport: Arc<dyn RemoteTransport>,
    file_size: u64,
    mtime: u64,
    line_count: usize,
    checkpoints: Vec<(usize, u64)>, // (line_index, byte_offset)
    block_cache: HashMap<u64, Vec<u8>>,
    max_cached_blocks: usize,
    bookmarks: Vec<usize>,
    status: SourceStatus,
    early_preview_ready: bool,
}

impl SftpLogReader {
    pub fn open(
        transport: Arc<dyn RemoteTransport>,
        profile_id: &str,
        host_fingerprint: &str,
        remote_path: &str,
    ) -> Result<Self, String> {
        let stat = transport.stat(remote_path)?;
        let title = remote_path
            .rsplit('/')
            .next()
            .unwrap_or(remote_path)
            .to_string();

        let mut reader = Self {
            id: allocate_source_id(),
            generation: 1,
            title,
            path: remote_path.to_string(),
            profile_id: profile_id.to_string(),
            host_fingerprint: host_fingerprint.to_string(),
            transport,
            file_size: stat.size,
            mtime: stat.mtime,
            line_count: 0,
            checkpoints: Vec::new(),
            block_cache: HashMap::new(),
            max_cached_blocks: 64, // ~4MB RAM max block cache
            bookmarks: Vec::new(),
            status: SourceStatus::Active,
            early_preview_ready: false,
        };

        // Perform early preview indexing of the first chunk (< 100ms)
        reader.index_initial_data()?;
        Ok(reader)
    }

    fn index_initial_data(&mut self) -> Result<(), String> {
        self.checkpoints.clear();
        self.checkpoints.push((0, 0));

        if self.file_size == 0 {
            self.line_count = 0;
            self.early_preview_ready = true;
            return Ok(());
        }

        // Read first chunk for instant early preview
        let initial_fetch_len = (self.file_size as usize).min(CHUNK_SIZE);
        let first_chunk = self
            .transport
            .read_chunk(&self.path, 0, initial_fetch_len)?;
        let mut line_count: usize = 0;

        for (i, &b) in first_chunk.iter().enumerate() {
            if b == b'\n' {
                line_count += 1;
                let last_newline_pos = (i + 1) as u64;
                if line_count.is_multiple_of(SPARSE_CHECKPOINT_STRIDE) {
                    self.checkpoints.push((line_count, last_newline_pos));
                }
            }
        }

        if !first_chunk.is_empty() && *first_chunk.last().unwrap() != b'\n' {
            line_count += 1;
        }

        self.block_cache.insert(0, first_chunk);
        self.line_count = line_count;
        self.early_preview_ready = true;

        // Progressively index remaining chunks if smaller than 50MB, or lazily handle larger files
        if self.file_size > CHUNK_SIZE as u64 && self.file_size <= 50 * 1024 * 1024 {
            let mut offset = CHUNK_SIZE as u64;
            while offset < self.file_size {
                let fetch_len = ((self.file_size - offset) as usize).min(CHUNK_SIZE);
                let chunk = self.transport.read_chunk(&self.path, offset, fetch_len)?;
                for (i, &b) in chunk.iter().enumerate() {
                    if b == b'\n' {
                        line_count += 1;
                        let byte_offset = offset + (i + 1) as u64;
                        if line_count.is_multiple_of(SPARSE_CHECKPOINT_STRIDE) {
                            self.checkpoints.push((line_count, byte_offset));
                        }
                    }
                }
                offset += fetch_len as u64;
            }
            self.line_count = line_count;
        }

        Ok(())
    }

    fn fetch_block(&mut self, block_idx: u64) -> Result<&[u8], String> {
        let offset = block_idx * CHUNK_SIZE as u64;
        let expected_len = ((self.file_size.saturating_sub(offset)) as usize).min(CHUNK_SIZE);

        let needs_fetch = match self.block_cache.get(&block_idx) {
            Some(data) => data.len() < expected_len,
            None => true,
        };

        if needs_fetch {
            if self.block_cache.len() >= self.max_cached_blocks {
                // Evict one arbitrary entry
                if let Some(&first_key) = self.block_cache.keys().next() {
                    self.block_cache.remove(&first_key);
                }
            }
            let chunk = self
                .transport
                .read_chunk(&self.path, offset, expected_len)?;
            self.block_cache.insert(block_idx, chunk);
        }
        Ok(self.block_cache.get(&block_idx).unwrap())
    }

    pub fn is_early_preview_ready(&self) -> bool {
        self.early_preview_ready
    }
}

impl LogSource for SftpLogReader {
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
        self.line_count
    }

    fn file_size(&self) -> u64 {
        self.file_size
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        if idx >= self.line_count {
            return Err(format!("Line {} out of bounds ({})", idx, self.line_count));
        }

        // Find closest checkpoint <= idx
        let mut cp_line = 0;
        let mut cp_offset = 0;
        for &(line, offset) in &self.checkpoints {
            if line <= idx {
                cp_line = line;
                cp_offset = offset;
            } else {
                break;
            }
        }

        let mut current_line = cp_line;
        let mut current_offset = cp_offset;

        while current_offset < self.file_size {
            let block_idx = current_offset / CHUNK_SIZE as u64;
            let offset_in_block = (current_offset % CHUNK_SIZE as u64) as usize;
            let block = self.fetch_block(block_idx)?.to_vec();

            if offset_in_block >= block.len() {
                current_offset = (block_idx + 1) * CHUNK_SIZE as u64;
                continue;
            }

            let mut start_pos = offset_in_block;
            let mut advanced = false;
            while start_pos < block.len() {
                if let Some(nl_pos) = block[start_pos..].iter().position(|&b| b == b'\n') {
                    let end_pos = start_pos + nl_pos;
                    if current_line == idx {
                        let line_slice = &block[start_pos..end_pos];
                        return Ok(String::from_utf8_lossy(line_slice)
                            .trim_end_matches('\r')
                            .to_string());
                    }
                    current_line += 1;
                    start_pos = end_pos + 1;
                    current_offset = block_idx * CHUNK_SIZE as u64 + start_pos as u64;
                    advanced = true;
                } else {
                    // No newline in remainder of block
                    if current_line == idx {
                        let mut line_bytes = block[start_pos..].to_vec();
                        let mut next_offset = (block_idx + 1) * CHUNK_SIZE as u64;
                        while next_offset < self.file_size {
                            let next_block_idx = next_offset / CHUNK_SIZE as u64;
                            let next_block = self.fetch_block(next_block_idx)?.to_vec();
                            if let Some(nl) = next_block.iter().position(|&b| b == b'\n') {
                                line_bytes.extend_from_slice(&next_block[..nl]);
                                break;
                            } else {
                                line_bytes.extend_from_slice(&next_block);
                                next_offset += next_block.len() as u64;
                            }
                        }
                        return Ok(String::from_utf8_lossy(&line_bytes)
                            .trim_end_matches('\r')
                            .to_string());
                    }
                    current_offset = (block_idx + 1) * CHUNK_SIZE as u64;
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                current_offset = (block_idx + 1) * CHUNK_SIZE as u64;
            }
        }

        Ok(String::new())
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
        let stat = self.transport.stat(&self.path)?;
        if stat.size > self.file_size {
            let old_size = self.file_size;
            let new_size = stat.size;
            self.file_size = new_size;
            self.mtime = stat.mtime;

            // Invalidate the boundary block in cache
            self.block_cache.remove(&(old_size / CHUNK_SIZE as u64));

            // Fetch newly appended bytes
            let append_len = (new_size - old_size) as usize;
            let appended_data = self
                .transport
                .read_chunk(&self.path, old_size, append_len)?;
            let mut new_lines = 0;
            for (i, &b) in appended_data.iter().enumerate() {
                if b == b'\n' {
                    new_lines += 1;
                    let total_line = self.line_count + new_lines;
                    if total_line.is_multiple_of(SPARSE_CHECKPOINT_STRIDE) {
                        self.checkpoints
                            .push((total_line, old_size + (i + 1) as u64));
                    }
                }
            }
            self.line_count += new_lines;
            Ok(UpdateEvent::Appended { new_lines })
        } else if stat.size < self.file_size {
            // File truncated or copy-truncate rotation occurred
            self.generation += 1;
            self.file_size = stat.size;
            self.mtime = stat.mtime;
            self.block_cache.clear();
            self.index_initial_data()?;
            Ok(UpdateEvent::Truncated)
        } else {
            Ok(UpdateEvent::NoChange)
        }
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        if idx >= self.line_count {
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
        SourceType::SftpFile
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            can_read: true,
            can_random_access: true,
            can_monitor: true,
            can_discover_files: true,
            can_detect_rotation: true,
            can_reconnect: true,
            can_resume: true,
            has_stable_identity: true,
            is_live_append: true,
            is_end_of_stream: false,
        }
    }

    fn identity(&self) -> SourceIdentity {
        SourceIdentity::new_remote(
            &self.profile_id,
            &self.host_fingerprint,
            &self.path,
            self.generation,
            self.file_size,
        )
    }

    fn status(&self) -> SourceStatus {
        self.status.clone()
    }
}
