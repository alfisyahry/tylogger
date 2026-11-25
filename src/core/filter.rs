use std::collections::BTreeSet;

use crate::core::columnizer::Columnizer;
use crate::core::search::{SearchEngine, SearchOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Include,
    Exclude,
    Invert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyColumnPolicy {
    NoHit,
    Hit,
}

/// Advanced filter criteria supporting keywords, regex, ranges,
/// context spread (back/fore), and column restrictions.
#[derive(Debug, Clone)]
pub struct FilterCriteria {
    pub pattern: String,
    pub mode: FilterMode,
    pub options: SearchOptions,
    /// Range filtering: optional start pattern and end pattern
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    /// Context spread
    pub back_spread: usize,
    pub fore_spread: usize,
    /// Column restriction
    pub column_index: Option<usize>,
    pub empty_column_policy: EmptyColumnPolicy,
}

impl FilterCriteria {
    pub fn new_include(pattern: &str, case_sensitive: bool) -> Self {
        Self {
            pattern: pattern.to_string(),
            mode: FilterMode::Include,
            options: SearchOptions::new_plain(case_sensitive, false),
            range_start: None,
            range_end: None,
            back_spread: 0,
            fore_spread: 0,
            column_index: None,
            empty_column_policy: EmptyColumnPolicy::NoHit,
        }
    }

    pub fn new_exclude(pattern: &str, case_sensitive: bool) -> Self {
        Self {
            pattern: pattern.to_string(),
            mode: FilterMode::Exclude,
            options: SearchOptions::new_plain(case_sensitive, false),
            range_start: None,
            range_end: None,
            back_spread: 0,
            fore_spread: 0,
            column_index: None,
            empty_column_policy: EmptyColumnPolicy::NoHit,
        }
    }

    pub fn new_range(start_pat: &str, end_pat: &str, invert: bool) -> Self {
        Self {
            pattern: String::new(),
            mode: if invert {
                FilterMode::Invert
            } else {
                FilterMode::Include
            },
            options: SearchOptions::default(),
            range_start: Some(start_pat.to_string()),
            range_end: Some(end_pat.to_string()),
            back_spread: 0,
            fore_spread: 0,
            column_index: None,
            empty_column_policy: EmptyColumnPolicy::NoHit,
        }
    }
}

pub struct FilterEngine;

impl FilterEngine {
    /// Applies advanced filter criteria across lines and returns matching 64-bit line indices.
    /// Handles pattern matches, range filters, and context spread with deduplication.
    pub fn apply_filter<F>(
        total_lines: usize,
        mut line_getter: F,
        criteria: &FilterCriteria,
        columnizer: Option<&dyn Columnizer>,
    ) -> Vec<usize>
    where
        F: FnMut(usize) -> Option<String>,
    {
        // 1. Range Filtering Mode
        if let (Some(start_pat), Some(end_pat)) = (&criteria.range_start, &criteria.range_end) {
            return Self::apply_range_filter(
                total_lines,
                line_getter,
                start_pat,
                end_pat,
                criteria.mode == FilterMode::Invert,
                &criteria.options,
            );
        }

        if criteria.pattern.is_empty() {
            return (0..total_lines).collect();
        }

        let mut primary_hits = Vec::new();

        for idx in 0..total_lines {
            if let Some(line) = line_getter(idx) {
                let text_to_check: String =
                    if let (Some(col_idx), Some(col)) = (criteria.column_index, columnizer) {
                        let cols = col.parse_line(&line);
                        if let Some(val) = cols.get(col_idx) {
                            if val.is_empty() {
                                match criteria.empty_column_policy {
                                    EmptyColumnPolicy::NoHit => String::new(),
                                    EmptyColumnPolicy::Hit => {
                                        primary_hits.push(idx);
                                        continue;
                                    }
                                }
                            } else {
                                val.clone()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        line
                    };

                let matches_pattern = SearchEngine::matches_line(
                    &text_to_check,
                    &criteria.pattern,
                    &criteria.options,
                );

                let is_hit = match criteria.mode {
                    FilterMode::Include => matches_pattern,
                    FilterMode::Exclude | FilterMode::Invert => !matches_pattern,
                };

                if is_hit {
                    primary_hits.push(idx);
                }
            }
        }

        // 2. Context Spread (Back Spread & Fore Spread) with automatic deduplication
        if criteria.back_spread > 0 || criteria.fore_spread > 0 {
            Self::apply_context_spread(
                total_lines,
                &primary_hits,
                criteria.back_spread,
                criteria.fore_spread,
            )
        } else {
            primary_hits
        }
    }

    /// Evaluates start..end pattern ranges.
    fn apply_range_filter<F>(
        total_lines: usize,
        mut line_getter: F,
        start_pat: &str,
        end_pat: &str,
        invert: bool,
        options: &SearchOptions,
    ) -> Vec<usize>
    where
        F: FnMut(usize) -> Option<String>,
    {
        let mut in_range = false;
        let mut matches = Vec::new();

        for idx in 0..total_lines {
            if let Some(line) = line_getter(idx) {
                if !in_range {
                    if SearchEngine::matches_line(&line, start_pat, options) {
                        in_range = true;
                        if !invert {
                            matches.push(idx);
                        }
                    } else if invert {
                        matches.push(idx);
                    }
                } else {
                    if !invert {
                        matches.push(idx);
                    }
                    if SearchEngine::matches_line(&line, end_pat, options) {
                        in_range = false;
                    }
                }
            }
        }

        matches
    }

    /// Expands matching indices with back/fore spread and deduplicates overlapping ranges.
    pub fn apply_context_spread(
        total_lines: usize,
        primary_hits: &[usize],
        back_spread: usize,
        fore_spread: usize,
    ) -> Vec<usize> {
        let mut set = BTreeSet::new();

        for &hit in primary_hits {
            let start = hit.saturating_sub(back_spread);
            let end = (hit + fore_spread).min(total_lines.saturating_sub(1));

            for line_idx in start..=end {
                set.insert(line_idx);
            }
        }

        set.into_iter().collect()
    }
}

/// Reusable named filter preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterPreset {
    pub name: String,
    pub pattern: String,
    pub mode: FilterMode,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub search_mode: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    pub back_spread: usize,
    pub fore_spread: usize,
    pub column_index: Option<usize>,
    pub columnizer_id: Option<String>,
}

impl FilterPreset {
    pub fn from_criteria(name: &str, crit: &FilterCriteria, col_id: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            pattern: crit.pattern.clone(),
            mode: crit.mode,
            case_sensitive: crit.options.case_sensitive,
            whole_word: crit.options.whole_word,
            is_regex: crit.options.is_regex,
            search_mode: format!("{:?}", crit.options.mode),
            range_start: crit.range_start.clone(),
            range_end: crit.range_end.clone(),
            back_spread: crit.back_spread,
            fore_spread: crit.fore_spread,
            column_index: crit.column_index,
            columnizer_id: col_id.map(|s| s.to_string()),
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"name\":\"{}\",\"pattern\":\"{}\",\"mode\":\"{:?}\",\"case_sensitive\":{},\"whole_word\":{},\"is_regex\":{},\"back_spread\":{},\"fore_spread\":{}}}",
            self.name,
            self.pattern,
            self.mode,
            self.case_sensitive,
            self.whole_word,
            self.is_regex,
            self.back_spread,
            self.fore_spread
        )
    }
}

/// Persistent manager for filter presets and privacy-safe recent search history.
#[derive(Debug, Clone)]
pub struct FilterPresetManager {
    pub presets: Vec<FilterPreset>,
    pub recent_history: Vec<String>,
    pub max_history: usize,
}

impl Default for FilterPresetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterPresetManager {
    pub fn new() -> Self {
        Self {
            presets: Vec::new(),
            recent_history: Vec::new(),
            max_history: 20,
        }
    }

    pub fn add_preset(&mut self, preset: FilterPreset) {
        if let Some(pos) = self.presets.iter().position(|p| p.name == preset.name) {
            self.presets[pos] = preset;
        } else {
            self.presets.push(preset);
        }
    }

    pub fn delete_preset(&mut self, name: &str) -> bool {
        if let Some(pos) = self.presets.iter().position(|p| p.name == name) {
            self.presets.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_preset(&self, name: &str) -> Option<&FilterPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    pub fn record_history(&mut self, query: &str) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return;
        }
        self.recent_history.retain(|q| q != trimmed);
        self.recent_history.insert(0, trimmed.to_string());
        if self.recent_history.len() > self.max_history {
            self.recent_history.truncate(self.max_history);
        }
    }

    pub fn clear_history(&mut self) {
        self.recent_history.clear();
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"presets\":[");
        for (i, p) in self.presets.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&p.to_json());
        }
        out.push_str("],\"recent\":[");
        for (i, q) in self.recent_history.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(q);
            out.push('"');
        }
        out.push_str("]}");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_include_and_exclude_filter() {
        let sample = [
            "Line 0: INFO Start",
            "Line 1: ERROR Database failed",
            "Line 2: INFO Connected",
            "Line 3: ERROR Disk full",
            "Line 4: WARN Cache miss",
        ];

        // 1. Include ERROR
        let crit_inc = FilterCriteria::new_include("ERROR", false);
        let inc_matches = FilterEngine::apply_filter(
            sample.len(),
            |idx| sample.get(idx).map(|s| s.to_string()),
            &crit_inc,
            None,
        );
        assert_eq!(inc_matches, vec![1, 3]);

        // 2. Exclude ERROR
        let crit_exc = FilterCriteria::new_exclude("ERROR", false);
        let exc_matches = FilterEngine::apply_filter(
            sample.len(),
            |idx| sample.get(idx).map(|s| s.to_string()),
            &crit_exc,
            None,
        );
        assert_eq!(exc_matches, vec![0, 2, 4]);
    }

    #[test]
    fn test_range_filter_and_invert() {
        let lines = [
            "Line 0: setup",
            "Line 1: === START BATCH ===",
            "Line 2: processing item 1",
            "Line 3: processing item 2",
            "Line 4: === END BATCH ===",
            "Line 5: cleanup",
        ];

        // Normal Range
        let crit = FilterCriteria::new_range("=== START BATCH ===", "=== END BATCH ===", false);
        let matches = FilterEngine::apply_filter(
            lines.len(),
            |idx| lines.get(idx).map(|s| s.to_string()),
            &crit,
            None,
        );
        assert_eq!(matches, vec![1, 2, 3, 4]);

        // Invert Range
        let crit_inv = FilterCriteria::new_range("=== START BATCH ===", "=== END BATCH ===", true);
        let inv_matches = FilterEngine::apply_filter(
            lines.len(),
            |idx| lines.get(idx).map(|s| s.to_string()),
            &crit_inv,
            None,
        );
        assert_eq!(inv_matches, vec![0, 5]);
    }

    #[test]
    fn test_context_spread_with_deduplication() {
        let primary_hits = [10, 12]; // Within spread distance
        let spread_matches = FilterEngine::apply_context_spread(100, &primary_hits, 2, 2);

        // Hit 10 -> 8..12
        // Hit 12 -> 10..14
        // Union without duplicates -> 8..14
        assert_eq!(spread_matches, vec![8, 9, 10, 11, 12, 13, 14]);
    }

    #[test]
    fn test_filter_preset_and_history_manager() {
        let mut mgr = FilterPresetManager::new();
        let crit = FilterCriteria::new_include("CRITICAL", true);
        let preset = FilterPreset::from_criteria("CriticalErrors", &crit, Some("clf"));

        mgr.add_preset(preset);
        assert_eq!(mgr.presets.len(), 1);
        assert_eq!(mgr.get_preset("CriticalErrors").unwrap().pattern, "CRITICAL");

        // History management
        mgr.record_history("query 1");
        mgr.record_history("query 2");
        mgr.record_history("query 1"); // Should move to front without duplication
        assert_eq!(mgr.recent_history, vec!["query 1", "query 2"]);

        // Serialization
        let json = mgr.to_json();
        assert!(json.contains("CriticalErrors"));
        assert!(json.contains("query 1"));

        mgr.clear_history();
        assert!(mgr.recent_history.is_empty());
        assert!(mgr.delete_preset("CriticalErrors"));
        assert_eq!(mgr.presets.len(), 0);
    }
}
