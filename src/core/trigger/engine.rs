// src/core/trigger/engine.rs
//
// Keyword Trigger & Action Automation Engine for Tylogger
// Incoming-only real-time evaluation, zero historical action spam, deduplication, cooldown.

use super::action::TriggerAction;
use super::deduplication::GlobalTriggerDeduplicator;
use super::history::BoundedActionHistory;
use super::queue::{BoundedActionQueue, QueuedAction};
use crate::core::search::{SearchEngine, SearchOptions};

#[derive(Debug, Clone)]
pub struct KeywordTriggerRule {
    pub id: u32,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub is_enabled: bool,
    pub actions: Vec<TriggerAction>,
    pub cooldown_ms: u64,
    pub last_executed_ms: u64,
}

impl KeywordTriggerRule {
    pub fn new(id: u32, name: &str, pattern: &str, is_regex: bool, case_sensitive: bool) -> Self {
        Self {
            id,
            name: name.to_string(),
            pattern: pattern.to_string(),
            is_regex,
            case_sensitive,
            is_enabled: true,
            actions: Vec::new(),
            cooldown_ms: 500, // 500ms default cooldown
            last_executed_ms: 0,
        }
    }

    pub fn matches(&self, line: &str) -> bool {
        if !self.is_enabled || self.pattern.is_empty() {
            return false;
        }

        let opt = if self.is_regex {
            SearchOptions::new_regex(self.case_sensitive)
        } else {
            SearchOptions::new_plain(self.case_sensitive, false)
        };

        SearchEngine::matches_line(line, &self.pattern, &opt)
    }
}

pub struct TriggerEngine {
    pub rules: Vec<KeywordTriggerRule>,
    pub dedup: GlobalTriggerDeduplicator,
    pub queue: BoundedActionQueue,
    pub history: BoundedActionHistory,
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            dedup: GlobalTriggerDeduplicator::new(),
            queue: BoundedActionQueue::default(),
            history: BoundedActionHistory::default(),
        }
    }

    pub fn add_rule(&mut self, rule: KeywordTriggerRule) {
        self.rules.push(rule);
    }

    /// Evaluate line against trigger rules.
    /// Strictly incoming-only: if `is_incoming == false` (e.g. initial file load or session restore),
    /// no external actions are executed or enqueued (zero historical spam guarantee).
    pub fn evaluate_line(
        &mut self,
        line: &str,
        source_id: u32,
        physical_line_number: u64,
        is_incoming: bool,
        current_time_ms: u64,
    ) -> Vec<TriggerAction> {
        if !is_incoming {
            return Vec::new();
        }

        let mut triggered_actions = Vec::new();

        for rule in &mut self.rules {
            if !rule.is_enabled {
                continue;
            }

            // Check cooldown
            if current_time_ms < rule.last_executed_ms.saturating_add(rule.cooldown_ms) {
                continue;
            }

            if rule.matches(line) {
                // Check cross-view deduplication
                if self
                    .dedup
                    .try_claim(source_id, physical_line_number, rule.id)
                {
                    rule.last_executed_ms = current_time_ms;

                    for act in &rule.actions {
                        triggered_actions.push(act.clone());

                        // Enqueue to bounded processing queue
                        self.queue.enqueue(QueuedAction {
                            rule_id: rule.id,
                            source_id,
                            line_number: physical_line_number,
                            action: act.clone(),
                            timestamp_ms: current_time_ms,
                        });

                        // Record audit history
                        self.history.record(
                            current_time_ms,
                            rule.id,
                            &rule.name,
                            source_id,
                            physical_line_number,
                            action_name(act),
                            "Dispatched",
                            10,
                        );
                    }
                }
            }
        }

        triggered_actions
    }

    /// Dry-run simulation against sample lines without side-effects or persistent execution
    pub fn simulate(&self, sample_lines: &[&str]) -> Vec<(usize, u32, String)> {
        let mut sim_results = Vec::new();
        for (idx, line) in sample_lines.iter().enumerate() {
            for rule in &self.rules {
                if rule.matches(line) {
                    sim_results.push((idx, rule.id, rule.name.clone()));
                }
            }
        }
        sim_results
    }
}

fn action_name(act: &TriggerAction) -> &'static str {
    match act {
        TriggerAction::SetBookmark { .. } => "SetBookmark",
        TriggerAction::StopFollowTail => "StopFollowTail",
        TriggerAction::AudioAlert { .. } => "AudioAlert",
        TriggerAction::DesktopNotification { .. } => "DesktopNotification",
        TriggerAction::PluginKeywordAction { .. } => "PluginKeywordAction",
        TriggerAction::IncrementCounter { .. } => "IncrementCounter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_only_execution_and_simulation() {
        let mut engine = TriggerEngine::new();
        let mut r = KeywordTriggerRule::new(1, "CriticalError", "FATAL|PANIC", true, true);
        r.actions.push(TriggerAction::StopFollowTail);
        r.actions.push(TriggerAction::SetBookmark {
            label: Some("Alert".to_string()),
            comment: None,
        });
        engine.add_rule(r);

        let historical_line = "2026-11-25 08:00:00 [PANIC] System crashed";

        // Historical evaluation on file open -> returns zero actions
        let hist_acts = engine.evaluate_line(historical_line, 1, 5, false, 1000);
        assert!(hist_acts.is_empty());
        assert_eq!(engine.history.entries().len(), 0);

        // Simulation mode -> identifies match without executing actions
        let sim = engine.simulate(&[historical_line]);
        assert_eq!(sim.len(), 1);
        assert_eq!(sim[0].1, 1); // Rule ID 1

        // Real incoming line during tailing -> executes actions once
        let live_acts = engine.evaluate_line(historical_line, 1, 5, true, 2000);
        assert_eq!(live_acts.len(), 2);
        assert_eq!(engine.history.entries().len(), 2);
    }
}
