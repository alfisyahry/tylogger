use crate::core::search::{SearchEngine, SearchOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct FilterCriteria {
    pub pattern: String,
    pub mode: FilterMode,
    pub options: SearchOptions,
}

impl FilterCriteria {
    pub fn new_include(pattern: &str, case_sensitive: bool) -> Self {
        Self {
            pattern: pattern.to_string(),
            mode: FilterMode::Include,
            options: SearchOptions {
                case_sensitive,
                whole_word: false,
                is_regex: false,
            },
        }
    }

    pub fn new_exclude(pattern: &str, case_sensitive: bool) -> Self {
        Self {
            pattern: pattern.to_string(),
            mode: FilterMode::Exclude,
            options: SearchOptions {
                case_sensitive,
                whole_word: false,
                is_regex: false,
            },
        }
    }
}

pub struct FilterEngine;

impl FilterEngine {
    /// Applies the filter criteria across lines and returns matching 64-bit line indices.
    /// Does not duplicate string memory; results strictly reference source line numbers.
    pub fn apply_filter<F>(
        total_lines: usize,
        mut line_getter: F,
        criteria: &FilterCriteria,
    ) -> Vec<usize>
    where
        F: FnMut(usize) -> Option<String>,
    {
        if criteria.pattern.is_empty() {
            return (0..total_lines).collect();
        }

        let mut matches = Vec::new();

        for idx in 0..total_lines {
            if let Some(line) = line_getter(idx) {
                let matches_pattern =
                    SearchEngine::matches_line(&line, &criteria.pattern, &criteria.options);
                let should_include = match criteria.mode {
                    FilterMode::Include => matches_pattern,
                    FilterMode::Exclude => !matches_pattern,
                };

                if should_include {
                    matches.push(idx);
                }
            }
        }

        matches
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
        );
        assert_eq!(inc_matches, vec![1, 3]);

        // 2. Exclude INFO
        let crit_exc = FilterCriteria::new_exclude("INFO", false);
        let exc_matches = FilterEngine::apply_filter(
            sample.len(),
            |idx| sample.get(idx).map(|s| s.to_string()),
            &crit_exc,
        );
        assert_eq!(exc_matches, vec![1, 3, 4]);
    }
}
