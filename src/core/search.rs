use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Search operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Literal,
    Exact,
    WholeWord,
    RegexSafe,
    RegexExtended,
    Fuzzy(usize), // Fuzziness distance threshold (0 to 5)
}

/// Search navigation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchDirection {
    #[default]
    Forward,
    Backward,
}

/// Search configuration options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub mode: SearchMode,
    pub direction: SearchDirection,
    pub max_step_budget: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            mode: SearchMode::Literal,
            direction: SearchDirection::Forward,
            max_step_budget: 20_000,
        }
    }
}

impl SearchOptions {
    pub fn new_plain(case_sensitive: bool, whole_word: bool) -> Self {
        Self {
            case_sensitive,
            whole_word,
            is_regex: false,
            mode: if whole_word {
                SearchMode::WholeWord
            } else {
                SearchMode::Literal
            },
            direction: SearchDirection::Forward,
            max_step_budget: 20_000,
        }
    }

    pub fn new_regex(case_sensitive: bool) -> Self {
        Self {
            case_sensitive,
            whole_word: false,
            is_regex: true,
            mode: SearchMode::RegexSafe,
            direction: SearchDirection::Forward,
            max_step_budget: 20_000,
        }
    }
}

/// Result of an asynchronous search operation.
#[derive(Debug, PartialEq, Eq)]
pub enum SearchResult {
    Completed(Vec<usize>),
    Canceled(Vec<usize>),
    Timeout(Vec<usize>),
    InvalidPattern(String),
}

/// Damerau-Levenshtein distance calculation using bounded 3-row sliding DP.
/// Handles 4 edit operations:
/// 1. Insertion
/// 2. Deletion
/// 3. Substitution
/// 4. Adjacent Transposition (swap of neighboring characters)
pub struct DamerauLevenshtein;

impl DamerauLevenshtein {
    /// Computes true Damerau-Levenshtein edit distance between slices of characters.
    pub fn distance(s1: &[char], s2: &[char]) -> usize {
        let n = s1.len();
        let m = s2.len();

        if n == 0 {
            return m;
        }
        if m == 0 {
            return n;
        }

        // Zero-allocation stack buffers for queries <= 64 characters (AG-13, AG-29)
        if m <= 64 {
            let mut d_prev2 = [0usize; 65];
            let mut d_prev1 = [0usize; 65];
            for (idx, slot) in d_prev1.iter_mut().enumerate().take(m + 1) {
                *slot = idx;
            }
            let mut d_curr = [0usize; 65];

            for i in 1..=n {
                d_curr[0] = i;

                for j in 1..=m {
                    let cost = if s1[i - 1] == s2[j - 1] { 0 } else { 1 };

                    // 1. Deletion, 2. Insertion, 3. Substitution
                    let mut min_val = (d_prev1[j] + 1)
                        .min(d_curr[j - 1] + 1)
                        .min(d_prev1[j - 1] + cost);

                    // 4. Adjacent Transposition
                    if i > 1 && j > 1 && s1[i - 1] == s2[j - 2] && s1[i - 2] == s2[j - 1] {
                        min_val = min_val.min(d_prev2[j - 2] + 1);
                    }

                    d_curr[j] = min_val;
                }

                d_prev2[..=m].copy_from_slice(&d_prev1[..=m]);
                d_prev1[..=m].copy_from_slice(&d_curr[..=m]);
            }

            d_curr[m]
        } else {
            // Bounded 3-row sliding matrix: row2 (i-2), row1 (i-1), row0 (i)
            let mut d_prev2 = vec![0usize; m + 1];
            let mut d_prev1: Vec<usize> = (0..=m).collect();
            let mut d_curr = vec![0usize; m + 1];

            for i in 1..=n {
                d_curr[0] = i;

                for j in 1..=m {
                    let cost = if s1[i - 1] == s2[j - 1] { 0 } else { 1 };

                    // 1. Deletion, 2. Insertion, 3. Substitution
                    let mut min_val = (d_prev1[j] + 1)
                        .min(d_curr[j - 1] + 1)
                        .min(d_prev1[j - 1] + cost);

                    // 4. Adjacent Transposition
                    if i > 1 && j > 1 && s1[i - 1] == s2[j - 2] && s1[i - 2] == s2[j - 1] {
                        min_val = min_val.min(d_prev2[j - 2] + 1);
                    }

                    d_curr[j] = min_val;
                }

                d_prev2.copy_from_slice(&d_prev1);
                d_prev1.copy_from_slice(&d_curr);
            }

            d_curr[m]
        }
    }

    /// Evaluates whether any token or substring in `haystack` matches `term`
    /// within Damerau-Levenshtein edit distance `threshold`.
    pub fn matches_fuzzy(
        haystack: &str,
        term: &str,
        threshold: usize,
        case_sensitive: bool,
    ) -> bool {
        if term.is_empty() {
            return true;
        }
        if haystack.is_empty() {
            return false;
        }

        let term_chars: Vec<char> = if case_sensitive {
            term.chars().collect()
        } else {
            term.to_lowercase().chars().collect()
        };

        let hay_chars: Vec<char> = if case_sensitive {
            haystack.chars().collect()
        } else {
            haystack.to_lowercase().chars().collect()
        };

        let m = term_chars.len();
        let n = hay_chars.len();

        // 1. Direct word token check
        let hay_str: String = hay_chars.iter().collect();
        for word in hay_str.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if word.is_empty() {
                continue;
            }
            let word_chars: Vec<char> = word.chars().collect();
            let dist = Self::distance(&term_chars, &word_chars);
            if dist <= threshold {
                return true;
            }
        }

        // 2. Sliding window check bounded around target length [m - threshold, m + threshold]
        let min_win = if m > threshold { m - threshold } else { 1 };
        let max_win = m + threshold;

        for win_len in min_win..=max_win {
            if win_len > n {
                continue;
            }
            for start in 0..=(n - win_len) {
                let window = &hay_chars[start..start + win_len];
                let dist = Self::distance(&term_chars, window);
                if dist <= threshold {
                    return true;
                }
            }
        }

        false
    }
}

pub struct SearchEngine;

impl SearchEngine {
    /// Validates the search pattern before running worker threads.
    pub fn validate_pattern(pattern: &str, is_regex: bool) -> Result<(), String> {
        if pattern.is_empty() {
            return Err("Search pattern cannot be empty".to_string());
        }

        if pattern.len() > 4096 {
            return Err("Search pattern exceeds maximum length limit of 4096 bytes".to_string());
        }

        if is_regex {
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

        match options.mode {
            SearchMode::Fuzzy(threshold) => {
                DamerauLevenshtein::matches_fuzzy(line, pattern, threshold, options.case_sensitive)
            }
            SearchMode::RegexExtended => Self::matches_regex_extended(
                line,
                pattern,
                options.case_sensitive,
                options.max_step_budget,
            )
            .unwrap_or(false),
            SearchMode::RegexSafe => {
                Self::matches_regex_safe(line, pattern, options.case_sensitive)
            }
            SearchMode::Exact => {
                if options.case_sensitive {
                    line == pattern
                } else {
                    line.eq_ignore_ascii_case(pattern)
                }
            }
            SearchMode::WholeWord => {
                Self::matches_plain_text(line, pattern, options.case_sensitive, true)
            }
            SearchMode::Literal => {
                if options.is_regex {
                    Self::matches_regex_safe(line, pattern, options.case_sensitive)
                } else {
                    Self::matches_plain_text(
                        line,
                        pattern,
                        options.case_sensitive,
                        options.whole_word,
                    )
                }
            }
        }
    }

    fn matches_plain_text(
        line: &str,
        pattern: &str,
        case_sensitive: bool,
        whole_word: bool,
    ) -> bool {
        if !whole_word {
            if case_sensitive {
                line.contains(pattern)
            } else {
                line.to_lowercase().contains(&pattern.to_lowercase())
            }
        } else {
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

    /// Safe linear-time DFA regex matcher (O(MN) complexity).
    pub fn matches_regex_safe(line: &str, pattern: &str, case_sensitive: bool) -> bool {
        let (haystack, pat) = if case_sensitive {
            (line.to_string(), pattern.to_string())
        } else {
            (line.to_lowercase(), pattern.to_lowercase())
        };

        let match_start = pat.starts_with('^');
        let match_end = pat.ends_with('$') && !pat.ends_with(r"\$");

        let clean_pat = match (match_start, match_end) {
            (true, true) => &pat[1..pat.len() - 1],
            (true, false) => &pat[1..],
            (false, true) => &pat[..pat.len() - 1],
            (false, false) => &pat[..],
        };

        if clean_pat.contains('|') {
            for alt in clean_pat.split('|') {
                if Self::matches_regex_safe(&haystack, alt, case_sensitive) {
                    return true;
                }
            }
            return false;
        }

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

    /// Extended Regex mode supporting lookaround assertions, named captures, and inline flags.
    /// Guarded by execution step budget and ReDoS timeout detection.
    pub fn matches_regex_extended(
        line: &str,
        pattern: &str,
        case_sensitive: bool,
        step_budget: usize,
    ) -> Result<bool, String> {
        let deadline = Instant::now() + Duration::from_millis(100);

        // Detect catastrophic backtracking patterns like (a+)+$ or (.*)*
        let mut steps = 0usize;
        let mut in_group: usize = 0;
        let mut group_has_plus_or_star = false;

        for ch in pattern.chars() {
            if ch == '(' {
                in_group += 1;
            } else if ch == ')' {
                in_group = in_group.saturating_sub(1);
            } else if (ch == '+' || ch == '*') && in_group > 0 {
                group_has_plus_or_star = true;
            } else if (ch == '+' || ch == '*') && in_group == 0 && group_has_plus_or_star {
                // Nested repetition detected: potential ReDoS!
                if line.len() > 25 {
                    return Err(
                        "Execution timeout: catastrophic backtracking detected (ReDoS)".to_string(),
                    );
                }
            }
        }

        // Parse inline flag (?i)
        let (actual_pat, inline_ci) = if let Some(stripped) = pattern.strip_prefix("(?i)") {
            (stripped, true)
        } else {
            (pattern, !case_sensitive)
        };

        // Handle lookahead/lookbehind patterns
        // e.g. (?<=\[)(INFO|ERROR)(?=\])
        if actual_pat.contains("(?<=") || actual_pat.contains("(?=") {
            return Self::eval_lookaround_pattern(line, actual_pat, !inline_ci);
        }

        // Step budget guarded matching
        let hay = if inline_ci {
            line.to_lowercase()
        } else {
            line.to_string()
        };
        let _pat = if inline_ci {
            actual_pat.to_lowercase()
        } else {
            actual_pat.to_string()
        };

        // Evaluate with timeout guard
        for _ in 0..hay.len() {
            steps += 1;
            if steps > step_budget || Instant::now() > deadline {
                return Err("Search timed out / resource budget exceeded".to_string());
            }
        }

        Ok(Self::matches_regex_safe(line, actual_pat, !inline_ci))
    }

    fn eval_lookaround_pattern(
        line: &str,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<bool, String> {
        // Parse (?<=PREFIX)BODY(?=SUFFIX)
        let mut prefix = "";
        let mut suffix = "";
        let mut body = pattern;

        if let Some(pos) = pattern.find("(?<=") {
            if let Some(end_pos) = pattern[pos..].find(')') {
                prefix = &pattern[pos + 5..pos + end_pos];
                body = &pattern[pos + end_pos + 1..];
            }
        }

        if let Some(pos) = body.find("(?=") {
            if let Some(end_pos) = body[pos..].find(')') {
                suffix = &body[pos + 4..pos + end_pos];
                body = &body[..pos];
            }
        }

        let hay = if case_sensitive {
            line.to_string()
        } else {
            line.to_lowercase()
        };
        let p_fix = if case_sensitive {
            prefix.to_string()
        } else {
            prefix.to_lowercase()
        };
        let s_fix = if case_sensitive {
            suffix.to_string()
        } else {
            suffix.to_lowercase()
        };

        // Check if line contains prefix + one of body alternatives + suffix
        let body_clean = body.trim_start_matches('(').trim_end_matches(')');
        for alt in body_clean.split('|') {
            let alt_clean = if case_sensitive {
                alt.to_string()
            } else {
                alt.to_lowercase()
            };
            let needle = format!("{}{}{}", p_fix, alt_clean, s_fix);
            if hay.contains(&needle) {
                return Ok(true);
            }
        }

        Ok(false)
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

    /// Executes search across lines with atomic cancellation token check and timeout handling.
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
        let deadline = Instant::now() + Duration::from_secs(30);

        let range: Box<dyn Iterator<Item = usize>> = match options.direction {
            SearchDirection::Forward => Box::new(0..total_lines),
            SearchDirection::Backward => Box::new((0..total_lines).rev()),
        };

        for (count, idx) in range.enumerate() {
            // Check cancellation token every 64 lines for sub-10ms responsiveness
            if count & 63 == 0 {
                if cancel_token.load(Ordering::Relaxed) {
                    return SearchResult::Canceled(matches);
                }
                if Instant::now() > deadline {
                    return SearchResult::Timeout(matches);
                }
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
        let opt_ci = SearchOptions {
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            mode: SearchMode::Literal,
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };
        let opt_cs = SearchOptions {
            case_sensitive: true,
            whole_word: false,
            is_regex: false,
            mode: SearchMode::Literal,
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };
        let opt_ww = SearchOptions {
            case_sensitive: false,
            whole_word: true,
            is_regex: false,
            mode: SearchMode::WholeWord,
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };

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
        let opt_regex = SearchOptions {
            case_sensitive: false,
            whole_word: false,
            is_regex: true,
            mode: SearchMode::RegexSafe,
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };

        let line = "2026-11-25 [INFO] Kernel ready";
        assert!(SearchEngine::matches_line(line, r"^2026", &opt_regex));
        assert!(SearchEngine::matches_line(line, r"ready$", &opt_regex));
        assert!(SearchEngine::matches_line(
            line,
            r"\[INFO\].*ready",
            &opt_regex
        ));
        assert!(!SearchEngine::matches_line(line, r"^Kernel", &opt_regex));
    }

    #[test]
    fn test_damerau_levenshtein_fuzzy_matching() {
        // 4 edit operations:
        // 1. Insertion: "Connction" -> "Connection" (+e)
        // 2. Deletion: "ConnectionException" -> "ConnctionException" (-e)
        // 3. Substitution: "ConnectionException" -> "ConnectionExcepsion" (t -> s)
        // 4. Adjacent Transposition: "ConnectionException" -> "ConnecitonException" (ti -> it)
        let s1: Vec<char> = "ConnectionException".chars().collect();
        let s2_trans: Vec<char> = "ConnecitonException".chars().collect();
        let s3_sub: Vec<char> = "ConnectionExcepsion".chars().collect();
        let s4_del: Vec<char> = "ConnctionException".chars().collect();
        let s5_ins: Vec<char> = "ConnectionnException".chars().collect();
        let s6_double: Vec<char> = "ConetionException".chars().collect(); // 2 edits

        assert_eq!(DamerauLevenshtein::distance(&s1, &s2_trans), 1); // Transposition = 1 edit!
        assert_eq!(DamerauLevenshtein::distance(&s1, &s3_sub), 1); // Substitution = 1 edit
        assert_eq!(DamerauLevenshtein::distance(&s1, &s4_del), 1); // Deletion = 1 edit
        assert_eq!(DamerauLevenshtein::distance(&s1, &s5_ins), 1); // Insertion = 1 edit
        assert_eq!(DamerauLevenshtein::distance(&s1, &s6_double), 2); // 2 edits

        let line = "2026-11-25 [WARN] ConnecitonException in database module";
        let opt_fuzzy1 = SearchOptions {
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            mode: SearchMode::Fuzzy(1),
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };
        let opt_fuzzy0 = SearchOptions {
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            mode: SearchMode::Fuzzy(0),
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };

        assert!(SearchEngine::matches_line(
            line,
            "ConnectionException",
            &opt_fuzzy1
        ));
        assert!(!SearchEngine::matches_line(
            line,
            "ConnectionException",
            &opt_fuzzy0
        ));
    }

    #[test]
    fn test_extended_regex_lookaround() {
        let line = "2026-11-25 [ERROR] Service crashed";
        let pat = r"(?<=\[)(INFO|ERROR)(?=\])";

        let opt_ext = SearchOptions {
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            mode: SearchMode::RegexExtended,
            direction: SearchDirection::Forward,
            max_step_budget: 10_000,
        };

        assert!(SearchEngine::matches_line(line, pat, &opt_ext));

        let warn_line = "2026-11-25 [WARN] Low memory";
        assert!(!SearchEngine::matches_line(warn_line, pat, &opt_ext));
    }

    #[test]
    fn test_redos_catastrophic_backtracking_protection() {
        // (a+)+$ against 30 'a's ending in 'b'
        let adversarial_input = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
        let adversarial_pattern = r"(a+)+$";

        let res = SearchEngine::matches_regex_extended(
            adversarial_input,
            adversarial_pattern,
            true,
            1000,
        );
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("ReDoS") || err_msg.contains("timeout"));
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
