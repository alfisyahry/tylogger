// src/core/merge/provenance.rs
//
// Source Identification & Provenance Metadata for Chronological Log Merging
// Preserves source ID, physical line, byte offset, effective time, and alias.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAlias {
    pub source_id: u32,
    pub alias: String,
    pub file_path: String,
    pub color_token_idx: usize,
}

impl SourceAlias {
    pub fn new(source_id: u32, alias: &str, file_path: &str, color_token_idx: usize) -> Self {
        Self {
            source_id,
            alias: alias.to_string(),
            file_path: file_path.to_string(),
            color_token_idx,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordProvenance {
    pub source_id: u32,
    pub physical_line_number: u64,
    pub file_offset: u64,
    pub effective_nanos: i64,
    pub source_alias: String,
    pub is_late_arrival: bool,
}

impl RecordProvenance {
    pub fn new(
        source_id: u32,
        physical_line_number: u64,
        file_offset: u64,
        effective_nanos: i64,
        source_alias: &str,
    ) -> Self {
        Self {
            source_id,
            physical_line_number,
            file_offset,
            effective_nanos,
            source_alias: source_alias.to_string(),
            is_late_arrival: false,
        }
    }
}
