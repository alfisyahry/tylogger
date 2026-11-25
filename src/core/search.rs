use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Search configuration options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
        }
    }
}

/// Result of an asynchronous search operation.
#[derive(Debug, PartialEq, Eq)]
pub enum SearchResult {
    Completed(Vec<usize>),
    Canceled(Vec<usize>),
    InvalidPattern(String),
}

pub struct SearchEngine;

impl SearchEngine {
    /// Validates the search pattern before running the worker thread.
    pub fn validate_pattern(pattern: &str, is_regex: bool) -> Result<(), String> {
        if pattern.is_empty() {
            return Err("Search pattern cannot be empty".to_string());
        }

        if is_regex {
            // Check character class bracket balance
            let mut in_bracket = false;
            let mut escape = false;

            for ch in pattern.chars() {
                if escape {
                    escape = false;
                    continue;
                }
                if ch == '\\' {
                    escape = true;
                    continue;
                }
                if ch == '[' && !in_bracket {
                    in_bracket = true;
                } else if ch == ']' && in_bracket {
                    in_bracket = false;
                }
            }

            if escape {
                return Err("Pattern ends with an unescaped trailing backslash".to_string());
            }
            if in_bracket {
                return Err("Unclosed character class '[' in regular expression".to_string());
            }
        }

        Ok(())
    }

    /// Evaluates whether `line` matches `pattern` given `options`.
    pub fn matches_line(line: &str, pattern: &str, options: &SearchOptions) -> bool {
        if pattern.is_empty() {
            return false;
        }

        if options.is_regex {
            Self::matches_regex_like(line, pattern, options.case_sensitive)
        } else {
            Self::matches_plain_text(line, pattern, options.case_sensitive, options.whole_word)
        }
    }

    fn matches_plain_text(line: &str, pattern: &str, case_sensitive: bool, whole_word: bool) -> bool {
        if !whole_word {
            if case_sensitive {
                line.contains(pattern)
            } else {
                line.to_lowercase().contains(&pattern.to_lowercase())
            }
        } else {
            // Whole word boundary matching
            let haystack = if case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            let needle = if case_sensitive {
                pattern.to_string()
            } else {
                pattern.to_lowercase()
            };

            let mut start_idx = 0;
            while let Some(pos) = haystack[start_idx..].find(&needle) {
                let actual_pos = start_idx + pos;
                let end_pos = actual_pos + needle.len();

                let left_boundary = if actual_pos == 0 {
                    true
                } else {
                    let prev_char = haystack[..actual_pos].chars().last().unwrap();
                    !prev_char.is_alphanumeric() && prev_char != '_'
                };

                let right_boundary = if end_pos >= haystack.len() {
                    true
                } else {
                    let next_char = haystack[end_pos..].chars().next().unwrap();
                    !next_char.is_alphanumeric() && next_char != '_'
                };

                if left_boundary && right_boundary {
                    return true;
                }

                start_idx = actual_pos + 1;
                if start_idx >= haystack.len() {
                    break;
                }
            }
            false
        }
    }

    fn matches_regex_like(line: &str, pattern: &str, case_sensitive: bool) -> bool {
        let (haystack, pat) = if case_sensitive {
            (line.to_string(), pattern.to_string())
        } else {
            (line.to_lowercase(), pattern.to_lowercase())
        };

        // Handle line start anchor ^ and line end anchor $
        let match_start = pat.starts_with('^');
        let match_end = pat.ends_with('$') && !pat.ends_with(r"\$");

        let clean_pat = match (match_start, match_end) {
            (true, true) => &pat[1..pat.len() - 1],
            (true, false) => &pat[1..],
            (false, true) => &pat[..pat.len() - 1],
            (false, false) => &pat[..],
        };

        // Linear-time wildcard and character-class matching
        if clean_pat.contains(".*") {
            let parts: Vec<&str> = clean_pat.split(".*").collect();
            let mut curr_haystack = &haystack[..];
            for (idx, raw_part) in parts.iter().enumerate() {
                if raw_part.is_empty() {
                    continue;
                }
                let part = Self::unescape_regex_literal(raw_part);
                if idx == 0 && match_start {
                    if !curr_haystack.starts_with(&part) {
                        return false;
                    }
                    curr_haystack = &curr_haystack[part.len()..];
                } else if let Some(p) = curr_haystack.find(&part) {
                    curr_haystack = &curr_haystack[p + part.len()..];
                } else {
                    return false;
                }
            }
            if match_end && !parts.is_empty() {
                let last_raw = *parts.last().unwrap();
                let last = Self::unescape_regex_literal(last_raw);
                if !last.is_empty() && !haystack.ends_with(&last) {
                    return false;
                }
            }
            true
        } else {
            let unescaped = Self::unescape_regex_literal(clean_pat);
            if match_start && match_end {
                haystack == unescaped
            } else if match_start {
                haystack.starts_with(&unescaped)
            } else if match_end {
                haystack.ends_with(&unescaped)
            } else {
                haystack.contains(&unescaped)
            }
        }
    }

    fn unescape_regex_literal(lit: &str) -> String {
        let mut res = String::with_capacity(lit.len());
        let mut escape = false;
        for ch in lit.chars() {
            if escape {
                res.push(ch);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else {
                res.push(ch);
            }
        }
        if escape {
            res.push('\\');
        }
        res
    }

    /// Executes search across lines with atomic cancellation token check.
    pub fn search<F>(
        total_lines: usize,
        mut line_getter: F,
        pattern: &str,
        options: &SearchOptions,
        cancel_token: Arc<AtomicBool>,
    ) -> SearchResult
    where
        F: FnMut(usize) -> Option<String>,
    {
        if let Err(e) = Self::validate_pattern(pattern, options.is_regex) {
            return SearchResult::InvalidPattern(e);
        }

        let mut matches = Vec::new();

        for idx in 0..total_lines {
            // Check cancellation token every 64 lines for sub-10ms responsiveness
            if (idx & 63 == 0) && cancel_token.load(Ordering::Relaxed) {
                return SearchResult::Canceled(matches);
            }

            if let Some(line) = line_getter(idx) {
                if Self::matches_line(&line, pattern, options) {
                    matches.push(idx);
                }
            }
        }

        SearchResult::Completed(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_search() {
        let opt_ci = SearchOptions { case_sensitive: false, whole_word: false, is_regex: false };
        let opt_cs = SearchOptions { case_sensitive: true, whole_word: false, is_regex: false };
        let opt_ww = SearchOptions { case_sensitive: false, whole_word: true, is_regex: false };

        let line = "2026-11-25 [ERROR] Service error occurred";
        assert!(SearchEngine::matches_line(line, "error", &opt_ci));
        assert!(SearchEngine::matches_line(line, "ERROR", &opt_cs));
        assert!(!SearchEngine::matches_line(line, "debug", &opt_ci));

        // Whole word checks
        assert!(SearchEngine::matches_line(line, "service", &opt_ww));
        assert!(!SearchEngine::matches_line(line, "serv", &opt_ww));
    }

    #[test]
    fn test_regex_anchors_and_wildcard() {
        let opt_regex = SearchOptions { case_sensitive: false, whole_word: false, is_regex: true };

        let line = "2026-11-25 [INFO] Kernel ready";
        assert!(SearchEngine::matches_line(line, r"^2026", &opt_regex));
        assert!(SearchEngine::matches_line(line, r"ready$", &opt_regex));
        assert!(SearchEngine::matches_line(line, r"\[INFO\].*ready", &opt_regex));
        assert!(!SearchEngine::matches_line(line, r"^Kernel", &opt_regex));
    }

    #[test]
    fn test_search_cancellation() {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();

        let cancel_trigger_line = 500;
        let mut count = 0;

        let res = SearchEngine::search(
            5000,
            |idx| {
                count += 1;
                if idx == cancel_trigger_line {
                    cancel_clone.store(true, Ordering::Relaxed);
                }
                Some(format!("Line {} data payload", idx))
            },
            "Line",
            &SearchOptions::default(),
            cancel,
        );

        match res {
            SearchResult::Canceled(matches) => {
                assert!(matches.len() >= cancel_trigger_line);
                assert!(matches.len() < 5000);
            }
            _ => panic!("Expected SearchResult::Canceled"),
        }
    }
}
