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

/// Category of log data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    LocalFile,
    SftpFile,
    SshStream,
    CompressedArchive,
    Plugin,
    VirtualMerged,
    VirtualFiltered,
    VirtualCaptured,
    Mock,
}

/// Fine-grained capability flags for a log data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    pub can_read: bool,
    pub can_random_access: bool,
    pub can_monitor: bool,
    pub can_discover_files: bool,
    pub can_detect_rotation: bool,
    pub can_reconnect: bool,
    pub can_resume: bool,
    pub has_stable_identity: bool,
    pub is_live_append: bool,
    pub is_end_of_stream: bool,
}

impl Default for SourceCapabilities {
    fn default() -> Self {
        Self {
            can_read: true,
            can_random_access: true,
            can_monitor: true,
            can_discover_files: false,
            can_detect_rotation: true,
            can_reconnect: false,
            can_resume: false,
            has_stable_identity: true,
            is_live_append: true,
            is_end_of_stream: false,
        }
    }
}

/// Operational state of a data source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    Active,
    Connecting,
    Reconnecting { attempt: u32, next_delay_ms: u64 },
    Closed,
    Stale,
    Error(String),
}

/// Composite unique identity of a data source preventing cross-server ambiguity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceIdentity {
    pub source_type: SourceType,
    pub profile_id: Option<String>,
    pub host_fingerprint: Option<String>,
    pub canonical_path: String,
    pub generation: u64,
    pub file_id: Option<u64>,
    pub mtime: Option<u64>,
    pub size: u64,
}

impl SourceIdentity {
    pub fn new_local(path: &Path, generation: u64, size: u64) -> Self {
        Self {
            source_type: SourceType::LocalFile,
            profile_id: None,
            host_fingerprint: None,
            canonical_path: path.to_string_lossy().to_string(),
            generation,
            file_id: None,
            mtime: None,
            size,
        }
    }

    pub fn new_remote(
        profile_id: &str,
        host_fingerprint: &str,
        path: &str,
        generation: u64,
        size: u64,
    ) -> Self {
        Self {
            source_type: SourceType::SftpFile,
            profile_id: Some(profile_id.to_string()),
            host_fingerprint: Some(host_fingerprint.to_string()),
            canonical_path: path.to_string(),
            generation,
            file_id: None,
            mtime: None,
            size,
        }
    }

    pub fn new_stream(profile_id: &str, template_name: &str, generation: u64) -> Self {
        Self {
            source_type: SourceType::SshStream,
            profile_id: Some(profile_id.to_string()),
            host_fingerprint: None,
            canonical_path: template_name.to_string(),
            generation,
            file_id: None,
            mtime: None,
            size: 0,
        }
    }

    pub fn new_archive(archive_path: &Path, member_name: &str, size: u64) -> Self {
        Self {
            source_type: SourceType::CompressedArchive,
            profile_id: None,
            host_fingerprint: None,
            canonical_path: format!("{}!/{}", archive_path.display(), member_name),
            generation: 1,
            file_id: None,
            mtime: None,
            size,
        }
    }

    /// Formats a stable unique identifier string for session persistence.
    pub fn to_string_key(&self) -> String {
        match (&self.profile_id, &self.host_fingerprint) {
            (Some(pid), Some(fp)) => format!("sftp://{}:{}@{}", pid, fp, self.canonical_path),
            (Some(pid), None) => format!("stream://{}@{}", pid, self.canonical_path),
            _ => self.canonical_path.clone(),
        }
    }
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

    /// Returns the category of this log source.
    fn source_type(&self) -> SourceType {
        SourceType::LocalFile
    }

    /// Returns capability flags supported by this source.
    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities::default()
    }

    /// Returns composite identity of this source.
    fn identity(&self) -> SourceIdentity {
        SourceIdentity {
            source_type: self.source_type(),
            profile_id: None,
            host_fingerprint: None,
            canonical_path: self
                .source_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| self.display_name().to_string()),
            generation: self.generation(),
            file_id: None,
            mtime: None,
            size: self.file_size(),
        }
    }

    /// Returns true if random access seeking is supported.
    fn is_seekable(&self) -> bool {
        self.capabilities().can_random_access
    }

    /// Returns true if source continues to receive live data.
    fn is_continuous(&self) -> bool {
        self.capabilities().is_live_append
    }

    /// Operational status of the source.
    fn status(&self) -> SourceStatus {
        SourceStatus::Active
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

    #[test]
    fn test_source_capabilities_and_identity() {
        let local_ident = SourceIdentity::new_local(Path::new("C:\\logs\\app.log"), 1, 1024);
        assert_eq!(local_ident.source_type, SourceType::LocalFile);
        assert_eq!(local_ident.to_string_key(), "C:\\logs\\app.log");

        let remote_ident =
            SourceIdentity::new_remote("srv1", "SHA256:abc123xyz", "/var/log/syslog", 2, 2048);
        assert_eq!(remote_ident.source_type, SourceType::SftpFile);
        assert_eq!(
            remote_ident.to_string_key(),
            "sftp://srv1:SHA256:abc123xyz@/var/log/syslog"
        );

        let caps = SourceCapabilities::default();
        assert!(caps.can_read);
        assert!(caps.can_random_access);
        assert!(caps.can_detect_rotation);

        let sample = ["line A", "line B"];
        let source = MockSource::new("demo.log", &sample);
        assert_eq!(source.source_type(), SourceType::LocalFile);
        assert!(source.is_seekable());
        assert!(source.is_continuous());
        assert_eq!(source.status(), SourceStatus::Active);
    }
}
