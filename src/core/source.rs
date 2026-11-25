use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::core::reader::{NonLockingLogReader, UpdateEvent};

static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(1);

pub fn allocate_source_id() -> u32 {
    NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Stable reference to a line in a log source.
/// Contains source identity, generation, line index, and byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLineRef {
    pub source_id: u32,
    pub generation: u64,
    pub line_idx: usize,
    pub byte_offset: u64,
}

/// Unified abstraction for log document sources.
/// Exposes uniform operations across single files, rotated multi-files,
/// and virtual filtered documents.
pub trait LogSource: Send + Sync {
    /// Unique identifier for this source instance.
    fn source_id(&self) -> u32;

    /// Current generation count (incremented on rotation, truncation, or schema change).
    fn generation(&self) -> u64;

    /// User-visible title or filename.
    fn display_name(&self) -> &str;

    /// Canonical path if backed by an on-disk file.
    fn source_path(&self) -> Option<&Path>;

    /// Total number of logical lines available.
    fn line_count(&self) -> usize;

    /// Aggregated file size in bytes.
    fn file_size(&self) -> u64;

    /// Retrieves the decoded text of logical line `idx`.
    fn get_line(&mut self, idx: usize) -> Result<String, String>;

    /// Retrieves the stable source line reference for `idx`.
    fn get_line_ref(&self, idx: usize) -> SourceLineRef;

    /// Polls for file appends, truncation, or rotation.
    fn check_for_updates(&mut self) -> Result<UpdateEvent, String>;

    /// Toggles bookmark state at `idx`.
    fn toggle_bookmark(&mut self, idx: usize) -> bool;

    /// Returns whether `idx` is bookmarked.
    fn is_bookmarked(&self, idx: usize) -> bool;

    /// Returns the sorted list of bookmarked line indices.
    fn bookmarks(&self) -> &[usize];

    /// Returns true if this is a virtual filtered document.
    fn is_filtered_source(&self) -> bool {
        false
    }

    /// Translates a logical line index in this source to its original parent source line index.
    fn resolve_to_parent_line(&self, idx: usize) -> Option<usize> {
        Some(idx)
    }

    /// Boundary navigation: jump to previous constituent file boundary (for MultiFile).
    fn jump_to_prev_boundary(&self, _current_idx: usize) -> Option<usize> {
        None
    }

    /// Boundary navigation: jump to next constituent file boundary (for MultiFile).
    fn jump_to_next_boundary(&self, _current_idx: usize) -> Option<usize> {
        None
    }

    /// Returns the specific constituent file name for line `idx` (for MultiFile).
    fn get_constituent_filename(&self, _idx: usize) -> Option<&str> {
        None
    }
}

/// Standard single file source backed by `NonLockingLogReader`.
pub struct SingleFileSource {
    id: u32,
    generation: u64,
    title: String,
    path: PathBuf,
    reader: NonLockingLogReader,
}

impl SingleFileSource {
    pub fn open(path: &Path) -> Result<Self, String> {
        let reader = NonLockingLogReader::open(path)?;
        let title = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        Ok(Self {
            id: allocate_source_id(),
            generation: 1,
            title,
            path: path.to_path_buf(),
            reader,
        })
    }
}

impl LogSource for SingleFileSource {
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
        Some(&self.path)
    }

    fn line_count(&self) -> usize {
        self.reader.line_count()
    }

    fn file_size(&self) -> u64 {
        self.reader.file_size()
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.reader.get_line(idx)
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
        let ev = self.reader.check_for_updates()?;
        if matches!(ev, UpdateEvent::Truncated) {
            self.generation += 1;
        }
        Ok(ev)
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        self.reader.toggle_bookmark(idx)
    }

    fn is_bookmarked(&self, idx: usize) -> bool {
        self.reader.is_bookmarked(idx)
    }

    fn bookmarks(&self) -> &[usize] {
        self.reader.bookmarks()
    }
}

/// In-memory mock source for unit testing and headless validation.
pub struct MockSource {
    id: u32,
    generation: u64,
    title: String,
    lines: Vec<String>,
    bookmarks: Vec<usize>,
}

impl MockSource {
    pub fn new(title: &str, lines: &[&str]) -> Self {
        Self {
            id: allocate_source_id(),
            generation: 1,
            title: title.to_string(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            bookmarks: Vec::new(),
        }
    }

    pub fn append_lines(&mut self, new_lines: &[&str]) {
        for line in new_lines {
            self.lines.push(line.to_string());
        }
    }
}

impl LogSource for MockSource {
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
            .ok_or_else(|| format!("Index {} out of bounds", idx))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_source_operations() {
        let sample = ["line 1", "line 2", "line 3"];
        let mut source = MockSource::new("test.log", &sample);

        assert_eq!(source.line_count(), 3);
        assert_eq!(source.get_line(1).unwrap(), "line 2");

        assert!(!source.is_bookmarked(1));
        assert!(source.toggle_bookmark(1));
        assert!(source.is_bookmarked(1));
        assert_eq!(source.bookmarks(), &[1]);
        assert!(!source.toggle_bookmark(1));
        assert!(!source.is_bookmarked(1));
    }
}
