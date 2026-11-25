// src/core/tools/capture.rs
//
// Captured External Tool Output (Stdout/Stderr) Virtual LogSource for Tylogger
// Supports live streaming lines, search, filter, bookmark, and graceful stop.

use crate::core::reader::UpdateEvent;
use crate::core::source::{allocate_source_id, LogSource, SourceLineRef};
use std::path::Path;

pub struct CapturedOutputSource {
    source_id: u32,
    generation: u64,
    display_name: String,
    lines: Vec<String>,
    bookmarks: Vec<usize>,
    pub is_running: bool,
    pub exit_code: Option<i32>,
    pub max_bytes: usize,
    pub current_bytes: usize,
}

impl CapturedOutputSource {
    pub fn new(display_name: &str, max_bytes: usize) -> Self {
        Self {
            source_id: allocate_source_id(),
            generation: 1,
            display_name: display_name.to_string(),
            lines: Vec::new(),
            bookmarks: Vec::new(),
            is_running: true,
            exit_code: None,
            max_bytes: max_bytes.max(64 * 1024),
            current_bytes: 0,
        }
    }

    pub fn append_stdout(&mut self, text: &str) {
        if self.current_bytes + text.len() > self.max_bytes {
            return;
        }
        self.lines.push(text.to_string());
        self.current_bytes += text.len();
        self.generation += 1;
    }

    pub fn append_stderr(&mut self, text: &str) {
        if self.current_bytes + text.len() > self.max_bytes {
            return;
        }
        self.lines.push(format!("[STDERR] {}", text));
        self.current_bytes += text.len();
        self.generation += 1;
    }

    pub fn finish(&mut self, exit_code: i32) {
        self.is_running = false;
        self.exit_code = Some(exit_code);
    }
}

impl LogSource for CapturedOutputSource {
    fn source_id(&self) -> u32 {
        self.source_id
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn source_path(&self) -> Option<&Path> {
        None
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn file_size(&self) -> u64 {
        self.current_bytes as u64
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.lines
            .get(idx)
            .cloned()
            .ok_or_else(|| format!("Line index {} out of bounds", idx))
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        SourceLineRef {
            source_id: self.source_id,
            generation: self.generation,
            line_idx: idx,
            byte_offset: 0,
        }
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        Ok(UpdateEvent::NoChange)
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        if let Ok(pos) = self.bookmarks.binary_search(&idx) {
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

    fn is_filtered_source(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_output_virtual_source() {
        let mut src = CapturedOutputSource::new("Ping Process Output", 1024 * 1024);
        src.append_stdout("Pinging 127.0.0.1 with 32 bytes of data:");
        src.append_stdout("Reply from 127.0.0.1: bytes=32 time<1ms TTL=128");
        src.append_stderr("Warning: host unreachable");
        src.finish(0);

        assert_eq!(src.line_count(), 3);
        assert!(!src.is_running);
        assert_eq!(src.exit_code, Some(0));
        assert!(src.get_line(2).unwrap().contains("[STDERR]"));
    }
}
