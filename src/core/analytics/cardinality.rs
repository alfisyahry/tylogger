use std::collections::HashMap;

/// An entry in the Space-Saving Top-K table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopKEntry {
    pub item: String,
    pub count: u64,
    pub error: u64,
}

/// Space-Saving algorithm for tracking Top-K frequent items in a stream with bounded memory.
/// Guarantees that any item occurring > N/K times is retained, and count is bounded:
/// true_count <= count <= true_count + error.
#[derive(Debug, Clone)]
pub struct SpaceSavingTopK {
    capacity: usize,
    entries: HashMap<String, (u64, u64)>, // item -> (count, error)
    total_processed: u64,
}

impl SpaceSavingTopK {
    pub fn new(k: usize) -> Self {
        assert!(k > 0, "Capacity must be > 0");
        Self {
            capacity: k,
            entries: HashMap::with_capacity(k),
            total_processed: 0,
        }
    }

    pub fn insert(&mut self, item: &str) {
        self.total_processed += 1;

        if let Some(entry) = self.entries.get_mut(item) {
            entry.0 += 1;
            return;
        }

        if self.entries.len() < self.capacity {
            self.entries.insert(item.to_string(), (1, 0));
            return;
        }

        // Table is full: find entry with minimum count
        let mut min_item = String::new();
        let mut min_count = u64::MAX;

        for (k, (count, _err)) in &self.entries {
            if *count < min_count {
                min_count = *count;
                min_item = k.clone();
            }
        }

        // Replace min item
        self.entries.remove(&min_item);
        self.entries
            .insert(item.to_string(), (min_count + 1, min_count));
    }

    pub fn top_k(&self) -> Vec<TopKEntry> {
        let mut list: Vec<TopKEntry> = self
            .entries
            .iter()
            .map(|(k, (c, e))| TopKEntry {
                item: k.clone(),
                count: *c,
                error: *e,
            })
            .collect();

        list.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.item.cmp(&b.item)));
        list
    }

    pub fn total_processed(&self) -> u64 {
        self.total_processed
    }

    pub fn max_error_bound(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            1.0 / (self.capacity as f64)
        }
    }
}

/// Count-Min Sketch for frequency estimation with constant memory footprint.
#[derive(Debug, Clone)]
pub struct CountMinSketch {
    depth: usize,
    width: usize,
    table: Vec<Vec<u32>>,
}

impl CountMinSketch {
    pub fn new(depth: usize, width: usize) -> Self {
        let depth = depth.max(1);
        let width = width.max(16);
        Self {
            depth,
            width,
            table: vec![vec![0; width]; depth],
        }
    }

    fn hash(&self, item: &str, seed: usize) -> usize {
        let mut h: u64 = 0xcbf29ce484222325 ^ (seed as u64);
        for b in item.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        (h as usize) % self.width
    }

    pub fn insert(&mut self, item: &str) {
        for d in 0..self.depth {
            let col = self.hash(item, d);
            self.table[d][col] = self.table[d][col].saturating_add(1);
        }
    }

    pub fn estimate(&self, item: &str) -> u32 {
        let mut min_val = u32::MAX;
        for d in 0..self.depth {
            let col = self.hash(item, d);
            min_val = min_val.min(self.table[d][col]);
        }
        min_val
    }
}

/// HyperLogLog cardinality estimator (64 to 256 registers) for bounded memory unique count.
#[derive(Debug, Clone)]
pub struct HyperLogLog {
    registers: Vec<u8>,
    b: u8,
    m: usize,
}

impl Default for HyperLogLog {
    fn default() -> Self {
        Self::new(8) // 256 registers = 256 bytes memory
    }
}

impl HyperLogLog {
    pub fn new(b: u8) -> Self {
        let b = b.clamp(4, 12);
        let m = 1 << b;
        Self {
            registers: vec![0; m],
            b,
            m,
        }
    }

    fn hash64(item: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in item.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        // Standard MurmurHash3 64-bit finalizer (fmix64) for uniform bit distribution
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;
        h
    }

    pub fn insert(&mut self, item: &str) {
        let x = Self::hash64(item);
        let j = (x >> (64 - self.b)) as usize;
        let w = x << self.b;
        let rho = if w == 0 {
            64 - self.b + 1
        } else {
            (w.leading_zeros() + 1) as u8
        };
        if rho > self.registers[j] {
            self.registers[j] = rho;
        }
    }

    pub fn estimate(&self) -> u64 {
        let m = self.m as f64;
        let alpha = match self.m {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m),
        };

        let mut sum = 0.0;
        let mut zeros = 0;
        for &reg in &self.registers {
            sum += 2.0f64.powi(-(reg as i32));
            if reg == 0 {
                zeros += 1;
            }
        }

        let mut raw_estimate = alpha * m * m / sum;

        // Small range correction (Linear Counting)
        if raw_estimate <= 2.5 * m && zeros > 0 {
            raw_estimate = m * (m / zeros as f64).ln();
        }

        raw_estimate.round() as u64
    }
}
