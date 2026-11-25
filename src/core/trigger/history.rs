// src/core/trigger/history.rs
//
// Bounded Action History & Audit Log for Tylogger
// Retains essential trigger metadata (ID, line, status, duration) up to 10,000 entries.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionHistoryEntry {
    pub timestamp_ms: u64,
    pub rule_id: u32,
    pub rule_name: String,
    pub source_id: u32,
    pub physical_line_number: u64,
    pub action_name: String,
    pub status: String,
    pub duration_us: u64,
}

#[derive(Debug, Clone)]
pub struct BoundedActionHistory {
    entries: Vec<ActionHistoryEntry>,
    max_capacity: usize,
}

impl Default for BoundedActionHistory {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl BoundedActionHistory {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_capacity: max_capacity.max(1),
        }
    }

    pub fn record(
        &mut self,
        timestamp_ms: u64,
        rule_id: u32,
        rule_name: &str,
        source_id: u32,
        physical_line_number: u64,
        action_name: &str,
        status: &str,
        duration_us: u64,
    ) {
        if self.entries.len() >= self.max_capacity {
            self.entries.remove(0); // Evict oldest
        }

        self.entries.push(ActionHistoryEntry {
            timestamp_ms,
            rule_id,
            rule_name: rule_name.to_string(),
            source_id,
            physical_line_number,
            action_name: action_name.to_string(),
            status: status.to_string(),
            duration_us,
        });
    }

    pub fn entries(&self) -> &[ActionHistoryEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn export_csv(&self) -> String {
        let mut out =
            String::from("TimestampMs,RuleId,RuleName,SourceId,Line,Action,Status,DurationUs\r\n");
        for e in &self.entries {
            out.push_str(&format!(
                "{},{},\"{}\",{},{},\"{}\",\"{}\",{}\r\n",
                e.timestamp_ms,
                e.rule_id,
                e.rule_name.replace('"', "\"\""),
                e.source_id,
                e.physical_line_number,
                e.action_name,
                e.status,
                e.duration_us
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_history_capacity() {
        let mut history = BoundedActionHistory::new(2);
        history.record(100, 1, "R1", 1, 10, "Bookmark", "OK", 50);
        history.record(200, 1, "R1", 1, 20, "StopTail", "OK", 60);
        assert_eq!(history.entries().len(), 2);

        history.record(300, 2, "R2", 1, 30, "Audio", "OK", 70);
        assert_eq!(history.entries().len(), 2);
        assert_eq!(history.entries()[0].physical_line_number, 20); // Oldest evicted
    }
}
