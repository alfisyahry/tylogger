use std::path::Path;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileSizeEx, ReadFile, SetFilePointerEx, FILE_ATTRIBUTE_NORMAL, FILE_BEGIN,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

/// File update event returned by `check_for_updates`
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateEvent {
    NoChange,
    Appended { new_lines: usize },
    Truncated,
}

/// Strictly non-locking log file reader and sparse line indexer.
/// Guarantees that external log writers are never blocked.
pub struct NonLockingLogReader {
    handle: HANDLE,
    file_size: u64,
    /// Byte offsets of the start of each line
    line_offsets: Vec<u64>,
    /// Whether the last scanned byte in the file was a newline
    last_ended_with_newline: bool,
    /// Bookmarked line indices
    bookmarks: Vec<usize>,
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

        if handle == INVALID_HANDLE_VALUE {
            return Err(format!(
                "Failed to open file {:?} in non-locking mode (Win32 error: {})",
                path_ref,
                std::io::Error::last_os_error()
            ));
        }

        let mut reader = Self {
            handle,
            file_size: 0,
            line_offsets: Vec::new(),
            last_ended_with_newline: true,
            bookmarks: Vec::new(),
        };

        reader.index_initial()?;
        Ok(reader)
    }

    /// Creates an in-memory mock reader for testing without disk I/O.
    pub fn from_mock_lines(lines: &[&str]) -> Self {
        let mut offsets = Vec::new();
        let mut curr_offset = 0u64;
        for line in lines {
            offsets.push(curr_offset);
            curr_offset += (line.len() + 1) as u64; // assuming \n
        }

        Self {
            handle: INVALID_HANDLE_VALUE,
            file_size: curr_offset,
            line_offsets: offsets,
            last_ended_with_newline: true,
            bookmarks: Vec::new(),
        }
    }

    /// Returns the current total line count.
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    /// Returns the current file size in bytes.
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Toggle bookmark on a given line index.
    pub fn toggle_bookmark(&mut self, line_idx: usize) -> bool {
        if line_idx >= self.line_offsets.len() {
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

    /// Indexes the initial file contents from byte 0 to EOF.
    fn index_initial(&mut self) -> Result<(), String> {
        let size = self.query_file_size()?;
        self.file_size = size;
        self.line_offsets.clear();
        self.last_ended_with_newline = true;

        if size == 0 {
            return Ok(());
        }

        // First line always starts at byte 0
        self.line_offsets.push(0);
        self.scan_lines(0, size)?;
        Ok(())
    }

    /// Scans byte range [from_offset, to_offset) and records newline offsets.
    fn scan_lines(&mut self, from_offset: u64, to_offset: u64) -> Result<usize, String> {
        const CHUNK_SIZE: usize = 65536;
        let mut buffer = [0u8; CHUNK_SIZE];
        let mut current_offset = from_offset;
        let mut lines_found = 0;

        if from_offset > 0 && self.last_ended_with_newline {
            self.line_offsets.push(from_offset);
            lines_found += 1;
        }

        self.seek(current_offset)?;

        while current_offset < to_offset {
            let to_read = std::cmp::min((to_offset - current_offset) as usize, CHUNK_SIZE);
            let mut bytes_read: u32 = 0;

            let success = unsafe {
                ReadFile(
                    self.handle,
                    buffer.as_mut_ptr() as _,
                    to_read as u32,
                    &mut bytes_read,
                    null_mut(),
                )
            };

            if success == 0 || bytes_read == 0 {
                break;
            }

            for (i, &byte) in buffer.iter().enumerate().take(bytes_read as usize) {
                if byte == b'\n' {
                    let next_line_start = current_offset + (i as u64) + 1;
                    if next_line_start < to_offset {
                        self.line_offsets.push(next_line_start);
                        lines_found += 1;
                    }
                }
            }

            if bytes_read > 0 {
                self.last_ended_with_newline = buffer[(bytes_read - 1) as usize] == b'\n';
            }

            current_offset += bytes_read as u64;
        }

        Ok(lines_found)
    }

    /// Checks if the file grew or was truncated, updating the index accordingly.
    pub fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        if self.handle == INVALID_HANDLE_VALUE {
            return Ok(UpdateEvent::NoChange);
        }

        let current_size = self.query_file_size()?;

        if current_size == self.file_size {
            return Ok(UpdateEvent::NoChange);
        }

        // Detect truncation (e.g. log rotation or file clear)
        if current_size < self.file_size {
            self.index_initial()?;
            return Ok(UpdateEvent::Truncated);
        }

        // File grew (appended data)
        let old_size = self.file_size;
        self.file_size = current_size;

        // If the file was previously empty, start line at 0
        if self.line_offsets.is_empty() && current_size > 0 {
            self.line_offsets.push(0);
        }

        let new_lines = self.scan_lines(old_size, current_size)?;
        Ok(UpdateEvent::Appended { new_lines })
    }

    /// Retrieves a single line's text by line index.
    pub fn get_line(&self, index: usize) -> Result<String, String> {
        if index >= self.line_offsets.len() {
            return Err(format!(
                "Line index {} out of bounds (count: {})",
                index,
                self.line_offsets.len()
            ));
        }

        if self.handle == INVALID_HANDLE_VALUE {
            return Ok(format!("[Mock Line {}] Sample log payload data", index + 1));
        }

        let start_offset = self.line_offsets[index];
        let end_offset = if index + 1 < self.line_offsets.len() {
            self.line_offsets[index + 1]
        } else {
            self.file_size
        };

        if start_offset >= end_offset {
            return Ok(String::new());
        }

        let length = (end_offset - start_offset) as usize;
        let mut buffer = vec![0u8; length];
        let mut bytes_read: u32 = 0;

        self.seek(start_offset)?;

        let success = unsafe {
            ReadFile(
                self.handle,
                buffer.as_mut_ptr() as _,
                length as u32,
                &mut bytes_read,
                null_mut(),
            )
        };

        if success == 0 {
            return Err("Failed to read line bytes from file".to_string());
        }

        buffer.truncate(bytes_read as usize);

        // Strip trailing \r and \n
        while let Some(&last) = buffer.last() {
            if last == b'\r' || last == b'\n' {
                buffer.pop();
            } else {
                break;
            }
        }

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    fn query_file_size(&self) -> Result<u64, String> {
        let mut size: i64 = 0;
        let success = unsafe { GetFileSizeEx(self.handle, &mut size) };
        if success == 0 {
            return Err("Failed to query file size".to_string());
        }
        Ok(size as u64)
    }

    fn seek(&self, offset: u64) -> Result<(), String> {
        let mut new_pos: i64 = 0;
        let success =
            unsafe { SetFilePointerEx(self.handle, offset as i64, &mut new_pos, FILE_BEGIN) };
        if success == 0 {
            return Err(format!("Failed to seek to offset {}", offset));
        }
        Ok(())
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

use std::os::windows::ffi::OsStrExt;

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
    }

    #[test]
    fn test_non_locking_file_append_and_truncate() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("logexpert_test_{}.log", std::process::id()));

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
