use crate::core::analytics::bucket::BucketStats;

pub struct TimelineView {
    pub is_visible: bool,
    pub height: i32,
    pub selected_bucket: Option<usize>,
}

impl Default for TimelineView {
    fn default() -> Self {
        Self::new()
    }
}

impl TimelineView {
    pub fn new() -> Self {
        Self {
            is_visible: true,
            height: 120,
            selected_bucket: None,
        }
    }

    /// Calculates bucket index from mouse X coordinate.
    pub fn hit_test(&self, x: i32, width: i32, bucket_count: usize) -> Option<usize> {
        if bucket_count == 0 || width <= 0 || x < 0 || x >= width {
            return None;
        }
        let bucket_w = (width as f64) / (bucket_count as f64);
        let idx = ((x as f64) / bucket_w) as usize;
        if idx < bucket_count {
            Some(idx)
        } else {
            None
        }
    }

    /// Prepares bar render rectangles for native GDI painting.
    pub fn compute_bars(
        &self,
        buckets: &[(i64, BucketStats)],
        width: i32,
        height: i32,
    ) -> Vec<(i32, i32, i32, i32, bool)> {
        // Returns (x, y, bar_width, bar_height, has_error)
        if buckets.is_empty() || width <= 0 || height <= 0 {
            return Vec::new();
        }

        let max_count = buckets
            .iter()
            .map(|(_, b)| b.count)
            .max()
            .unwrap_or(1)
            .max(1);
        let bar_w = ((width as f64) / (buckets.len() as f64)).max(2.0);
        let avail_h = (height - 24).max(10) as f64;

        let mut bars = Vec::with_capacity(buckets.len());
        for (i, (_, b)) in buckets.iter().enumerate() {
            let bar_h = ((b.count as f64) / (max_count as f64) * avail_h).max(2.0) as i32;
            let x = ((i as f64) * bar_w) as i32;
            let y = height - bar_h - 4;
            let w = (bar_w - 1.0).max(1.0) as i32;
            let has_error = b.error_count > 0;
            bars.push((x, y, w, bar_h, has_error));
        }

        bars
    }
}
