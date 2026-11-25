use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

/// An item read from a spill partition.
#[derive(Debug, Eq, PartialEq)]
struct SpillHead {
    key: String,
    partition_idx: usize,
}

impl Ord for SpillHead {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap ordering by key
        other
            .key
            .cmp(&self.key)
            .then_with(|| other.partition_idx.cmp(&self.partition_idx))
    }
}

impl PartialOrd for SpillHead {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Disk spill partitioner and multi-way merge-sorter for high-cardinality exact aggregations.
pub struct DiskSpillManager {
    spill_dir: PathBuf,
    max_in_memory_items: usize,
    in_memory_buffer: Vec<String>,
    partition_files: Vec<PathBuf>,
    total_spilled_items: usize,
}

impl DiskSpillManager {
    pub fn new(max_in_memory_items: usize) -> Self {
        let pid = std::process::id();
        let token = format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let spill_dir = std::env::temp_dir().join(format!("tylogger_spill_{}_{}", pid, token));
        let _ = fs::create_dir_all(&spill_dir);

        Self {
            spill_dir,
            max_in_memory_items: max_in_memory_items.max(10),
            in_memory_buffer: Vec::with_capacity(max_in_memory_items.max(10)),
            partition_files: Vec::new(),
            total_spilled_items: 0,
        }
    }

    pub fn insert(&mut self, item: impl Into<String>) {
        self.in_memory_buffer.push(item.into());
        if self.in_memory_buffer.len() >= self.max_in_memory_items {
            self.flush_partition();
        }
    }

    fn flush_partition(&mut self) {
        if self.in_memory_buffer.is_empty() {
            return;
        }

        self.in_memory_buffer.sort();
        let part_file = self
            .spill_dir
            .join(format!("part_{}.tmp", self.partition_files.len()));
        if let Ok(f) = File::create(&part_file) {
            let mut writer = BufWriter::new(f);
            for item in &self.in_memory_buffer {
                let _ = writeln!(writer, "{}", item);
            }
            let _ = writer.flush();
            self.total_spilled_items += self.in_memory_buffer.len();
            self.partition_files.push(part_file);
        }
        self.in_memory_buffer.clear();
    }

    /// Performs streaming K-way merge sort across all spilled partitions and returns
    /// aggregated distinct items with counts.
    pub fn merge_and_aggregate(&mut self) -> Vec<(String, u64)> {
        self.flush_partition();

        if self.partition_files.is_empty() {
            return Vec::new();
        }

        let mut readers: Vec<Option<std::io::Lines<BufReader<File>>>> = Vec::new();
        let mut min_heap = BinaryHeap::new();

        for (idx, path) in self.partition_files.iter().enumerate() {
            if let Ok(file) = File::open(path) {
                let mut lines = BufReader::new(file).lines();
                if let Some(Ok(first_line)) = lines.next() {
                    min_heap.push(SpillHead {
                        key: first_line,
                        partition_idx: idx,
                    });
                    readers.push(Some(lines));
                } else {
                    readers.push(None);
                }
            } else {
                readers.push(None);
            }
        }

        let mut results = Vec::new();
        let mut current_key: Option<String> = None;
        let mut current_count: u64 = 0;

        while let Some(SpillHead { key, partition_idx }) = min_heap.pop() {
            if let Some(curr) = &current_key {
                if *curr == key {
                    current_count += 1;
                } else {
                    results.push((curr.clone(), current_count));
                    current_key = Some(key.clone());
                    current_count = 1;
                }
            } else {
                current_key = Some(key.clone());
                current_count = 1;
            }

            // Advance reader for this partition
            if let Some(Some(lines)) = readers.get_mut(partition_idx) {
                if let Some(Ok(next_line)) = lines.next() {
                    min_heap.push(SpillHead {
                        key: next_line,
                        partition_idx,
                    });
                }
            }
        }

        if let Some(curr) = current_key {
            results.push((curr, current_count));
        }

        results
    }

    pub fn total_spilled_items(&self) -> usize {
        self.total_spilled_items
    }

    pub fn partition_count(&self) -> usize {
        self.partition_files.len()
    }
}

impl Drop for DiskSpillManager {
    fn drop(&mut self) {
        for path in &self.partition_files {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir_all(&self.spill_dir);
    }
}
