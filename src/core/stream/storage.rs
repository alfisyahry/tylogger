use std::collections::VecDeque;
use std::path::Path;

use crate::core::reader::UpdateEvent;
use crate::core::source::{
    allocate_source_id, LogSource, SourceCapabilities, SourceIdentity, SourceLineRef, SourceStatus,
    SourceType,
};

pub struct BoundedStreamStorage {
    id: u32,
    generation: u64,
    title: String,
    profile_id: String,
    template_name: String,
    ring_buffer: VecDeque<String>,
    max_memory_lines: usize,
    max_memory_bytes: usize,
    current_memory_bytes: usize,
    total_received: u64,
    history_evicted: bool,
    bookmarks: Vec<usize>,
    file_size_accumulated: u64,
}

impl BoundedStreamStorage {
    pub const DEFAULT_MAX_STREAM_BYTES: usize = 64 * 1024 * 1024; // 64 MB

    pub fn new(profile_id: &str, template_name: &str, max_memory_lines: usize) -> Self {
        Self::with_byte_budget(
            profile_id,
            template_name,
            max_memory_lines,
            Self::DEFAULT_MAX_STREAM_BYTES,
        )
    }

    pub fn with_byte_budget(
        profile_id: &str,
        template_name: &str,
        max_memory_lines: usize,
        max_memory_bytes: usize,
    ) -> Self {
        Self {
            id: allocate_source_id(),
            generation: 1,
            title: format!("SSH Stream: {} ({})", template_name, profile_id),
            profile_id: profile_id.to_string(),
            template_name: template_name.to_string(),
            ring_buffer: VecDeque::with_capacity(max_memory_lines.min(50000)),
            max_memory_lines,
            max_memory_bytes: max_memory_bytes.max(1),
            current_memory_bytes: 0,
            total_received: 0,
            history_evicted: false,
            bookmarks: Vec::new(),
            file_size_accumulated: 0,
        }
    }

    pub fn set_max_memory_bytes(&mut self, max_bytes: usize) {
        self.max_memory_bytes = max_bytes.max(1);
        while self.current_memory_bytes > self.max_memory_bytes && !self.ring_buffer.is_empty() {
            if let Some(removed) = self.ring_buffer.pop_front() {
                self.current_memory_bytes =
                    self.current_memory_bytes.saturating_sub(removed.len() + 1);
                self.history_evicted = true;
                self.bookmarks.retain(|&idx| idx > 0);
                for b in self.bookmarks.iter_mut() {
                    *b -= 1;
                }
            }
        }
    }

    pub fn current_memory_bytes(&self) -> usize {
        self.current_memory_bytes
    }

    pub fn max_memory_bytes(&self) -> usize {
        self.max_memory_bytes
    }

    pub fn append_line(&mut self, line: &str) {
        let line_len = line.len() + 1;
        self.file_size_accumulated += line_len as u64;
        self.total_received += 1;

        // Evict if line count limit or real byte-based budget is exceeded (AG-17, RSK-06)
        while !self.ring_buffer.is_empty()
            && (self.ring_buffer.len() >= self.max_memory_lines
                || self.current_memory_bytes + line_len > self.max_memory_bytes)
        {
            if let Some(removed) = self.ring_buffer.pop_front() {
                self.current_memory_bytes =
                    self.current_memory_bytes.saturating_sub(removed.len() + 1);
                self.history_evicted = true;
                // Adjust bookmarks on eviction
                self.bookmarks.retain(|&idx| idx > 0);
                for b in self.bookmarks.iter_mut() {
                    *b -= 1;
                }
            }
        }

        self.current_memory_bytes += line_len;
        self.ring_buffer.push_back(line.to_string());
    }

    pub fn append_lines(&mut self, lines: &[&str]) {
        for line in lines {
            self.append_line(line);
        }
    }

    pub fn is_history_evicted(&self) -> bool {
        self.history_evicted
    }

    pub fn total_lines_received(&self) -> u64 {
        self.total_received
    }

    pub fn retained_line_count(&self) -> usize {
        self.ring_buffer.len()
    }
}

impl LogSource for BoundedStreamStorage {
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
        self.ring_buffer.len()
    }

    fn file_size(&self) -> u64 {
        self.file_size_accumulated
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.ring_buffer
            .get(idx)
            .cloned()
            .ok_or_else(|| format!("Line {} out of bounds ({})", idx, self.ring_buffer.len()))
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
        if idx >= self.ring_buffer.len() {
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
        SourceType::SshStream
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            can_read: true,
            can_random_access: true,
            can_monitor: true,
            can_discover_files: false,
            can_detect_rotation: false,
            can_reconnect: true,
            can_resume: true,
            has_stable_identity: false,
            is_live_append: true,
            is_end_of_stream: false,
        }
    }

    fn identity(&self) -> SourceIdentity {
        SourceIdentity::new_stream(&self.profile_id, &self.template_name, self.generation)
    }

    fn is_continuous(&self) -> bool {
        true
    }

    fn status(&self) -> SourceStatus {
        SourceStatus::Active
    }
}
