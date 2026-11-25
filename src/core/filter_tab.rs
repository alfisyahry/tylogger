use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::core::filter::{FilterCriteria, FilterEngine};
use crate::core::reader::UpdateEvent;
use crate::core::source::{allocate_source_id, LogSource, SourceLineRef};

/// Virtual Filtered Document Source backed by a shared parent `LogSource`.
/// Does NOT duplicate log lines in RAM or on disk. Stores only 64-bit indices.
/// Operates independently: owns its own bookmarks, follow tail, and search state.
pub struct FilteredDocumentSource {
    id: u32,
    generation: u64,
    title: String,
    parent_source: Arc<Mutex<Box<dyn LogSource>>>,
    filtered_indices: Vec<usize>,
    criteria: FilterCriteria,
    bookmarks: Vec<usize>,
}

impl FilteredDocumentSource {
    /// Creates an independent virtual filtered document from a parent source.
    /// Batches line retrieval in short-lived lock windows to eliminate GUI thread contention.
    pub fn create(
        parent_source: Arc<Mutex<Box<dyn LogSource>>>,
        criteria: FilterCriteria,
    ) -> Result<Self, String> {
        let (total_parent_lines, parent_title) = {
            let parent_lock = parent_source.lock().map_err(|e| e.to_string())?;
            (
                parent_lock.line_count(),
                parent_lock.display_name().to_string(),
            )
        };

        // Narrowed chunk size (128) guarantees critical section duration < 2ms (AG-20, RSK-19)
        const CHUNK_SIZE: usize = 128;
        let mut chunk_start = 0;
        let mut chunk_buf: Vec<Option<String>> = Vec::new();
        let parent_clone = Arc::clone(&parent_source);

        let filtered_indices = FilterEngine::apply_filter(
            total_parent_lines,
            |idx| {
                if chunk_buf.is_empty() || idx < chunk_start || idx >= chunk_start + chunk_buf.len()
                {
                    chunk_start = idx;
                    let count = (total_parent_lines - idx).min(CHUNK_SIZE);
                    chunk_buf.clear();
                    if let Ok(mut parent_lock) = parent_clone.lock() {
                        for i in 0..count {
                            chunk_buf.push(parent_lock.get_line(idx + i).ok());
                        }
                    } else {
                        return None;
                    }
                }
                chunk_buf.get(idx - chunk_start).cloned().flatten()
            },
            &criteria,
            None,
        );

        let title = format!("{} [Filter: {}]", parent_title, criteria.pattern);

        Ok(Self {
            id: allocate_source_id(),
            generation: 1,
            title,
            parent_source: Arc::clone(&parent_source),
            filtered_indices,
            criteria,
            bookmarks: Vec::new(),
        })
    }

    /// Appends newly matched lines when parent source live tail receives appends.
    /// Employs chunked reads to avoid long mutex lock retention.
    pub fn update_with_parent_appends(&mut self, start_parent_idx: usize, new_parent_lines: usize) {
        // Narrowed chunk size (128) guarantees critical section duration < 2ms (AG-20, RSK-19)
        const CHUNK_SIZE: usize = 128;
        let end_parent_idx = start_parent_idx + new_parent_lines;
        let mut curr_idx = start_parent_idx;

        while curr_idx < end_parent_idx {
            let chunk_end = (curr_idx + CHUNK_SIZE).min(end_parent_idx);
            let mut batch = Vec::with_capacity(chunk_end - curr_idx);
            if let Ok(mut parent_lock) = self.parent_source.lock() {
                for idx in curr_idx..chunk_end {
                    batch.push(parent_lock.get_line(idx).ok());
                }
            } // Lock released immediately

            for (offset, line_opt) in batch.into_iter().enumerate() {
                let actual_idx = curr_idx + offset;
                if let Some(line) = line_opt {
                    let matches = crate::core::search::SearchEngine::matches_line(
                        &line,
                        &self.criteria.pattern,
                        &self.criteria.options,
                    );
                    if matches {
                        self.filtered_indices.push(actual_idx);
                    }
                }
            }
            curr_idx = chunk_end;
        }
    }
}

impl LogSource for FilteredDocumentSource {
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
        // Virtual tab has no direct file path of its own
        None
    }

    fn line_count(&self) -> usize {
        self.filtered_indices.len()
    }

    fn file_size(&self) -> u64 {
        if let Ok(parent) = self.parent_source.lock() {
            parent.file_size()
        } else {
            0
        }
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        let parent_idx = *self.filtered_indices.get(idx).ok_or_else(|| {
            format!(
                "Filtered line {} out of bounds ({})",
                idx,
                self.filtered_indices.len()
            )
        })?;

        let mut parent = self.parent_source.lock().map_err(|e| e.to_string())?;
        parent.get_line(parent_idx)
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        let parent_idx = self.filtered_indices.get(idx).copied().unwrap_or(idx);
        SourceLineRef {
            source_id: self.id,
            generation: self.generation,
            line_idx: idx,
            byte_offset: parent_idx as u64,
        }
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        let mut parent = self.parent_source.lock().map_err(|e| e.to_string())?;
        let old_count = parent.line_count();
        let ev = parent.check_for_updates()?;

        match ev {
            UpdateEvent::Appended { new_lines } => {
                drop(parent);
                let prev_filter_len = self.filtered_indices.len();
                self.update_with_parent_appends(old_count, new_lines);
                let diff = self.filtered_indices.len() - prev_filter_len;
                if diff > 0 {
                    Ok(UpdateEvent::Appended { new_lines: diff })
                } else {
                    Ok(UpdateEvent::NoChange)
                }
            }
            UpdateEvent::Truncated => {
                self.generation += 1;
                let new_total = {
                    let parent_lock = self.parent_source.lock().map_err(|e| e.to_string())?;
                    parent_lock.line_count()
                };

                // Narrowed chunk size (128) guarantees critical section duration < 2ms (AG-20, RSK-19)
                const CHUNK_SIZE: usize = 128;
                let mut chunk_start = 0;
                let mut chunk_buf: Vec<Option<String>> = Vec::new();
                let parent_clone = Arc::clone(&self.parent_source);

                self.filtered_indices = FilterEngine::apply_filter(
                    new_total,
                    |idx| {
                        if chunk_buf.is_empty()
                            || idx < chunk_start
                            || idx >= chunk_start + chunk_buf.len()
                        {
                            chunk_start = idx;
                            let count = (new_total - idx).min(CHUNK_SIZE);
                            chunk_buf.clear();
                            if let Ok(mut parent_lock) = parent_clone.lock() {
                                for i in 0..count {
                                    chunk_buf.push(parent_lock.get_line(idx + i).ok());
                                }
                            } else {
                                return None;
                            }
                        }
                        chunk_buf.get(idx - chunk_start).cloned().flatten()
                    },
                    &self.criteria,
                    None,
                );
                Ok(UpdateEvent::Truncated)
            }
            UpdateEvent::NoChange => Ok(UpdateEvent::NoChange),
        }
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        if idx >= self.filtered_indices.len() {
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

    fn is_filtered_source(&self) -> bool {
        true
    }

    fn resolve_to_parent_line(&self, idx: usize) -> Option<usize> {
        self.filtered_indices.get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::MockSource;

    #[test]
    fn test_filter_to_tab_creation_and_independence() {
        let sample = [
            "2026-11-25 10:00:00 [INFO] Start",
            "2026-11-25 10:00:01 [CRITICAL] Core failure",
            "2026-11-25 10:00:02 [DEBUG] Checking",
            "2026-11-25 10:00:03 [CRITICAL] Out of memory",
        ];
        let parent: Box<dyn LogSource> = Box::new(MockSource::new("app.log", &sample));
        let parent_arc = Arc::new(Mutex::new(parent));

        let crit = FilterCriteria::new_include("CRITICAL", false);
        let mut filter_tab = FilteredDocumentSource::create(parent_arc, crit).unwrap();

        assert_eq!(filter_tab.line_count(), 2);
        assert_eq!(filter_tab.resolve_to_parent_line(0), Some(1));
        assert_eq!(filter_tab.resolve_to_parent_line(1), Some(3));

        assert!(filter_tab.get_line(0).unwrap().contains("Core failure"));
        assert!(filter_tab.get_line(1).unwrap().contains("Out of memory"));

        // Independent bookmarks
        assert!(filter_tab.toggle_bookmark(0));
        assert!(filter_tab.is_bookmarked(0));
    }

    #[test]
    fn test_concurrent_parent_access_during_filtering() {
        let lines: Vec<String> = (0..5000)
            .map(|i| {
                if i % 10 == 0 {
                    format!("Line {} [CRITICAL] Alert", i)
                } else {
                    format!("Line {} [INFO] Normal", i)
                }
            })
            .collect();
        let str_lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let parent: Box<dyn LogSource> = Box::new(MockSource::new("big.log", &str_lines));
        let parent_arc = Arc::new(Mutex::new(parent));

        let crit = FilterCriteria::new_include("CRITICAL", false);
        let filter_tab = FilteredDocumentSource::create(Arc::clone(&parent_arc), crit).unwrap();

        assert_eq!(filter_tab.line_count(), 500);

        // Verify parent lock is easily accessible immediately
        let mut guard = parent_arc.lock().unwrap();
        assert_eq!(guard.line_count(), 5000);
        assert!(guard.get_line(0).unwrap().contains("[CRITICAL]"));
    }
}
