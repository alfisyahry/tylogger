// src/core/merge/source.rs
//
// Virtual Merged LogSource for Tylogger (LogExpert Rust Reimplementation)
// Aggregates multiple constituent LogSources ordered chronologically. Zero disk concatenation.

use std::collections::HashMap;
use std::path::Path;

use super::provenance::{RecordProvenance, SourceAlias};
use crate::core::reader::UpdateEvent;
use crate::core::source::{allocate_source_id, LogSource, SourceLineRef};

pub struct MergedLogSource {
    source_id: u32,
    generation: u64,
    display_name: String,
    sources: HashMap<u32, Box<dyn LogSource>>,
    aliases: HashMap<u32, SourceAlias>,
    records: Vec<RecordProvenance>,
    bookmarks: Vec<usize>,
    pub show_source_prefix: bool,
}

impl MergedLogSource {
    pub fn new(display_name: &str) -> Self {
        Self {
            source_id: allocate_source_id(),
            generation: 1,
            display_name: display_name.to_string(),
            sources: HashMap::new(),
            aliases: HashMap::new(),
            records: Vec::new(),
            bookmarks: Vec::new(),
            show_source_prefix: true,
        }
    }

    pub fn add_source(&mut self, source: Box<dyn LogSource>, alias: &str, color_token_idx: usize) {
        let sid = source.source_id();
        let path = source
            .source_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.aliases
            .insert(sid, SourceAlias::new(sid, alias, &path, color_token_idx));
        self.sources.insert(sid, source);
    }

    pub fn set_merged_records(&mut self, records: Vec<RecordProvenance>) {
        self.records = records;
        self.generation += 1;
    }

    pub fn get_provenance(&self, idx: usize) -> Option<&RecordProvenance> {
        self.records.get(idx)
    }

    /// Jump to original physical source line: returns (source_id, physical_line_number)
    pub fn jump_to_original_source(&self, virtual_idx: usize) -> Option<(u32, usize)> {
        self.records
            .get(virtual_idx)
            .map(|r| (r.source_id, r.physical_line_number as usize))
    }

    pub fn source_aliases(&self) -> &HashMap<u32, SourceAlias> {
        &self.aliases
    }
}

impl LogSource for MergedLogSource {
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
        self.records.len()
    }

    fn file_size(&self) -> u64 {
        self.sources.values().map(|s| s.file_size()).sum()
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        let rec = self
            .records
            .get(idx)
            .ok_or_else(|| format!("Line index {} out of bounds", idx))?;

        let source_id = rec.source_id;
        let phys_line = rec.physical_line_number as usize;
        let alias = rec.source_alias.clone();

        let source = self
            .sources
            .get_mut(&source_id)
            .ok_or_else(|| format!("Constituent source {} not found", source_id))?;

        let raw_line = source.get_line(phys_line)?;

        if self.show_source_prefix {
            Ok(format!("[{}] {}", alias, raw_line))
        } else {
            Ok(raw_line)
        }
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        if let Some(rec) = self.records.get(idx) {
            SourceLineRef {
                source_id: rec.source_id,
                generation: self.generation,
                line_idx: rec.physical_line_number as usize,
                byte_offset: rec.file_offset,
            }
        } else {
            SourceLineRef {
                source_id: self.source_id,
                generation: self.generation,
                line_idx: idx,
                byte_offset: 0,
            }
        }
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        // Constituent sources update polling
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
        true
    }

    fn resolve_to_parent_line(&self, idx: usize) -> Option<usize> {
        self.records
            .get(idx)
            .map(|r| r.physical_line_number as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::MockSource;

    #[test]
    fn test_merged_log_source_provenance_and_lines() {
        let mut merged = MergedLogSource::new("Incident Cluster");
        let src1 = Box::new(MockSource::new("web.log", &["Line W1", "Line W2"]));
        let src2 = Box::new(MockSource::new("db.log", &["Line D1", "Line D2"]));

        let s1_id = src1.source_id();
        let s2_id = src2.source_id();

        merged.add_source(src1, "Web", 1);
        merged.add_source(src2, "DB", 2);

        // Ordered chronologically: W1 (t=10), D1 (t=20), W2 (t=30), D2 (t=40)
        let records = vec![
            RecordProvenance::new(s1_id, 0, 0, 10, "Web"),
            RecordProvenance::new(s2_id, 0, 0, 20, "DB"),
            RecordProvenance::new(s1_id, 1, 10, 30, "Web"),
            RecordProvenance::new(s2_id, 1, 10, 40, "DB"),
        ];
        merged.set_merged_records(records);

        assert_eq!(merged.line_count(), 4);
        assert_eq!(merged.get_line(0).unwrap(), "[Web] Line W1");
        assert_eq!(merged.get_line(1).unwrap(), "[DB] Line D1");
        assert_eq!(merged.get_line(2).unwrap(), "[Web] Line W2");
        assert_eq!(merged.get_line(3).unwrap(), "[DB] Line D2");

        // Jump to original source
        let jump = merged.jump_to_original_source(1).unwrap();
        assert_eq!(jump, (s2_id, 0));
    }
}
