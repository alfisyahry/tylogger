use crate::core::compare::normalizer::{ComparisonEngine, DatasetComparisonSummary};

pub struct CompareDialog {
    pub source_a_name: String,
    pub source_b_name: String,
    pub duration_a_secs: f64,
    pub duration_b_secs: f64,
}

impl CompareDialog {
    pub fn new(source_a: impl Into<String>, source_b: impl Into<String>) -> Self {
        Self {
            source_a_name: source_a.into(),
            source_b_name: source_b.into(),
            duration_a_secs: 3600.0,
            duration_b_secs: 3600.0,
        }
    }

    pub fn execute_compare(
        &self,
        records_a: u64,
        errors_a: u64,
        records_b: u64,
        errors_b: u64,
        keys_a: &[(String, u64)],
        keys_b: &[(String, u64)],
    ) -> DatasetComparisonSummary {
        ComparisonEngine::compare(
            &self.source_a_name,
            records_a,
            errors_a,
            self.duration_a_secs,
            &self.source_b_name,
            records_b,
            errors_b,
            self.duration_b_secs,
            keys_a,
            keys_b,
        )
    }
}
