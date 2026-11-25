use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::core::analytics::bucket::{BucketInterval, BucketStats, SparseTimeBucketer};
use crate::core::analytics::cardinality::SpaceSavingTopK;
use crate::core::analytics::query::{AggregationType, AnalyticsQuery, EvaluationMode};
use crate::core::analytics::spill::DiskSpillManager;
use crate::core::columnizer::Columnizer;
use crate::core::source::LogSource;

#[derive(Debug, Clone, PartialEq)]
pub struct MetricRow {
    pub key: String,
    pub count: u64,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct AggregationResult {
    pub query_title: String,
    pub total_lines_scanned: usize,
    pub matched_records: usize,
    pub execution_millis: u64,
    pub is_exact: bool,
    pub error_bound: Option<f64>,
    pub headers: Vec<String>,
    pub rows: Vec<MetricRow>,
    pub buckets: Vec<(i64, BucketStats)>,
}

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn execute_streaming(
        source: &mut dyn LogSource,
        columnizer: &dyn Columnizer,
        query: &AnalyticsQuery,
        cancel_token: &AtomicBool,
    ) -> AggregationResult {
        let start_time = Instant::now();
        let total_lines = source.line_count();
        let col_names = columnizer.column_names();

        let mut matched_records = 0;
        let mut bucketer = SparseTimeBucketer::new(BucketInterval::Minutes(1));

        let is_exact = matches!(query.mode, EvaluationMode::Exact);
        let mut top_k = if !is_exact {
            Some(SpaceSavingTopK::new(query.limit.max(50)))
        } else {
            None
        };

        let mut exact_map: HashMap<String, (u64, Vec<f64>)> = HashMap::new();
        let mut spill_mgr = DiskSpillManager::new(50_000);

        let chunk_size = 4096;
        let mut line_buf = Vec::with_capacity(chunk_size);
        let mut current_idx = 0;

        while current_idx < total_lines {
            if cancel_token.load(Ordering::Relaxed) {
                break;
            }

            let end_idx = (current_idx + chunk_size).min(total_lines);
            line_buf.clear();
            for i in current_idx..end_idx {
                if let Ok(line) = source.get_line(i) {
                    line_buf.push(line);
                }
            }

            for line in &line_buf {
                let cols = columnizer.parse_line(line);

                // Helper to get field value
                let get_val = |field_name: &str| -> String {
                    if field_name == "*" || field_name.is_empty() {
                        return line.clone();
                    }
                    if let Some(pos) = col_names
                        .iter()
                        .position(|c| c.eq_ignore_ascii_case(field_name))
                    {
                        if pos < cols.len() {
                            return cols[pos].clone();
                        }
                    }
                    String::new()
                };

                // Check filter conditions
                let mut passes = true;
                for cond in &query.filters {
                    let val = get_val(&cond.field);
                    if !cond.evaluate(&val) {
                        passes = false;
                        break;
                    }
                }

                if !passes {
                    continue;
                }

                matched_records += 1;

                // Determine group key
                let key = if query.group_by.is_empty() {
                    "Total".to_string()
                } else {
                    let parts: Vec<String> = query.group_by.iter().map(|g| get_val(g)).collect();
                    parts.join(" | ")
                };

                // Update bucketer (if timestamp column exists, else synthetic)
                let ts = (matched_records as i64) * 1_000_000_000;
                let is_err = line.contains(" 4") || line.contains(" 5") || line.contains("ERROR");
                bucketer.insert(ts, is_err, line.len() as u64);

                // Approximate mode vs Exact mode
                if let Some(tk) = &mut top_k {
                    tk.insert(&key);
                } else {
                    // Check if memory is growing too large (>100,000 keys)
                    if exact_map.len() > 100_000 {
                        spill_mgr.insert(&key);
                    } else {
                        let entry = exact_map
                            .entry(key.clone())
                            .or_insert_with(|| (0, Vec::new()));
                        entry.0 += 1;
                    }
                }
            }

            current_idx = end_idx;
        }

        // Finalize rows
        let mut rows = Vec::new();
        let error_bound = if let Some(tk) = &top_k {
            let tk_rows = tk.top_k();
            for item in tk_rows.into_iter().take(query.limit) {
                rows.push(MetricRow {
                    key: item.item,
                    count: item.count,
                    values: vec![item.error as f64],
                });
            }
            Some(tk.max_error_bound())
        } else {
            if spill_mgr.partition_count() > 0 {
                // Merge spilled partitions
                for (k, v) in exact_map {
                    for _ in 0..v.0 {
                        spill_mgr.insert(&k);
                    }
                }
                let merged = spill_mgr.merge_and_aggregate();
                for (k, c) in merged.into_iter().take(query.limit) {
                    rows.push(MetricRow {
                        key: k,
                        count: c,
                        values: Vec::new(),
                    });
                }
            } else {
                let mut sorted_entries: Vec<(String, (u64, Vec<f64>))> =
                    exact_map.into_iter().collect();
                sorted_entries.sort_by_key(|b| std::cmp::Reverse(b.1 .0));
                for (k, (c, v)) in sorted_entries.into_iter().take(query.limit) {
                    rows.push(MetricRow {
                        key: k,
                        count: c,
                        values: v,
                    });
                }
            }
            None
        };

        let mut headers = vec![
            if query.group_by.is_empty() {
                "Metric".to_string()
            } else {
                query.group_by.join(" / ")
            },
            "Count".to_string(),
        ];
        for agg in &query.aggregates {
            if agg.agg_type != AggregationType::Count {
                headers.push(agg.alias.clone());
            }
        }

        let execution_millis = start_time.elapsed().as_millis() as u64;

        AggregationResult {
            query_title: query.title.clone(),
            total_lines_scanned: current_idx,
            matched_records,
            execution_millis,
            is_exact,
            error_bound,
            headers,
            rows,
            buckets: bucketer.downsample(120),
        }
    }
}
