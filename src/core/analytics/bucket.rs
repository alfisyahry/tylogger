use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketInterval {
    Seconds(u32),
    Minutes(u32),
    Hours(u32),
    Days(u32),
}

impl BucketInterval {
    pub fn as_nanos(&self) -> i64 {
        match self {
            Self::Seconds(s) => (*s as i64) * 1_000_000_000,
            Self::Minutes(m) => (*m as i64) * 60 * 1_000_000_000,
            Self::Hours(h) => (*h as i64) * 3600 * 1_000_000_000,
            Self::Days(d) => (*d as i64) * 86400 * 1_000_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketStats {
    pub count: u64,
    pub error_count: u64,
    pub bytes_sum: u64,
    pub min_nanos: i64,
    pub max_nanos: i64,
}

impl Default for BucketStats {
    fn default() -> Self {
        Self {
            count: 0,
            error_count: 0,
            bytes_sum: 0,
            min_nanos: i64::MAX,
            max_nanos: i64::MIN,
        }
    }
}

/// Sparse Time Bucketer allocating buckets only when events exist.
#[derive(Debug, Clone)]
pub struct SparseTimeBucketer {
    interval: BucketInterval,
    buckets: BTreeMap<i64, BucketStats>,
    total_count: u64,
    total_errors: u64,
}

impl SparseTimeBucketer {
    pub fn new(interval: BucketInterval) -> Self {
        Self {
            interval,
            buckets: BTreeMap::new(),
            total_count: 0,
            total_errors: 0,
        }
    }

    pub fn insert(&mut self, ts_nanos: i64, is_error: bool, bytes: u64) {
        let interval_nanos = self.interval.as_nanos();
        let bucket_key = if ts_nanos >= 0 {
            (ts_nanos / interval_nanos) * interval_nanos
        } else {
            ((ts_nanos - interval_nanos + 1) / interval_nanos) * interval_nanos
        };

        let stat = self.buckets.entry(bucket_key).or_default();
        stat.count += 1;
        if is_error {
            stat.error_count += 1;
            self.total_errors += 1;
        }
        stat.bytes_sum = stat.bytes_sum.saturating_add(bytes);
        stat.min_nanos = stat.min_nanos.min(ts_nanos);
        stat.max_nanos = stat.max_nanos.max(ts_nanos);
        self.total_count += 1;
    }

    pub fn buckets(&self) -> &BTreeMap<i64, BucketStats> {
        &self.buckets
    }

    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    /// Peak-preserving downsampling for display in limited horizontal pixels.
    pub fn downsample(&self, max_points: usize) -> Vec<(i64, BucketStats)> {
        if self.buckets.len() <= max_points || max_points == 0 {
            return self.buckets.iter().map(|(k, v)| (*k, *v)).collect();
        }

        let chunk_size = self.buckets.len().div_ceil(max_points);
        let mut downsampled = Vec::with_capacity(max_points);
        let items: Vec<(&i64, &BucketStats)> = self.buckets.iter().collect();

        for chunk in items.chunks(chunk_size) {
            let mut merged = BucketStats::default();
            let mut peak_count = 0;
            let mut peak_time = *chunk[0].0;

            for (ts, stat) in chunk {
                merged.count += stat.count;
                merged.error_count += stat.error_count;
                merged.bytes_sum += stat.bytes_sum;
                merged.min_nanos = merged.min_nanos.min(stat.min_nanos);
                merged.max_nanos = merged.max_nanos.max(stat.max_nanos);

                // Preserve peak timestamp
                if stat.count > peak_count {
                    peak_count = stat.count;
                    peak_time = **ts;
                }
            }

            downsampled.push((peak_time, merged));
        }

        downsampled
    }
}
