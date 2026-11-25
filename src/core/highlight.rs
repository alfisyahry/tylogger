use crate::core::search::{SearchEngine, SearchOptions};

/// Single highlighting rule defining pattern, styling, and priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightRule {
    pub id: u32,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub column_index: Option<usize>,
    pub fg_color: u32, // COLORREF: 0x00BBGGRR
    pub bg_color: u32,
    pub bold: bool,
    pub priority: i32, // Higher numbers take precedence
    pub marker_bar: bool,
    pub enabled: bool,
}

impl HighlightRule {
    pub fn new(
        id: u32,
        name: &str,
        pattern: &str,
        fg_color: u32,
        bg_color: u32,
        priority: i32,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            pattern: pattern.to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: false,
            column_index: None,
            fg_color,
            bg_color,
            bold: false,
            priority,
            marker_bar: true,
            enabled: true,
        }
    }

    pub fn matches(&self, line: &str, parsed_cols: &[String]) -> bool {
        if !self.enabled || self.pattern.is_empty() {
            return false;
        }

        let target_str = if let Some(col_idx) = self.column_index {
            parsed_cols.get(col_idx).map(|s| s.as_str()).unwrap_or("")
        } else {
            line
        };

        let opts = if self.is_regex {
            SearchOptions::new_regex(self.case_sensitive)
        } else {
            SearchOptions::new_plain(self.case_sensitive, self.whole_word)
        };

        SearchEngine::matches_line(target_str, &self.pattern, &opts)
    }
}

/// Named collection of highlight rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightGroup {
    pub name: String,
    pub rules: Vec<HighlightRule>,
}

impl HighlightGroup {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            rules: Vec::new(),
        }
    }

    /// Default severity preset group: TRACE, DEBUG, INFO, WARN, ERROR, CRITICAL, FATAL
    pub fn default_severity_group() -> Self {
        let mut group = Self::new("Severity Levels");
        // Colors formatted as COLORREF 0x00BBGGRR
        group.add_rule(HighlightRule::new(
            1, "FATAL", "FATAL", 0x00FFFFFF, 0x00000088, 100,
        )); // White on Dark Red
        group.add_rule(HighlightRule::new(
            2, "CRITICAL", "CRITICAL", 0x00FFFFFF, 0x002222AA, 90,
        )); // White on Red
        group.add_rule(HighlightRule::new(
            3, "ERROR", "ERROR", 0x00C1B1FF, 0x0015122A, 80,
        )); // Soft Red text
        group.add_rule(HighlightRule::new(
            4, "WARN", "WARN", 0x0083C1E5, 0x0018303D, 60,
        )); // Gold/Yellow text
        group.add_rule(HighlightRule::new(
            5, "INFO", "INFO", 0x00EEEEEE, 0x00000000, 40,
        ));
        group.add_rule(HighlightRule::new(
            6, "DEBUG", "DEBUG", 0x00A0A0A0, 0x00000000, 20,
        ));
        group.add_rule(HighlightRule::new(
            7, "TRACE", "TRACE", 0x00707070, 0x00000000, 10,
        ));
        group
    }

    /// Preset for HTTP Status codes: 5xx, 4xx, 3xx, 2xx
    pub fn http_status_group() -> Self {
        let mut group = Self::new("HTTP Status Codes");
        group.add_rule(HighlightRule::new(
            10,
            "5xx Server Error",
            " 50",
            0x00FFFFFF,
            0x00000088,
            80,
        ));
        group.add_rule(HighlightRule::new(
            11,
            "4xx Client Error",
            " 40",
            0x0083C1E5,
            0x0018303D,
            60,
        ));
        group.add_rule(HighlightRule::new(
            12,
            "2xx Success",
            " 20",
            0x0090EE90,
            0x00000000,
            40,
        ));
        group
    }

    pub fn add_rule(&mut self, rule: HighlightRule) {
        self.rules.push(rule);
        // Sort descending by priority so highest priority rule is evaluated first
        self.rules.sort_by_key(|b| std::cmp::Reverse(b.priority));
    }

    /// Evaluates a line against all active rules in priority order.
    /// Returns the winning rule if any matches.
    pub fn evaluate_line<'a>(
        &'a self,
        line: &str,
        parsed_cols: &[String],
    ) -> Option<&'a HighlightRule> {
        self.rules
            .iter()
            .find(|rule| rule.matches(line, parsed_cols))
    }

    /// Navigates to the next or previous highlighted line (F8 / Shift+F8 upstream).
    /// `direction`: +1 for next, -1 for previous.
    pub fn find_next_highlighted<F>(
        &self,
        total_lines: usize,
        current_idx: usize,
        direction: i32,
        mut get_line: F,
    ) -> Option<usize>
    where
        F: FnMut(usize) -> Option<String>,
    {
        if total_lines == 0 || self.rules.is_empty() {
            return None;
        }

        let empty_cols = Vec::new();
        if direction > 0 {
            // Forward search (F8)
            for idx in (current_idx + 1)..total_lines {
                if let Some(line) = get_line(idx) {
                    if self.evaluate_line(&line, &empty_cols).is_some() {
                        return Some(idx);
                    }
                }
            }
        } else {
            // Backward search (Shift+F8)
            for idx in (0..current_idx).rev() {
                if let Some(line) = get_line(idx) {
                    if self.evaluate_line(&line, &empty_cols).is_some() {
                        return Some(idx);
                    }
                }
            }
        }

        None
    }

    /// Collects all highlighted line indices for vertical Marker Bar rendering.
    pub fn generate_marker_ticks<F>(&self, total_lines: usize, mut get_line: F) -> Vec<usize>
    where
        F: FnMut(usize) -> Option<String>,
    {
        let mut ticks = Vec::new();
        let empty_cols = Vec::new();

        for idx in 0..total_lines {
            if let Some(line) = get_line(idx) {
                if let Some(rule) = self.evaluate_line(&line, &empty_cols) {
                    if rule.marker_bar {
                        ticks.push(idx);
                    }
                }
            }
        }

        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_priority_resolution() {
        let mut group = HighlightGroup::new("PriorityTest");
        // Rule 1: Lower priority (50)
        group.add_rule(HighlightRule::new(
            1,
            "ErrorRule",
            "ERROR",
            0x111111,
            0x222222,
            50,
        ));
        // Rule 2: Higher priority (100)
        group.add_rule(HighlightRule::new(
            2,
            "FatalRule",
            "FATAL",
            0xFFFFFF,
            0x000088,
            100,
        ));

        let line_both = "2026-11-25 10:00:00 [ERROR] Connection FATAL failure";
        let winning_rule = group.evaluate_line(line_both, &[]).unwrap();

        // FATAL has priority 100 > ERROR priority 50 -> FatalRule must win
        assert_eq!(winning_rule.name, "FatalRule");
        assert_eq!(winning_rule.priority, 100);
    }

    #[test]
    fn test_highlight_navigation_f8_shift_f8() {
        let lines = [
            "line 0: neutral message",
            "line 1: WARN disk low",
            "line 2: neutral message",
            "line 3: ERROR timeout",
            "line 4: neutral message",
        ];
        let group = HighlightGroup::default_severity_group();

        // Next from 0 -> Line 1 (WARN)
        let next_from_0 =
            group.find_next_highlighted(lines.len(), 0, 1, |i| lines.get(i).map(|s| s.to_string()));
        assert_eq!(next_from_0, Some(1));

        // Next from 1 -> Line 3 (ERROR)
        let next_from_1 =
            group.find_next_highlighted(lines.len(), 1, 1, |i| lines.get(i).map(|s| s.to_string()));
        assert_eq!(next_from_1, Some(3));

        // Prev from 3 -> Line 1 (WARN)
        let prev_from_3 = group
            .find_next_highlighted(lines.len(), 3, -1, |i| lines.get(i).map(|s| s.to_string()));
        assert_eq!(prev_from_3, Some(1));
    }
}
