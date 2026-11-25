#[derive(Debug, Clone, PartialEq)]
pub struct KeyComparison {
    pub key: String,
    pub count_a: u64,
    pub count_b: u64,
    pub rate_a: f64,
    pub rate_b: f64,
    pub diff_str: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatasetComparisonSummary {
    pub name_a: String,
    pub name_b: String,
    pub records_a: u64,
    pub records_b: u64,
    pub duration_a_secs: f64,
    pub duration_b_secs: f64,
    pub rate_a: f64,
    pub rate_b: f64,
    pub errors_a: u64,
    pub errors_b: u64,
    pub error_rate_a: f64,
    pub error_rate_b: f64,
    pub keys: Vec<KeyComparison>,
}

pub struct ComparisonEngine;

impl ComparisonEngine {
    /// Compares two log dataset summaries with observation duration normalization and
    /// safe zero-baseline division handling (RSK-08).
    pub fn compare(
        name_a: &str,
        records_a: u64,
        errors_a: u64,
        duration_a_secs: f64,
        name_b: &str,
        records_b: u64,
        errors_b: u64,
        duration_b_secs: f64,
        breakdown_a: &[(String, u64)],
        breakdown_b: &[(String, u64)],
    ) -> DatasetComparisonSummary {
        let dur_a = duration_a_secs.max(1.0);
        let dur_b = duration_b_secs.max(1.0);

        let rate_a = (records_a as f64) / dur_a;
        let rate_b = (records_b as f64) / dur_b;

        let err_rate_a = if records_a == 0 {
            0.0
        } else {
            (errors_a as f64) / (records_a as f64)
        };
        let err_rate_b = if records_b == 0 {
            0.0
        } else {
            (errors_b as f64) / (records_b as f64)
        };

        // Merge key breakdowns
        use std::collections::BTreeMap;
        let mut key_map: BTreeMap<String, (u64, u64)> = BTreeMap::new();
        for (k, c) in breakdown_a {
            key_map.entry(k.clone()).or_insert((0, 0)).0 += *c;
        }
        for (k, c) in breakdown_b {
            key_map.entry(k.clone()).or_insert((0, 0)).1 += *c;
        }

        let mut keys = Vec::new();
        for (k, (ca, cb)) in key_map {
            let k_rate_a = (ca as f64) / dur_a;
            let k_rate_b = (cb as f64) / dur_b;

            let diff_str = if ca == 0 && cb > 0 {
                format!("+{} (new)", cb)
            } else if ca > 0 && cb == 0 {
                format!("-{} (removed)", ca)
            } else if ca == 0 && cb == 0 {
                "0 (no change)".to_string()
            } else {
                let change_pct = ((k_rate_b - k_rate_a) / k_rate_a) * 100.0;
                if change_pct >= 0.0 {
                    format!("+{:.1}%", change_pct)
                } else {
                    format!("{:.1}%", change_pct)
                }
            };

            keys.push(KeyComparison {
                key: k,
                count_a: ca,
                count_b: cb,
                rate_a: k_rate_a,
                rate_b: k_rate_b,
                diff_str,
            });
        }

        DatasetComparisonSummary {
            name_a: name_a.to_string(),
            name_b: name_b.to_string(),
            records_a,
            records_b,
            duration_a_secs: dur_a,
            duration_b_secs: dur_b,
            rate_a,
            rate_b,
            errors_a,
            errors_b,
            error_rate_a: err_rate_a,
            error_rate_b: err_rate_b,
            keys,
        }
    }
}
