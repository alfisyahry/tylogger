use std::sync::atomic::{AtomicUsize, Ordering};

/// Default global memory budget across all documents and caches: 256 MB.
pub const DEFAULT_GLOBAL_MEMORY_BUDGET: usize = 256 * 1024 * 1024;

/// Maximum line length for UI preview/rendering: 1 MB.
pub const MAX_LINE_PREVIEW_BYTES: usize = 1024 * 1024;

/// Centralized memory budget manager that coordinates memory allocation
/// across open documents, raw readers, parsed column caches, and filter results.
#[derive(Debug)]
pub struct GlobalMemoryBudget {
    total_budget: usize,
    raw_cache_bytes: AtomicUsize,
    decoded_lines_bytes: AtomicUsize,
    parsed_columns_bytes: AtomicUsize,
    filter_index_bytes: AtomicUsize,
    highlight_marker_bytes: AtomicUsize,
}

impl Default for GlobalMemoryBudget {
    fn default() -> Self {
        Self::new(DEFAULT_GLOBAL_MEMORY_BUDGET)
    }
}

impl GlobalMemoryBudget {
    pub fn new(total_budget: usize) -> Self {
        Self {
            total_budget: total_budget.max(1024 * 1024),
            raw_cache_bytes: AtomicUsize::new(0),
            decoded_lines_bytes: AtomicUsize::new(0),
            parsed_columns_bytes: AtomicUsize::new(0),
            filter_index_bytes: AtomicUsize::new(0),
            highlight_marker_bytes: AtomicUsize::new(0),
        }
    }

    pub fn total_budget(&self) -> usize {
        self.total_budget
    }

    pub fn current_usage(&self) -> usize {
        self.raw_cache_bytes.load(Ordering::Relaxed)
            + self.decoded_lines_bytes.load(Ordering::Relaxed)
            + self.parsed_columns_bytes.load(Ordering::Relaxed)
            + self.filter_index_bytes.load(Ordering::Relaxed)
            + self.highlight_marker_bytes.load(Ordering::Relaxed)
    }

    pub fn is_over_budget(&self) -> bool {
        self.current_usage() >= self.total_budget
    }

    pub fn available_bytes(&self) -> usize {
        self.total_budget.saturating_sub(self.current_usage())
    }

    // Category allocation updates
    pub fn add_raw_cache(&self, bytes: usize) {
        self.raw_cache_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_raw_cache(&self, bytes: usize) {
        self.raw_cache_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(bytes))
            })
            .ok();
    }

    pub fn add_decoded_lines(&self, bytes: usize) {
        self.decoded_lines_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_decoded_lines(&self, bytes: usize) {
        self.decoded_lines_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(bytes))
            })
            .ok();
    }

    pub fn add_parsed_columns(&self, bytes: usize) {
        self.parsed_columns_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_parsed_columns(&self, bytes: usize) {
        self.parsed_columns_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(bytes))
            })
            .ok();
    }

    pub fn add_filter_index(&self, bytes: usize) {
        self.filter_index_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_filter_index(&self, bytes: usize) {
        self.filter_index_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(bytes))
            })
            .ok();
    }

    pub fn add_highlight_markers(&self, bytes: usize) {
        self.highlight_marker_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_highlight_markers(&self, bytes: usize) {
        self.highlight_marker_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(bytes))
            })
            .ok();
    }
}

/// Helper function to safely truncate oversized log lines (> 1 MB) for rendering/preview
/// without panicking, corrupting UTF-8 characters, or hanging GDI text renderers.
pub fn format_safe_preview_line(raw: &str, max_bytes: usize) -> (String, bool) {
    if raw.len() <= max_bytes {
        return (raw.to_string(), false);
    }

    // Find the nearest valid UTF-8 character boundary before max_bytes
    let mut boundary = max_bytes;
    while boundary > 0 && !raw.is_char_boundary(boundary) {
        boundary -= 1;
    }

    let truncated_slice = &raw[..boundary];
    let indicator = format!(
        "{}... [Truncated: {} bytes total, preview limited to {} bytes]",
        truncated_slice,
        raw.len(),
        max_bytes
    );
    (indicator, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_memory_budget_tracking() {
        let budget = GlobalMemoryBudget::new(10 * 1024 * 1024);
        assert_eq!(budget.current_usage(), 0);
        assert!(!budget.is_over_budget());

        budget.add_raw_cache(4 * 1024 * 1024);
        budget.add_decoded_lines(4 * 1024 * 1024);
        assert_eq!(budget.current_usage(), 8 * 1024 * 1024);
        assert!(!budget.is_over_budget());

        budget.add_parsed_columns(3 * 1024 * 1024);
        assert_eq!(budget.current_usage(), 11 * 1024 * 1024);
        assert!(budget.is_over_budget());

        budget.sub_raw_cache(4 * 1024 * 1024);
        assert_eq!(budget.current_usage(), 7 * 1024 * 1024);
        assert!(!budget.is_over_budget());
    }

    #[test]
    fn test_oversized_line_truncation() {
        let normal_line = "2026-11-25 10:00:00 [INFO] System ok";
        let (preview, truncated) = format_safe_preview_line(normal_line, 100);
        assert!(!truncated);
        assert_eq!(preview, normal_line);

        let large_line = "A".repeat(500);
        let (preview, truncated) = format_safe_preview_line(&large_line, 100);
        assert!(truncated);
        assert!(preview.starts_with(&"A".repeat(100)));
        assert!(preview.contains("[Truncated: 500 bytes total"));
    }
}
