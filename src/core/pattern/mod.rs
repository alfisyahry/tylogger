pub mod tokenizer;

use std::collections::HashMap;
pub use tokenizer::DynamicTokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternTemplate {
    pub template: String,
    pub count: u64,
    pub first_seen_line: usize,
    pub last_seen_line: usize,
    pub sample_message: String,
}

pub struct PatternGrouper {
    templates: HashMap<String, (u64, usize, usize, String)>, // (count, first, last, sample)
}

impl Default for PatternGrouper {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternGrouper {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn record_line(&mut self, line: &str, line_idx: usize) {
        let template = DynamicTokenizer::tokenize(line);
        let entry = self
            .templates
            .entry(template)
            .or_insert_with(|| (0, line_idx, line_idx, line.to_string()));

        entry.0 += 1;
        entry.2 = line_idx;
    }

    pub fn top_patterns(&self, limit: usize) -> Vec<PatternTemplate> {
        let mut list: Vec<PatternTemplate> = self
            .templates
            .iter()
            .map(|(tmpl, (count, first, last, sample))| PatternTemplate {
                template: tmpl.clone(),
                count: *count,
                first_seen_line: *first,
                last_seen_line: *last,
                sample_message: sample.clone(),
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.count));
        list.truncate(limit);
        list
    }
}
