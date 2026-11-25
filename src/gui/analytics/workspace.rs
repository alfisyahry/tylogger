use crate::core::analytics::engine::AggregationResult;
use crate::core::analytics::preset::{AnalyticsPreset, PresetManager};
use crate::core::analytics::query::AnalyticsQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsTab {
    Overview,
    HttpStatus,
    TopIps,
    Endpoints,
    Latency,
    PatternGroups,
    AnomalyDetection,
    SecurityPresets,
}

pub struct AnalyticsWorkspace {
    pub is_open: bool,
    pub active_tab: AnalyticsTab,
    pub current_query: AnalyticsQuery,
    pub last_result: Option<AggregationResult>,
    pub presets: Vec<AnalyticsPreset>,
}

impl Default for AnalyticsWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyticsWorkspace {
    pub fn new() -> Self {
        Self {
            is_open: false,
            active_tab: AnalyticsTab::Overview,
            current_query: AnalyticsQuery::default(),
            last_result: None,
            presets: PresetManager::builtin_presets(),
        }
    }

    pub fn set_active_tab(&mut self, tab: AnalyticsTab) {
        self.active_tab = tab;
    }

    pub fn set_result(&mut self, res: AggregationResult) {
        self.last_result = Some(res);
    }

    pub fn drill_down_filter(&self, row_idx: usize) -> Option<String> {
        if let Some(res) = &self.last_result {
            if let Some(row) = res.rows.get(row_idx) {
                return Some(row.key.clone());
            }
        }
        None
    }
}
