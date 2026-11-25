use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_BOOKMARK_ID: AtomicU64 = AtomicU64::new(1001);

pub fn allocate_bookmark_id() -> u64 {
    NEXT_BOOKMARK_ID.fetch_add(1, Ordering::Relaxed)
}

/// Validity status of a persistent bookmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BookmarkStatus {
    #[default]
    Valid,
    Unresolved,
    SourceUnavailable,
    SourceChanged,
    Orphaned,
}

/// Source-aware persistent bookmark.
/// Anchored to source identity, generation, and 64-bit byte offset rather than transient view row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub id: u64,
    pub source_id: u32,
    pub source_path: Option<String>,
    pub generation: u64,
    pub byte_offset: u64,
    pub physical_line: usize,
    pub logical_line: usize,
    pub label: Option<String>,
    pub comment: String,
    pub timestamp: u64,
    pub status: BookmarkStatus,
}

impl Bookmark {
    pub fn new(
        source_id: u32,
        source_path: Option<&str>,
        generation: u64,
        byte_offset: u64,
        line: usize,
    ) -> Self {
        Self {
            id: allocate_bookmark_id(),
            source_id,
            source_path: source_path.map(|s| s.to_string()),
            generation,
            byte_offset,
            physical_line: line,
            logical_line: line,
            label: None,
            comment: String::new(),
            timestamp: 0,
            status: BookmarkStatus::Valid,
        }
    }
}

/// Global Bookmark Repository and Cross-View Synchronization Engine.
#[derive(Debug, Clone, Default)]
pub struct GlobalBookmarkManager {
    bookmarks: Vec<Bookmark>,
}

impl GlobalBookmarkManager {
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn all(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    /// Toggles a bookmark at given source and line.
    /// If bookmark exists at that line/offset, removes it. Otherwise inserts it.
    /// Returns true if bookmark was added, false if removed.
    pub fn toggle_bookmark(
        &mut self,
        source_id: u32,
        source_path: Option<&str>,
        generation: u64,
        byte_offset: u64,
        line: usize,
    ) -> bool {
        if let Some(pos) = self.bookmarks.iter().position(|b| {
            b.source_id == source_id
                && (b.physical_line == line || (b.byte_offset == byte_offset && byte_offset > 0))
        }) {
            self.bookmarks.remove(pos);
            false
        } else {
            let bm = Bookmark::new(source_id, source_path, generation, byte_offset, line);
            self.bookmarks.push(bm);
            self.bookmarks
                .sort_by_key(|b| (b.source_id, b.physical_line));
            true
        }
    }

    pub fn is_bookmarked(&self, source_id: u32, line: usize) -> bool {
        self.bookmarks.iter().any(|b| {
            b.source_id == source_id && b.physical_line == line && b.status == BookmarkStatus::Valid
        })
    }

    pub fn get_for_source(&self, source_id: u32) -> Vec<usize> {
        self.bookmarks
            .iter()
            .filter(|b| b.source_id == source_id && b.status == BookmarkStatus::Valid)
            .map(|b| b.physical_line)
            .collect()
    }

    pub fn edit_bookmark(&mut self, id: u64, label: Option<String>, comment: String) -> bool {
        if let Some(bm) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            bm.label = label;
            bm.comment = comment;
            true
        } else {
            false
        }
    }

    pub fn remove_bookmark(&mut self, id: u64) -> bool {
        if let Some(pos) = self.bookmarks.iter().position(|b| b.id == id) {
            self.bookmarks.remove(pos);
            true
        } else {
            false
        }
    }

    /// Handles log file rollover in MultiFile:
    /// Updates generation and keeps bookmark validity intact when files rotate.
    pub fn handle_rotation(&mut self, source_id: u32, new_generation: u64) {
        for bm in self
            .bookmarks
            .iter_mut()
            .filter(|b| b.source_id == source_id)
        {
            bm.generation = new_generation;
            // Preserves Valid status across rotations
            bm.status = BookmarkStatus::Valid;
        }
    }

    /// Cleans up orphaned or invalid bookmarks. Returns number of removed entries.
    pub fn clear_invalid(&mut self) -> usize {
        let prev_len = self.bookmarks.len();
        self.bookmarks.retain(|b| b.status == BookmarkStatus::Valid);
        prev_len - self.bookmarks.len()
    }

    /// Exports bookmarks to RFC 4180 CSV with spreadsheet formula injection protection.
    pub fn export_csv(&self) -> String {
        let mut out = String::from("ID,SourceID,SourcePath,Line,ByteOffset,Status,Label,Comment\n");
        for b in &self.bookmarks {
            let label = b.label.as_deref().unwrap_or("");
            let clean_label = crate::core::export::StreamingExporter::sanitize_csv_field(label);
            let clean_comment =
                crate::core::export::StreamingExporter::sanitize_csv_field(&b.comment);
            let path_str = b.source_path.as_deref().unwrap_or("");

            out.push_str(&format!(
                "{},{},\"{}\",{},{},\"{:?}\",\"{}\",\"{}\"\n",
                b.id,
                b.source_id,
                path_str,
                b.physical_line,
                b.byte_offset,
                b.status,
                clean_label,
                clean_comment
            ));
        }
        out
    }

    /// Imports bookmarks from CSV text. Returns count of successfully imported bookmarks.
    pub fn import_csv(&mut self, csv_text: &str) -> usize {
        let mut imported = 0;
        for line in csv_text.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                if let (Ok(src_id), Ok(l_num), Ok(b_offset)) = (
                    parts[1].trim().parse::<u32>(),
                    parts[3].trim().parse::<usize>(),
                    parts[4].trim().parse::<u64>(),
                ) {
                    let mut bm = Bookmark::new(src_id, None, 1, b_offset, l_num);
                    if parts.len() >= 7 {
                        let raw_lbl = parts[6].trim().trim_matches('"');
                        if !raw_lbl.is_empty() {
                            bm.label = Some(raw_lbl.to_string());
                        }
                    }
                    if parts.len() >= 8 {
                        bm.comment = parts[7].trim().trim_matches('"').to_string();
                    }
                    self.bookmarks.push(bm);
                    imported += 1;
                }
            }
        }
        imported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_lifecycle_and_cross_view_sync() {
        let mut mgr = GlobalBookmarkManager::new();

        // Add bookmark on Parent source (ID = 1, line = 450)
        assert!(mgr.toggle_bookmark(1, Some("access.log"), 1, 45000, 450));
        assert!(mgr.is_bookmarked(1, 450));
        assert_eq!(mgr.get_for_source(1), vec![450]);

        // Edit bookmark
        let id = mgr.all()[0].id;
        assert!(mgr.edit_bookmark(
            id,
            Some("DB Crash".to_string()),
            "Root cause investigate".to_string()
        ));
        assert_eq!(mgr.all()[0].label.as_deref(), Some("DB Crash"));

        // Toggle removes it
        assert!(!mgr.toggle_bookmark(1, Some("access.log"), 1, 45000, 450));
        assert!(!mgr.is_bookmarked(1, 450));
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_bookmark_rotation_and_csv_export_import() {
        let mut mgr = GlobalBookmarkManager::new();
        mgr.toggle_bookmark(2, Some("syslog.1"), 1, 12000, 100);

        // Simulate MultiFile rollover
        mgr.handle_rotation(2, 2);
        assert_eq!(mgr.all()[0].generation, 2);
        assert_eq!(mgr.all()[0].status, BookmarkStatus::Valid);

        // Export CSV
        let csv = mgr.export_csv();
        assert!(csv.contains("syslog.1"));
        assert!(csv.contains("100"));

        // Import CSV
        let mut mgr2 = GlobalBookmarkManager::new();
        let count = mgr2.import_csv(&csv);
        assert_eq!(count, 1);
        assert_eq!(mgr2.all()[0].physical_line, 100);
    }
}
