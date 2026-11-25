use std::path::{Path, PathBuf};

use crate::core::reader::{NonLockingLogReader, UpdateEvent};

/// A single open document tab in Tylogger.
/// Maintains independent file handle, reader, bookmarks, scroll position,
/// selection, follow tail state, and search/filter states.
pub struct Document {
    pub id: usize,
    pub path: Option<PathBuf>,
    pub title: String,
    pub reader: NonLockingLogReader,
    pub top_line: usize,
    pub scroll_x: i32,
    pub selected_line: usize,
    pub selection_anchor: Option<usize>,
    pub follow_tail: bool,
    pub monitoring_paused: bool,
    pub encoding: String,
    pub has_new_activity: bool,
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub current_match_idx: Option<usize>,
    pub filter_active: bool,
    pub filter_matches: Vec<usize>,
}

impl Document {
    pub fn new(
        id: usize,
        path: Option<PathBuf>,
        title: String,
        reader: NonLockingLogReader,
    ) -> Self {
        Self {
            id,
            path,
            title,
            reader,
            top_line: 0,
            scroll_x: 0,
            selected_line: 0,
            selection_anchor: None,
            follow_tail: true,
            monitoring_paused: false,
            encoding: "UTF-8".to_string(),
            has_new_activity: false,
            search_query: None,
            search_matches: Vec::new(),
            current_match_idx: None,
            filter_active: false,
            filter_matches: Vec::new(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.reader.line_count()
    }

    pub fn file_size(&self) -> u64 {
        self.reader.file_size()
    }

    pub fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.reader.get_line(idx)
    }

    pub fn toggle_bookmark(&mut self, idx: usize) -> bool {
        self.reader.toggle_bookmark(idx)
    }

    pub fn is_bookmarked(&self, idx: usize) -> bool {
        self.reader.is_bookmarked(idx)
    }

    pub fn bookmarks(&self) -> &[usize] {
        self.reader.bookmarks()
    }
}

/// Multi-document workspace manager for Tylogger.
pub struct Workspace {
    documents: Vec<Document>,
    active_idx: usize,
    next_doc_id: usize,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            active_idx: 0,
            next_doc_id: 1,
        }
    }

    /// Opens a log file in a new tab, or switches to it if already open.
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<usize, String> {
        let path_ref = path.as_ref();
        let canonical_target = path_ref
            .canonicalize()
            .unwrap_or_else(|_| path_ref.to_path_buf());

        // Check if already open
        for (idx, doc) in self.documents.iter().enumerate() {
            if let Some(ref doc_path) = doc.path {
                let canonical_doc = doc_path.canonicalize().unwrap_or_else(|_| doc_path.clone());
                if canonical_doc == canonical_target {
                    self.active_idx = idx;
                    return Ok(idx);
                }
            }
        }

        let reader = NonLockingLogReader::open(path_ref)?;
        let title = path_ref
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled.log".to_string());

        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;

        let doc = Document::new(doc_id, Some(path_ref.to_path_buf()), title, reader);
        self.documents.push(doc);
        self.active_idx = self.documents.len() - 1;
        Ok(self.active_idx)
    }

    /// Adds an in-memory mock document for testing and demo purposes.
    pub fn add_mock_document(&mut self, title: &str, lines: &[&str]) -> usize {
        let reader = NonLockingLogReader::from_mock_lines(lines);
        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;

        let doc = Document::new(doc_id, None, title.to_string(), reader);
        self.documents.push(doc);
        self.active_idx = self.documents.len() - 1;
        self.active_idx
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    pub fn active_index(&self) -> usize {
        self.active_idx
    }

    pub fn active_document(&self) -> Option<&Document> {
        self.documents.get(self.active_idx)
    }

    pub fn active_document_mut(&mut self) -> Option<&mut Document> {
        self.documents.get_mut(self.active_idx)
    }

    pub fn document(&self, idx: usize) -> Option<&Document> {
        self.documents.get(idx)
    }

    pub fn document_mut(&mut self, idx: usize) -> Option<&mut Document> {
        self.documents.get_mut(idx)
    }

    pub fn switch_tab(&mut self, idx: usize) -> bool {
        if idx < self.documents.len() {
            self.active_idx = idx;
            if let Some(doc) = self.documents.get_mut(idx) {
                doc.has_new_activity = false;
            }
            true
        } else {
            false
        }
    }

    pub fn next_tab(&mut self) {
        if !self.documents.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.documents.len();
            if let Some(doc) = self.documents.get_mut(self.active_idx) {
                doc.has_new_activity = false;
            }
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.documents.is_empty() {
            if self.active_idx == 0 {
                self.active_idx = self.documents.len() - 1;
            } else {
                self.active_idx -= 1;
            }
            if let Some(doc) = self.documents.get_mut(self.active_idx) {
                doc.has_new_activity = false;
            }
        }
    }

    pub fn close_document(&mut self, idx: usize) -> bool {
        if idx >= self.documents.len() {
            return false;
        }

        self.documents.remove(idx);
        if self.documents.is_empty() {
            self.active_idx = 0;
        } else if self.active_idx >= self.documents.len() {
            self.active_idx = self.documents.len() - 1;
        } else if self.active_idx > idx {
            self.active_idx -= 1;
        }
        true
    }

    pub fn close_active_document(&mut self) -> bool {
        if self.documents.is_empty() {
            false
        } else {
            self.close_document(self.active_idx)
        }
    }

    pub fn close_other_documents(&mut self) {
        if self.documents.len() <= 1 {
            return;
        }
        let active = self.documents.remove(self.active_idx);
        self.documents.clear();
        self.documents.push(active);
        self.active_idx = 0;
    }

    pub fn close_to_the_right(&mut self, from_idx: usize) {
        if from_idx + 1 < self.documents.len() {
            self.documents.truncate(from_idx + 1);
            if self.active_idx >= self.documents.len() {
                self.active_idx = self.documents.len() - 1;
            }
        }
    }

    /// Checks all open documents for file updates (tailing/truncation).
    /// Returns vector of (document_index, UpdateEvent).
    pub fn check_all_for_updates(&mut self) -> Vec<(usize, UpdateEvent)> {
        let mut events = Vec::new();
        for (idx, doc) in self.documents.iter_mut().enumerate() {
            if doc.monitoring_paused {
                continue;
            }
            if let Ok(ev) = doc.reader.check_for_updates() {
                if ev != UpdateEvent::NoChange {
                    if idx != self.active_idx {
                        doc.has_new_activity = true;
                    } else if doc.follow_tail {
                        let total = doc.reader.line_count();
                        if total > 0 {
                            doc.top_line = total.saturating_sub(25);
                            doc.selected_line = total - 1;
                        }
                    }
                    events.push((idx, ev));
                }
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_multi_doc_isolation() {
        let mut ws = Workspace::new();
        assert_eq!(ws.document_count(), 0);

        let lines_a = ["Line A1", "Line A2", "Line A3"];
        let lines_b = ["Line B1", "Line B2", "Line B3", "Line B4"];

        let idx_a = ws.add_mock_document("doc_a.log", &lines_a);
        assert_eq!(idx_a, 0);
        let idx_b = ws.add_mock_document("doc_b.log", &lines_b);
        assert_eq!(idx_b, 1);
        assert_eq!(ws.document_count(), 2);
        assert_eq!(ws.active_index(), 1);

        // Modify Tab B state
        {
            let doc_b = ws.active_document_mut().unwrap();
            doc_b.top_line = 2;
            doc_b.selected_line = 3;
            doc_b.toggle_bookmark(1);
            assert!(doc_b.is_bookmarked(1));
        }

        // Switch back to Tab A
        ws.switch_tab(0);
        assert_eq!(ws.active_index(), 0);
        {
            let doc_a = ws.active_document().unwrap();
            assert_eq!(doc_a.top_line, 0);
            assert_eq!(doc_a.selected_line, 0);
            assert!(!doc_a.is_bookmarked(1)); // Tab A has no bookmark at line 1!
        }

        // Switch to Tab B and verify state is perfectly restored
        ws.switch_tab(1);
        {
            let doc_b = ws.active_document().unwrap();
            assert_eq!(doc_b.top_line, 2);
            assert_eq!(doc_b.selected_line, 3);
            assert!(doc_b.is_bookmarked(1));
        }

        // Close Tab A
        assert!(ws.close_document(0));
        assert_eq!(ws.document_count(), 1);
        assert_eq!(ws.active_index(), 0);
        assert_eq!(ws.active_document().unwrap().title, "doc_b.log");
    }
}
