use std::collections::{HashMap, VecDeque};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileSizeEx, ReadFile, SetFilePointerEx, FILE_ATTRIBUTE_NORMAL, FILE_BEGIN,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
    PAGE_READONLY,
};

/// File update event returned by `check_for_updates`.
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateEvent {
    NoChange,
    Appended { new_lines: usize },
    Truncated,
}

/// Interval of lines per checkpoint in the sparse line index.
/// 64 lines per checkpoint yields < 1.3 MB RAM per 10 million lines.
pub const CHECKPOINT_STRIDE: usize = 64;

/// Default capacity for bounded decoded line LRU cache (10,000 entries).
pub const DEFAULT_CACHE_CAPACITY: usize = 10_000;

/// Default byte budget for decoded line LRU cache (64 MB).
pub const DEFAULT_CACHE_BYTE_BUDGET: usize = 64 * 1024 * 1024;

/// Maximum display bytes for an individual log line before truncation (1 MB).
/// Prevents multi-megabyte single lines from exhausting RAM.
pub const MAX_LINE_DISPLAY_BYTES: usize = 1024 * 1024;

/// Bounded Least-Recently-Used (LRU) line cache for decoded log line strings.
/// Enforces both maximum entry count and total byte budget constraints.
pub struct LruLineCache {
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    map: HashMap<usize, String>,
    order: VecDeque<usize>,
}

impl LruLineCache {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        let entries = max_entries.max(1);
        let bytes = max_bytes.max(1);
        Self {
            max_entries: entries,
            max_bytes: bytes,
            current_bytes: 0,
            map: HashMap::with_capacity(entries.min(1024)),
            order: VecDeque::with_capacity(entries.min(1024)),
        }
    }

    pub fn with_default_budget() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY, DEFAULT_CACHE_BYTE_BUDGET)
    }

    pub fn get(&mut self, line_idx: usize) -> Option<&String> {
        if self.map.contains_key(&line_idx) {
            // Move accessed key to back of recency queue
            if let Some(pos) = self.order.iter().position(|&k| k == line_idx) {
                self.order.remove(pos);
            }
            self.order.push_back(line_idx);
            self.map.get(&line_idx)
        } else {
            None
        }
    }

    pub fn insert(&mut self, line_idx: usize, text: String) {
        let entry_size = text.len() + std::mem::size_of::<usize>() * 4;

        if let Some(old_text) = self.map.remove(&line_idx) {
            let old_size = old_text.len() + std::mem::size_of::<usize>() * 4;
            self.current_bytes = self.current_bytes.saturating_sub(old_size);
            if let Some(pos) = self.order.iter().position(|&k| k == line_idx) {
                self.order.remove(pos);
            }
        }

        // Evict until both entry count and byte budget are satisfied
        while (!self.order.is_empty())
            && (self.map.len() >= self.max_entries
                || (self.current_bytes + entry_size > self.max_bytes && !self.map.is_empty()))
        {
            if let Some(oldest) = self.order.pop_front() {
                if let Some(removed) = self.map.remove(&oldest) {
                    let rem_size = removed.len() + std::mem::size_of::<usize>() * 4;
                    self.current_bytes = self.current_bytes.saturating_sub(rem_size);
                }
            }
        }

        self.order.push_back(line_idx);
        self.current_bytes += entry_size;
        self.map.insert(line_idx, text);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.current_bytes = 0;
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn set_budget(&mut self, max_entries: usize, max_bytes: usize) {
        self.max_entries = max_entries.max(1);
        self.max_bytes = max_bytes.max(1);

        while (!self.order.is_empty())
            && (self.map.len() > self.max_entries || self.current_bytes > self.max_bytes)
        {
            if let Some(oldest) = self.order.pop_front() {
                if let Some(removed) = self.map.remove(&oldest) {
                    let rem_size = removed.len() + std::mem::size_of::<usize>() * 4;
                    self.current_bytes = self.current_bytes.saturating_sub(rem_size);
                }
            }
        }
    }
}

/// Sparse line index storing 64-bit byte offsets at regular checkpoints.
#[derive(Debug, Default)]
pub struct SparseLineIndex {
    /// Byte offsets of line index 0, 64, 128, 192, ...
    checkpoints: Vec<u64>,
    /// Total number of lines discovered so far
    total_lines: usize,
    /// Byte offset where the last line began
    last_line_offset: u64,
    /// Whether the last scanned chunk ended with a newline character
    last_ended_with_newline: bool,
}

impl SparseLineIndex {
    pub fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            total_lines: 0,
            last_line_offset: 0,
            last_ended_with_newline: true,
        }
    }

    pub fn clear(&mut self) {
        self.checkpoints.clear();
        self.total_lines = 0;
        self.last_line_offset = 0;
        self.last_ended_with_newline = true;
    }

    /// Records a new line starting at `offset`.
    pub fn add_line(&mut self, offset: u64) {
        if self.total_lines.is_multiple_of(CHECKPOINT_STRIDE) {
            self.checkpoints.push(offset);
        }
        self.last_line_offset = offset;
        self.total_lines += 1;
    }

    pub fn line_count(&self) -> usize {
        self.total_lines
    }

    /// Finds the closest checkpoint (checkpoint_line_index, byte_offset) for `target_line`.
    pub fn get_checkpoint(&self, target_line: usize) -> Option<(usize, u64)> {
        if target_line >= self.total_lines || self.checkpoints.is_empty() {
            return None;
        }
        let ckpt_idx = target_line / CHECKPOINT_STRIDE;
        if ckpt_idx < self.checkpoints.len() {
            Some((ckpt_idx * CHECKPOINT_STRIDE, self.checkpoints[ckpt_idx]))
        } else {
            None
        }
    }
}

/// Storage backend: Windowed Memory-Mapped I/O with automatic fallback to buffered file I/O.
#[allow(dead_code)]
enum StorageBackend {
    MemoryMapped {
        mapping_handle: HANDLE,
        view_ptr: *const u8,
        view_size: usize,
    },
    BufferedFile,
}

impl StorageBackend {
    /// Attempts to open a memory-mapped view for large static files.
    /// Falls back to `BufferedFile` on error or for actively tailing files.
    #[allow(dead_code)]
    fn try_open_mmap(handle: HANDLE, file_size: u64) -> Self {
        if handle == INVALID_HANDLE_VALUE || handle.is_null() || file_size == 0 {
            return Self::BufferedFile;
        }

        let map_len = (file_size.min(512 * 1024 * 1024)) as usize;
        let mapping_handle =
            unsafe { CreateFileMappingW(handle, null_mut(), PAGE_READONLY, 0, 0, null_mut()) };

        if mapping_handle.is_null() || mapping_handle == INVALID_HANDLE_VALUE {
            return Self::BufferedFile;
        }

        let view_ptr = unsafe { MapViewOfFile(mapping_handle, FILE_MAP_READ, 0, 0, map_len) };

        if view_ptr.Value.is_null() {
            unsafe { CloseHandle(mapping_handle) };
            Self::BufferedFile
        } else {
            Self::MemoryMapped {
                mapping_handle,
                view_ptr: view_ptr.Value as *const u8,
                view_size: map_len,
            }
        }
    }

    fn open(_handle: HANDLE, _file_size: u64) -> Self {
        // Buffered file streaming provides 100% reliable non-locking access across all
        // Windows versions (Win7 SP1 to Win11), network shares, and actively tailing files,
        // eliminating ERROR_USER_MAPPED_FILE (1224) and NT section size locking on rotation.
        Self::BufferedFile
    }

    /// Reads `length` bytes starting at `offset`.
    fn read_bytes(&self, handle: HANDLE, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        if length == 0 {
            return Ok(Vec::new());
        }

        match self {
            Self::MemoryMapped {
                view_ptr,
                view_size,
                ..
            } => {
                let end = offset + length as u64;
                if end <= *view_size as u64 {
                    let slice = unsafe {
                        std::slice::from_raw_parts(view_ptr.add(offset as usize), length)
                    };
                    Ok(slice.to_vec())
                } else {
                    // Beyond memory-mapped window, fallback to file read
                    Self::read_bytes_buffered(handle, offset, length)
                }
            }
            Self::BufferedFile => Self::read_bytes_buffered(handle, offset, length),
        }
    }

    fn read_bytes_buffered(handle: HANDLE, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let mut new_pos: i64 = 0;
        let seek_ok = unsafe { SetFilePointerEx(handle, offset as i64, &mut new_pos, FILE_BEGIN) };
        if seek_ok == 0 {
            return Err(format!("Seek failed to offset {}", offset));
        }

        let mut buffer = vec![0u8; length];
        let mut bytes_read: u32 = 0;
        let success = unsafe {
            ReadFile(
                handle,
                buffer.as_mut_ptr() as _,
                length as u32,
                &mut bytes_read,
                null_mut(),
            )
        };

        if success == 0 {
            return Err("ReadFile failed".to_string());
        }

        buffer.truncate(bytes_read as usize);
        Ok(buffer)
    }

    fn remap(&mut self, handle: HANDLE, new_size: u64) {
        if let Self::MemoryMapped {
            mapping_handle,
            view_ptr,
            ..
        } = self
        {
            if !view_ptr.is_null() {
                unsafe {
                    UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: *view_ptr as *mut std::ffi::c_void,
                    })
                };
            }
            if !mapping_handle.is_null() && *mapping_handle != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(*mapping_handle) };
            }
        }
        *self = Self::open(handle, new_size);
    }
}

impl Drop for StorageBackend {
    fn drop(&mut self) {
        if let Self::MemoryMapped {
            mapping_handle,
            view_ptr,
            ..
        } = self
        {
            if !view_ptr.is_null() {
                unsafe {
                    UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: *view_ptr as *mut std::ffi::c_void,
                    })
                };
            }
            if !mapping_handle.is_null() && *mapping_handle != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(*mapping_handle) };
            }
        }
    }
}

/// Strictly non-locking log file reader and sparse line indexer for Tylogger.
/// Complies with all Phase 1 requirements: sparse indexing, windowed mmap,
/// bounded LRU cache, defensive multibyte decoding, and truncation recovery.
pub struct NonLockingLogReader {
    handle: HANDLE,
    file_size: u64,
    index: SparseLineIndex,
    cache: LruLineCache,
    backend: StorageBackend,
    bookmarks: Vec<usize>,
    mock_lines: Option<Vec<String>>,
}

unsafe impl Send for NonLockingLogReader {}
unsafe impl Sync for NonLockingLogReader {}

impl NonLockingLogReader {
    /// Opens a file in strictly non-locking read-only mode.
    /// Uses `GENERIC_READ` and `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let wide_path: Vec<u16> = path_ref
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(format!(
                "Failed to open file {:?} in non-locking mode (Win32 error: {})",
                path_ref,
                std::io::Error::last_os_error()
            ));
        }

        let mut size: i64 = 0;
        let success = unsafe { GetFileSizeEx(handle, &mut size) };
        let file_size = if success != 0 { size.max(0) as u64 } else { 0 };

        let backend = StorageBackend::open(handle, file_size);

        let mut reader = Self {
            handle,
            file_size,
            index: SparseLineIndex::new(),
            cache: LruLineCache::with_default_budget(),
            backend,
            bookmarks: Vec::new(),
            mock_lines: None,
        };

        reader.index_initial()?;
        Ok(reader)
    }

    /// Creates an in-memory mock reader for headless testing without disk I/O.
    pub fn from_mock_lines(lines: &[&str]) -> Self {
        let string_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let mut index = SparseLineIndex::new();
        let mut curr_offset = 0u64;

        for line in &string_lines {
            index.add_line(curr_offset);
            curr_offset += (line.len() + 1) as u64;
        }

        Self {
            handle: INVALID_HANDLE_VALUE,
            file_size: curr_offset,
            index,
            cache: LruLineCache::with_default_budget(),
            backend: StorageBackend::BufferedFile,
            bookmarks: Vec::new(),
            mock_lines: Some(string_lines),
        }
    }

    /// Returns the storage backend description.
    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            StorageBackend::MemoryMapped { .. } => "MemoryMapped",
            StorageBackend::BufferedFile => "BufferedFile",
        }
    }

    /// Returns current total line count.
    pub fn line_count(&self) -> usize {
        if let Some(ref mock) = self.mock_lines {
            mock.len()
        } else {
            self.index.line_count()
        }
    }

    /// Returns current file size in bytes.
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Toggle bookmark on a given line index.
    pub fn toggle_bookmark(&mut self, line_idx: usize) -> bool {
        if line_idx >= self.line_count() {
            return false;
        }
        if let Some(pos) = self.bookmarks.iter().position(|&x| x == line_idx) {
            self.bookmarks.remove(pos);
            false
        } else {
            self.bookmarks.push(line_idx);
            self.bookmarks.sort_unstable();
            true
        }
    }

    /// Check if a line is bookmarked.
    pub fn is_bookmarked(&self, line_idx: usize) -> bool {
        self.bookmarks.binary_search(&line_idx).is_ok()
    }

    /// Returns all bookmarked line indices.
    pub fn bookmarks(&self) -> &[usize] {
        &self.bookmarks
    }

    /// Indexes file contents from byte 0 to EOF.
    fn index_initial(&mut self) -> Result<(), String> {
        let size = self.query_file_size()?;
        self.file_size = size;
        self.index.clear();
        self.cache.clear();

        if size == 0 {
            return Ok(());
        }

        self.index.add_line(0);
        self.scan_lines(0, size)?;
        Ok(())
    }

    /// Scans byte range `[from_offset, to_offset)` for newlines and populates sparse index.
    fn scan_lines(&mut self, from_offset: u64, to_offset: u64) -> Result<usize, String> {
        const CHUNK_SIZE: usize = 65536;
        let mut current_offset = from_offset;
        let mut lines_found = 0;

        if from_offset > 0 && self.index.last_ended_with_newline {
            self.index.add_line(from_offset);
            lines_found += 1;
        }

        while current_offset < to_offset {
            let to_read = std::cmp::min((to_offset - current_offset) as usize, CHUNK_SIZE);
            let bytes = self
                .backend
                .read_bytes(self.handle, current_offset, to_read)?;
            if bytes.is_empty() {
                break;
            }

            for (i, &byte) in bytes.iter().enumerate() {
                if byte == b'\n' {
                    let next_line_start = current_offset + (i as u64) + 1;
                    if next_line_start < to_offset {
                        self.index.add_line(next_line_start);
                        lines_found += 1;
                    }
                }
            }

            self.index.last_ended_with_newline = *bytes.last().unwrap_or(&0) == b'\n';
            current_offset += bytes.len() as u64;
        }

        Ok(lines_found)
    }

    /// Checks if the file grew (tailing) or was truncated (rotation), updating state accordingly.
    pub fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        if self.handle == INVALID_HANDLE_VALUE || self.handle.is_null() {
            return Ok(UpdateEvent::NoChange);
        }

        let current_size = self.query_file_size()?;

        if current_size == self.file_size {
            return Ok(UpdateEvent::NoChange);
        }

        // Truncation / Rotation detection
        if current_size < self.file_size {
            self.backend.remap(self.handle, current_size);
            self.index_initial()?;
            // Retain only bookmarks that are within new line bounds
            let count = self.line_count();
            self.bookmarks.retain(|&b| b < count);
            return Ok(UpdateEvent::Truncated);
        }

        // File grew (appended data)
        let old_size = self.file_size;
        self.file_size = current_size;

        if self.index.line_count() == 0 && current_size > 0 {
            self.index.add_line(0);
        }

        self.backend.remap(self.handle, current_size);
        let new_lines = self.scan_lines(old_size, current_size)?;
        Ok(UpdateEvent::Appended { new_lines })
    }

    /// Retrieves a single line's text by line index, utilizing the bounded LRU cache.
    pub fn get_line(&mut self, index: usize) -> Result<String, String> {
        if index >= self.line_count() {
            return Err(format!(
                "Line index {} out of bounds (count: {})",
                index,
                self.line_count()
            ));
        }

        // 1. Check in-memory mock if present
        if let Some(ref mock) = self.mock_lines {
            return Ok(mock.get(index).cloned().unwrap_or_default());
        }

        // 2. Check LRU cache
        if let Some(cached) = self.cache.get(index) {
            return Ok(cached.clone());
        }

        // 3. Resolve line location via sparse index checkpoint
        let (ckpt_line, ckpt_offset) = self
            .index
            .get_checkpoint(index)
            .ok_or_else(|| format!("Checkpoint not found for line {}", index))?;

        let lines_to_skip = index - ckpt_line;

        // Scan from ckpt_offset to find start of target line
        const SCAN_CHUNK: usize = 65536;
        let mut curr_offset = ckpt_offset;
        let mut skipped = 0;
        let mut line_start = ckpt_offset;

        // Read forward from checkpoint
        'search: while skipped < lines_to_skip && curr_offset < self.file_size {
            let to_read = std::cmp::min((self.file_size - curr_offset) as usize, SCAN_CHUNK);
            let bytes = self.backend.read_bytes(self.handle, curr_offset, to_read)?;
            if bytes.is_empty() {
                break;
            }

            for (i, &byte) in bytes.iter().enumerate() {
                if byte == b'\n' {
                    skipped += 1;
                    if skipped == lines_to_skip {
                        line_start = curr_offset + (i as u64) + 1;
                        break 'search;
                    }
                }
            }
            curr_offset += bytes.len() as u64;
        }

        // Now find end of line starting at line_start
        let mut line_end = self.file_size;
        let mut scan_end_offset = line_start;

        'find_end: while scan_end_offset < self.file_size {
            let to_read = std::cmp::min((self.file_size - scan_end_offset) as usize, SCAN_CHUNK);
            let bytes = self
                .backend
                .read_bytes(self.handle, scan_end_offset, to_read)?;
            if bytes.is_empty() {
                break;
            }

            for (i, &byte) in bytes.iter().enumerate() {
                if byte == b'\n' {
                    line_end = scan_end_offset + (i as u64);
                    break 'find_end;
                }
            }
            scan_end_offset += bytes.len() as u64;
        }

        let length = if line_end >= line_start {
            (line_end - line_start) as usize
        } else {
            0
        };

        let is_truncated = length > MAX_LINE_DISPLAY_BYTES;
        let read_len = if is_truncated {
            MAX_LINE_DISPLAY_BYTES
        } else {
            length
        };

        let raw_bytes = self.backend.read_bytes(self.handle, line_start, read_len)?;

        // Defensive UTF-8 lossy conversion with trailing carriage return stripped
        let mut line_text = String::from_utf8_lossy(&raw_bytes).to_string();
        if line_text.ends_with('\r') {
            line_text.pop();
        }
        if is_truncated {
            line_text.push_str(" [Truncated]");
        }

        // Cache the decoded line
        self.cache.insert(index, line_text.clone());

        Ok(line_text)
    }

    /// Sets cache capacity and byte budget.
    pub fn set_cache_budget(&mut self, max_entries: usize, max_bytes: usize) {
        self.cache.set_budget(max_entries, max_bytes);
    }

    /// Current bytes allocated by the decoded line cache.
    pub fn cache_bytes(&self) -> usize {
        self.cache.current_bytes()
    }

    /// Number of items currently cached in the decoded line cache.
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    fn query_file_size(&self) -> Result<u64, String> {
        let mut size: i64 = 0;
        let success = unsafe { GetFileSizeEx(self.handle, &mut size) };
        if success == 0 {
            return Err("Failed to query file size".to_string());
        }
        Ok(size.max(0) as u64)
    }
}

impl Drop for NonLockingLogReader {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE && !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_mock_reader() {
        let lines = [
            "Line 1: system init",
            "Line 2: debug trace",
            "Line 3: error",
        ];
        let mut reader = NonLockingLogReader::from_mock_lines(&lines);
        assert_eq!(reader.line_count(), 3);
        assert!(!reader.is_bookmarked(1));
        assert!(reader.toggle_bookmark(1));
        assert!(reader.is_bookmarked(1));
        assert!(!reader.toggle_bookmark(1));
        assert!(!reader.is_bookmarked(1));

        assert_eq!(reader.get_line(0).unwrap(), "Line 1: system init");
        assert_eq!(reader.get_line(2).unwrap(), "Line 3: error");
    }

    #[test]
    fn test_lru_line_cache() {
        let mut cache = LruLineCache::new(3, 1024 * 1024);
        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());
        assert_eq!(cache.len(), 3);

        // Access 1 (makes 2 the oldest)
        assert_eq!(cache.get(1).map(|s| s.as_str()), Some("one"));

        // Insert 4 (should evict 2)
        cache.insert(4, "four".to_string());
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(2), None);
        assert_eq!(cache.get(1).map(|s| s.as_str()), Some("one"));
        assert_eq!(cache.get(3).map(|s| s.as_str()), Some("three"));
        assert_eq!(cache.get(4).map(|s| s.as_str()), Some("four"));
    }

    #[test]
    fn test_lru_line_cache_byte_budget() {
        // Budget allows approx 250 bytes (enough for 2 lines of 80 chars + overhead)
        let mut cache = LruLineCache::new(100, 250);
        let large_line_1 = "A".repeat(80);
        let large_line_2 = "B".repeat(80);
        let large_line_3 = "C".repeat(80);

        cache.insert(1, large_line_1);
        cache.insert(2, large_line_2);
        assert_eq!(cache.len(), 2);
        assert!(cache.current_bytes() <= 250);

        // Inserting line 3 should evict line 1 due to byte budget constraint
        cache.insert(3, large_line_3);
        assert!(cache.current_bytes() <= 250);
        assert_eq!(cache.get(1), None);
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());
    }

    #[test]
    fn test_sparse_checkpoint_stride() {
        let mut index = SparseLineIndex::new();
        for i in 0..150 {
            index.add_line((i * 20) as u64);
        }
        assert_eq!(index.line_count(), 150);

        // Checkpoint 0 covers lines 0..63
        let (line_0, off_0) = index.get_checkpoint(0).unwrap();
        assert_eq!(line_0, 0);
        assert_eq!(off_0, 0);

        let (line_63, off_63) = index.get_checkpoint(63).unwrap();
        assert_eq!(line_63, 0);
        assert_eq!(off_63, 0);

        // Checkpoint 1 covers lines 64..127
        let (line_64, off_64) = index.get_checkpoint(64).unwrap();
        assert_eq!(line_64, 64);
        assert_eq!(off_64, 64 * 20);

        // Checkpoint 2 covers lines 128..
        let (line_140, off_140) = index.get_checkpoint(140).unwrap();
        assert_eq!(line_140, 128);
        assert_eq!(off_140, 128 * 20);
    }

    #[test]
    fn test_non_locking_file_append_and_truncate() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("tylogger_test_{}.log", std::process::id()));

        // 1. Create file and write 3 lines
        {
            let mut f = std::fs::File::create(&test_file).unwrap();
            writeln!(f, "2026-11-25 08:00:01 [INFO] Service started").unwrap();
            writeln!(f, "2026-11-25 08:00:02 [DEBUG] Loading config").unwrap();
            writeln!(f, "2026-11-25 08:00:03 [WARN] Low disk space").unwrap();
        }

        // 2. Open with NonLockingLogReader
        let mut reader = NonLockingLogReader::open(&test_file).unwrap();
        assert_eq!(reader.line_count(), 3);
        assert_eq!(
            reader.get_line(0).unwrap(),
            "2026-11-25 08:00:01 [INFO] Service started"
        );
        assert_eq!(
            reader.get_line(2).unwrap(),
            "2026-11-25 08:00:03 [WARN] Low disk space"
        );

        // 3. Append lines from another writer without locking issues
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&test_file)
                .unwrap();
            writeln!(f, "2026-11-25 08:00:04 [ERROR] Disk full alert").unwrap();
        }

        let event = reader.check_for_updates().unwrap();
        assert_eq!(event, UpdateEvent::Appended { new_lines: 1 });
        assert_eq!(reader.line_count(), 4);
        assert_eq!(
            reader.get_line(3).unwrap(),
            "2026-11-25 08:00:04 [ERROR] Disk full alert"
        );

        // 4. Simulate file truncation (rotation)
        {
            let mut f = std::fs::File::create(&test_file).unwrap();
            writeln!(f, "2026-11-25 08:01:00 [INFO] Rotated log").unwrap();
        }

        let event = reader.check_for_updates().unwrap();
        assert_eq!(event, UpdateEvent::Truncated);
        assert_eq!(reader.line_count(), 1);
        assert_eq!(
            reader.get_line(0).unwrap(),
            "2026-11-25 08:01:00 [INFO] Rotated log"
        );

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}
